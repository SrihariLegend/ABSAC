#!/bin/bash
# gate3_test.sh — verify ABSAC vectorized code is correct and approaches the witness frontier
#
# Compiles: original (clang -O3 -march=native), ABSAC (clang -O2 -march=native), hand-written witness
# Tests: differential correctness across sizes and distributions
# Benchmarks: at 4096 bytes (cache-resident sweet spot)

ROOT="/home/tom/dev/experiments/ABSAC"
SIR="$ROOT/sir"
WORK="/tmp/gate3"
mkdir -p "$WORK"

# Emit ABSAC vectorized C for all 3 kernels
for k in k18_all_equal k43_sum_ascii k50_count_masked; do
    (cd "$SIR" && cargo run -p sir_benchmarks --bin emit_vectorized -- "$ROOT/corpus/kernels.ll" $k 2>/dev/null) > "$WORK/${k}_absac.c"
done

# Create the test harness
cat > "$WORK/harness.c" << 'HARNESS'
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sched.h>
#include <immintrin.h>

// Include hand-written witness versions (these also define _orig variants)
#include "witness_k18.h"
#include "witness_k43.h"
#include "witness_k50.h"

// Include ABSAC vectorized versions
#include "k18_all_equal_absac.c"
#include "k43_sum_ascii_absac.c"
#include "k50_count_masked_absac.c"

static volatile uint64_t g_sink = 0;

static inline double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

int main() {
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(0, &cpuset);
    sched_setaffinity(0, sizeof(cpuset), &cpuset);

    srand(42);
    uint8_t *buf = aligned_alloc(64, 1048576);
    for (int i = 0; i < 1048576; i++) buf[i] = (uint8_t)(rand() & 0xFF);

    uint8_t val = 0x42;
    uint8_t mask = 0x0F;
    uint8_t target = 0x05;

    int errors = 0;

    // ═══ Correctness tests ═══
    printf("=== CORRECTNESS TESTS ===\n");
    for (uint64_t n = 0; n <= 128; n++) {
        // k18
        uint64_t r_orig = k18_all_equal_orig(buf, n, val);
        uint64_t r_absac = k18_all_equal(buf, n, val);
        uint64_t r_witness = k18_all_equal_avx2(buf, n, val);
        if (r_absac != r_orig) { printf("FAIL k18 n=%lu: absac=%lu orig=%lu\n", n, r_absac, r_orig); errors++; }
        if (r_witness != r_orig) { printf("FAIL k18 n=%lu: witness=%lu orig=%lu\n", n, r_witness, r_orig); errors++; }

        // k43
        r_orig = k43_sum_ascii_orig(buf, n);
        r_absac = k43_sum_ascii(buf, n);
        r_witness = k43_sum_ascii_avx2(buf, n);
        if (r_absac != r_orig) { printf("FAIL k43 n=%lu: absac=%lu orig=%lu\n", n, r_absac, r_orig); errors++; }
        if (r_witness != r_orig) { printf("FAIL k43 n=%lu: witness=%lu orig=%lu\n", n, r_witness, r_orig); errors++; }

        // k50
        r_orig = k50_count_masked_orig(buf, n, mask, target);
        r_absac = k50_count_masked(buf, n, mask, target);
        r_witness = k50_count_masked_avx2(buf, n, mask, target);
        if (r_absac != r_orig) { printf("FAIL k50 n=%lu: absac=%lu orig=%lu\n", n, r_absac, r_orig); errors++; }
        if (r_witness != r_orig) { printf("FAIL k50 n=%lu: witness=%lu orig=%lu\n", n, r_witness, r_orig); errors++; }
    }

    // Test with all-equal buffer
    memset(buf, val, 4096);
    for (uint64_t n = 0; n <= 128; n++) {
        uint64_t r_orig = k18_all_equal_orig(buf, n, val);
        uint64_t r_absac = k18_all_equal(buf, n, val);
        if (r_absac != r_orig) { printf("FAIL k18(all_equal) n=%lu: absac=%lu orig=%lu\n", n, r_absac, r_orig); errors++; }
    }

    // Restore random buffer
    srand(42);
    for (int i = 0; i < 1048576; i++) buf[i] = (uint8_t)(rand() & 0xFF);

    if (errors == 0) {
        printf("ALL CORRECTNESS TESTS PASSED\n");
    } else {
        printf("%d CORRECTNESS ERRORS\n", errors);
        return 1;
    }

    // ═══ Benchmark at 4096 bytes ═══
    printf("\n=== BENCHMARK (4096 bytes, cache-resident) ===\n");
    printf("%-25s %-12s %-12s %-12s %-12s %-12s\n", "Kernel", "orig-O3(ns)", "absac-O2(ns)", "witness(ns)", "absac/orig", "witness/orig");
    printf("%-25s %-12s %-12s %-12s %-12s %-12s\n", "------", "-----------", "-----------", "----------", "----------", "-----------");

    uint64_t n = 4096;
    uint64_t iters = 10000;
    volatile uint64_t offset = 0;

    // k18 — use all-equal buffer to prevent early exit
    {
        memset(buf, val, 1048576); // all bytes = val → no early exit
        double t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal_orig(buf+offset, n, val); }
        double t_orig = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal(buf+offset, n, val); }
        double t_absac = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal_avx2(buf+offset, n, val); }
        double t_witness = (now_sec() - t0) / iters * 1e9;

        printf("%-25s %-12.1f %-12.1f %-12.1f %-12.3f %-12.3f\n",
               "k18_all_equal", t_orig, t_absac, t_witness, t_absac/t_orig, t_witness/t_orig);

        // Also test with random buffer (early exit scenario)
        srand(42);
        for (int i = 0; i < 1048576; i++) buf[i] = (uint8_t)(rand() & 0xFF);

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal_orig(buf+offset, n, val); }
        t_orig = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal(buf+offset, n, val); }
        t_absac = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k18_all_equal_avx2(buf+offset, n, val); }
        t_witness = (now_sec() - t0) / iters * 1e9;

        printf("%-25s %-12.1f %-12.1f %-12.1f %-12.3f %-12.3f\n",
               "k18_all_equal(random)", t_orig, t_absac, t_witness, t_absac/t_orig, t_witness/t_orig);
    }

    // k43
    {
        double t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k43_sum_ascii_orig(buf+offset, n); }
        double t_orig = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k43_sum_ascii(buf+offset, n); }
        double t_absac = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k43_sum_ascii_avx2(buf+offset, n); }
        double t_witness = (now_sec() - t0) / iters * 1e9;

        printf("%-25s %-12.1f %-12.1f %-12.1f %-12.3f %-12.3f\n",
               "k43_sum_ascii", t_orig, t_absac, t_witness, t_absac/t_orig, t_witness/t_orig);
    }

    // k50
    {
        double t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k50_count_masked_orig(buf+offset, n, mask, target); }
        double t_orig = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k50_count_masked(buf+offset, n, mask, target); }
        double t_absac = (now_sec() - t0) / iters * 1e9;

        t0 = now_sec();
        for (uint64_t i = 0; i < iters; i++) { offset = (offset+1)&0xFFF; g_sink ^= k50_count_masked_avx2(buf+offset, n, mask, target); }
        double t_witness = (now_sec() - t0) / iters * 1e9;

        printf("%-25s %-12.1f %-12.1f %-12.1f %-12.3f %-12.3f\n",
               "k50_count_masked", t_orig, t_absac, t_witness, t_absac/t_orig, t_witness/t_orig);
    }

    printf("\n=== Gate 3 pass condition ===\n");
    printf("ABSAC-generated code should be within 10-15%% of the hand-written witness.\n");
    printf("Both should be significantly faster than the original at -O3.\n");

    free(buf);
    return 0;
}
HARNESS

# Create witness headers (copies from the witness frontier experiment)
cp "$ROOT/witness/k18_all_equal.h" "$WORK/witness_k18.h"
cp "$ROOT/witness/k43_sum_ascii.h" "$WORK/witness_k43.h"
cp "$ROOT/witness/k50_count_masked.h" "$WORK/witness_k50.h"

# Fix include names in ABSAC files (they have #include <immintrin.h> which we already have)
for f in "$WORK"/*_absac.c; do
    sed -i '/#include/d' "$f"
done

# Compile
echo "=== Compiling ==="
cd "$WORK"
clang -O3 -march=native -std=c11 -D_GNU_SOURCE -mavx2 -msse4.2 -mpopcnt harness.c -o harness -lm 2>&1
if [ $? -ne 0 ]; then
    echo "COMPILE FAILED"
    exit 1
fi

echo "=== Running ==="
./harness
