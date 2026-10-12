#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static long find_first(const unsigned char *a, long n, unsigned char c) {
    for (long i = 0; i < n; i++)
        if (a[i] == c) return i;
    return -1;
}

static double now(void) {
    struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec * 1e-9;
}

int main(void) {
    long n = 64L << 20;                 /* 64 MiB */
    unsigned char *buf = malloc(n);
    for (long i = 0; i < n; i++) buf[i] = (unsigned char)(i * 2654435761u >> 24);

    /* absent byte: the loop must examine every byte */
    unsigned char needle = 0x7f;
    /* ensure it is truly absent */
    for (long i = 0; i < n; i++) if (buf[i] == needle) buf[i] ^= 0x01;

    double t0 = now();
    long r1 = find_first(buf, n, needle);
    double t1 = now();
    void *r2 = memchr(buf, needle, n);
    double t2 = now();

    printf("n = %ld MiB\n", n >> 20);
    printf("clang -O3 loop : %.1f ms   (result %ld)\n", (t1-t0)*1e3, r1);
    printf("memchr         : %.1f ms   (result %s)\n", (t2-t1)*1e3, r2 ? "found" : "NULL");
    printf("GAP            : %.1fx\n", (t1-t0)/(t2-t1));
    free(buf);
    return 0;
}
