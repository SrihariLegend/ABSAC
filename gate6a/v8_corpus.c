// Gate 6A-v8 — fresh held-out corpus (Generation H8)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v8_expected.csv) and
// sealed (v8_manifest.sha256) before the frozen candidate runs once.
// Raw output is committed before inspection; failures promote this
// corpus to the next regression set. Never reused as held-out proof.
//
// Frontier: early-exit search lowering (header/latch/merge -> found-flag
// SIR), scan binding up to 512 elements with the >64 symbolic identity
// path, the sentinel guard (no-hit result must equal the extent), and
// the integer predicate-collection bitscan recipe (implicit non-zero
// only). v8 probes unseen extents/types/sentinels plus the usual safety
// classes.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES / PROBES
// ═══════════════════════════════════════════════════════════════

// P01: early-return search, extent 48, sentinel 48 (solver path)
uint64_t p01_first_set_48(const uint8_t *buf) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 48;
}

// P02: early-return search, extent 96 (>64: symbolic identity path)
uint64_t p02_first_set_96(const uint8_t *buf) {
    for (uint64_t i = 0; i < 96; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 96;
}

// P03: u16 elements, extent 80 (>64), implicit non-zero predicate
uint64_t p03_first_nonzero_u16_80(const uint16_t *vals) {
    for (uint64_t i = 0; i < 80; i++) {
        if (vals[i]) {
            return i;
        }
    }
    return 80;
}

// P04: scalar-eq predicate (NOT implicit non-zero): recognition allowed,
// but the bitscan recipe must refuse to emit a ctz mask.
uint64_t p04_first_eq_key_u8_64(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] == key) {
            return i;
        }
    }
    return 64;
}

// P05: extent 128 (symbolic identity path)
uint64_t p05_first_set_128(const uint8_t *buf) {
    for (uint64_t i = 0; i < 128; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 128;
}

// P06: two sequential reductions over one constant extent (multi-loop
// composition probe)
uint64_t p06_two_loops_count_sum_const(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 40; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 40; i++) {
        total += buf[i];
    }
    return hits ^ total;
}

// P07: plain sum over a constant extent (no Sum recipe; recognition only)
uint64_t p07_sum_u8_const_112(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 112; i++) {
        total += buf[i];
    }
    return total;
}

// P08: cardinality >=, extent 56 (rewrite expected)
uint64_t p08_count_ge_const_u8_56(const uint8_t *buf, uint8_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 56; i++) {
        hits += (buf[i] >= floor_v);
    }
    return hits;
}

// P09: cardinality <=, u32, extent 72 (rewrite expected)
uint64_t p09_count_le_const_u32_72(const uint32_t *words, uint32_t ceil_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 72; i++) {
        hits += (words[i] <= ceil_v);
    }
    return hits;
}

// P10: all >=, u16, extent 40 (recognition; candidate may exist)
uint64_t p10_all_ge_const_u16_40(const uint16_t *vals, uint16_t floor_v) {
    uint64_t ok = 1;
    for (uint64_t i = 0; i < 40; i++) {
        ok &= (vals[i] >= floor_v);
    }
    return ok;
}

// P11: map-then-sum, extent 64 (known candidate gap)
uint64_t p11_map_then_sum_const(const uint16_t *vals) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (uint64_t)(vals[i] ^ 0x21);
    }
    return total;
}

// P12: sub-range search over a longer buffer: accesses 0..47, no-hit
// result 128. Recognition allowed; the sentinel guard must refuse the
// rewrite (128 != 48) and the lowering must stay native-faithful.
uint64_t p12_search_subrange_short(const uint8_t *buf) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 128;
}

// P13: search with a zero no-hit sentinel: sentinel guard must refuse
// (0 != 64), lowering native-faithful.
uint64_t p13_search_sentinel_zero(const uint8_t *buf) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return 0;
}

// P14: runtime-extent search: no constant extent, no fabricated view.
uint64_t p14_runtime_extent_search(const uint8_t *buf, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i]) {
            return i;
        }
    }
    return n;
}

// P15: post-tested do-while with stride 3 (non-unit stride; lowering
// fidelity probe, no reduction expected)
uint64_t p15_do_while_stride3_sum(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 63; i += 3) {
        total += buf[i];
    }
    return total;
}

// P16: an early-return search followed by another loop. The search's
// merge path CONTINUES into the second loop (it does not return
// directly), which the current single-search lowering does not compose;
// pre-registered as a known gap.
uint64_t p16_search_then_sum(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 40; i++) {
        if (buf[i] == key) {
            return i;
        }
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 40; i++) {
        total += buf[i];
    }
    return 40 + total;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES / CONTAINED ROWS
// ═══════════════════════════════════════════════════════════════

// N01: running max lookalike (X06 class)
uint16_t n01_running_max_const(const uint16_t *vals, uint16_t init) {
    uint16_t m = init;
    for (uint64_t i = 0; i < 64; i++) {
        if (vals[i] > m) {
            m = vals[i];
        }
    }
    return m;
}

// N02: ternary threshold masked sum (D5 class)
uint64_t n02_masked_sum_ternary_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (buf[i] > 5) ? buf[i] : 0;
    }
    return total;
}

// N03: high-nibble masked sum (D5 class)
uint64_t n03_masked_sum_hi_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (buf[i] & 0xF0);
    }
    return total;
}

// N04: volatile load over a constant extent
uint64_t n04_volatile_load_const(volatile const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 32; i++) {
        total += buf[i];
    }
    return total;
}

// N05: C11 atomic fetch-add in the loop
uint64_t n05_atomic_fetch_add(_Atomic uint32_t *counter, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += atomic_fetch_add_explicit(counter, 1, memory_order_relaxed);
    }
    return total;
}

// N06: may-alias store inside a constant-extent loop
uint64_t n06_may_alias_store_const(uint16_t *buf, uint16_t *out, uint16_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < 48; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

// N07: two-array relation over a constant extent
uint64_t n07_two_arrays_const(const uint8_t *a, const uint8_t *b) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < 36; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

// N08: unsigned `i >= 0` reverse search (totality gate)
uint64_t n08_nonterminating_reverse(const uint8_t *buf) {
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

// N09: early exit with a global side effect
static uint64_t g_sink;

uint64_t n09_early_exit_global_const(const uint8_t *buf, uint8_t t) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i] == t) {
            g_sink += i;
            return i;
        }
    }
    return 48;
}

// N10: runtime-extent sum (contained: no fabricated extent)
uint64_t n10_runtime_sum_u16(const uint16_t *vals, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += vals[i];
    }
    return total;
}

// N11: runtime-extent cardinality (contained)
uint64_t n11_runtime_count_ge_u32(const uint32_t *words, uint64_t n, uint32_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (words[i] >= floor_v);
    }
    return hits;
}

// N12: signed accumulation (contained: signed comparisons unmodeled)
int64_t n12_signed_sum_const(const int64_t *vals) {
    int64_t total = 0;
    for (int64_t i = 0; i < 32; i++) {
        total += vals[i];
    }
    return total;
}
