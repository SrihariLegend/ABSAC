// ABSAC Kernel Corpus v1.0 — 50 real-world kernel patterns
//
// Each function is a self-contained kernel that could appear in a real
// codebase. They are organized by category. All are pure (no side effects,
// no heap allocation, no function calls except intrinsics) so they can be
// auto-lowered from LLVM IR to SIR.
//
// Targeting criteria: kernels where clang -O3 might NOT already be optimal,
// because of lookup tables, conditional logic, SWAR patterns, or semantic
// equivalences that require mathematical recognition.

#include <stdint.h>

// ═══════════════════════════════════════════════════════════════
// Category 1: Bit counting / population count variants (10)
// ═══════════════════════════════════════════════════════════════

// K01: Count set bits via AND 1 in a loop
uint64_t k01_count_bits_and1(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += buf[i] & 1;
    return count;
}

// K02: Count set bits via lookup table (Redis bitsinbyte pattern)
static const uint8_t popcount_table[256] = {
    0,1,1,2,1,2,2,3,1,2,2,3,2,3,3,4,1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    1,2,2,3,2,3,3,4,2,3,3,4,3,4,4,5,2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    2,3,3,4,3,4,4,5,3,4,4,5,4,5,5,6,3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,
    3,4,4,5,4,5,5,6,4,5,5,6,5,6,6,7,4,5,5,6,5,6,6,7,5,6,6,7,6,7,7,8,
};
uint64_t k02_count_bits_table(const uint8_t *buf, uint64_t n) {
    uint64_t bits = 0;
    for (uint64_t i = 0; i < n; i++) bits += popcount_table[buf[i]];
    return bits;
}

// K03: Brian Kernighan's bit count on each byte
uint64_t k03_kernighan_bytes(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t x = buf[i];
        uint8_t c = 0;
        while (x) { c++; x &= x - 1; }
        total += c;
    }
    return total;
}

// K04: Parity of all bytes (XOR all, check bit 0)
uint64_t k04_parity_bytes(const uint8_t *buf, uint64_t n) {
    uint8_t x = 0;
    for (uint64_t i = 0; i < n; i++) x ^= buf[i];
    return x & 1;
}

// K05: Count bytes with at least one bit set (nonzero bytes)
uint64_t k05_count_nonzero(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] != 0);
    return count;
}

// K06: Count bytes with exactly one bit set (power of 2)
uint64_t k06_count_pow2(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t x = buf[i];
        count += (x != 0) && ((x & (x - 1)) == 0);
    }
    return count;
}

// K07: Count trailing zeros across all bytes (sum of ctz)
uint64_t k07_sum_trailing_zeros(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t x = buf[i];
        if (x == 0) { total += 8; continue; }
        uint8_t c = 0;
        while (!(x & 1)) { c++; x >>= 1; }
        total += c;
    }
    return total;
}

// K08: Count leading zeros across all bytes (sum of clz)
uint64_t k08_sum_leading_zeros(const uint8_t *buf, uint64_t n) {
    uint64_t total = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t x = buf[i];
        if (x == 0) { total += 8; continue; }
        uint8_t c = 0;
        while (!(x & 0x80)) { c++; x <<= 1; }
        total += c;
    }
    return total;
}

// K09: Check if any byte has its high bit set (sign bit)
uint64_t k09_any_high_bit(const uint8_t *buf, uint64_t n) {
    uint64_t found = 0;
    for (uint64_t i = 0; i < n; i++) found |= (buf[i] >> 7);
    return found;
}

// K10: Count bytes where bit 3 is set
uint64_t k10_count_bit3(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] >> 3) & 1;
    return count;
}

// ═══════════════════════════════════════════════════════════════
// Category 2: Comparison / matching (10)
// ═══════════════════════════════════════════════════════════════

// K11: Constant-time comparison (OpenSSL CRYPTO_memcmp pattern)
uint64_t k11_ct_memcmp(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint8_t diff = 0;
    for (uint64_t i = 0; i < n; i++) diff |= a[i] ^ b[i];
    return diff;
}

// K12: Count matching bytes between two arrays
uint64_t k12_count_matches(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (a[i] == b[i]);
    return count;
}

// K13: Count mismatching bytes
uint64_t k13_count_mismatches(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (a[i] != b[i]);
    return count;
}

// K14: Find minimum element
uint8_t k14_min_element(const uint8_t *buf, uint64_t n) {
    uint8_t m = 255;
    for (uint64_t i = 0; i < n; i++)
        m = (buf[i] < m) ? buf[i] : m;
    return m;
}

// K15: Find maximum element
uint8_t k15_max_element(const uint8_t *buf, uint64_t n) {
    uint8_t m = 0;
    for (uint64_t i = 0; i < n; i++)
        m = (buf[i] > m) ? buf[i] : m;
    return m;
}

// K16: Count elements greater than threshold
uint64_t k16_count_gt(const uint8_t *buf, uint64_t n, uint8_t threshold) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] > threshold);
    return count;
}

// K17: Count elements less than threshold
uint64_t k17_count_lt(const uint8_t *buf, uint64_t n, uint8_t threshold) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] < threshold);
    return count;
}

// K18: Check if all elements equal a value
uint64_t k18_all_equal(const uint8_t *buf, uint64_t n, uint8_t val) {
    uint64_t all = 1;
    for (uint64_t i = 0; i < n; i++) all &= (buf[i] == val);
    return all;
}

// K19: Check if any element equals a value
uint64_t k19_any_equal(const uint8_t *buf, uint64_t n, uint8_t val) {
    uint64_t found = 0;
    for (uint64_t i = 0; i < n; i++) found |= (buf[i] == val);
    return found;
}

// K20: Sum of absolute differences
uint64_t k20_sad(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t d = a[i] > b[i] ? a[i] - b[i] : b[i] - a[i];
        sum += d;
    }
    return sum;
}

// ═══════════════════════════════════════════════════════════════
// Category 3: Checksums / hashing (8)
// ═══════════════════════════════════════════════════════════════

// K21: XOR checksum
uint8_t k21_xor_checksum(const uint8_t *buf, uint64_t n) {
    uint8_t x = 0;
    for (uint64_t i = 0; i < n; i++) x ^= buf[i];
    return x;
}

// K22: Additive checksum (mod 256)
uint8_t k22_add_checksum(const uint8_t *buf, uint64_t n) {
    uint8_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}

// K23: FNV-1a hash (simplified, 32-bit)
uint32_t k23_fnv1a(const uint8_t *buf, uint64_t n) {
    uint32_t hash = 2166136261u;
    for (uint64_t i = 0; i < n; i++) {
        hash ^= buf[i];
        hash *= 16777619u;
    }
    return hash;
}

// K24: DJB2 hash
uint32_t k24_djb2(const uint8_t *buf, uint64_t n) {
    uint32_t hash = 5381u;
    for (uint64_t i = 0; i < n; i++)
        hash = ((hash << 5) + hash) + buf[i];
    return hash;
}

// K25: Sum of elements (simple reduction)
uint64_t k25_sum(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}

// K26: Sum of squares
uint64_t k26_sum_squares(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += (uint64_t)buf[i] * buf[i];
    return sum;
}

// K27: Weighted sum (index * value)
uint64_t k27_weighted_sum(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += i * buf[i];
    return sum;
}

// K28: Checksum with rotate (simplified CRC-like)
uint32_t k28_crc_simple(const uint8_t *buf, uint64_t n) {
    uint32_t crc = 0xFFFFFFFFu;
    for (uint64_t i = 0; i < n; i++) {
        crc ^= buf[i];
        for (int j = 0; j < 8; j++) {
            if (crc & 1) crc = (crc >> 1) ^ 0xEDB88320u;
            else crc >>= 1;
        }
    }
    return crc;
}

// ═══════════════════════════════════════════════════════════════
// Category 4: Boolean array operations (8)
// ═══════════════════════════════════════════════════════════════

// K29: Count true elements in bool array
uint64_t k29_count_true(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += buf[i] != 0;
    return count;
}

// K30: Check if all true (AND reduction)
uint64_t k30_all_true(const uint8_t *buf, uint64_t n) {
    uint64_t all = 1;
    for (uint64_t i = 0; i < n; i++) all &= (buf[i] != 0);
    return all;
}

// K31: Check if any true (OR reduction)
uint64_t k31_any_true(const uint8_t *buf, uint64_t n) {
    uint64_t found = 0;
    for (uint64_t i = 0; i < n; i++) found |= (buf[i] != 0);
    return found;
}

// K32: OR of two arrays (boolean union)
void k32_bool_union(const uint8_t *a, const uint8_t *b, uint8_t *out, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) out[i] = a[i] | b[i];
}

// K33: AND of two arrays (boolean intersection)
void k33_bool_intersect(const uint8_t *a, const uint8_t *b, uint8_t *out, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) out[i] = a[i] & b[i];
}

// K34: XOR of two arrays (boolean symmetric difference)
void k34_bool_xor(const uint8_t *a, const uint8_t *b, uint8_t *out, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) out[i] = a[i] ^ b[i];
}

// K35: Count positions where both arrays are true
uint64_t k35_count_both_true(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (a[i] != 0) && (b[i] != 0);
    return count;
}

// K36: Count positions where exactly one is true (XOR count)
uint64_t k36_count_xor_true(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (a[i] != 0) != (b[i] != 0);
    return count;
}

// ═══════════════════════════════════════════════════════════════
// Category 5: String / text operations (7)
// ═══════════════════════════════════════════════════════════════

// K37: String length (find null terminator)
uint64_t k37_strlen(const uint8_t *buf, uint64_t max_len) {
    uint64_t len = 0;
    while (len < max_len && buf[len] != 0) len++;
    return len;
}

// K38: Count occurrences of a character
uint64_t k38_count_char(const uint8_t *buf, uint64_t n, uint8_t c) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] == c);
    return count;
}

// K39: Check if string contains a character
uint64_t k39_contains_char(const uint8_t *buf, uint64_t n, uint8_t c) {
    uint64_t found = 0;
    for (uint64_t i = 0; i < n; i++) found |= (buf[i] == c);
    return found;
}

// K40: Count whitespace characters (space, tab, newline)
uint64_t k40_count_whitespace(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t c = buf[i];
        count += (c == ' ') | (c == '\t') | (c == '\n') | (c == '\r');
    }
    return count;
}

// K41: Count uppercase letters (A-Z)
uint64_t k41_count_upper(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t c = buf[i];
        count += (c >= 'A') & (c <= 'Z');
    }
    return count;
}

// K42: Count digits (0-9)
uint64_t k42_count_digits(const uint8_t *buf, uint64_t n) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) {
        uint8_t c = buf[i];
        count += (c >= '0') & (c <= '9');
    }
    return count;
}

// K43: Sum of ASCII values
uint64_t k43_sum_ascii(const uint8_t *buf, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i];
    return sum;
}

// ═══════════════════════════════════════════════════════════════
// Category 6: Mathematical / numerical (7)
// ═══════════════════════════════════════════════════════════════

// K44: Dot product of two uint8 arrays
uint64_t k44_dot_product(const uint8_t *a, const uint8_t *b, uint64_t n) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += (uint64_t)a[i] * b[i];
    return sum;
}

// K45: Count elements in range [lo, hi]
uint64_t k45_count_in_range(const uint8_t *buf, uint64_t n, uint8_t lo, uint8_t hi) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++)
        count += (buf[i] >= lo) & (buf[i] <= hi);
    return count;
}

// K46: Modulo reduction (sum of buf[i] % modulus)
uint64_t k46_sum_mod(const uint8_t *buf, uint64_t n, uint8_t modulus) {
    uint64_t sum = 0;
    for (uint64_t i = 0; i < n; i++) sum += buf[i] % modulus;
    return sum;
}

// K47: Count elements divisible by divisor
uint64_t k47_count_divisible(const uint8_t *buf, uint64_t n, uint8_t divisor) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++) count += (buf[i] % divisor == 0);
    return count;
}

// K48: Bitwise AND of all elements
uint8_t k48_and_all(const uint8_t *buf, uint64_t n) {
    uint8_t result = 0xFF;
    for (uint64_t i = 0; i < n; i++) result &= buf[i];
    return result;
}

// K49: Bitwise OR of all elements (already seen in K09, but different shape)
uint8_t k49_or_all(const uint8_t *buf, uint64_t n) {
    uint8_t result = 0;
    for (uint64_t i = 0; i < n; i++) result |= buf[i];
    return result;
}

// K50: Mixed: count bytes where (byte & mask) == target
uint64_t k50_count_masked(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {
    uint64_t count = 0;
    for (uint64_t i = 0; i < n; i++)
        count += ((buf[i] & mask) == target);
    return count;
}
