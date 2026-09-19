#!/bin/bash
# archive_gate3.sh — freeze all Gate 3 evidence artifacts
set -e

ROOT="/home/tom/dev/experiments/ABSAC"
SIR="$ROOT/sir"
ARCH="$ROOT/witness/archive/gate3"
mkdir -p "$ARCH"

# ═══════════════════════════════════════════════════════════════
# 1. Environment metadata
# ═══════════════════════════════════════════════════════════════
cat > "$ARCH/ENVIRONMENT.txt" << EOF
=== Gate 3 Evidence Archive ===
Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Commit: $(cd "$ROOT" && git log --oneline -1)

=== Compilers ===
$(clang --version 2>&1)
$(gcc --version 2>&1)

=== CPU ===
$(cat /proc/cpuinfo | grep "model name" | head -1)
$(cat /proc/cpuinfo | grep "microcode" | head -1)
$(cat /proc/cpuinfo | grep "cpu MHz" | head -1)

=== CPU Features ===
$(cat /proc/cpuinfo | grep flags | head -1 | tr ' ' '\n' | grep -E "avx2|sse4_2|popcnt|bmi1|bmi2|avx512")

=== Cache ===
$(lscpu | grep -E "L1|L2|L3")

=== OS ===
$(uname -a)

=== Benchmark flags (all kernels) ===
Clang original:   clang -O3 -march=native -mavx2 -msse4.2 -mpopcnt
Clang ABSAC:      clang -O2 -march=native -mavx2 -msse4.2 -mpopcnt  (and -O3 for matrix)
Clang witness:    clang -O3 -march=native -mavx2 -msse4.2 -mpopcnt

=== Benchmark conditions ===
- CPU pinned to core 0 via sched_setaffinity
- 4096-byte buffer: fits in L1 (48KB per core) — HOT CACHE RESIDENT
- offset-varying: volatile offset = (offset+1)&0xFFF prevents hoisting
- g_sink: volatile uint64_t XOR accumulator prevents dead-code elimination
- 10000 iterations per measurement
- Buffer: aligned_alloc(64, 1048576), 1MB total
- Distributions: random (srand(42)), all_equal (memset)
EOF

echo "1. Environment saved"

# ═══════════════════════════════════════════════════════════════
# 2. Generated ABSAC source for all 3 kernels
# ═══════════════════════════════════════════════════════════════
for k in k18_all_equal k43_sum_ascii k50_count_masked; do
    (cd "$SIR" && cargo run -p sir_benchmarks --bin emit_vectorized -- "$ROOT/corpus/kernels.ll" $k 2>/dev/null) > "$ARCH/${k}_absac.c"
done
echo "2. ABSAC generated source saved"

# ═══════════════════════════════════════════════════════════════
# 3. LLVM input (source kernels + IR)
# ═══════════════════════════════════════════════════════════════
cp "$ROOT/corpus/kernels.c" "$ARCH/kernels_source.c"
# Extract just the 3 kernels' LLVM IR
for k in k18_all_equal k43_sum_ascii k50_count_masked; do
    awk "/^define.*$k/,/^}/" "$ROOT/corpus/kernels.ll" > "$ARCH/${k}.ll"
done
echo "3. LLVM input saved"

# ═══════════════════════════════════════════════════════════════
# 4. Assembly: original (-O3), ABSAC (-O2 and -O3), witness (-O3)
# ═══════════════════════════════════════════════════════════════
# Original kernels assembly
cat > /tmp/orig_kernels.c << 'ORIGC'
#include <stdint.h>
uint64_t k18_all_equal(const uint8_t *buf, uint64_t n, uint8_t val) {
    uint64_t all = 1;
    for (uint64_t i = 0; i < n; i++) all &= (buf[i] == val);
    return all;
}
uint64_t k43_sum_ascii(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}
uint64_t k50_count_masked(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++)
        count += ((buf[i] & mask) == target);
    return count;
}
ORIGC

clang -O3 -march=native -mavx2 -msse4.2 -mpopcnt -S /tmp/orig_kernels.c -o "$ARCH/orig_kernels_O3.s" 2>/dev/null
gcc -O3 -march=native -mavx2 -msse4.2 -mpopcnt -S /tmp/orig_kernels.c -o "$ARCH/orig_kernels_gcc_O3.s" 2>/dev/null

# ABSAC assembly at both -O2 and -O3
for k in k18_all_equal k43_sum_ascii k50_count_masked; do
    # Strip includes from ABSAC file, re-add minimal
    (echo '#include <stdint.h>' && echo '#include <immintrin.h>' && grep -v '#include' "$ARCH/${k}_absac.c") > /tmp/${k}_compile.c
    clang -O2 -march=native -mavx2 -msse4.2 -mpopcnt -S /tmp/${k}_compile.c -o "$ARCH/${k}_absac_O2.s" 2>/dev/null
    clang -O3 -march=native -mavx2 -msse4.2 -mpopcnt -S /tmp/${k}_compile.c -o "$ARCH/${k}_absac_O3.s" 2>/dev/null
done

# Witness assembly
cp "$ROOT/witness/k18_all_equal.h" "$ARCH/witness_k18.h"
cp "$ROOT/witness/k43_sum_ascii.h" "$ARCH/witness_k43.h"
cp "$ROOT/witness/k50_count_masked.h" "$ARCH/witness_k50.h"

echo "4. Assembly saved"

# ═══════════════════════════════════════════════════════════════
# 5. LLVM vectorization remarks (already captured, copy)
# ═══════════════════════════════════════════════════════════════
clang -O3 -march=native -mavx2 -Rpass=loop-vectorize -Rpass-missed=loop-vectorize -Rpass-analysis=loop-vectorize -c /tmp/orig_kernels.c -o /dev/null 2> "$ARCH/llvm_remarks_clang.txt"
gcc -O3 -march=native -mavx2 -fopt-info-vec -c /tmp/orig_kernels.c -o /dev/null 2> "$ARCH/llvm_remarks_gcc.txt"
echo "5. LLVM remarks saved"

# ═══════════════════════════════════════════════════════════════
# 6. Checksums
# ═══════════════════════════════════════════════════════════════
cd "$ARCH"
sha256sum *.c *.ll *.s *.h *.txt > SHA256SUMS 2>/dev/null
echo "6. Checksums saved"

echo ""
echo "=== Archive complete ==="
ls -la "$ARCH/"
echo ""
echo "Total files: $(ls "$ARCH" | wc -l)"
