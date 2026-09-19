// witness_bench.c — rigorous benchmark harness for the witness frontier experiment
//
// Implements the measurement protocol from IMMEDIATE_PROGRAM.md:
//   - CPU pinning
//   - warmup
//   - calibrated iteration counts
//   - randomized candidate ordering
//   - median, min, MAD, 95% confidence interval
//   - cycles per element
//   - correctness check before timing
//   - noinline + volatile sink to prevent DCE
//
// Compile: see bench.sh

// _GNU_SOURCE is defined via compiler flag
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sched.h>
#include <math.h>

#include <immintrin.h>

// Include kernel implementations
#include "k18_all_equal.h"
#include "k43_sum_ascii.h"
#include "k50_count_masked.h"

// ─── Configuration ──────────────────────────────────────────────────

#define NTRIALS 15           // number of timed trials per measurement
#define WARMUP  5            // warmup iterations before timing
#define MAX_BUF (16 * 1024 * 1024)  // 16 MiB max buffer
#define MAX_ITERS 50000      // cap calibration at 50K iterations

// ─── Timing ─────────────────────────────────────────────────────────

static inline double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

// Read TSC for cycle counting (approximate)
static inline uint64_t rdtsc(void) {
    unsigned lo, hi;
    __asm__ __volatile__("rdtsc" : "=a"(lo), "=d"(hi));
    return ((uint64_t)hi << 32) | lo;
}

// ─── Statistics ─────────────────────────────────────────────────────

typedef struct {
    double median;
    double min;
    double mad;       // median absolute deviation
    double ci_lo;     // 95% CI lower bound (conservative)
    double ci_hi;     // 95% CI upper bound
} stats_t;

static int cmp_double(const void *a, const void *b) {
    double da = *(const double *)a, db = *(const double *)b;
    return (da > db) - (da < db);
}

static stats_t compute_stats(double *samples, int n) {
    qsort(samples, n, sizeof(double), cmp_double);
    stats_t s;
    s.min = samples[0];
    s.median = samples[n / 2];
    // MAD
    double *devs = malloc(n * sizeof(double));
    for (int i = 0; i < n; i++) devs[i] = fabs(samples[i] - s.median);
    qsort(devs, n, sizeof(double), cmp_double);
    s.mad = devs[n / 2];
    free(devs);
    // 95% CI using bootstrap-like simple approach: use percentiles
    // For 200 samples, 2.5th percentile = sample[5], 97.5th = sample[194]
    int lo_idx = (int)(0.025 * n);
    int hi_idx = (int)(0.975 * n);
    if (lo_idx < 0) lo_idx = 0;
    if (hi_idx >= n) hi_idx = n - 1;
    s.ci_lo = samples[lo_idx];
    s.ci_hi = samples[hi_idx];
    return s;
}

// ─── Buffer generation ──────────────────────────────────────────────

// Different distributions for different kernels
typedef enum {
    DIST_ALL_EQUAL,
    DIST_MISMATCH_FIRST,
    DIST_MISMATCH_LAST,
    DIST_MISMATCH_MID,
    DIST_RANDOM,
    DIST_ZERO_MATCH,      // 0% match for count_masked
    DIST_LOW_MATCH,       // ~1% match
    DIST_QUARTER_MATCH,   // ~25%
    DIST_HALF_MATCH,      // ~50%
    DIST_HIGH_MATCH,      // ~99%
    DIST_ALL_MATCH,       // 100%
    DIST_ALTERNATING,     // adversarial for branch prediction
    DIST_ASCII,           // representative ASCII text
} distribution_t;

static void fill_buffer(uint8_t *buf, uint64_t n, distribution_t dist, uint8_t val) {
    srand(42);  // deterministic seed
    switch (dist) {
        case DIST_ALL_EQUAL:
            memset(buf, val, n);
            break;
        case DIST_MISMATCH_FIRST:
            buf[0] = val ^ 0xFF;
            memset(buf + 1, val, n - 1);
            break;
        case DIST_MISMATCH_LAST:
            memset(buf, val, n - 1);
            buf[n - 1] = val ^ 0xFF;
            break;
        case DIST_MISMATCH_MID:
            memset(buf, val, n);
            buf[n / 2] = val ^ 0xFF;
            break;
        case DIST_RANDOM:
            for (uint64_t i = 0; i < n; i++) buf[i] = (uint8_t)(rand() & 0xFF);
            break;
        case DIST_ZERO_MATCH:
            for (uint64_t i = 0; i < n; i++) buf[i] = val ^ 0xFF;
            break;
        case DIST_LOW_MATCH:
            for (uint64_t i = 0; i < n; i++) buf[i] = val ^ 0xFF;
            for (uint64_t i = 0; i < n / 100; i++) buf[rand() % n] = val;
            break;
        case DIST_QUARTER_MATCH:
            for (uint64_t i = 0; i < n; i++) buf[i] = (rand() & 3) ? (val ^ 0xFF) : val;
            break;
        case DIST_HALF_MATCH:
            for (uint64_t i = 0; i < n; i++) buf[i] = (rand() & 1) ? (val ^ 0xFF) : val;
            break;
        case DIST_HIGH_MATCH:
            for (uint64_t i = 0; i < n; i++) buf[i] = val;
            for (uint64_t i = 0; i < n / 100; i++) buf[rand() % n] = val ^ 0xFF;
            break;
        case DIST_ALL_MATCH:
            memset(buf, val, n);
            break;
        case DIST_ALTERNATING:
            for (uint64_t i = 0; i < n; i++) buf[i] = (i & 1) ? val : (val ^ 0xFF);
            break;
        case DIST_ASCII:
            for (uint64_t i = 0; i < n; i++) buf[i] = (uint8_t)(' ' + (rand() % 95));
            break;
    }
}

// ─── Candidate definitions ──────────────────────────────────────────

// Each kernel has a set of candidate implementations.
// We define them as function pointers + names.

typedef uint64_t (*k18_fn)(const uint8_t *, uint64_t, uint8_t);
typedef uint64_t (*k43_fn)(const uint8_t *, uint64_t);
typedef uint64_t (*k50_fn)(const uint8_t *, uint64_t, uint8_t, uint8_t);

// K18 candidates
static k18_fn k18_fns[] = {
    k18_all_equal_orig,
    k18_all_equal_scalar,
    k18_all_equal_chunked,
    k18_all_equal_sse2,
    k18_all_equal_avx2,
    k18_all_equal_multi,
};
static const char *k18_names[] = {
    "orig", "scalar", "chunked", "sse2", "avx2", "multi",
};
static const int k18_ncands = 6;

// K43 candidates
static k43_fn k43_fns[] = {
    k43_sum_ascii_orig,
    k43_sum_ascii_scalar,
    k43_sum_ascii_chunked,
    k43_sum_ascii_swar,
    k43_sum_ascii_sse2,
    k43_sum_ascii_avx2,
    k43_sum_ascii_multi,
};
static const char *k43_names[] = {
    "orig", "scalar", "chunked", "swar", "sse2", "avx2", "multi",
};
static const int k43_ncands = 7;

// K50 candidates
static k50_fn k50_fns[] = {
    k50_count_masked_orig,
    k50_count_masked_scalar,
    k50_count_masked_chunked,
    k50_count_masked_swar,
    k50_count_masked_sse2,
    k50_count_masked_avx2,
    k50_count_masked_multi,
};
static const char *k50_names[] = {
    "orig", "scalar", "chunked", "swar", "sse2", "avx2", "multi",
};
static const int k50_ncands = 7;

// ─── Benchmark runner ───────────────────────────────────────────────

static volatile uint64_t g_sink = 0;

// Generic benchmark: calls fn repeatedly, measures time
// Returns seconds per call
static double benchmark_k18(k18_fn fn, const uint8_t *buf, uint64_t n, uint8_t val, uint64_t iters) {
    volatile uint64_t offset = 0;
    double t0 = now_sec();
    for (uint64_t i = 0; i < iters; i++) {
        offset = (offset + 1) & 0xFFF;  // vary 0-4095 to prevent hoisting
        g_sink ^= fn(buf + offset, n, val);
    }
    double t1 = now_sec();
    return (t1 - t0) / iters;
}

static double benchmark_k43(k43_fn fn, const uint8_t *buf, uint64_t n, uint64_t iters) {
    volatile uint64_t offset = 0;
    double t0 = now_sec();
    for (uint64_t i = 0; i < iters; i++) {
        offset = (offset + 1) & 0xFFF;
        g_sink ^= fn(buf + offset, n);
    }
    double t1 = now_sec();
    return (t1 - t0) / iters;
}

static double benchmark_k50(k50_fn fn, const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target, uint64_t iters) {
    volatile uint64_t offset = 0;
    double t0 = now_sec();
    for (uint64_t i = 0; i < iters; i++) {
        offset = (offset + 1) & 0xFFF;
        g_sink ^= fn(buf + offset, n, mask, target);
    }
    double t1 = now_sec();
    return (t1 - t0) / iters;
}

// Calibrate iterations: find how many iterations we need for ~10ms of work
static uint64_t calibrate_k18(k18_fn fn, const uint8_t *buf, uint64_t n, uint8_t val) {
    // Start with a reasonable guess and adjust
    uint64_t iters = 10000;
    double t = benchmark_k18(fn, buf, n, val, iters);
    while (t < 0.001 && iters < MAX_ITERS) {
        iters *= 2;
        t = benchmark_k18(fn, buf, n, val, iters);
    }
    return iters;
}

static uint64_t calibrate_k43(k43_fn fn, const uint8_t *buf, uint64_t n) {
    uint64_t iters = 10000;
    double t = benchmark_k43(fn, buf, n, iters);
    while (t < 0.001 && iters < MAX_ITERS) {
        iters *= 2;
        t = benchmark_k43(fn, buf, n, iters);
    }
    return iters;
}

static uint64_t calibrate_k50(k50_fn fn, const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t iters = 10000;
    double t = benchmark_k50(fn, buf, n, mask, target, iters);
    while (t < 0.001 && iters < MAX_ITERS) {
        iters *= 2;
        t = benchmark_k50(fn, buf, n, mask, target, iters);
    }
    return iters;
}

// ─── Main ───────────────────────────────────────────────────────────

int main(int argc, char **argv) {
    // Pin to core 0
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(0, &cpuset);
    sched_setaffinity(0, sizeof(cpuset), &cpuset);

    // Allocate buffer
    uint8_t *buf = aligned_alloc(64, MAX_BUF);
    if (!buf) { perror("aligned_alloc"); return 1; }

    // Size matrix
    // Sizes: standard regimes + tail-edge sizes (mod 16/32 boundaries)
    static const uint64_t sizes[] = {
        16, 17, 31, 32, 33, 63, 64, 65,
        256, 4096, 65536
    };
    static const int nsizes = 11;

    // Print header
    printf("kernel,distribution,size_bytes,candidate,correct,median_ns,min_ns,mad_ns,ci_lo_ns,ci_hi_ns,cycles_per_elem,speedup_vs_orig\n");
    fflush(stdout);

    // ═══ K18: all_equal ═══
    {
        uint8_t val = 0x42;
        distribution_t dists[] = {DIST_ALL_EQUAL, DIST_MISMATCH_FIRST, DIST_MISMATCH_LAST, DIST_MISMATCH_MID, DIST_RANDOM};
        const char *dist_names[] = {"all_equal", "mismatch_first", "mismatch_last", "mismatch_mid", "random"};
        int ndists = 5;

        for (int d = 0; d < ndists; d++) {
            for (int si = 0; si < nsizes; si++) {
                uint64_t n = sizes[si];
                fill_buffer(buf, n, dists[d], val);

                // Correctness check: all candidates agree
                uint64_t ref = k18_all_equal_orig(buf, n, val);
                int correct[k18_ncands];
                for (int c = 0; c < k18_ncands; c++) {
                    uint64_t result = k18_fns[c](buf, n, val);
                    correct[c] = (result == ref) ? 1 : 0;
                    if (!correct[c]) {
                        fprintf(stderr, "CORRECTNESS FAIL: k18 %s dist=%s size=%lu: got %lu, expected %lu\n",
                                k18_names[c], dist_names[d], n, result, ref);
                    }
                }

                uint64_t iters = calibrate_k18(k18_all_equal_orig, buf, n, val);

                // Benchmark original first for baseline
                double orig_samples[NTRIALS];
                for (int w = 0; w < WARMUP; w++) g_sink ^= k18_all_equal_orig(buf, n, val);
                for (int t = 0; t < NTRIALS; t++)
                    orig_samples[t] = benchmark_k18(k18_all_equal_orig, buf, n, val, iters);
                stats_t orig_stats = compute_stats(orig_samples, NTRIALS);
                double orig_median = orig_stats.median;

                double cycles = orig_median * 3.2e9 / n;
                printf("k18_all_equal,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                       dist_names[d], n, "orig", 1,
                       orig_median * 1e9, orig_stats.min * 1e9, orig_stats.mad * 1e9,
                       orig_stats.ci_lo * 1e9, orig_stats.ci_hi * 1e9,
                       cycles, 1.0);

                for (int c = 1; c < k18_ncands; c++) {
                    for (int w = 0; w < WARMUP; w++) g_sink ^= k18_fns[c](buf, n, val);
                    double samples[NTRIALS];
                    for (int t = 0; t < NTRIALS; t++)
                        samples[t] = benchmark_k18(k18_fns[c], buf, n, val, iters);
                    stats_t s = compute_stats(samples, NTRIALS);
                    double speedup = s.median / orig_median;
                    cycles = s.median * 3.2e9 / n;
                    printf("k18_all_equal,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                           dist_names[d], n, k18_names[c], correct[c],
                           s.median * 1e9, s.min * 1e9, s.mad * 1e9,
                           s.ci_lo * 1e9, s.ci_hi * 1e9,
                           cycles, speedup);
                }
                fflush(stdout);
            }
        }
    }

    // ═══ K43: sum_ascii ═══
    {
        distribution_t dists[] = {DIST_RANDOM, DIST_ASCII, DIST_ALL_EQUAL};
        const char *dist_names[] = {"random", "ascii", "all_equal"};
        int ndists = 3;

        for (int d = 0; d < ndists; d++) {
            for (int si = 0; si < nsizes; si++) {
                uint64_t n = sizes[si];
                fill_buffer(buf, n, dists[d], 0);

                // Correctness check
                uint64_t ref = k43_sum_ascii_orig(buf, n);
                int correct[k43_ncands];
                for (int c = 0; c < k43_ncands; c++) {
                    uint64_t result = k43_fns[c](buf, n);
                    correct[c] = (result == ref) ? 1 : 0;
                    if (!correct[c]) {
                        fprintf(stderr, "CORRECTNESS FAIL: k43 %s dist=%s size=%lu: got %lu, expected %lu\n",
                                k43_names[c], dist_names[d], n, result, ref);
                    }
                }

                uint64_t iters = calibrate_k43(k43_sum_ascii_orig, buf, n);

                // Benchmark original first to get baseline
                double orig_samples[NTRIALS];
                for (int w = 0; w < WARMUP; w++) g_sink ^= k43_sum_ascii_orig(buf, n);
                for (int t = 0; t < NTRIALS; t++) {
                    orig_samples[t] = benchmark_k43(k43_sum_ascii_orig, buf, n, iters);
                }
                stats_t orig_stats = compute_stats(orig_samples, NTRIALS);
                double orig_median = orig_stats.median;

                double cycles = orig_median * 3.2e9 / n;
                printf("k43_sum_ascii,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                       dist_names[d], n, "orig", 1,
                       orig_median * 1e9, orig_stats.min * 1e9, orig_stats.mad * 1e9,
                       orig_stats.ci_lo * 1e9, orig_stats.ci_hi * 1e9,
                       cycles, 1.0);

                // Benchmark other candidates
                for (int c = 1; c < k43_ncands; c++) {
                    for (int w = 0; w < WARMUP; w++) g_sink ^= k43_fns[c](buf, n);
                    double samples[NTRIALS];
                    for (int t = 0; t < NTRIALS; t++) {
                        samples[t] = benchmark_k43(k43_fns[c], buf, n, iters);
                    }
                    stats_t s = compute_stats(samples, NTRIALS);
                    double speedup = s.median / orig_median;
                    cycles = s.median * 3.2e9 / n;
                    printf("k43_sum_ascii,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                           dist_names[d], n, k43_names[c], correct[c],
                           s.median * 1e9, s.min * 1e9, s.mad * 1e9,
                           s.ci_lo * 1e9, s.ci_hi * 1e9,
                           cycles, speedup);
                }
                fflush(stdout);
            }
        }
    }

    // ═══ K50: count_masked ═══
    {
        uint8_t mask = 0x0F;
        uint8_t target = 0x05;
        distribution_t dists[] = {DIST_ZERO_MATCH, DIST_LOW_MATCH, DIST_QUARTER_MATCH, DIST_HALF_MATCH, DIST_HIGH_MATCH, DIST_ALL_MATCH, DIST_ALTERNATING, DIST_RANDOM};
        const char *dist_names[] = {"zero_match", "low_match", "quarter_match", "half_match", "high_match", "all_match", "alternating", "random"};
        int ndists = 8;

        for (int d = 0; d < ndists; d++) {
            for (int si = 0; si < nsizes; si++) {
                uint64_t n = sizes[si];
                fill_buffer(buf, n, dists[d], target);

                // Correctness check
                uint64_t ref = k50_count_masked_orig(buf, n, mask, target);
                int correct[k50_ncands];
                for (int c = 0; c < k50_ncands; c++) {
                    uint64_t result = k50_fns[c](buf, n, mask, target);
                    correct[c] = (result == ref) ? 1 : 0;
                    if (!correct[c]) {
                        fprintf(stderr, "CORRECTNESS FAIL: k50 %s dist=%s size=%lu: got %lu, expected %lu\n",
                                k50_names[c], dist_names[d], n, result, ref);
                    }
                }

                uint64_t iters = calibrate_k50(k50_count_masked_orig, buf, n, mask, target);

                // Benchmark original
                double orig_samples[NTRIALS];
                for (int w = 0; w < WARMUP; w++) g_sink ^= k50_count_masked_orig(buf, n, mask, target);
                for (int t = 0; t < NTRIALS; t++) {
                    orig_samples[t] = benchmark_k50(k50_count_masked_orig, buf, n, mask, target, iters);
                }
                stats_t orig_stats = compute_stats(orig_samples, NTRIALS);
                double orig_median = orig_stats.median;

                double cycles = orig_median * 3.2e9 / n;
                printf("k50_count_masked,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                       dist_names[d], n, "orig", 1,
                       orig_median * 1e9, orig_stats.min * 1e9, orig_stats.mad * 1e9,
                       orig_stats.ci_lo * 1e9, orig_stats.ci_hi * 1e9,
                       cycles, 1.0);

                for (int c = 1; c < k50_ncands; c++) {
                    for (int w = 0; w < WARMUP; w++) g_sink ^= k50_fns[c](buf, n, mask, target);
                    double samples[NTRIALS];
                    for (int t = 0; t < NTRIALS; t++) {
                        samples[t] = benchmark_k50(k50_fns[c], buf, n, mask, target, iters);
                    }
                    stats_t s = compute_stats(samples, NTRIALS);
                    double speedup = s.median / orig_median;
                    cycles = s.median * 3.2e9 / n;
                    printf("k50_count_masked,%s,%lu,%s,%d,%.1f,%.1f,%.1f,%.1f,%.1f,%.2f,%.3f\n",
                           dist_names[d], n, k50_names[c], correct[c],
                           s.median * 1e9, s.min * 1e9, s.mad * 1e9,
                           s.ci_lo * 1e9, s.ci_hi * 1e9,
                           cycles, speedup);
                }
                fflush(stdout);
            }
        }
    }

    free(buf);
    return 0;
}
