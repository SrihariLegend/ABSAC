// Gate 6A-v5 — fresh held-out generalization corpus (Generation H5-recog)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v5_expected.csv), sealed
// (v5_manifest.sha256) before the frozen candidate runs once; raw output
// committed before inspection; failures promote this corpus to the next
// regression set. Never reused as held-out proof.
//
// Purpose: validate the D5 remediation (conditional/masked accumulations
// are not raw-element sums) on a fresh frontier after the v4 corpus
// classification defect: v4's n03 guard `x != 0 ? x : 0` is semantically
// redundant (`x`), so clang canonicalizes it to a plain sum and
// SumReduction was correct there. v5 uses genuinely non-vacuous guards.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES
// ═══════════════════════════════════════════════════════════════

// P01: Sum, u8 elements, u32 accumulator/induction
uint32_t p01_sum_u8_u32(const uint8_t *buf, uint32_t n) {
    uint32_t total = 0;
    for (uint32_t i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

// P02: Cardinality, u32 elements, >= predicate
uint64_t p02_count_ge_u32(const uint32_t *vals, uint64_t n, uint32_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] >= floor_v);
    }
    return hits;
}

// P03: Cardinality, u16 elements, < predicate
uint64_t p03_count_lt_u16(const uint16_t *vals, uint64_t n, uint16_t ceil_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] < ceil_v);
    }
    return hits;
}

// P04: All, u16 elements, == predicate
uint64_t p04_all_eq_u16(const uint16_t *vals, uint64_t n, uint16_t want) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok = (vals[i] == want) ? ok : 0;
    }
    return ok;
}

// P05: All, u8 elements, != predicate, bitwise-and form
uint64_t p05_all_ne_u8(const uint8_t *b, uint64_t n, uint8_t banned) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok &= (b[i] != banned);
    }
    return ok;
}

// P06: Sum, u32 elements, u32 accumulator
uint32_t p06_sum_u32_u32(const uint32_t *vals, uint32_t n) {
    uint32_t total = 0;
    for (uint32_t i = 0; i < n; i++) {
        total += vals[i];
    }
    return total;
}

// P07: Cardinality with a constant bound
uint64_t p07_count_eq_const_bound(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 96; i++) {
        hits += (buf[i] == key);
    }
    return hits;
}

// P08: Disjunctive reduction over u32 elements (Any family)
uint64_t p08_any_or_u32(const uint32_t *words, uint64_t n) {
    uint32_t acc = 0;
    for (uint64_t i = 0; i < n; i++) {
        acc |= words[i];
    }
    return (acc != 0);
}

// P09: All, u16 elements, >= predicate
uint64_t p09_all_ge_u16(const uint16_t *vals, uint64_t n, uint16_t floor_v) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok &= (vals[i] >= floor_v);
    }
    return ok;
}

// P10: Cardinality in a do-while shape over u16, != predicate
uint64_t p10_count_ne_do_while_u16(const uint16_t *vals, uint64_t n, uint16_t ref_b) {
    uint64_t bad = 0;
    uint64_t i = 0;
    do {
        bad += (vals[i] != ref_b);
        i++;
    } while (i < n);
    return bad;
}

// P11: transformed-element sum (map-then-sum probe) — known open recall
// gap; pre-registered as recall_gap_known.
uint64_t p11_map_then_sum(const uint16_t *w, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)(w[i] ^ 0x0F);
    }
    return s;
}

// P12: two sequential reductions — known open frontend gap;
// pre-registered as recall_gap_known.
uint64_t p12_two_loops(const uint8_t *buf, uint64_t n) {
    uint64_t a = 1;
    for (uint64_t i = 0; i < n; i++) {
        a &= (buf[i] == 9);
    }
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += buf[i];
    }
    return a ^ s;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES
// ═══════════════════════════════════════════════════════════════

// N01: masked-guard sum (non-vacuous: odd values differ from 0)
uint64_t n01_sum_if_mask(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] & 1) {
            s += buf[i];
        }
    }
    return s;
}

// N02: ternary threshold (non-vacuous: 1..3 are nonzero but excluded)
uint64_t n02_sum_ternary_gt(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (buf[i] > 3) ? buf[i] : 0;
    }
    return s;
}

// N03: high-nibble masked sum
uint64_t n03_sum_masked_hi(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (buf[i] & 0xF0);
    }
    return s;
}

// N04: index-conditional sum (stride-like, not a raw-element sum)
uint64_t n04_sum_index_conditional(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (i & 1) {
            s += buf[i];
        }
    }
    return s;
}

// N05: signed nsw accumulation with an unsigned bound (contained: the
// concept is true; authorization must refuse without overflow semantics).
int64_t n05_sum_i64_nsw(const int64_t *vals, uint64_t n) {
    int64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += vals[i];
    }
    return s;
}

// N06: running max seeded from a parameter
uint16_t n06_running_max_u16(const uint16_t *v, uint64_t n, uint16_t init) {
    uint16_t m = init;
    for (uint64_t i = 0; i < n; i++) {
        if (v[i] > m) {
            m = v[i];
        }
    }
    return m;
}

// N07: two-array relation
uint64_t n07_two_arrays_u8(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < n; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N08: may-alias store inside the loop
uint64_t n08_store_maybe_alias(uint16_t *buf, uint16_t *out, uint64_t n, uint16_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N09: volatile loads
uint64_t n09_volatile_load(volatile const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += buf[i];
    }
    return s;
}

// N10: fixed stride 4
uint64_t n10_stride4(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i += 4) {
        s += buf[i];
    }
    return s;
}

// N11: C11 atomic loads
uint64_t n11_atomic_flags(const _Atomic uint32_t *flags, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += atomic_load_explicit(&flags[i], memory_order_relaxed);
    }
    return s;
}

// N12: self-referential recurrence without memory
uint64_t n12_self_recurrence(uint64_t n) {
    uint64_t x = 3;
    for (uint64_t i = 0; i < n; i++) {
        x += x >> 1;
    }
    return x;
}
