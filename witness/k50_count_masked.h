// K50: count_masked — count bytes where (buf[i] & mask) == target
//
// Correctness contract:
//   Input:    const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target
//   Output:   uint64_t (count of matching bytes)
//   Overflow: wraps on uint64_t overflow (defined for unsigned)
//   Alignment: buf may be unaligned
//   Aliasing: buf is read-only (nocapture readonly)
//   OOB reads: forbidden — must not read past buf[n-1]
//   Early termination: N/A (must inspect every element)
//   Constant-time: N/A

#pragma once
#include <stdint.h>
#include <string.h>

// A. Original source (from kernels.c)
static uint64_t k50_count_masked_orig(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++)
        count += ((buf[i] & mask) == target);
    return count;
}

// B. Clean conventional scalar
static uint64_t k50_count_masked_scalar(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++)
        count += ((buf[i] & mask) == target);
    return count;
}

// C. Chunked scalar — process 8 bytes per iteration
//    Apply mask to each byte, compare to target, count matches.
//    Uses SWAR to count matching bytes in a 64-bit word.
static uint64_t k50_count_masked_chunked(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    uint64_t i = 0;

    uint64_t mask64 = (uint64_t)mask * 0x0101010101010101ULL;
    uint64_t target64 = (uint64_t)target * 0x0101010101010101ULL;

    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        uint64_t masked = chunk & mask64;

        // Compare each byte to target: XOR, then check for zero bytes
        uint64_t xor_val = masked ^ target64;

        // SWAR zero-byte detection: a byte is zero if (v - 0x01) & ~v & 0x80 is set
        const uint64_t lo = 0x0101010101010101ULL;
        const uint64_t hi = 0x8080808080808080ULL;
        uint64_t zeros = (xor_val - lo) & ~xor_val & hi;

        // Count set bits in the high bits (popcount of zeros >> 7)
        // Each matching byte contributes one bit at position 7, 15, 23, ...
        count += __builtin_popcountll(zeros >> 7);
        i += 8;
    }

    while (i < n) {
        count += ((buf[i] & mask) == target);
        i++;
    }
    return count;
}

// D. SWAR — same as chunked, but processes 8 bytes with a different technique
//    Uses the "has zero byte" trick, then sums the 1-bit results.
static uint64_t k50_count_masked_swar(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    uint64_t i = 0;

    uint64_t mask64 = (uint64_t)mask * 0x0101010101010101ULL;
    uint64_t target64 = (uint64_t)target * 0x0101010101010101ULL;

    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        uint64_t masked = chunk & mask64;
        uint64_t xor_val = masked ^ target64;

        // Check if each byte is zero after XOR
        // If byte == 0, then (byte - 1) has bit 7 set while ~byte has bit 7 set
        uint64_t t = ~xor_val & (xor_val - 0x0101010101010101ULL);
        t &= 0x8080808080808080ULL;

        // Each set high bit = one match. Shift down and popcount.
        count += __builtin_popcountll(t) / 8;  // divide by 8 because popcount counts all 8 bit positions
        // Actually we need only the high bits. t already only has high bits set.
        // So popcount(t) = number of matching bytes directly.
        i += 8;
    }

    while (i < n) {
        count += ((buf[i] & mask) == target);
        i++;
    }
    return count;
}

// G. SSE2 implementation — 16 bytes per iteration
//    Uses vector AND, compare, and PMOVMSKB to extract mask, then popcount.
static uint64_t k50_count_masked_sse2(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    __m128i mask_v = _mm_set1_epi8((char)mask);
    __m128i target_v = _mm_set1_epi8((char)target);
    uint64_t count = 0;
    uint64_t i = 0;

    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        __m128i masked = _mm_and_si128(chunk, mask_v);
        __m128i cmp = _mm_cmpeq_epi8(masked, target_v);
        int bits = _mm_movemask_epi8(cmp);
        count += __builtin_popcount(bits);
        i += 16;
    }

    while (i < n) {
        count += ((buf[i] & mask) == target);
        i++;
    }
    return count;
}

// H. AVX2 implementation — 32 bytes per iteration
static uint64_t k50_count_masked_avx2(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    __m256i mask_v = _mm256_set1_epi8((char)mask);
    __m256i target_v = _mm256_set1_epi8((char)target);
    uint64_t count = 0;
    uint64_t i = 0;

    while (i + 32 <= n) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(buf + i));
        __m256i masked = _mm256_and_si256(chunk, mask_v);
        __m256i cmp = _mm256_cmpeq_epi8(masked, target_v);
        int bits = _mm256_movemask_epi8(cmp);
        count += __builtin_popcount(bits);
        i += 32;
    }

    // Handle remaining 16-byte chunks
    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        __m128i masked = _mm_and_si128(chunk, _mm_set1_epi8((char)mask));
        __m128i cmp = _mm_cmpeq_epi8(masked, _mm_set1_epi8((char)target));
        int bits = _mm_movemask_epi8(cmp);
        count += __builtin_popcount(bits);
        i += 16;
    }

    while (i < n) {
        count += ((buf[i] & mask) == target);
        i++;
    }
    return count;
}

// J. Size-specialized multi-versioned
static uint64_t k50_count_masked_multi(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    if (n < 32) return k50_count_masked_scalar(buf, n, mask, target);
    return k50_count_masked_avx2(buf, n, mask, target);
}
