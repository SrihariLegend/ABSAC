#include <stdint.h>
#include <stdbool.h>
#include <string.h>

/* memchr-style bounded search: first index of key below n, or n.
   The first bytes are scanned inline so early hits avoid the
   call/setup cost of a vectorized memchr; later hits still get
   the vectorized scan. */
static uint64_t __sir_memchr(const uint8_t *base, int key, uint64_t n) {
    if (n == 0) return 0;
    uint64_t head = n < 16 ? n : 16;
    for (uint64_t i = 0; i < head; i++)
        if (base[i] == (uint8_t)key) return i;
    if (n <= 16) return n;
    const void *hit = memchr((const void *)(base + 16), key, (size_t)(n - 16));
    return hit ? (uint64_t)((const uint8_t *)hit - base) : n;
}

uint64_t find_first(const uint8_t *p0, uint64_t p1, uint8_t p2) {
    uint64_t v23 = ((uint64_t)__sir_memchr((const uint8_t *)(p0), (int)(p2), (uint64_t)(p1)));
    return v23;
}
