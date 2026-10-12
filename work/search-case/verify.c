/* The referee's differential uses lens[] = {0..32}, so it never exercises the
   path where __sir_memchr actually calls memchr (n > 16). Check the values
   agree across the sizes that matter, including hits at many positions. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

__attribute__((noinline))
long find_orig(const unsigned char *a, long n, unsigned char c) {
    long i;
    for (i = 0; i < n; i++)
        if (a[i] == c) return i;
    return n;
}
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

int main(void) {
    unsigned char needle = 0xA5;
    long sizes[] = {0,1,15,16,17,31,32,33,64,1000,65536,1L<<20,4L<<20};
    long bad = 0, checked = 0;
    for (int si = 0; si < 13; si++) {
        long n = sizes[si];
        unsigned char *buf = malloc(n ? n : 1);
        /* positions to place the needle: absent, first, boundary around 16,
           middle, last. All are inside the range the two disagree on if the
           memchr path is wrong. */
        long positions[] = {-1, 0, 15, 16, 17, 31, 32, n/2, n-1};
        for (int pi = 0; pi < 9; pi++) {
            long pos = positions[pi];
            if (pos >= n) continue;
            for (long i = 0; i < n; i++) buf[i] = (unsigned char)(i * 7 + 3);
            for (long i = 0; i < n; i++) if (buf[i] == needle) buf[i] ^= 1;
            if (pos >= 0) buf[pos] = needle;
            long a = find_orig(buf, n, needle);
            long b = find_sir(buf, n, needle);
            checked++;
            if (a != b) {
                printf("MISMATCH n=%ld pos=%ld orig=%ld sir=%ld\n", n, pos, a, b);
                bad++;
            }
        }
        free(buf);
    }
    printf("%s: %ld configurations checked, %ld mismatches\n",
           bad ? "FAIL" : "PASS", checked, bad);
    return bad != 0;
}
