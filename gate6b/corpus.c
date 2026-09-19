// Gate 6B — sealed multi-reduction fusion corpus (blind-authoring protocol).
//
// Compiled with the C3 freeze flags:
//   clang -O1 -emit-llvm -S gate6b/corpus.c -o gate6b/corpus.ll
//
// Each kernel contains two or three sequential counted loops over one or
// more byte buffers. The return value combines every accumulator, so no
// loop is dead and every reduction is observable in the result.
//
// P rows: the traversals are independent, share a buffer base and an
//         iteration domain, and must be fused into a single pass.
// N rows: fusion must be refused (distinct bases, distinct domains,
//         side effects, unrecognized traversal, or a carried dependency).
//
// Reductions are written in the exact shapes of the recognized Gate 5A
// kernels (k50 count-masked, k43 sum, k18 all-equal) so every P row's
// loops are in the documented recognized dialect.

#include <stdint.h>

// ── P rows: expected to fuse ─────────────────────────────────────────

// g6p01 — count + sum + all over one buffer (the Gate 5B triad).
uint64_t g6p01_triad(const uint8_t *buf, uint64_t n, uint8_t mask,
                     uint8_t target, uint8_t val) {
    uint64_t c = 0, s = 0, a = 1;
    for (uint64_t i = 0; i < n; i++) c += ((buf[i] & mask) == target);
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    for (uint64_t i = 0; i < n; i++) a &= (buf[i] == val);
    return c ^ s ^ a;
}

// g6p02 — count + sum (pair fusion).
uint64_t g6p02_count_sum(const uint8_t *buf, uint64_t n, uint8_t mask,
                         uint8_t target) {
    uint64_t c = 0, s = 0;
    for (uint64_t i = 0; i < n; i++) c += ((buf[i] & mask) == target);
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    return c ^ s;
}

// g6p03 — sum + all (pair fusion).
uint64_t g6p03_sum_all(const uint8_t *buf, uint64_t n, uint8_t val) {
    uint64_t s = 0, a = 1;
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    for (uint64_t i = 0; i < n; i++) a &= (buf[i] == val);
    return s ^ a;
}

// g6p04 — two predicate counts + sum (three reductions, two predicates).
uint64_t g6p04_two_predicates(const uint8_t *buf, uint64_t n, uint8_t mask,
                              uint8_t t1, uint8_t t2) {
    uint64_t c1 = 0, c2 = 0, s = 0;
    for (uint64_t i = 0; i < n; i++) c1 += ((buf[i] & mask) == t1);
    for (uint64_t i = 0; i < n; i++) c2 += ((buf[i] & mask) == t2);
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    return c1 ^ c2 ^ s;
}

// g6p05 — literal predicates (no scalar parameters).
uint64_t g6p05_literals(const uint8_t *buf, uint64_t n) {
    uint64_t c = 0, s = 0, a = 1;
    for (uint64_t i = 0; i < n; i++) c += ((buf[i] & 15) == 5);
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    for (uint64_t i = 0; i < n; i++) a &= (buf[i] == 66);
    return c ^ s ^ a;
}

// ── N rows: expected to refuse fusion ────────────────────────────────

// g6n06 — distinct buffer bases: no shared traversal to fuse.
uint64_t g6n06_two_buffers(const uint8_t *a, const uint8_t *b, uint64_t n,
                           uint8_t mask, uint8_t target) {
    uint64_t c = 0, s = 0;
    for (uint64_t i = 0; i < n; i++) c += ((a[i] & mask) == target);
    for (uint64_t i = 0; i < n; i++) s += b[i];
    return c ^ s;
}

// g6n07 — distinct iteration domains over the same buffer.
uint64_t g6n07_half_domain(const uint8_t *buf, uint64_t n, uint8_t mask,
                           uint8_t target) {
    uint64_t c = 0, s = 0;
    uint64_t half = n / 2;
    for (uint64_t i = 0; i < n; i++) c += ((buf[i] & mask) == target);
    for (uint64_t i = 0; i < half; i++) s += buf[i];
    return c ^ s;
}

// g6n08 — a loop with an observable store is not a fusable reduction.
extern uint8_t g6_out[64];
uint64_t g6n08_store_loop(const uint8_t *buf, uint64_t n, uint8_t v) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    for (uint64_t i = 0; i < n && i < 64; i++) g6_out[i] = v;
    return s;
}

// g6n09 — a descending traversal is outside the recognized forward dialect.
uint64_t g6n09_reverse(const uint8_t *buf, uint64_t n, uint8_t mask,
                       uint8_t target) {
    uint64_t s = 0, c = 0;
    for (uint64_t i = 0; i < n; i++) s += buf[i];
    for (long i = (long)n - 1; i >= 0; i--) c += ((buf[i] & mask) == target);
    return c ^ s;
}

// g6n10 — the second traversal's domain depends on the first's result.
uint64_t g6n10_dependent(const uint8_t *buf, uint64_t n, uint8_t mask,
                         uint8_t target) {
    uint64_t c = 0, s = 0;
    for (uint64_t i = 0; i < n; i++) c += ((buf[i] & mask) == target);
    for (uint64_t i = 0; i < c; i++) s += buf[i];
    return c ^ s;
}
