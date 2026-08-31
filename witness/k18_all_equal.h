// K18: all_equal — check if all bytes in buf equal val
//
// Correctness contract:
//   Input:    const uint8_t *buf, uint64_t n, uint8_t val
//   Output:   uint64_t (1 if all equal, 0 otherwise)
//   Overflow: no (sum of booleans, bounded by n)
//   Alignment: buf may be unaligned
//   Aliasing: buf is read-only (nocapture readonly)
//   OOB reads: forbidden — must not read past buf[n-1]
//   Early termination: observable only via performance, not output
//     (output is always 0 or 1 regardless of where mismatch occurs)
//   Constant-time: NOT required (early exit is allowed)

#pragma once
#include <stdint.h>
#include <stdbool.h>

// A. Original source (from kernels.c)
static uint64_t k18_all_equal_orig(const uint8_t *buf, uint64_t n, uint8_t val) {
    uint64_t all = 1;
    for (uint64_t i = 0; i < n; i++) all &= (buf[i] == val);
    return all;
}

// B. Clean conventional scalar with early exit
static uint64_t k18_all_equal_scalar(const uint8_t *buf, uint64_t n, uint8_t val) {
    for (uint64_t i = 0; i < n; i++)
        if (buf[i] != val) return 0;
    return 1;
}

// C. Chunked scalar — process 8 bytes per iteration as uint64_t
//     Handles unaligned reads (memcpy) and tail.
static uint64_t k18_all_equal_chunked(const uint8_t *buf, uint64_t n, uint8_t val) {
    if (n == 0) return 1;

    // First check byte-by-byte until aligned to 8
    uint64_t i = 0;
    uintptr_t addr = (uintptr_t)buf;
    while (i < n && (addr + i) % 8 != 0) {
        if (buf[i] != val) return 0;
        i++;
    }

    // Check 8 bytes at a time
    uint64_t pattern = (uint64_t)val * 0x0101010101010101ULL;
    while (i + 8 <= n) {
        uint64_t chunk;
        __builtin_memcpy(&chunk, buf + i, 8);
        if (chunk != pattern) {
            // Find which byte differs
            for (uint64_t j = 0; j < 8; j++)
                if (buf[i + j] != val) return 0;
        }
        i += 8;
    }

    // Tail
    while (i < n) {
        if (buf[i] != val) return 0;
        i++;
    }
    return 1;
}

// D. SWAR — same as chunked, just different naming for the technique
//    (For all_equal, SWAR and chunked scalar are essentially the same.)
//    We skip D for this kernel to avoid redundancy.

// E. Compiler builtins — no applicable builtin for all_equal
//    (We skip E for this kernel.)

// G. SSE2 implementation — 16 bytes per iteration
static uint64_t k18_all_equal_sse2(const uint8_t *buf, uint64_t n, uint8_t val) {
    if (n == 0) return 1;

    // Scalar prefix until we have 16 bytes
    uint64_t i = 0;
    while (i + 16 <= n) {
        // Load 16 bytes (unaligned)
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        __m128i pat = _mm_set1_epi8((char)val);
        __m128i cmp = _mm_cmpeq_epi8(chunk, pat);
        int mask = _mm_movemask_epi8(cmp);
        if (mask != 0xFFFF) return 0;
        i += 16;
    }

    // Tail
    while (i < n) {
        if (buf[i] != val) return 0;
        i++;
    }
    return 1;
}

// H. AVX2 implementation — 32 bytes per iteration
static uint64_t k18_all_equal_avx2(const uint8_t *buf, uint64_t n, uint8_t val) {
    if (n == 0) return 1;

    uint64_t i = 0;
    while (i + 32 <= n) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(buf + i));
        __m256i pat = _mm256_set1_epi8((char)val);
        __m256i cmp = _mm256_cmpeq_epi8(chunk, pat);
        int mask = _mm256_movemask_epi8(cmp);
        if (mask != -1) return 0;
        i += 32;
    }

    // Handle remaining 16-byte chunks
    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(buf + i));
        __m128i pat = _mm_set1_epi8((char)val);
        __m128i cmp = _mm_cmpeq_epi8(chunk, pat);
        int mask = _mm_movemask_epi8(cmp);
        if (mask != 0xFFFF) return 0;
        i += 16;
    }

    // Tail
    while (i < n) {
        if (buf[i] != val) return 0;
        i++;
    }
    return 1;
}

// J. Size-specialized multi-versioned implementation
//    Uses scalar for tiny inputs, AVX2 for large.
static uint64_t k18_all_equal_multi(const uint8_t *buf, uint64_t n, uint8_t val) {
    if (n < 32) return k18_all_equal_scalar(buf, n, val);
    return k18_all_equal_avx2(buf, n, val);
}
