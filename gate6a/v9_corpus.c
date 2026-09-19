// Gate 6A-v9 — fresh held-out corpus (Generation H9)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v9_expected.csv) and
// sealed (v9_manifest.sha256) before the frozen candidate runs once.
// Raw output is committed before inspection; failures promote this
// corpus to the next regression set. Never reused as held-out proof.
//
// Frontier: the position pack helper now extracts the search predicate
// (Eq/Ne/ordered, negation- and swap-normalized) from the loop's
// position select. v9 blind-tests that extraction across operators,
// element widths, extents (solver and symbolic), literal vs parameter
// scalars, and swapped operands, plus compound-predicate refusal and the
// usual safety classes.

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES / PROBES
// ═══════════════════════════════════════════════════════════════

uint64_t p01_first_eq_key_u16_72(const uint16_t *vals, uint16_t key) {
    for (uint64_t i = 0; i < 72; i++) {
        if (vals[i] == key) return i;
    }
    return 72;
}

uint64_t p02_first_ne_key_u8_64(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] != key) return i;
    }
    return 64;
}

uint64_t p03_first_ge_key_u32_56(const uint32_t *words, uint32_t key) {
    for (uint64_t i = 0; i < 56; i++) {
        if (words[i] >= key) return i;
    }
    return 56;
}

uint64_t p04_first_lt_key_u8_48(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i] < key) return i;
    }
    return 48;
}

uint64_t p05_first_gt_key_u16_80(const uint16_t *vals, uint16_t key) {
    for (uint64_t i = 0; i < 80; i++) {
        if (vals[i] > key) return i;
    }
    return 80;
}

uint64_t p06_first_eq_zero_u8_64(const uint8_t *buf) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] == 0) return i;
    }
    return 64;
}

uint64_t p07_first_ne_zero_u8_96(const uint8_t *buf) {
    for (uint64_t i = 0; i < 96; i++) {
        if (buf[i] != 0) return i;
    }
    return 96;
}

uint64_t p08_first_eq_literal_u8_56(const uint8_t *buf) {
    for (uint64_t i = 0; i < 56; i++) {
        if (buf[i] == 42) return i;
    }
    return 56;
}

uint64_t p09_first_key_lt_elem_u8_40(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 40; i++) {
        if (key < buf[i]) return i;
    }
    return 40;
}

uint64_t p10_first_key_ge_elem_u16_64(const uint16_t *vals, uint16_t key) {
    for (uint64_t i = 0; i < 64; i++) {
        if (key >= vals[i]) return i;
    }
    return 64;
}

uint64_t p11_count_ge_const_48(const uint8_t *buf, uint8_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 48; i++) {
        hits += (buf[i] >= floor_v);
    }
    return hits;
}

uint64_t p12_two_loops_count_sum_64(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 64; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += buf[i];
    }
    return hits ^ total;
}

uint64_t p13_first_eq_key_u32_128(const uint32_t *words, uint32_t key) {
    for (uint64_t i = 0; i < 128; i++) {
        if (words[i] == key) return i;
    }
    return 128;
}

// Compound predicate: recognition allowed, the recipe must refuse the
// mask (not a single comparison).
uint64_t p14_search_compound_and(const uint8_t *buf) {
    for (uint64_t i = 0; i < 64; i++) {
        if ((buf[i] & 1) && buf[i] > 3) return i;
    }
    return 64;
}

// Sub-range search: sentinel 128 != extent 48; recognition allowed,
// rewrite refused by the sentinel guard.
uint64_t p15_search_subrange_short(const uint8_t *buf) {
    for (uint64_t i = 0; i < 48; i++) {
        if (buf[i]) return i;
    }
    return 128;
}

// Runtime-extent search (known frontend gap).
uint64_t p16_runtime_extent_search(const uint8_t *buf, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i]) return i;
    }
    return n;
}

// Post-tested do-while stride 4 (lowering fidelity probe).
uint64_t p17_do_while_stride4_sum(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i += 4) {
        total += buf[i];
    }
    return total;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVES / CONTAINED ROWS
// ═══════════════════════════════════════════════════════════════

uint16_t n01_running_max_const(const uint16_t *vals, uint16_t init) {
    uint16_t m = init;
    for (uint64_t i = 0; i < 64; i++) {
        if (vals[i] > m) m = vals[i];
    }
    return m;
}

uint64_t n02_masked_sum_ternary_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (buf[i] > 5) ? buf[i] : 0;
    }
    return total;
}

uint64_t n03_masked_sum_hi_const(const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += (buf[i] & 0xF0);
    }
    return total;
}

uint64_t n04_volatile_load_const(volatile const uint8_t *buf) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 32; i++) {
        total += buf[i];
    }
    return total;
}

uint64_t n05_atomic_load_const(const _Atomic uint32_t *flags) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 32; i++) {
        total += atomic_load_explicit(&flags[i], memory_order_relaxed);
    }
    return total;
}

uint64_t n06_may_alias_store_const(uint16_t *buf, uint16_t *out, uint16_t v) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < 48; i++) {
        out[i] = v;
        c += (buf[i] == v);
    }
    return c;
}

uint64_t n07_two_arrays_const(const uint8_t *a, const uint8_t *b) {
    uint64_t eq = 0;
    for (uint64_t i = 0; i < 36; i++) {
        eq += (a[i] == b[i]);
    }
    return eq;
}

uint64_t n08_nonterminating_reverse(const uint8_t *buf) {
    uint64_t found = 0;
    uint64_t idx = 96;
    for (uint64_t i = 95; i >= 0; i--) {
        uint64_t hit = (buf[i] != 0);
        if (!found && hit) idx = i;
        found |= hit;
    }
    return idx;
}

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

uint64_t n10_runtime_sum_u16(const uint16_t *vals, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += vals[i];
    }
    return total;
}

uint64_t n11_runtime_count_ge_u32(const uint32_t *words, uint64_t n, uint32_t floor_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (words[i] >= floor_v);
    }
    return hits;
}

int64_t n12_signed_sum_const(const int64_t *vals) {
    int64_t total = 0;
    for (int64_t i = 0; i < 32; i++) {
        total += vals[i];
    }
    return total;
}
