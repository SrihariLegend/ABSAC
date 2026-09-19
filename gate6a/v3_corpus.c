// Gate 6A-v3 — fresh held-out generalization corpus (Generation H3-recog)
//
// BLINDNESS PROTOCOL (same as v1/v2):
// 1. This corpus is classified BEFORE running ABSAC (v3_expected.csv).
// 2. Source + expected classification + SHA256 hashes are sealed
//    (v3_manifest.sha256) BEFORE the frozen candidate runs.
// 3. The frozen candidate runs ONCE; raw results are committed before
//    inspection.
// 4. Failures promote this corpus to the next regression set.
// 5. This corpus must NEVER be reused as proof of held-out generalization.
//
// Design: fresh shapes with metamorphic near-miss negatives. The negatives
// target the failure classes of earlier generations (overflow semantics,
// partial-truth candidate generation, footprint completeness) plus the
// standing abstention classes (effects, stride, reversal, atomics).

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES — expected: lowered, verified, recognized
// ═══════════════════════════════════════════════════════════════

// P01: Cardinality, u16 elements, >= predicate (unsigned)
uint64_t p01_count_ge_u16(const uint16_t *vals, uint64_t n, uint16_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] >= floor_v);
    }
    return hits;
}

// P02: Sum, u8 elements, u32 accumulator and u32 induction
uint32_t p02_sum_u8_u32acc(const uint8_t *buf, uint32_t n) {
    uint32_t total = 0;
    for (uint32_t i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

// P03: All, i8 elements, != predicate (select-reset form, unsigned loop)
uint64_t p03_all_ne_const(const int8_t *data, uint64_t n, int8_t banned) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok = (data[i] != banned) ? ok : 0;
    }
    return ok;
}

// P04: Cardinality, u32 elements, >= predicate with a constant bound
uint64_t p04_count_ge_const_u32(const uint32_t *vals, uint32_t pivot) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 128; i++) {
        hits += (vals[i] >= pivot);
    }
    return hits;
}

// P05: Sum, u16 elements, u64 accumulator
uint64_t p05_sum_u16_u64acc(const uint16_t *samples, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += samples[i];
    }
    return total;
}

// P06: Cardinality, u8 elements, != predicate with a parameter bound
uint64_t p06_count_ne_u8(const uint8_t *buf, uint64_t n, uint8_t ref_b) {
    uint64_t bad = 0;
    for (uint64_t i = 0; i < n; i++) {
        bad += (buf[i] != ref_b);
    }
    return bad;
}

// P07: All, u64 elements, == predicate
uint64_t p07_all_eq_u64(const uint64_t *words, uint64_t n, uint64_t want) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok = (words[i] == want) ? ok : 0;
    }
    return ok;
}

// P08: All, u8 elements, >= predicate, u64 accumulator
uint64_t p08_all_ge_u8(const uint8_t *b, uint64_t n, uint8_t floor_v) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < n; i++) {
        ok &= (b[i] >= floor_v);
    }
    return ok;
}

// P09: Disjunctive reduction over raw bytes (Any family, D4.2 shape)
uint64_t p09_any_or_u8(const uint8_t *buf, uint64_t n) {
    uint8_t acc = 0;
    for (uint64_t i = 0; i < n; i++) {
        acc |= buf[i];
    }
    return (acc != 0);
}

// P10: Cardinality in a do-while shape
uint64_t p10_count_eq_do_while(const uint8_t *buf, uint64_t n, uint8_t key) {
    uint64_t hits = 0;
    uint64_t i = 0;
    do {
        hits += (buf[i] == key);
        i++;
    } while (i < n);
    return hits;
}

// P11: Sum of a transformed element (buf[i] + 1) — map-then-sum probe.
// Known open recall gap (H2 W03); pre-registered as recall_gap_known.
uint64_t p11_sum_plus_one_u16(const uint16_t *vals, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)(vals[i] + 1);
    }
    return s;
}

// P12: Two sequential reductions in one function — two-loop lowering
// probe. Known open frontend gap (H2 W08); pre-registered as
// recall_gap_known.
uint64_t p12_two_reductions(const uint32_t *a, uint64_t n) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        c += (a[i] == 0);
    }
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += a[i];
    }
    return c ^ s;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES — must NOT be recognized, generate candidates, or rewrite
// ═══════════════════════════════════════════════════════════════

// N01: signed i64 accumulation (nsw) — overflow semantics gap class
int64_t n01_sum_i64_nsw(const int64_t *vals, int64_t n) {
    int64_t s = 0;
    for (int64_t i = 0; i < n; i++) {
        s += vals[i];
    }
    return s;
}

// N02: running max — partial-truth candidate generation class
int32_t n02_running_max(const int32_t *vals, int64_t n) {
    int32_t m = vals[0];
    for (int64_t i = 1; i < n; i++) {
        if (vals[i] > m) {
            m = vals[i];
        }
    }
    return m;
}

// N03: two-array relation — footprint completeness class
uint64_t n03_count_equal_pairs(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < n; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N04: store that may alias the scanned buffer (memory dependence)
uint64_t n04_store_maybe_alias(uint8_t *buf, uint8_t *out, uint64_t n, uint8_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N05: volatile loads inside the loop (effects)
uint64_t n05_volatile_load(volatile const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += buf[i];
    }
    return s;
}

// N06: fixed stride 2 (non-unit traversal)
uint64_t n06_stride2(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i += 2) {
        s += buf[i];
    }
    return s;
}

// N07: runtime stride parameter
uint64_t n07_dynamic_stride(const uint8_t *buf, uint64_t n, uint64_t step) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i += step) {
        s += buf[i];
    }
    return s;
}

// N08: conditional accumulator (not a monoid reduction over elements)
uint64_t n08_conditional_sum(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] & 1) {
            s += buf[i];
        }
    }
    return s;
}

// N09: self-referential recurrence without memory
uint64_t n09_self_recurrence(uint64_t n) {
    uint64_t x = 1;
    for (uint64_t i = 0; i < n; i++) {
        x += x >> 1;
    }
    return x;
}

// N10: early exit with a global side effect
extern uint64_t g6a_v3_out;
uint64_t n10_early_exit_write(const uint8_t *buf, uint64_t n, uint8_t t) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == t) {
            g6a_v3_out = i;
            return 1;
        }
    }
    return 0;
}

// N11: descending traversal
uint64_t n11_reverse_sum(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (int64_t i = (int64_t)n - 1; i >= 0; i--) {
        s += buf[i];
    }
    return s;
}

// N12: C11 atomic loads (ordering semantics)
uint64_t n12_atomic_sum(const _Atomic uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += atomic_load_explicit(&buf[i], memory_order_relaxed);
    }
    return s;
}
