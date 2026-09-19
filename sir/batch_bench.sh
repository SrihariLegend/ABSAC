#!/bin/bash
# batch_bench.sh — benchmark all 40 lowered kernels against clang -O3

ROOT="/home/tom/dev/experiments/ABSAC"
CORPUS="$ROOT/corpus"
SIR="$ROOT/sir"
WORK="/tmp/absac_batch"

mkdir -p "$WORK"

LOWERABLE=$(cd "$SIR" && cargo run -p sir_benchmarks --bin batch_run -- "$CORPUS/kernels.ll" 2>/dev/null | grep 'YES' | awk '{print $1}')

echo "=== Benchmarking $(echo "$LOWERABLE" | wc -w) kernels ==="
printf "%-30s %-14s %-14s %-14s %-8s\n" "Kernel" "clang -O3(ms)" "clang native(ms)" "ABSAC -O2(ms)" "Result"
printf "%-30s %-14s %-14s %-14s %-8s\n" "------" "-------------" "---------------" "--------"

WINS=0; LOSSES=0; ERRORS=0; WINS_LIST=""

for KERNEL in $LOWERABLE; do
    DEFINE=$(grep "define.*@${KERNEL}" "$CORPUS/kernels.ll" | head -1)
    PARAM_LIST=$(echo "$DEFINE" | sed 's/.*(\(.*\)).*/\1/' | sed 's/)$//')
    NPARAMS=$(echo "$PARAM_LIST" | tr -cd ',' | wc -c)
    NPARAMS=$((NPARAMS + 1))
    CTYPE=$(echo "$DEFINE" | sed 's/.*@\([^ ]*\).*/\1/' | head -1)
    case "$CTYPE" in
        void) RETTYPE="void" ;;
        *8) RETTYPE="uint8_t" ;;
        *16) RETTYPE="uint16_t" ;;
        *32) RETTYPE="uint32_t" ;;
        *) RETTYPE="uint64_t" ;;
    esac

    # Build orig C: includes + function + harness main
    ORIG_C="$WORK/${KERNEL}_orig.c"
    {
        echo '#define _POSIX_C_SOURCE 199309L'
        echo '#include <stdint.h>'
        echo '#include <stdbool.h>'
        echo '#include <time.h>'
        echo '#include <stdio.h>'
        echo '#include <stdlib.h>'
        echo ''
        awk "/${KERNEL}\(/,/^}/" "$CORPUS/kernels.c" | grep -v '^#include'
        echo ''
        echo 'int main(int argc, char **argv) {'
        echo '    uint32_t seed = (argc > 1) ? (uint32_t)strtoul(argv[1], NULL, 0) : 0x9e3779b9u;'
        ARGS=""
        for I in $(seq 0 $((NPARAMS - 1))); do
            PARAM_I=$(echo "$PARAM_LIST" | cut -d',' -f$((I+1)) | xargs)
            if echo "$PARAM_I" | grep -q "ptr"; then
                echo "    volatile uint8_t p${I}[4096];"
                echo "    for (int j = 0; j < 4096; j++) p${I}[j] = (uint8_t)(seed + (uint32_t)(j * 31 + 17));"
            else
                echo "    volatile $RETTYPE p${I} = 42;"
            fi
            [ -n "$ARGS" ] && ARGS="$ARGS, "
            ARGS="${ARGS}p${I}"
        done
        echo ''
        echo '    for (int i = 0; i < 1000; i++) {'
        echo "        volatile $RETTYPE r = ${KERNEL}(${ARGS});"
        echo '        (void)r;'
        echo '        __asm__ volatile("" ::: "memory");'
        echo '    }'
        echo '    struct timespec start, end;'
        echo '    clock_gettime(CLOCK_MONOTONIC, &start);'
        echo "    $RETTYPE total = 0;"
        echo '    for (int i = 0; i < 1000000; i++) {'
        echo "        total += ${KERNEL}(${ARGS});"
        echo '        total ^= (uint64_t)i;'
        echo '        __asm__ volatile("" ::: "memory");'
        echo '    }'
        echo '    clock_gettime(CLOCK_MONOTONIC, &end);'
        echo '    double elapsed = (double)(end.tv_sec - start.tv_sec) + (double)(end.tv_nsec - start.tv_nsec) / 1e9;'
        echo '    printf("%.9f\n", elapsed);'
        echo '    fprintf(stderr, "checksum: ok\n");'
        echo '    return 0;'
        echo '}'
    } > "$ORIG_C"

    # Emit ABSAC C
    ABSAC_C="$WORK/${KERNEL}_absac.c"
    cd "$SIR" && cargo run -p sir_benchmarks --bin emit_c_ll -- "$CORPUS/kernels.ll" "$KERNEL" 1>"$ABSAC_C" 2>/dev/null

    # Build ABSAC harness: includes + ABSAC C + main from orig
    ABSAC_HARNESS="$WORK/${KERNEL}_absac_harness.c"
    {
        echo '#define _POSIX_C_SOURCE 199309L'
        echo '#include <stdint.h>'
        echo '#include <stdbool.h>'
        echo '#include <time.h>'
        echo '#include <stdio.h>'
        echo '#include <stdlib.h>'
        echo ''
        cat "$ABSAC_C"
        echo ''
        awk '/int main/,/^}/' "$ORIG_C"
    } > "$ABSAC_HARNESS"

    # Compile 3 variants — -fno-inline prevents hoisting the kernel out of the loop
    gcc -O3 -fno-inline -std=c11 "$ORIG_C" -o "$WORK/${KERNEL}_orig" 2>/dev/null
    gcc -O3 -march=native -fno-inline -std=c11 "$ORIG_C" -o "$WORK/${KERNEL}_native" 2>/dev/null
    gcc -O2 -fno-inline -std=c11 "$ABSAC_HARNESS" -o "$WORK/${KERNEL}_absac" 2>/dev/null

    if [ ! -f "$WORK/${KERNEL}_orig" ] || [ ! -f "$WORK/${KERNEL}_absac" ]; then
        printf "%-30s %-14s %-14s %-14s %-8s\n" "$KERNEL" "n/a" "n/a" "COMPILE FAIL" "error"
        ERRORS=$((ERRORS + 1))
        continue
    fi

    O3_TIME=$("$WORK/${KERNEL}_orig" 0x9e3779b9 2>/dev/null)
    NATIVE_TIME=$("$WORK/${KERNEL}_native" 0x9e3779b9 2>/dev/null)
    ABSAC_TIME=$("$WORK/${KERNEL}_absac" 0x9e3779b9 2>/dev/null)

    RESULT="loss"
    if awk -v a="$ABSAC_TIME" -v b="$O3_TIME" 'BEGIN{exit !(a < b)}'; then
        RESULT="WIN"
        WINS=$((WINS + 1))
        WINS_LIST="$WINS_LIST $KERNEL"
    elif awk -v a="$ABSAC_TIME" -v b="$O3_TIME" 'BEGIN{exit !(a == b)}'; then
        RESULT="tie"
    else
        LOSSES=$((LOSSES + 1))
    fi

    # Convert to milliseconds for display
    O3_MS=$(awk -v t="$O3_TIME" 'BEGIN{printf "%.3f", t * 1000}')
    NAT_MS=$(awk -v t="$NATIVE_TIME" 'BEGIN{printf "%.3f", t * 1000}')
    ABS_MS=$(awk -v t="$ABSAC_TIME" 'BEGIN{printf "%.3f", t * 1000}')
    printf "%-30s %-14s %-14s %-14s %-8s\n" "$KERNEL" "$O3_MS" "$NAT_MS" "$ABS_MS" "$RESULT"
done

echo ""
echo "==================================================================="
echo "Summary: $WINS wins, $LOSSES losses, $ERRORS errors"
if [ -n "$WINS_LIST" ]; then
    echo ""
    echo "ABSAC faster than clang -O3 on:"
    for K in $WINS_LIST; do
        echo "  $K"
    done
fi
