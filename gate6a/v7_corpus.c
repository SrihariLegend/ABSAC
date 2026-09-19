// Gate 6A-v7 — fresh held-out corpus (Generation H7)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v7_expected.csv) and
// sealed (v7_manifest.sha256) before the frozen candidate runs once.
// Raw output is committed before inspection; failures promote this
// corpus to the next regression set. Never reused as held-out proof.
//
// Frontier: the v6 remediation landed two fixes — the P0A candidate
// gate now requires a region certificate, and clang's unguarded
// post-tested (do-while) carry test is reconstructed
// (`carry < K + step`) or refused. The multi-loop composer also skips
// blocks consumed by earlier loops, so constant-extent promotion and
// whole-function candidates work for multi-loop kernels. This corpus
// probes unseen variants of all three: extra do-while strides, mixed
// constant bounds, three sequential loops, and a runtime step.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES
// ═══════════════════════════════════════════════════════════════

// P01: Sum, u8 elements into u64, constant bound 100
uint64_t p01_sum_u8_const_100(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 100; i++) {
        total += buf[i];
    }
    return total;
}

// P02: Cardinality, u16 elements, <= predicate, constant bound 72
uint64_t p02_count_le_const_u16(const uint16_t *vals, uint16_t ceil_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 72; i++) {
        hits += (vals[i] <= ceil_v);
    }
    return hits;
}

// P03: Cardinality, u32 elements, != predicate, constant bound 36
uint64_t p03_count_ne_const_u32(const uint32_t *words, uint32_t ref_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 36; i++) {
        hits += (words[i] != ref_v);
    }
    return hits;
}

// P04: All, u8 elements, >= predicate, constant bound 48
uint64_t p04_all_ge_const_u8(const uint8_t *buf, uint8_t floor_v) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < 48; i++) {
        ok &= (buf[i] >= floor_v);
    }
    return ok;
}

// P05: Any, u16 raw-element OR, constant bound 80
uint64_t p05_any_or_const_u16(const uint16_t *vals) {
    uint16_t acc = 0;
    for (uint64_t i = 0; i < 80; i++) {
        acc |= vals[i];
    }
    return (acc != 0);
}

// P06: post-tested do-while with constant stride 2 (clang emits a
// carry test `i < 62`; the domain must reconstruct as `i < 64`).
// Pre-registered recall_gap_known: non-unit stride may or may not
// derive SumReduction; the NATIVE lowering must be faithful.
uint64_t p06_do_while_stride2_sum(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i += 2) {
        total += buf[i];
    }
    return total;
}

// P07: three sequential reductions over one constant extent (40):
// count (== key), sum, raw OR. Exercises the multi-loop composer and
// promotion with three loops.
uint64_t p07_three_loops_const(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 40; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 40; i++) {
        total += buf[i];
    }
    uint64_t any = 0;
    for (uint64_t i = 0; i < 40; i++) {
        any |= buf[i];
    }
    return (hits ^ total) + (any != 0);
}

// P08: two sequential reductions with DIFFERENT constant extents
// (40 and 64): the promoted view must cover the maximum extent.
uint64_t p08_two_loops_mixed_const(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 40; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += buf[i];
    }
    return hits ^ total;
}

// P09: first set element with an early return (known early-exit gap;
// pre-registered recall_gap_known).
uint64_t p09_first_set_early_return(const uint8_t *buf) {
    for (uint64_t i = 0; i < 80; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 80;
}

// P10: map-then-sum over a constant extent (known candidate gap;
// pre-registered recall_gap_known with the D5-safe concept).
uint64_t p10_map_then_sum_const(const uint16_t *vals) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 48; i++) {
        total += (uint64_t)(vals[i] ^ 0x33);
    }
    return total;
}

// P11: post-tested do-while with constant stride 8 (carry test
// `i < 56`; reconstruct as `i < 64`). Pre-registered
// recall_gap_known for the same reason as P06.
uint64_t p11_do_while_stride8_sum(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i += 8) {
        total += buf[i];
    }
    return total;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES / CONTAINED ROWS
// ═══════════════════════════════════════════════════════════════

// N01: runtime-extent sum (contained_abstain: no fabricated extent).
uint64_t n01_runtime_sum_u16(const uint16_t *vals, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += vals[i];
    }
    return total;
}

// N02: runtime-extent cardinality (contained_abstain).
uint64_t n02_runtime_count_le_u32(const uint32_t *words, uint64_t n, uint32_t ceil_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (words[i] <= ceil_v);
    }
    return hits;
}

// N03: signed accumulation with a constant bound (contained_abstain:
// signed comparisons are unmodeled and must refuse).
int64_t n03_signed_sum_const(const int64_t *vals) {
    int64_t total = 0;
    for (int64_t i = 0; i < 32; i++) {
        total += vals[i];
    }
    return total;
}

// N04: unguarded post-tested do-while with a RUNTIME step
// (contained_abstain: the reconstruction needs a constant step, so the
// lowerer must refuse loudly — never silently mis-lower).
uint64_t n04_runtime_step_do_while(const uint8_t *buf, uint64_t step) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i += step) {
        total += buf[i];
    }
    return total;
}

// N05: masked-guard sum over a constant extent (D5 class).
uint64_t n05_masked_sum_ternary_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 48; i++) {
        total += (buf[i] > 3) ? buf[i] : 0;
    }
    return total;
}

// N06: running max lookalike over a constant extent (X06 class).
uint16_t n06_running_max_const(const uint16_t *vals, uint16_t init) {
    uint16_t m = init;
    for (uint64_t i = 0; i < 48; i++) {
        if (vals[i] > m) {
            m = vals[i];
        }
    }
    return m;
}

// N07: high-nibble masked sum over a constant extent (D5 class).
uint64_t n07_masked_sum_hi_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 48; i++) {
        total += (buf[i] & 0xF0);
    }
    return total;
}

// N08: volatile load over a constant extent.
uint64_t n08_volatile_load_const(volatile const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 32; i++) {
        total += buf[i];
    }
    return total;
}

// N09: C11 atomic read-modify-write in the loop.
uint64_t n09_atomic_fetch_add(_Atomic uint32_t *counter, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += atomic_fetch_add_explicit(counter, 1, memory_order_relaxed);
    }
    return total;
}

// N10: may-alias store inside a constant-extent loop.
uint64_t n10_may_alias_store_const(uint16_t *buf, uint16_t *out, uint16_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < 48; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N11: two-array relation over a constant extent.
uint64_t n11_two_arrays_const(const uint8_t *a, const uint8_t *b) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < 36; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N12: unsigned reverse search with `i >= 0` (totality gate).
uint64_t n12_nonterminating_reverse(const uint8_t *buf) {
    uint64_t found = 0;
    uint64_t idx = 80;
    for (uint64_t i = 79; i >= 0; i--) {
        uint64_t hit = (buf[i] != 0);
        if (!found && hit) {
            idx = i;
        }
        found |= hit;
    }
    return idx;
}

// N13: early exit with a global side effect.
static uint64_t g_sink;

uint64_t n13_early_exit_global_const(const uint8_t *buf, uint8_t t) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i] == t) {
            g_sink += i;
            return i;
        }
    }
    return 48;
}
