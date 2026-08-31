// K43: sum_ascii — sum all bytes in buffer
//
// Correctness contract:
//   Input:    const uint8_t *buf, uint64_t n
//   Output:   uint64_t (sum of all byte values)
//   Overflow: wraps on uint64_t overflow (defined for unsigned)
//   Alignment: buf may be unaligned
//   Aliasing: buf is read-only (nocapture readonly)
//   OOB reads: forbidden — must not read past buf[n-1]
//   Early termination: N/A (must inspect every element)
//   Constant-time: N/A (no early exit possible)

#pragma once
#include <stdint.h>
#include <string.h>

// A. Original source (from kernels.c)
static uint64_t k43_sum_ascii_orig(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}

// B. Clean conventional scalar
static uint64_t k43_sum_ascii_scalar(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}

// C. Chunked scalar — process 8 bytes per iteration as uint64_t
//     Widens each byte to 64-bit and accumulates.
//     Uses SWAR technique to sum bytes within a 64-bit word.
static uint64_t k43_sum_ascii_chunked(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    uint64_t i = 0;

    // Process 8 bytes at a time using SWAR byte-sum
    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);

        // SWAR: sum 8 bytes packed in a 64-bit word
        // Mask each byte, shift and add
        // chunk = b0 | b1<<8 | b2<<16 | ... | b7<<56
        // Sum = b0+b1+b2+b3+b4+b5+b6+b7
        // Method: pairwise add with mask to prevent carry propagation
        const uint64_t mask = 0x00FF00FF00FF00FFULL;
        uint64_t v = chunk;
        v = ((v >> 8) & mask) + (v & mask);     // 4x 16-bit
        v = ((v >> 16) & 0x0000FFFF0000FFFFULL) + (v & 0x0000FFFF0000FFFFULL); // 2x 32-bit
        v = ((v >> 32) + (v & 0xFFFFFFFFULL));   // 1x 64-bit
        sum += v;
        i += 8;
    }

    // Tail
    while (i < n) {
        sum += buf[i];
        i++;
    }
    return sum;
}

// D. SWAR — same technique as chunked, processing 8 bytes
//    For sum, the SWAR and chunked are the same algorithm.
//    We use a different SWAR variant: accumulate full 64-bit adds
//    of zero-extended bytes, processing 8 at once via unpacking.
static uint64_t k43_sum_ascii_swar(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    uint64_t i = 0;

    // Process 8 bytes: load as uint64_t, then extract and sum each byte
    // This is a simpler SWAR that just avoids per-byte loop overhead
    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        // Extract each byte and add
        sum += chunk & 0xFF;
        sum += (chunk >> 8) & 0xFF;
        sum += (chunk >> 16) & 0xFF;
        sum += (chunk >> 24) & 0xFF;
        sum += (chunk >> 32) & 0xFF;
        sum += (chunk >> 40) & 0xFF;
        sum += (chunk >> 48) & 0xFF;
        sum += (chunk >> 56) & 0xFF;
        i += 8;
    }

    while (i < n) {
        sum += buf[i];
        i++;
    }
    return sum;
}

// G. SSE2 implementation — 16 bytes per iteration
//    Uses PSADBW (Packed Sum of Absolute Differences Byte-Wise)
//    which computes the sum of absolute differences against zero
//    = sum of bytes, in two 16-bit lanes.
static uint64_t k43_sum_ascii_sse2(const uint8_t *buf, uint64_t n) {
    __m128i zero = _mm_setzero_si128();
    __m128i acc = zero;
    uint64_t i = 0;

    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        // PSADBW against zero = sum of bytes in each 8-byte lane
        __m128i sums = _mm_sad_epu8(chunk, zero);
        acc = _mm_add_epi64(acc, sums);
        i += 16;
    }

    // Extract the two 16-bit sums from the 128-bit register
    uint64_t lo = _mm_cvtsi128_si64(acc);
    uint64_t hi = _mm_extract_epi64(acc, 1);
    uint64_t sum = lo + hi;

    // Tail
    while (i < n) {
        sum += buf[i];
        i++;
    }
    return sum;
}

// H. AVX2 implementation — 32 bytes per iteration
//    Uses PSADBW on two 128-bit lanes within the 256-bit register.
static uint64_t k43_sum_ascii_avx2(const uint8_t *buf, uint64_t n) {
    __m256i zero = _mm256_setzero_si256();
    __m256i acc = zero;
    uint64_t i = 0;

    while (i + 32 <= n) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(buf + i));
        // PSADBW: each 128-bit lane produces two 16-bit sums
        __m256i sums = _mm256_sad_epu8(chunk, zero);
        acc = _mm256_add_epi64(acc, sums);
        i += 32;
    }

    // Extract 4 16-bit sums (two per 128-bit lane)
    __m128i lo128 = _mm256_castsi256_si128(acc);
    __m128i hi128 = _mm256_extracti128_si256(acc, 1);
    uint64_t s0 = _mm_cvtsi128_si64(lo128);
    uint64_t s1 = _mm_extract_epi64(lo128, 1);
    uint64_t s2 = _mm_cvtsi128_si64(hi128);
    uint64_t s3 = _mm_extract_epi64(hi128, 1);
    uint64_t sum = s0 + s1 + s2 + s3;

    // Handle remaining 16-byte chunks
    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        __m128i sums = _mm_sad_epu8(chunk, _mm_setzero_si128());
        sum += _mm_cvtsi128_si64(sums) + _mm_extract_epi64(sums, 1);
        i += 16;
    }

    // Tail
    while (i < n) {
        sum += buf[i];
        i++;
    }
    return sum;
}

// J. Size-specialized multi-versioned
static uint64_t k43_sum_ascii_multi(const uint8_t *buf, uint64_t n) {
    if (n < 32) return k43_sum_ascii_scalar(buf, n);
    return k43_sum_ascii_avx2(buf, n);
}
