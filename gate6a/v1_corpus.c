// Gate 6A-v1 — Fresh held-out generalization corpus
//
// BLINDNESS PROTOCOL:
// 1. This corpus was classified BEFORE running ABSAC.
// 2. Source + expected classification + SHA256 hashes are archived.
// 3. The frozen post-remediation commit (70a3e81) runs ONCE.
// 4. Results are frozen before failures are inspected.
// 5. Failures promote this corpus to regression set D2.
// 6. This corpus must NEVER be reused as proof of held-out generalization.
//
// This corpus is Generation H1. It must never be used to tune the system.
// After the blind run, this file's classification is frozen.

#include <stdint.h>
#include <stddef.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVE CASES — expected: recognized as a reduction, no false refusal
// ═══════════════════════════════════════════════════════════════

// V01: Cardinality, signed accumulator (int32_t count), do-while shape
// Different loop shape (do-while), signed accumulator, unsigned char data
int32_t v01_count_matches_do_while(const unsigned char *src, int32_t total, unsigned char key) {
    int32_t hits = 0;
    int64_t pos = 0;
    do {
        hits += (src[pos] == key);
        pos++;
    } while (pos < (int64_t)total);
    return hits;
}

// V02: Sum, uint16 elements, uint32 accumulator
// Different element width AND accumulator width from development kernels
uint32_t v02_sum_u16(const uint16_t *samples, uint32_t count) {
    uint32_t total = 0;
    for (uint32_t idx = 0; idx < count; idx++) {
        total += samples[idx];
    }
    return total;
}

// V03: All reduction with early counter form (break-free but via flag),
//       i64 elements — different element type entirely
uint64_t v03_all_same_i64(const int64_t *values, uint64_t total, int64_t want) {
    uint64_t ok = 1;
    for (uint64_t idx = 0; idx < total; ++idx) {
        ok = ok & (values[idx] == want);
    }
    return ok;
}

// V04: Cardinality with Gt predicate and parameter threshold, u32 elements
uint64_t v04_count_above_u32(const uint32_t *vals, uint64_t n, uint32_t limit) {
    uint64_t above = 0;
    for (uint64_t i = 0; i < n; i++) {
        above += (vals[i] > limit);
    }
    return above;
}

// V05: Sum over signed char with negative values, i64 accumulator
int64_t v05_sum_signed(const signed char *data, int64_t count) {
    int64_t acc = 0;
    for (int64_t j = 0; j < count; ++j) {
        acc += data[j];
    }
    return acc;
}

// V06: Cardinality via != predicate (count non-matching), inverted orientation
uint64_t v06_count_not_equal(const uint8_t *buf, uint64_t n, uint8_t banned) {
    uint64_t result = 0;
    uint64_t i = 0;
    while (i != n) {
        result += (uint64_t)(buf[i] != banned);
        i++;
    }
    return result;
}

// V07: Two independent reductions in one function (separate loops)
// Cardinality then Sum — fusion candidate shape
uint64_t v07_count_then_sum(const uint8_t *buf, uint64_t n, uint8_t t) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] == t);
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return count + sum;
}

// V08: Sum with non-zero start bound
uint64_t v08_sum_bounded(const uint8_t *buf, uint64_t total, uint64_t skip) {
    uint64_t s = 0;
    for (uint64_t i = skip; i < total; i++) {   // non-zero start
        s += buf[i];
    }
    return s;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVE CASES — must NOT be recognized as safe reductions
// ═══════════════════════════════════════════════════════════════

// N09: Atomic load — must not be merged
uint64_t n09_atomic_load(const volatile uint8_t *flags, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        count += (flags[i] != 0);
    }
    return count;
}

// N10: Aliasing store inside the loop — accumulator reads memory
// written by the same loop
uint64_t n10_store_alias(uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        buf[i] = (uint8_t)i;        // write
        total += buf[i];            // read of possibly-same memory
    }
    return total;
}

// N11: Reverse traversal — decrementing induction variable
uint64_t n11_reverse_sum(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = n; i > 0; i--) {
        total += buf[i - 1];
    }
    return total;
}

// N12: Side-effecting call in loop body
extern void record_hit(void);
uint64_t n12_count_with_call(const uint8_t *buf, uint64_t n, uint8_t t) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == t) record_hit();  // opaque call — side effects unknown
        c += (buf[i] == t);
    }
    return c;
}

// N13: Dynamic stride — step amount is a runtime parameter
uint64_t n13_dynamic_stride(const uint8_t *buf, uint64_t n, uint64_t step) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i += step) {
        sum += buf[i];
    }
    return sum;
}

// N14: Lookalike — accumulates but into a saturating clamp (not a monoid sum)
uint64_t n14_saturating_count(const uint8_t *buf, uint64_t n, uint8_t t) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == t) {
            if (c < 1000) c = c + 1;   // conditional update — not a pure add-reduction
        }
    }
    return c;
}

// N15: Two arrays with related indices — count where a[i] == b[i]
uint64_t n15_count_equal_pairs(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t same = 0;
    for (uint64_t i = 0; i < n; i++) {
        same += (a[i] == b[i]);
    }
    return same;
}

// N16: Signed overflow difference — signed char sum with signed accumulator,
// UB on overflow differs from wrapping u64 vector accumulation
int32_t n16_sum_signed_nsw(const int8_t *buf, int32_t n) {
    int32_t s = 0;
    for (int32_t i = 0; i < n; i++) {
        s += buf[i];    // signed — vector reduction must preserve overflow semantics
    }
    return s;
}
