// Gate 6A — Held-out generalization corpus
//
// These kernels are NOT development cases. They were written to test
// whether ABSAC's frozen recognizers and lowering generalize to unseen code.
//
// Positive cases: should be recognized and optimizable
// Negative cases: should be safely refused (no false positives)
//
// Syntactic variations from development kernels:
// - different loop forms (while instead of for)
// - different induction variable names
// - reordered expressions
// - inverted predicates
// - multiple operations in one function
// - different integer types (int instead of uint64_t)
// - constants instead of parameters
// - dead surrounding computations
// - pointer arithmetic instead of array indexing

#include <stdint.h>
#include <stddef.h>

// ═══════════════════════════════════════════════════════════════
// POSITIVE CASES — should be recognized
// ═══════════════════════════════════════════════════════════════

// H01: Sum reduction, while loop, int type, pointer arithmetic
// Variant of Sum but syntactically different
int h01_sum_while(const unsigned char *p, int len) {
    int s = 0;
    int i = 0;
    while (i < len) {
        s += *p;
        p++;
        i++;
    }
    return s;
}

// H02: All-equal check, for loop, inverted predicate, size_t
// Same semantics as k18 but different syntax
int h02_all_match(const unsigned char *buf, size_t n, unsigned char expected) {
    int match = 1;
    for (size_t j = 0; j < n; j++) {
        if (buf[j] != expected) match = 0;
    }
    return match;
}

// H03: Count non-zero, while loop, dead code surrounding
// Same semantics as k05 but different structure
uint64_t h03_count_nonzero(const uint8_t *data, uint64_t length) {
    uint64_t result = 0;
    uint64_t dummy = 42;  // dead code
    uint64_t i = 0;
    while (i < length) {
        result += (data[i] != 0);
        i = i + 1;
    }
    dummy += result;  // dead but uses result to prevent elimination
    return result;
}

// H04: Count matching with mask, for loop, reordered operands
// Same semantics as k50 but (mask & buf[i]) instead of (buf[i] & mask)
uint64_t h04_count_masked_reversed(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t cnt = 0;
    for (uint64_t k = 0; k < n; k++) {
        cnt += ((mask & buf[k]) == target);
    }
    return cnt;
}

// H05: Sum with int32 accumulator, while loop, different variable names
int32_t h05_sum_int32(const uint8_t *arr, size_t count) {
    int32_t acc = 0;
    size_t idx = 0;
    while (idx < count) {
        acc = acc + arr[idx];
        idx += 1;
    }
    return acc;
}

// H06: All-equal with constant target (not a parameter)
uint64_t h06_all_zero(const uint8_t *buf, uint64_t n) {
    uint64_t all = 1;
    for (uint64_t i = 0; i < n; i++) {
        all &= (buf[i] == 0);  // constant 0, not parameter
    }
    return all;
}

// H07: Count equal to constant
uint64_t h07_count_zero(const uint8_t *buf, uint64_t n) {
    uint64_t c = 0;
    for (uint64_t i = 0; i < n; i++) {
        c += (buf[i] == 0);
    }
    return c;
}

// H08: Count greater than threshold (predicate variant)
// This is a Cardinality with a different predicate (Lt instead of Eq)
uint64_t h08_count_above(const uint8_t *buf, uint64_t n, uint8_t threshold) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        count += (buf[i] > threshold);
    }
    return count;
}

// ═══════════════════════════════════════════════════════════════
// NEGATIVE CASES — should NOT be optimized (safe refusal)
// ═══════════════════════════════════════════════════════════════

// N01: Loop with side effect (write to output buffer)
// Should NOT be vectorized as a pure reduction
uint64_t n01_count_with_write(const uint8_t *buf, uint64_t n, uint8_t *out) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        out[i] = buf[i];  // side effect!
        count += (buf[i] != 0);
    }
    return count;
}

// N02: Volatile load — cannot assume data doesn't change between reads
uint64_t n02_volatile_read(const volatile uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) {
        sum += buf[i];
    }
    return sum;
}

// N03: Data-dependent iteration bound (loop may terminate early based on data)
uint64_t n03_early_terminate(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] == 0xFF) break;  // data-dependent exit
        sum += buf[i];
    }
    return sum;
}

// N04: Non-standard stride (not contiguous)
uint64_t n04_stride2(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < 2*n; i += 2) {  // stride 2
        sum += buf[i];
    }
    return sum;
}

// N05: Reduction with floating point (different overflow semantics)
double n05_fsum(const double *buf, uint64_t n) {
    double s = 0.0;
    for (uint64_t i = 0; i < n; i++) {
        s += buf[i];
    }
    return s;
}

// N06: Multiple reductions with data dependency between them
uint64_t n06_dependent(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        sum += buf[i];
        count += (sum > 1000);  // depends on running sum!
    }
    return count;
}

// N07: String length (null-terminated, data-dependent bound)
uint64_t n07_strlen(const char *buf) {
    uint64_t len = 0;
    while (buf[len] != 0) len++;
    return len;
}

// N08: Looks like All but isn't — it's a search that returns index
uint64_t n08_find_first_mismatch(const uint8_t *buf, uint64_t n, uint8_t val) {
    for (uint64_t i = 0; i < n; i++) {
        if (buf[i] != val) return i;  // returns position, not boolean
    }
    return n;
}
