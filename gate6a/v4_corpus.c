// Gate 6A-v4 — fresh held-out generalization corpus (Generation H4-recog)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v4_expected.csv), sealed
// (v4_manifest.sha256) before the frozen candidate runs once; raw output
// committed before inspection; failures promote this corpus to the next
// regression set. This corpus must NEVER be reused as held-out proof.
//
// Purpose: validate that the D5 remediation (conditional/masked
// accumulations are not raw-element sums, Gate 6A-v3 finding n08)
// GENERALIZES, on fresh kernels and fresh negative variants of the
// masked-sum class, while re-probing the standing classes.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES — expected: lowered, verified, recognized
// ═══════════════════════════════════════════════════════════════

// P01: Sum, u8 elements, u64 accumulator
uint64_t p01_sum_u8_u64(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; ++i) {
        total += buf[i];
    }
    return total;
}

// P02: Cardinality, u16 elements, == predicate
uint64_t p02_count_eq_u16(const uint16_t *vals, uint64_t n, uint16_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] == key);
    }
    return hits;
}

// P03: Cardinality, u32 elements, <= predicate
uint64_t p03_count_le_u32(const uint32_t *vals, uint64_t n, uint32_t cap) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] <= cap);
    }
    return hits;
}

// P04: All, u16 elements, != predicate
uint64_t p04_all_ne_u16(const uint16_t *vals, uint64_t n, uint16_t banned) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok = (vals[i] != banned) ? ok : 0;
    }
    return ok;
}

// P05: Sum, u32 elements, u64 accumulator
uint64_t p05_sum_u32_u64(const uint32_t *samples, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += samples[i];
    }
    return total;
}

// P06: All, u8 elements, == predicate, bitwise-and form
uint64_t p06_all_eq_u8(const uint8_t *b, uint64_t n, uint8_t want) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok &= (b[i] == want);
    }
    return ok;
}

// P07: Cardinality, u8 elements, > predicate
uint64_t p07_count_gt_u8(const uint8_t *buf, uint64_t n, uint8_t pivot) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (buf[i] > pivot);
    }
    return hits;
}

// P08: Sum, u16 elements, u32 accumulator
uint32_t p08_sum_u16_u32(const uint16_t *samples, uint32_t n) {
    uint32_t total = 0;
    for (uint32_t i = 0; i < n; i++) {
        total += samples[i];
    }
    return total;
}

// P09: Cardinality in a do-while shape over u32
uint64_t p09_count_eq_do_while_u32(const uint32_t *vals, uint64_t n, uint32_t key) {
    uint64_t hits = 0;
    uint64_t i = 0;
    do {
        hits += (vals[i] == key);
        i++;
    } while (i < n);
    return hits;
}

// P10: Disjunctive reduction over u16 elements (Any family)
uint64_t p10_any_or_u16(const uint16_t *words, uint64_t n) {
    uint16_t acc = 0;
    for (uint64_t i = 0; i < n; i++) {
        acc |= words[i];
    }
    return (acc != 0);
}

// P11: transformed-element sum (map-then-sum probe) — known open recall
// gap (H2 W03 class); pre-registered as recall_gap_known.
uint64_t p11_map_then_sum(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)buf[i] + 2;
    }
    return s;
}

// P12: two sequential reductions — known open frontend gap (H2 W08
// class); pre-registered as recall_gap_known.
uint64_t p12_two_loops(const uint16_t *vals, uint64_t n) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        c += (vals[i] == 7);
    }
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += vals[i];
    }
    return c ^ s;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES
// ═══════════════════════════════════════════════════════════════

// N01: masked sum — the D5 class must not be recognized as a raw sum
uint64_t n01_sum_masked_and(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (buf[i] & 0x0F);
    }
    return s;
}

// N02: conditional sum, ternary form — the D5 class
uint64_t n02_sum_ternary(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (buf[i] > 3) ? buf[i] : 0;
    }
    return s;
}

// N03: conditional sum, if-form — the D5 class
uint64_t n03_sum_if_ne(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] != 0) {
            s += buf[i];
        }
    }
    return s;
}

// N04: shifted sum (shr is not a transparent value conversion)
uint64_t n04_sum_shifted(const uint16_t *w, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)(w[i] >> 3);
    }
    return s;
}

// N05: scaled sum (mul is not a transparent value conversion)
uint64_t n05_sum_scaled(const uint8_t *b, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)b[i] * 3;
    }
    return s;
}

// N06: signed nsw accumulation with an unsigned loop bound. The concept
// may be recognized (it is a sum), but no candidate or rewrite may be
// authorized without overflow semantics (contained abstention).
int32_t n06_sum_i32_nsw(const int32_t *vals, uint64_t n) {
    int32_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += vals[i];
    }
    return s;
}

// N07: running max (partial-truth candidate generation class)
uint32_t n07_running_max_u32(const uint32_t *v, uint64_t n) {
    uint32_t m = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (v[i] > m) {
            m = v[i];
        }
    }
    return m;
}

// N08: two-array relation (footprint completeness class)
uint64_t n08_count_equal_pairs_u16(const uint16_t *a, const uint16_t *b, uint64_t n) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < n; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N09: may-alias store inside the loop
uint64_t n09_store_maybe_alias(uint8_t *buf, uint8_t *out, uint64_t n, uint8_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N10: volatile loads
uint64_t n10_volatile_load(volatile const uint16_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += buf[i];
    }
    return s;
}

// N11: fixed stride 3
uint64_t n11_stride3(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i += 3) {
        s += buf[i];
    }
    return s;
}

// N12: C11 atomic loads
uint64_t n12_atomic_sum(const _Atomic uint16_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += atomic_load_explicit(&buf[i], memory_order_relaxed);
    }
    return s;
}
