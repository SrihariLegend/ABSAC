// Library-implementation baselines for the kernels the optimizer targets.
//
// Measured with clang/gcc -O1 on the same host as the SIR timing harness.
// Each measurement times a tight loop around the implementation; the sink is
// volatile so calls cannot be elided. This is external-baseline evidence for
// P5, not part of the compilation pipeline.

#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#define BUF 4096
#define WORDS 4096

static uint8_t hay_early[BUF];
static uint8_t hay_nohit[BUF];
static uint64_t words[WORDS];

static uint64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static size_t lib_first_zero(const uint8_t *a, size_t n) {
    const uint8_t *p = (const uint8_t *)memchr(a, 0, n);
    return p ? (size_t)(p - a) : n;
}

static size_t naive_first_zero(const uint8_t *a, size_t n) {
    for (size_t i = 0; i < n; i++)
        if (a[i] == 0)
            return i;
    return n;
}

static unsigned lib_popcount(uint64_t x) {
    return (unsigned)__builtin_popcountll(x);
}

static unsigned naive_popcount(uint64_t x) {
    unsigned count = 0;
    while (x) {
        x &= x - 1;
        count++;
    }
    return count;
}

static unsigned lib_ctz(uint64_t x) {
    // The builtin is undefined at zero; `x|1` keeps the measurement defined
    // and does not change nonzero inputs.
    return (unsigned)__builtin_ctzll(x | 1);
}

static unsigned naive_ctz(uint64_t x) {
    unsigned count = 0;
    while (count < 64 && (x & 1) == 0) {
        x >>= 1;
        count++;
    }
    return count;
}

typedef size_t (*search_fn)(const uint8_t *, size_t);
typedef unsigned (*word_fn)(uint64_t);

static void bench_search(const char *name, search_fn fn, const uint8_t *buf,
                         size_t n, uint64_t reps) {
    volatile uint64_t sink = 0;
    uint64_t start = now_ns();
    for (uint64_t i = 0; i < reps; i++) {
        __asm__ __volatile__("" ::: "memory");
        sink += (uint64_t)fn(buf, n);
    }
    uint64_t elapsed = now_ns() - start;
    printf("SEARCH %s n=%zu reps=%llu ns_per_call=%.2f sink=%llu\n", name, n,
           (unsigned long long)reps, (double)elapsed / (double)reps,
           (unsigned long long)sink);
}

static void bench_words(const char *name, word_fn fn, uint64_t reps) {
    volatile uint64_t sink = 0;
    uint64_t start = now_ns();
    for (uint64_t i = 0; i < reps; i++) {
        __asm__ __volatile__("" ::: "memory");
        sink += fn(words[i % WORDS]);
    }
    uint64_t elapsed = now_ns() - start;
    printf("WORD %s reps=%llu ns_per_call=%.2f sink=%llu\n", name,
           (unsigned long long)reps, (double)elapsed / (double)reps,
           (unsigned long long)sink);
}

int main(void) {
    memset(hay_early, 1, BUF);
    memset(hay_nohit, 1, BUF);
    hay_early[3] = 0;
    for (int i = 0; i < WORDS; i++) {
        uint64_t x = 0x9E3779B97F4A7C15ull * (uint64_t)(i + 1);
        words[i] = x | (1ull << (i % 64));
    }

    bench_search("lib_memchr", lib_first_zero, hay_early, 64, 200000);
    bench_search("naive_loop", naive_first_zero, hay_early, 64, 200000);
    bench_search("lib_memchr", lib_first_zero, hay_nohit, 64, 200000);
    bench_search("naive_loop", naive_first_zero, hay_nohit, 64, 200000);
    bench_search("lib_memchr", lib_first_zero, hay_early, 4096, 5000);
    bench_search("naive_loop", naive_first_zero, hay_early, 4096, 5000);
    bench_search("lib_memchr", lib_first_zero, hay_nohit, 4096, 5000);
    bench_search("naive_loop", naive_first_zero, hay_nohit, 4096, 5000);

    bench_words("lib_builtin_popcount", lib_popcount, 20000000);
    bench_words("naive_kernighan", naive_popcount, 2000000);
    bench_words("lib_builtin_ctz", lib_ctz, 20000000);
    bench_words("naive_shift_ctz", naive_ctz, 2000000);
    return 0;
}
