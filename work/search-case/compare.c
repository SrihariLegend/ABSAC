/* Original developer-written loop vs ABSAC's emitted rewrite.
 *
 * Same compiler, same flags, both noinline so the harness cannot inline one
 * and not the other. The ABSAC side is emitted.c from the referee's workdir,
 * copied verbatim.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* ---- what the developer wrote; clang -O3 compiles this to a scalar byte
 * ---- loop with two branches per byte, and does NOT call memchr ---- */
__attribute__((noinline))
long find_orig(const unsigned char *a, long n, unsigned char c) {
    long i;
    for (i = 0; i < n; i++)
        if (a[i] == c) return i;
    return n;
}

/* ---- ABSAC's emitted rewrite, verbatim from emitted.c ---- */
static uint64_t __sir_memchr(const uint8_t *base, int key, uint64_t n) {
    if (n == 0) return 0;
    uint64_t head = n < 16 ? n : 16;
    for (uint64_t i = 0; i < head; i++)
        if (base[i] == (uint8_t)key) return i;
    if (n <= 16) return n;
    const void *hit = memchr((const void *)(base + 16), key, (size_t)(n - 16));
    return hit ? (uint64_t)((const uint8_t *)hit - base) : n;
}
__attribute__((noinline))
long find_sir(const unsigned char *p0, long p1, unsigned char p2) {
    return (long)__sir_memchr(p0, (int)p2, (uint64_t)p1);
}

static double now(void) {
    struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec * 1e-9;
}

static long best(long (*f)(const unsigned char*, long, unsigned char),
                 const unsigned char *b, long n, unsigned char c, int reps) {
    double bst = 1e18;
    for (int r = 0; r < reps; r++) {
        double t0 = now();
        volatile long s = f(b, n, c);
        double t1 = now();
        (void)s;
        if (t1 - t0 < bst) bst = t1 - t0;
    }
    return (long)(bst * 1e9);
}

int main(void) {
    long n = 64L << 20;
    unsigned char *buf = malloc(n);
    for (long i = 0; i < n; i++) buf[i] = (unsigned char)(i * 2654435761u >> 24);
    unsigned char needle = 0x7f;
    for (long i = 0; i < n; i++) if (buf[i] == needle) buf[i] ^= 1;

    printf("%-12s %-16s %-16s %s\n", "size", "clang -O3 loop", "ABSAC rewrite", "speedup");
    long sizes[] = {64, 1024, 65536, 1L << 20, 16L << 20, 64L << 20};
    int reps[]   = {200000, 100000, 20000, 2000, 200, 50};
    for (int i = 0; i < 6; i++) {
        long sz = sizes[i];
        long a = best(find_orig, buf, sz, needle, reps[i]);
        long b = best(find_sir,  buf, sz, needle, reps[i]);
        printf("%-12ld %-16ld %-16ld %.2fx\n", sz, a, b, (double)a / (double)b);
    }
    free(buf);
    return 0;
}
