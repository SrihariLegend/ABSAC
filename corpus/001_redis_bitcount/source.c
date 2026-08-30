// Provenance: redis/redis (unstable branch), src/bitops.c, redisPopcount()
// Git: https://github.com/redis/redis/blob/unstable/src/bitops.c
// Function: redisPopcount — the `remain:` tail loop
//
// Original C:
//   static const uint8_t bitsinbyte[256] = { ... };
//   long long bits = 0;
//   unsigned char *p = s;
//   while (count--) bits += bitsinbyte[*p++];
//   return bits;
//
// This is the fallback popcount path in Redis's BITCOUNT command.
// It counts the number of set bits in a byte buffer using a 256-entry
// lookup table (bitsinbyte), where bitsinbyte[b] == popcount(b).

#include <stdint.h>

static const uint8_t bitsinbyte[256] = {
    0,1,1,2,1,2,2,3,1,2,2,3,2,3,3,4,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    4,5,5,6,5,6,6,7,5,6,6,7,6,7,7,8,
};

// The kernel under test — the tail loop extracted from redisPopcount.
long long redis_bitcount_tail(const uint8_t *buf, long count) {
    long long bits = 0;
    long i = 0;
    while (i < count) {
        bits += bitsinbyte[buf[i]];
        i++;
    }
    return bits;
}
