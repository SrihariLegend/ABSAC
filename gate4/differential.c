#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <setjmp.h>
#include <sys/mman.h>
#include <immintrin.h>

// Include witness versions (define _orig)
#include "witness_k18.h"
#include "witness_k43.h"
#include "witness_k50.h"

// Include ABSAC vectorized versions
#include "k18_absac.c"
#include "k43_absac.c"
#include "k50_absac.c"

static int tests_run = 0;
static int tests_failed = 0;

#define CHECK(cond, msg, ...) do { \
    tests_run++; \
    if (!(cond)) { \
        printf("FAIL: " msg "\n", ##__VA_ARGS__); \
        tests_failed++; \
    } \
} while(0)

// ═══════════════════════════════════════════════════════════════
// 1. Exhaustive length testing (0-128) with multiple distributions
// ═══════════════════════════════════════════════════════════════

static void test_exhaustive_lengths(void) {
    printf("=== Test 1: Exhaustive lengths 0-128 ===\n");
    uint8_t buf[256];

    for (int dist = 0; dist < 6; dist++) {
        // Fill buffer according to distribution
        for (int i = 0; i < 256; i++) {
            switch (dist) {
                case 0: buf[i] = 0x42; break;             // all-equal (val)
                case 1: buf[i] = 0x00; break;             // all-zero
                case 2: buf[i] = 0xFF; break;             // all-ones
                case 3: buf[i] = (uint8_t)(i * 37); break; // pseudo-random
                case 4: buf[i] = (uint8_t)(i & 0x0F); break; // low nibble cycling
                case 5: buf[i] = (uint8_t)((i & 1) ? 0x05 : 0x00); break; // alternating target
            }
        }

        for (uint64_t n = 0; n <= 128; n++) {
            // k18
            uint64_t r_orig = k18_all_equal_orig(buf, n, 0x42);
            uint64_t r_absac = k18_all_equal(buf, n, 0x42);
            CHECK(r_absac == r_orig, "k18 dist=%d n=%lu: absac=%lu orig=%lu", dist, n, r_absac, r_orig);

            // k43
            r_orig = k43_sum_ascii_orig(buf, n);
            r_absac = k43_sum_ascii(buf, n);
            CHECK(r_absac == r_orig, "k43 dist=%d n=%lu: absac=%lu orig=%lu", dist, n, r_absac, r_orig);

            // k50 — mask=0x0F, target=0x05
            r_orig = k50_count_masked_orig(buf, n, 0x0F, 0x05);
            r_absac = k50_count_masked(buf, n, 0x0F, 0x05);
            CHECK(r_absac == r_orig, "k50 dist=%d n=%lu: absac=%lu orig=%lu", dist, n, r_absac, r_orig);
        }
    }
    printf("  %d tests, %d failures\n", tests_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 2. Vector boundary testing — every size around 16, 32, 48, 64
// ═══════════════════════════════════════════════════════════════

static void test_vector_boundaries(void) {
    printf("=== Test 2: Vector boundaries ===\n");
    int prev_run = tests_run;
    uint8_t buf[512];
    srand(12345);
    for (int i = 0; i < 512; i++) buf[i] = (uint8_t)(rand() & 0xFF);

    // Test every size from 0 to 200 (covers 32-byte and 16-byte boundaries)
    for (uint64_t n = 0; n <= 200; n++) {
        uint64_t r_o, r_a;

        r_o = k18_all_equal_orig(buf, n, 0x42);
        r_a = k18_all_equal(buf, n, 0x42);
        CHECK(r_a == r_o, "k18 boundary n=%lu: absac=%lu orig=%lu", n, r_a, r_o);

        r_o = k43_sum_ascii_orig(buf, n);
        r_a = k43_sum_ascii(buf, n);
        CHECK(r_a == r_o, "k43 boundary n=%lu: absac=%lu orig=%lu", n, r_a, r_o);

        r_o = k50_count_masked_orig(buf, n, 0x0F, 0x05);
        r_a = k50_count_masked(buf, n, 0x0F, 0x05);
        CHECK(r_a == r_o, "k50 boundary n=%lu: absac=%lu orig=%lu", n, r_a, r_o);
    }
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 3. All alignment offsets 0-31
// ═══════════════════════════════════════════════════════════════

static void test_alignment(void) {
    printf("=== Test 3: All alignment offsets 0-31 ===\n");
    int prev_run = tests_run;
    // Over-allocate so we can test unaligned access from any offset
    uint8_t *buf = aligned_alloc(64, 512);
    srand(999);
    for (int i = 0; i < 512; i++) buf[i] = (uint8_t)(rand() & 0xFF);

    for (int align = 0; align < 32; align++) {
        for (uint64_t n = 0; n <= 128; n++) {
            uint8_t *p = buf + align;

            uint64_t r_o = k18_all_equal_orig(p, n, 0x42);
            uint64_t r_a = k18_all_equal(p, n, 0x42);
            CHECK(r_a == r_o, "k18 align=%d n=%lu: absac=%lu orig=%lu", align, n, r_a, r_o);

            r_o = k43_sum_ascii_orig(p, n);
            r_a = k43_sum_ascii(p, n);
            CHECK(r_a == r_o, "k43 align=%d n=%lu: absac=%lu orig=%lu", align, n, r_a, r_o);

            r_o = k50_count_masked_orig(p, n, 0x0F, 0x05);
            r_a = k50_count_masked(p, n, 0x0F, 0x05);
            CHECK(r_a == r_o, "k50 align=%d n=%lu: absac=%lu orig=%lu", align, n, r_a, r_o);
        }
    }
    free(buf);
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 4. Guard-page test: detect illegal over-reads
// ═══════════════════════════════════════════════════════════════

static jmp_buf guard_jmp;
static volatile int guard_triggered = 0;

static void guard_handler(int sig) {
    guard_triggered = 1;
    longjmp(guard_jmp, 1);
}

static void test_guard_page(void) {
    printf("=== Test 4: Guard-page over-read detection ===\n");
    int prev_run = tests_run;

    // Install signal handler for SIGSEGV/SIGBUS
    struct sigaction sa;
    sa.sa_handler = guard_handler;
    sa.sa_flags = 0;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS, &sa, NULL);

    // Allocate a page-aligned buffer, then protect the page after it
    long page_size = sysconf(_SC_PAGESIZE);
    uint8_t *base = mmap(NULL, page_size * 2, PROT_READ | PROT_WRITE,
                         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base == MAP_FAILED) { printf("  mmap failed, skipping\n"); return; }
    // Protect the second page
    mprotect(base + page_size, page_size, PROT_NONE);

    // Fill the first page with known data
    for (int i = 0; i < page_size; i++) base[i] = (uint8_t)(i * 37);

    // Test sizes near the page boundary — if the code over-reads, it'll
    // hit the guard page and trigger SIGSEGV
    for (int offset = page_size - 128; offset < page_size; offset++) {
        uint64_t n = page_size - offset;  // remaining bytes in first page
        if (n > 128) continue;

        guard_triggered = 0;
        if (setjmp(guard_jmp) == 0) {
            uint64_t r_o = k18_all_equal_orig(base + offset, n, 0x42);
            (void)r_o;
            CHECK(!guard_triggered, "k18 orig over-read at offset=%d n=%lu", offset, n);
        } else {
            printf("  FAIL: k18 orig SIGSEGV at offset=%d n=%lu\n", offset, n);
            tests_failed++;
        }

        guard_triggered = 0;
        if (setjmp(guard_jmp) == 0) {
            uint64_t r_a = k18_all_equal(base + offset, n, 0x42);
            (void)r_a;
            CHECK(!guard_triggered, "k18 absac over-read at offset=%d n=%lu", offset, n);
        } else {
            printf("  FAIL: k18 absac SIGSEGV at offset=%d n=%lu\n", offset, n);
            tests_failed++;
        }

        guard_triggered = 0;
        if (setjmp(guard_jmp) == 0) {
            uint64_t r_a = k43_sum_ascii(base + offset, n);
            (void)r_a;
            CHECK(!guard_triggered, "k43 absac over-read at offset=%d n=%lu", offset, n);
        } else {
            printf("  FAIL: k43 absac SIGSEGV at offset=%d n=%lu\n", offset, n);
            tests_failed++;
        }

        guard_triggered = 0;
        if (setjmp(guard_jmp) == 0) {
            uint64_t r_a = k50_count_masked(base + offset, n, 0x0F, 0x05);
            (void)r_a;
            CHECK(!guard_triggered, "k50 absac over-read at offset=%d n=%lu", offset, n);
        } else {
            printf("  FAIL: k50 absac SIGSEGV at offset=%d n=%lu\n", offset, n);
            tests_failed++;
        }
    }

    // Restore default signal handler
    signal(SIGSEGV, SIG_DFL);
    signal(SIGBUS, SIG_DFL);
    munmap(base, page_size * 2);
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 5. Adversarial byte values
// ═══════════════════════════════════════════════════════════════

static void test_adversarial_values(void) {
    printf("=== Test 5: Adversarial byte values ===\n");
    int prev_run = tests_run;
    uint8_t buf[256];

    // SWAR borrow-propagation patterns (the bug that was fixed)
    // 0x01 followed by a byte that borrows from it
    uint8_t swar_patterns[][32] = {
        {0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00}, // 01-00 alternating
        {0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01}, // all 01
        {0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80}, // high bit set
        {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}, // all zero
        {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}, // all ones
        {0x7F, 0x80, 0x7F, 0x80, 0x7F, 0x80, 0x7F, 0x80}, // boundary
        {0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08}, // ascending
        {0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0}, // nibble swap
    };

    for (int p = 0; p < 8; p++) {
        // Copy pattern into buffer repeatedly
        for (int i = 0; i < 256; i++) buf[i] = swar_patterns[p][i % 8];

        for (uint64_t n = 0; n <= 128; n++) {
            // k50 with various masks and targets
            uint8_t masks[] = {0x0F, 0xFF, 0x80, 0x01, 0x7F};
            uint8_t targets[] = {0x00, 0x01, 0x05, 0x0F, 0x80};
            for (int mi = 0; mi < 5; mi++) {
                for (int ti = 0; ti < 5; ti++) {
                    uint64_t r_o = k50_count_masked_orig(buf, n, masks[mi], targets[ti]);
                    uint64_t r_a = k50_count_masked(buf, n, masks[mi], targets[ti]);
                    CHECK(r_a == r_o,
                          "k50 pattern=%d n=%lu mask=0x%02x target=0x%02x: absac=%lu orig=%lu",
                          p, n, masks[mi], targets[ti], r_a, r_o);
                }
            }
        }
    }
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 6. k18 mismatch positions at chunk/tail boundaries
// ═══════════════════════════════════════════════════════════════

static void test_k18_mismatch_positions(void) {
    printf("=== Test 6: k18 mismatch positions at boundaries ===\n");
    int prev_run = tests_run;
    uint8_t buf[256];

    // Test: all bytes equal except one mismatch at position `pos`
    for (uint64_t n = 1; n <= 200; n++) {
        for (uint64_t pos = 0; pos < n; pos++) {
            // Fill with val
            memset(buf, 0x42, n);
            // Insert mismatch
            buf[pos] = 0x99;

            uint64_t r_o = k18_all_equal_orig(buf, n, 0x42);
            uint64_t r_a = k18_all_equal(buf, n, 0x42);
            // Should return 0 (not all equal) for both
            CHECK(r_a == r_o && r_a == 0,
                  "k18 mismatch n=%lu pos=%lu: absac=%lu orig=%lu (expected 0)",
                  n, pos, r_a, r_o);
        }
    }

    // Also test: all bytes equal (should return 1)
    for (uint64_t n = 0; n <= 200; n++) {
        memset(buf, 0x42, n);
        uint64_t r_o = k18_all_equal_orig(buf, n, 0x42);
        uint64_t r_a = k18_all_equal(buf, n, 0x42);
        CHECK(r_a == r_o && r_a == 1,
              "k18 all-equal n=%lu: absac=%lu orig=%lu (expected 1)",
              n, r_a, r_o);
    }
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 7. k43 Sum — max byte values and accumulator overflow boundary
// ═══════════════════════════════════════════════════════════════

static void test_k43_sum_overflow(void) {
    printf("=== Test 7: k43 Sum — max values and overflow boundary ===\n");
    int prev_run = tests_run;
    uint8_t buf[256];

    // All 0xFF (max byte value) — tests accumulator overflow
    // Sum of n bytes at 0xFF = n * 255
    // For n=256: 256*255 = 65280, fits in u64
    // For n=256000+ : would overflow u32 but not u64
    memset(buf, 0xFF, 256);
    for (uint64_t n = 0; n <= 256; n++) {
        uint64_t r_o = k43_sum_ascii_orig(buf, n);
        uint64_t r_a = k43_sum_ascii(buf, n);
        uint64_t expected = n * 255;
        CHECK(r_a == r_o, "k43 0xFF n=%lu: absac=%lu orig=%lu", n, r_a, r_o);
        CHECK(r_a == expected, "k43 0xFF n=%lu: absac=%lu expected=%lu", n, r_a, expected);
    }

    // Large buffer with max values — test near u32 overflow boundary
    // u32 max = 4294967295. At 255/byte, overflow at ~16843009 bytes
    // We can't test that large in this harness, but test 65536
    uint8_t *bigbuf = aligned_alloc(64, 1048576);
    memset(bigbuf, 0xFF, 1048576);
    for (uint64_t n = 65536; n <= 65536; n += 4096) {
        uint64_t r_o = k43_sum_ascii_orig(bigbuf, n);
        uint64_t r_a = k43_sum_ascii(bigbuf, n);
        CHECK(r_a == r_o, "k43 0xFF large n=%lu: absac=%lu orig=%lu", n, r_a, r_o);
    }

    // Mix of 0x00 and 0xFF
    for (int i = 0; i < 256; i++) buf[i] = (i & 1) ? 0xFF : 0x00;
    for (uint64_t n = 0; n <= 256; n++) {
        uint64_t r_o = k43_sum_ascii_orig(buf, n);
        uint64_t r_a = k43_sum_ascii(buf, n);
        CHECK(r_a == r_o, "k43 mixed n=%lu: absac=%lu orig=%lu", n, r_a, r_o);
    }

    free(bigbuf);
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 8. Random differential execution (large-scale)
// ═══════════════════════════════════════════════════════════════

static void test_random_differential(void) {
    printf("=== Test 8: Random differential execution ===\n");
    int prev_run = tests_run;
    uint8_t *buf = aligned_alloc(64, 1048576);
    srand(42424242);

    for (int trial = 0; trial < 10000; trial++) {
        // Random length
        uint64_t n = (uint64_t)(rand() % 100000);
        // Random offset
        uint64_t offset = (uint64_t)(rand() & 0xFFF);
        uint8_t *p = buf + offset;

        // Fill with random data
        for (uint64_t i = 0; i < n + 32; i++) p[i] = (uint8_t)(rand() & 0xFF);

        uint8_t val = (uint8_t)(rand() & 0xFF);
        uint8_t mask = (uint8_t)(rand() & 0xFF);
        uint8_t target = (uint8_t)(rand() & 0xFF);

        uint64_t r_o, r_a;

        r_o = k18_all_equal_orig(p, n, val);
        r_a = k18_all_equal(p, n, val);
        CHECK(r_a == r_o, "k18 random trial=%d n=%lu: absac=%lu orig=%lu", trial, n, r_a, r_o);

        r_o = k43_sum_ascii_orig(p, n);
        r_a = k43_sum_ascii(p, n);
        CHECK(r_a == r_o, "k43 random trial=%d n=%lu: absac=%lu orig=%lu", trial, n, r_a, r_o);

        r_o = k50_count_masked_orig(p, n, mask, target);
        r_a = k50_count_masked(p, n, mask, target);
        CHECK(r_a == r_o, "k50 random trial=%d n=%lu mask=0x%02x target=0x%02x: absac=%lu orig=%lu",
              trial, n, mask, target, r_a, r_o);
    }
    free(buf);
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 9. All mask/target combinations for k50 (exhaustive per byte)
// ═══════════════════════════════════════════════════════════════

static void test_k50_all_mask_target(void) {
    printf("=== Test 9: k50 all mask/target combinations (1 byte) ===\n");
    int prev_run = tests_run;
    uint8_t buf[1];

    for (int b = 0; b < 256; b++) {
        buf[0] = (uint8_t)b;
        for (int m = 0; m < 256; m++) {
            for (int t = 0; t < 256; t++) {
                uint64_t r_o = k50_count_masked_orig(buf, 1, (uint8_t)m, (uint8_t)t);
                uint64_t r_a = k50_count_masked(buf, 1, (uint8_t)m, (uint8_t)t);
                CHECK(r_a == r_o,
                      "k50 byte=0x%02x mask=0x%02x target=0x%02x: absac=%lu orig=%lu",
                      b, m, t, r_a, r_o);
            }
        }
    }
    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════
// 10. Null pointer at zero length
// ═══════════════════════════════════════════════════════════════

static void test_null_zero_length(void) {
    printf("=== Test 10: NULL pointer at zero length ===\n");
    int prev_run = tests_run;

    // At n=0, the functions should not dereference the pointer.
    // Pass NULL — if the code checks n==0 first, it's safe.
    // Note: this is technically UB, but we want to verify no dereference occurs.
    uint8_t *null_ptr = NULL;

    // k18 at n=0 should return 1 (vacuously true)
    uint64_t r = k18_all_equal(null_ptr, 0, 0x42);
    CHECK(r == 1, "k18 n=0 NULL: returned %lu (expected 1)", r);

    // k43 at n=0 should return 0
    r = k43_sum_ascii(null_ptr, 0);
    CHECK(r == 0, "k43 n=0 NULL: returned %lu (expected 0)", r);

    // k50 at n=0 should return 0
    r = k50_count_masked(null_ptr, 0, 0x0F, 0x05);
    CHECK(r == 0, "k50 n=0 NULL: returned %lu (expected 0)", r);

    printf("  %d tests, %d failures\n", tests_run - prev_run, tests_failed);
}

// ═══════════════════════════════════════════════════════════════

int main(void) {
    printf("╔══════════════════════════════════════════════════════════════╗\n");
    printf("║  Gate 4 — Strengthened Differential Validation              ║\n");
    printf("╚══════════════════════════════════════════════════════════════╝\n\n");

    test_exhaustive_lengths();
    printf("\n");
    test_vector_boundaries();
    printf("\n");
    test_alignment();
    printf("\n");
    test_guard_page();
    printf("\n");
    test_adversarial_values();
    printf("\n");
    test_k18_mismatch_positions();
    printf("\n");
    test_k43_sum_overflow();
    printf("\n");
    test_random_differential();
    printf("\n");
    test_k50_all_mask_target();
    printf("\n");
    test_null_zero_length();

    printf("\n════════════════════════════════════════════════════════════════\n");
    printf("TOTAL: %d tests run, %d failures\n", tests_run, tests_failed);
    if (tests_failed == 0) {
        printf("ALL TESTS PASSED ✓\n");
        return 0;
    } else {
        printf("%d TESTS FAILED ✗\n", tests_failed);
        return 1;
    }
}
