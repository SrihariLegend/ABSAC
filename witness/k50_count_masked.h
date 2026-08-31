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
//    Loads 8 bytes as uint64_t, then checks each byte individually (unrolled).
//    This avoids the SWAR zero-byte detection false-positive problem
//    (the haszero trick (v - lo) & ~v & hi has false positives from borrow propagation).
static uint64_t k50_count_masked_chunked(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    uint64_t i = 0;

    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        count += ((chunk & 0xFF) & mask) == target;
        count += (((chunk >> 8) & 0xFF) & mask) == target;
        count += (((chunk >> 16) & 0xFF) & mask) == target;
        count += (((chunk >> 24) & 0xFF) & mask) == target;
        count += (((chunk >> 32) & 0xFF) & mask) == target;
        count += (((chunk >> 40) & 0xFF) & mask) == target;
        count += (((chunk >> 48) & 0xFF) & mask) == target;
        count += (((chunk >> 56) & 0xFF) & mask) == target;
        i += 8;
    }

    while (i < n) {
        count += ((buf[i] & mask) == target);
        i++;
    }
    return count;
}

// D. SWAR — uses haszero trick as a fast filter, falls back to per-byte.
//    The haszero trick (v - lo) & ~v & hi correctly detects if ANY byte is zero,
//    but cannot count which bytes are zero (borrow propagation creates false positives).
//    So we use it to skip chunks with no matches, and do per-byte for the rest.
static uint64_t k50_count_masked_swar(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    uint64_t i = 0;

    uint64_t mask64 = (uint64_t)mask * 0x0101010101010101ULL;
    uint64_t target64 = (uint64_t)target * 0x0101010101010101ULL;
    const uint64_t lo = 0x0101010101010101ULL;
    const uint64_t hi = 0x8080808080808080ULL;

    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        uint64_t xor_val = (chunk & mask64) ^ target64;

        // Fast filter: if no byte is zero, skip per-byte check
        uint64_t might_have_zero = (xor_val - lo) & ~xor_val & hi;
        if (might_have_zero) {
            count += ((chunk & 0xFF) & mask) == target;
            count += (((chunk >> 8) & 0xFF) & mask) == target;
            count += (((chunk >> 16) & 0xFF) & mask) == target;
            count += (((chunk >> 24) & 0xFF) & mask) == target;
            count += (((chunk >> 32) & 0xFF) & mask) == target;
            count += (((chunk >> 40) & 0xFF) & mask) == target;
            count += (((chunk >> 48) & 0xFF) & mask) == target;
            count += (((chunk >> 56) & 0xFF) & mask) == target;
        }
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
