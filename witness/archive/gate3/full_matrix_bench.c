#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sched.h>
#include <immintrin.h>

// Include witness versions (define _orig, _sse2, _avx2, etc.)
#include "witness_k18.h"
#include "witness_k43.h"
#include "witness_k50.h"

// Include ABSAC vectorized versions (strip their includes)
#include "k18_all_equal_absac.c"
#include "k43_sum_ascii_absac.c"
#include "k50_count_masked_absac.c"

static volatile uint64_t g_sink = 0;

static inline double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

#define NTRIALS 25
#define ITERS   20000

// Run one measurement and return median ns
static double measure_k18(const uint8_t *buf, uint64_t n, uint8_t val, int use_absac) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            if (use_absac)
                g_sink ^= k18_all_equal(buf + offset, n, val);
            else
                g_sink ^= k18_all_equal_orig(buf + offset, n, val);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    // Sort and return median
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

static double measure_k18_witness(const uint8_t *buf, uint64_t n, uint8_t val) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            g_sink ^= k18_all_equal_avx2(buf + offset, n, val);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

static double measure_k43(const uint8_t *buf, uint64_t n, int use_absac) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            if (use_absac)
                g_sink ^= k43_sum_ascii(buf + offset, n);
            else
                g_sink ^= k43_sum_ascii_orig(buf + offset, n);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

static double measure_k43_witness(const uint8_t *buf, uint64_t n) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            g_sink ^= k43_sum_ascii_avx2(buf + offset, n);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

static double measure_k50(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target, int use_absac) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            if (use_absac)
                g_sink ^= k50_count_masked(buf + offset, n, mask, target);
            else
                g_sink ^= k50_count_masked_orig(buf + offset, n, mask, target);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

static double measure_k50_witness(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    volatile uint64_t offset = 0;
    double times[NTRIALS];
    for (int t = 0; t < NTRIALS; t++) {
        double t0 = now_sec();
        for (uint64_t i = 0; i < ITERS; i++) {
            offset = (offset + 1) & 0xFFF;
            g_sink ^= k50_count_masked_avx2(buf + offset, n, mask, target);
        }
        times[t] = (now_sec() - t0) / ITERS * 1e9;
    }
    for (int i = 0; i < NTRIALS - 1; i++)
        for (int j = i+1; j < NTRIALS; j++)
            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }
    return times[NTRIALS/2];
}

int main() {
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(0, &cpuset);
    sched_setaffinity(0, sizeof(cpuset), &cpuset);

    uint8_t *buf = aligned_alloc(64, 1048576);

    uint8_t val = 0x42;
    uint8_t mask = 0x0F;
    uint8_t target = 0x05;

    // CSV header: kernel, distribution, size, variant, median_ns
    printf("kernel,distribution,size,variant,median_ns\n");

    // ═══ k18: test both all_equal and random distributions ═══
    // all_equal distribution (no early exit)
    memset(buf, val, 1048576);
    for (int s = 0; s < (int)(sizeof(uint64_t)); s++) {} // no-op
    uint64_t sizes[] = {16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 65536};
    int nsizes = sizeof(sizes)/sizeof(sizes[0]);

    for (int si = 0; si < nsizes; si++) {
        uint64_t n = sizes[si];
        double t;
        t = measure_k18(buf, n, val, 0); // orig
        printf("k18_all_equal,all_equal,%lu,orig_O3,%.1f\n", n, t);
        t = measure_k18(buf, n, val, 1); // absac
        printf("k18_all_equal,all_equal,%lu,absac,%.1f\n", n, t);
        t = measure_k18_witness(buf, n, val); // witness
        printf("k18_all_equal,all_equal,%lu,witness_avx2,%.1f\n", n, t);
    }

    // random distribution (early exit scenario)
    srand(42);
    for (int i = 0; i < 1048576; i++) buf[i] = (uint8_t)(rand() & 0xFF);
    for (int si = 0; si < nsizes; si++) {
        uint64_t n = sizes[si];
        double t;
        t = measure_k18(buf, n, val, 0);
        printf("k18_all_equal,random,%lu,orig_O3,%.1f\n", n, t);
        t = measure_k18(buf, n, val, 1);
        printf("k18_all_equal,random,%lu,absac,%.1f\n", n, t);
        t = measure_k18_witness(buf, n, val);
        printf("k18_all_equal,random,%lu,witness_avx2,%.1f\n", n, t);
    }

    // ═══ k43: random distribution ═══
    for (int si = 0; si < nsizes; si++) {
        uint64_t n = sizes[si];
        double t;
        t = measure_k43(buf, n, 0);
        printf("k43_sum_ascii,random,%lu,orig_O3,%.1f\n", n, t);
        t = measure_k43(buf, n, 1);
        printf("k43_sum_ascii,random,%lu,absac,%.1f\n", n, t);
        t = measure_k43_witness(buf, n);
        printf("k43_sum_ascii,random,%lu,witness_avx2,%.1f\n", n, t);
    }

    // ═══ k50: half_match distribution ═══
    for (int si = 0; si < nsizes; si++) {
        uint64_t n = sizes[si];
        double t;
        t = measure_k50(buf, n, mask, target, 0);
        printf("k50_count_masked,half_match,%lu,orig_O3,%.1f\n", n, t);
        t = measure_k50(buf, n, mask, target, 1);
        printf("k50_count_masked,half_match,%lu,absac,%.1f\n", n, t);
        t = measure_k50_witness(buf, n, mask, target);
        printf("k50_count_masked,half_match,%lu,witness_avx2,%.1f\n", n, t);
    }

    free(buf);
    return 0;
}
