// Gate 6A-v16 — fresh held-out corpus (P5 increment)
//
// PREREGISTRATION: classifications live in v16_expected.csv and the source is
// hashed in v16_manifest.sha256 before the first evaluation run. This corpus
// targets edges the v12–v14 sets do not cover:
//   - runtime-extent searches with explicit zero-length guards, alternate
//     predicates (ne, ge), u16/u32 elements, and descending order;
//   - wider-element and map-then-sum reductions (recognized but currently
//     unrewritten per the P2 investigation);
//   - scalar bit permutations (rotate, byte swap, bit reverse) that have no
//     corpus coverage at all.
// Uncertain lowering/recognition shapes are recorded as `recall_gap_known`
// up front rather than discovered after the run.

#include <stdint.h>

// ── Runtime-extent searches ─────────────────────────────────────

uint64_t r01_runtime_first_zero_guard(const uint8_t *a, uint64_t n) {
    if (n == 0)
        return 0;
    for (uint64_t i = 0; i < n; ++i)
        if (a[i] == 0)
            return i;
    return n;
}

uint64_t r02_runtime_first_ne_key_u32(const uint32_t *a, uint64_t n, uint32_t key) {
    for (uint64_t i = 0; i < n; ++i)
        if (a[i] != key)
            return i;
    return n;
}

uint64_t r03_runtime_last_eq_key_u16(const uint16_t *a, uint64_t n, uint16_t key) {
    for (uint64_t i = n; i-- > 0;)
        if (a[i] == key)
            return i;
    return n;
}

uint64_t r04_runtime_first_ge_key_u16(const uint16_t *a, uint64_t n, uint16_t key) {
    for (uint64_t i = 0; i < n; ++i)
        if (a[i] >= key)
            return i;
    return n;
}

// ── Wider-element and map-then-sum reductions ───────────────────

uint64_t s01_sum_u16_64(const uint16_t *a) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 64; ++i)
        sum += a[i];
    return sum;
}

uint64_t s02_xor_map_sum_i16_64(const int16_t *a) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 64; ++i)
        sum += (uint16_t)((uint16_t)a[i] ^ 90u);
    return sum;
}

uint64_t s03_cond_sum_u8_64(const uint8_t *a) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 64; ++i)
        if (a[i] & 1u)
            sum += a[i];
    return sum;
}

uint64_t s04_sum_u32_32(const uint32_t *a) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 32; ++i)
        sum += a[i];
    return sum;
}

// ── Scalar bit permutations ─────────────────────────────────────

uint32_t p01_rotl13_u32(uint32_t x) {
    return (x << 13) | (x >> 19);
}

uint32_t p02_bswap_u32(uint32_t x) {
    return __builtin_bswap32(x);
}

uint32_t p03_bitrev_u32(uint32_t x) {
    x = ((x >> 1) & 0x55555555u) | ((x & 0x55555555u) << 1);
    x = ((x >> 2) & 0x33333333u) | ((x & 0x33333333u) << 2);
    x = ((x >> 4) & 0x0F0F0F0Fu) | ((x & 0x0F0F0F0Fu) << 4);
    x = ((x >> 8) & 0x00FF00FFu) | ((x & 0x00FF00FFu) << 8);
    return (x >> 16) | (x << 16);
}

// ── Composition ─────────────────────────────────────────────────

uint64_t c01_search_then_sum(const uint8_t *a, uint8_t key) {
    uint64_t found = 0;
    for (uint64_t i = 0; i < 48; ++i) {
        if (a[i] == key) {
            found = i + 1;
            break;
        }
    }
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 48; ++i)
        sum += a[i];
    return found * 1000 + sum;
}
