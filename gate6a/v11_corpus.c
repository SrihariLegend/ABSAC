// Gate 6A-v11 — fresh held-out corpus (Generation H11)
//
// BLINDNESS PROTOCOL: classified BEFORE the run (v11_expected.csv) and
// sealed (v11_manifest.sha256) before the frozen candidate runs once.
// Raw output is committed before inspection; failures promote this
// corpus to the next regression set. Never reused as held-out proof.
//
// Frontier: the early-exit synthesis just gained (a) merge-form matching
// (identity vs umin clamp), (b) a guard-on-trip-bound requirement, and
// (c) latch selection that ignores the entry guard. v11 blind-tests
// unseen header polarities, predicates, sentinels and element widths on
// runtime and constant searches, plus stable positives and the safety
// classes.

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVES / PROBES
// ═══════════════════════════════════════════════════════════════

// P01: runtime search for a ZERO element (negated load predicate)
uint64_t p01_runtime_search_zero_elem(const uint8_t *buf, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (!buf[i]) return i;
    }
    return n;
}

// P02: runtime search with an Eq-key predicate and -1 sentinel
uint64_t p02_runtime_search_eq_key_neg1(const uint8_t *buf, uint64_t n, uint8_t key) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == key) return i;
    }
    return (uint64_t)-1;
}

// P03: runtime search with a Ne-key predicate and n sentinel
uint64_t p03_runtime_search_ne_key_n(const uint8_t *buf, uint64_t n, uint8_t key) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] != key) return i;
    }
    return n;
}

// P04: u16 runtime search
uint64_t p04_runtime_search_u16(const uint16_t *vals, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (vals[i]) return i;
    }
    return n;
}

// P05: computed n-1 sentinel (previous known gap)
uint64_t p05_runtime_search_sentinel_nm1(const uint8_t *buf, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i]) return i;
    }
    return n - 1;
}

// P06: do-while runtime search (forced first iteration)
uint64_t p06_runtime_search_do_while(const uint8_t *buf, uint64_t n) {
    uint64_t i = 0;
    do {
        if (buf[i]) return i;
        i++;
    } while (i < n);
    return n;
}

// P07: descending runtime search
uint64_t p07_runtime_descending_search(const uint8_t *buf, uint64_t n) {
    for (uint64_t i = n; i-- > 0;) {
        if (buf[i]) return i;
    }
    return n;
}

// P08: constant search for a zero element (rewrite expected)
uint64_t p08_const_search_zero_elem(const uint8_t *buf) {
    for (uint64_t i = 0; i < 64; i++) {
        if (!buf[i]) return i;
    }
    return 64;
}

// P09: constant ordered search (rewrite expected)
uint64_t p09_const_search_ge_key(const uint8_t *buf, uint8_t key) {
    for (uint64_t i = 0; i < 64; i++) {
        if (buf[i] >= key) return i;
    }
    return 64;
}

// P10: cardinality Ne, constant extent (rewrite expected)
uint64_t p10_count_ne_const_48(const uint8_t *buf, uint8_t ref_v) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 48; i++) {
        hits += (buf[i] != ref_v);
    }
    return hits;
}

// P11: two sequential reductions
uint64_t p11_two_loops_const_56(const uint8_t *buf, uint8_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < 56; i++) {
        hits += (buf[i] == key);
    }
    uint64_t total = 0;
    for (uint64_t i = 0; i < 56; i++) {
        total += buf[i];
    }
    return hits ^ total;
}

// P12: u16 sum (recognition only)
uint64_t p12_sum_u16_const_64(const uint16_t *vals) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < 64; i++) {
        total += vals[i];
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
