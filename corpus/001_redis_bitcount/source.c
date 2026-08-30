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
//
// NOTE: bitsinbyte is passed as a parameter (not a static const) so that
// the SIR model and the original C have identical signatures for benchmarking.

#include <stdint.h>

// The kernel under test — the tail loop extracted from redisPopcount.
long long redis_bitcount_tail(const uint8_t *buf, long count, const uint64_t *bitsinbyte) {
    long long bits = 0;
    long i = 0;
    while (i < count) {
        bits += bitsinbyte[buf[i]];
        i++;
    }
    return bits;
}
