// Gate 6A-v6 — fresh held-out corpus (Generation H6)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v6_expected.csv) and
// sealed (v6_manifest.sha256) before the frozen candidate runs once.
// Raw output is committed before inspection; failures promote this
// corpus to the next regression set. Never reused as held-out proof.
//
// Frontier: the F6–F8 emitter fixes + sequential multi-loop emission +
// constant-extent buffer promotion landed 2026-09-17. This corpus
// probes them on unseen shapes: constant-extent reductions of several
// element widths/predicates, a sequential two-loop kernel, the
// map-then-sum frontier, the reverse-search totality gate, and the
// recorded safety classes.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES
// ═══════════════════════════════════════════════════════════════

// P01: Cardinality, u16 elements, >= predicate, constant bound 80
uint64_t p01_count_ge_const_u16(const uint16_t *vals, uint16_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 80; i++) {
        hits += (vals[i] >= floor_v);
    }
    return hits;
}

// P02: Cardinality, u8 elements, == predicate, constant bound 96
uint64_t p02_count_eq_const_u8(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 96; i++) {
        hits += (buf[i] == key);
    }
    return hits;
}

// P03: Cardinality, u32 elements, < predicate, constant bound 64
uint64_t p03_count_lt_const_u32(const uint32_t *words, uint32_t ceil_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 64; i++) {
        hits += (words[i] < ceil_v);
    }
    return hits;
}

// P04: Sum, u8 elements into u64, constant bound 128
uint64_t p04_sum_u8_const_u64(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 128; i++) {
        total += buf[i];
    }
    return total;
}

// P05: Sum, u16 elements into u32, constant bound 40
uint32_t p05_sum_u16_const_u32(const uint16_t *vals) {
    uint32_t total = 0;
    for (uint64_t i = 0; i < 40; i++) {
        total += vals[i];
    }
    return total;
}

// P06: All, u8 elements, != predicate, bitwise-and form, constant bound 64
uint64_t p06_all_ne_const_u8(const uint8_t *buf, uint8_t banned) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < 64; i++) {
        ok &= (buf[i] != banned);
    }
    return ok;
}

// P07: Any, u32 raw-element OR, constant bound 64
uint64_t p07_any_or_const_u32(const uint32_t *words) {
    uint32_t acc = 0;
    for (uint64_t i = 0; i < 64; i++) {
        acc |= words[i];
    }
    return (acc != 0);
}

// P08: two sequential reductions over one constant-extent buffer:
// count (== key) then sum; combine with xor. Exercises whole-function
// sequential-loop lowering AND the multi-loop emitter natively.
uint64_t p08_two_loops_const(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 48; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 48; i++) {
        total += buf[i];
    }
    return hits ^ total;
}

// P09: first set element with an early return (known early-exit
// frontend gap; pre-registered as recall_gap_known).
uint64_t p09_first_set_const(const uint8_t *buf) {
    for (uint64_t i = 0; i < 96; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 96;
}

// P10: map-then-sum over a constant extent (known open recall gap;
// pre-registered as recall_gap_known with the D5-safe concept).
uint64_t p10_map_then_sum_const(const uint16_t *words) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (uint64_t)(words[i] ^ 0x5A);
    }
    return total;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES / CONTAINED ROWS
// ═══════════════════════════════════════════════════════════════

// N01: runtime-extent sum: recognition allowed, no candidate may be
// fabricated without a proven extent (contained_abstain).
uint64_t n01_runtime_sum_u8(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

// N02: runtime-extent cardinality (contained_abstain).
uint64_t n02_runtime_count_ge_u32(const uint32_t *vals, uint64_t n, uint32_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (vals[i] >= floor_v);
    }
    return hits;
}

// N03: signed nsw accumulation (contained_abstain: concept may be true,
// authorization must refuse without overflow semantics).
int64_t n03_sum_i64_nsw(const int64_t *vals, uint64_t n) {
    int64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += vals[i];
    }
    return total;
}

// N04: running max seeded from a parameter (X06 class; never a
// reduction).
uint16_t n04_running_max_const(const uint16_t *vals, uint16_t init) {
    uint16_t m = init;
    for (uint64_t i = 0; i < 64; i++) {
        if (vals[i] > m) {
            m = vals[i];
        }
    }
    return m;
}

// N05: masked-guard sum, constant extent (D5 class: not a raw-element
// sum even with a proven extent).
uint64_t n05_masked_sum_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] & 1) {
            total += buf[i];
        }
    }
    return total;
}

// N06: fixed stride 4 over a constant extent (not a unit-stride domain,
// so no promotion and no reduction).
uint64_t n06_stride4_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i += 4) {
        total += buf[i];
    }
    return total;
}

// N07: volatile store in the loop.
uint64_t n07_volatile_store(volatile uint8_t *out, uint64_t n, uint8_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = v;
        c += (out[i] == v);
    }
    return c;
}

// N08: C11 atomic loads.
uint64_t n08_atomic_load(const _Atomic uint32_t *flags, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += atomic_load_explicit(&flags[i], memory_order_relaxed);
    }
    return total;
}

// N09: may-alias store inside the loop.
uint64_t n09_may_alias_store(uint16_t *buf, uint16_t *out, uint64_t n, uint16_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N10: two-array relation over a constant extent (footprint
// completeness: two bases, not a single-collection reduction).
uint64_t n10_two_arrays_const(const uint8_t *a, const uint8_t *b) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < 32; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N11: unsigned reverse search with `i >= 0` (always true): recognition
// is allowed, but the reverse-totality witness must withhold the
// PositionSearch certificate, so no candidate may be generated.
uint64_t n11_nonterminating_reverse(const uint8_t *buf) {
    uint64_t found = 0;
    uint64_t idx = 96;
    for (uint64_t i = 95; i >= 0; i--) {
        uint64_t hit = (buf[i] != 0);
        if (!found && hit) {
            idx = i;
        }
        found |= hit;
    }
    return idx;
}

// N12: early exit with a global side effect.
static uint64_t g_sink;

uint64_t n12_early_exit_global(const uint8_t *buf, uint8_t t) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] == t) {
            g_sink += i;
            return i;
        }
    }
    return 64;
}
