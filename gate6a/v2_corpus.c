// Gate 6A-v2 — Fresh held-out generalization corpus (Generation H2)
//
// BLINDNESS PROTOCOL:
// 1. This corpus was classified BEFORE running ABSAC.
// 2. Source + expected classification + SHA256 hashes are archived
//    (gate6a/archive_v2.sh) BEFORE the frozen candidate runs.
// 3. The frozen candidate (commit 5355778, "C2") runs ONCE.
// 4. Results are frozen before failures are inspected.
// 5. Failures promote this corpus to regression set D3.
// 6. This corpus must NEVER be reused as proof of held-out generalization.
//
// Design: includes metamorphic near-miss pairs — each negative differs
// from a valid positive by exactly one semantic property (advisor:
// generative adversarial corpus).

#include <stdint.h>
#include <stddef.h>
#include <stdatomic.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVE CASES — expected: complete recognition, no false refusal
// ═══════════════════════════════════════════════════════════════

// W01: Cardinality, i16 elements, zext to u64 count
uint64_t w01_count_hits_i16(const int16_t *vals, uint64_t n, int16_t key) {
    uint64_t hits = 0;
    for (uint64_t i = 0; i < n; i++) {
        hits += (uint64_t)(vals[i] == 42);
    }
    return hits;
}

// W02: Sum, u32 elements, u64 accumulator
uint64_t w02_sum_u32(const uint32_t *samples, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        total += samples[i];
    }
    return total;
}

// W03: Sum of transformed elements (map-then-sum): total += buf[i] + 1
uint64_t w03_sum_plus_one(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i++) {
        s += (uint64_t)(buf[i] + 1);
    }
    return s;
}

// W04: All over signed bytes (all equal to constant), select-reset form
int64_t w04_all_equal_i8(const int8_t *data, int64_t n, int8_t want) {
    int64_t ok = 1;
    for (int64_t j = 0; j < n; ++j) {
        ok = (data[j] == want) ? ok : 0;
    }
    return ok;
}

// W05: Cardinality counting positive values (signed Gt zero), i32
uint64_t w05_count_positive(const int32_t *vals, uint64_t n) {
    uint64_t pos = 0;
    for (uint64_t i = 0; i < n; ++i) {
        pos += (uint64_t)(vals[i] > 0);
    }
    return pos;
}

// W06: Cardinality over != (count mismatches), with constant length
uint64_t w06_count_mismatch_const(const uint8_t *buf, uint8_t ref_b) {
    uint64_t bad = 0;
    for (uint64_t i = 0; i < 256; i++) {   // constant bound
        bad += (uint64_t)(buf[i] != ref_b);
    }
    return bad;
}

// W07: All over unsigned bytes (all >= min), early-counter-free form
uint8_t w07_all_min(const uint8_t *b, uint32_t len, uint8_t floor_v) {
    uint8_t ok = 1;
    for (uint32_t k = 0; k < len; ++k) {
        ok = ok & (uint8_t)(b[k] >= floor_v ? 1 : 0);
    }
    return ok;
}

// W08: Two independent reductions in one function (fusion shape, new types)
// Cardinality + Sum over the same buffer
uint64_t w08_two_reductions(const uint8_t *a, uint64_t n, uint8_t t) {
    uint64_t cnt = 0;
    for (uint64_t i = 0; i < n; i++) cnt += (a[i] == t);
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += a[i];
    return cnt + sum;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVE CASES — must NOT be recognized as safe reductions
// ═══════════════════════════════════════════════════════════════

// X01: Volatile STORE inside the loop (writes observable memory)
uint64_t x01_volatile_store(uint8_t *buf, uint64_t n, uint8_t t) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        *(volatile uint8_t *)&buf[i] = t;   // volatile write each iteration
        c += (buf[i] == t);
    }
    return c;
}

// X02: Signed overflow-sensitive sum (i32 acc, nsw attributed by frontend)
// Vector accumulation must preserve signed overflow semantics; SIR
// currently models Wrapping only — must not authorize.
int32_t x02_sum_i32_nsw(const int32_t *buf, int32_t n) {
    int32_t s = 0;
    for (int32_t i = 0; i < n; i++) {
        s += buf[i];    // signed overflow UB in C; frontend may add nsw
    }
    return s;
}

// X03: Shifted access function — accumulator reads buf[i-1] (sliding
// window). Still one base, still stride 1, but the element map is
// f(i) = buf[i] + buf[i-1]: two access functions, boundary-dependent.
// A single-input reduction certificate cannot bind this completely.
int64_t x03_sliding_sum(const uint8_t *buf, int64_t n) {
    int64_t s = 0;
    for (int64_t i = 1; i < n; i++) {
        s += buf[i] + buf[i - 1];   // reads location i-1 written? no — read-only
        // but access function involves i-1: different iteration coupling
    }
    return s;
}

// X04: Fixed stride 4 (non-unit, constant — distinct from dynamic stride)
uint64_t x04_stride4(const uint8_t *buf, uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; i += 4) {
        s += buf[i];
    }
    return s;
}

// X05: Atomic loads (C11 _Atomic) — ordering must not be merged
uint64_t x05_atomic_flags(const _Atomic uint8_t *flags, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        count += (atomic_load_explicit(&flags[i], memory_order_acquire) != 0);
    }
    return count;
}

// X06: Max-reduction lookalike (not an All/Sum/Cardinality)
uint8_t x06_running_max(const uint8_t *buf, uint64_t n) {
    uint8_t m = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] > m) m = buf[i];
    }
    return m;
}

// X07: Self-referential recurrence — accumulator used inside its own
// update beyond a pure reduction (acc = acc + (acc % 3))
uint64_t x07_self_recurrence(const uint8_t *buf, uint64_t n) {
    uint64_t acc = 1;
    for (uint64_t i = 0; i < n; i++) {
        acc = acc + (acc % 3) + buf[i];   // not a pure element sum
    }
    return acc;
}

// X08: Early-exit with side effect — breaks out and writes a flag
static uint64_t g_found;
uint64_t x08_early_exit_write(uint8_t *buf, uint64_t n, uint8_t t) {
    g_found = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == t) {
            g_found = 1;         // writes non-local state
            return i;
        }
    }
    return (uint64_t)n;
}
