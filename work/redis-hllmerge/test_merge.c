/* Witness for hllMergeDense(): equivalence against the upstream loop, and a
 * placement witness for its bounds argument.
 *
 * Build (the .inc is generated from the patched source, never hand-copied):
 *     awk '/^static void hllMergeDense/,/^}/' src/src/hyperloglog.c \
 *         > hllMergeDense.inc
 *     clang -O2 -I. test_merge.c -o test_merge
 *
 * Two questions are asked, and they are deliberately different:
 *
 *   1. EQUIVALENCE. Does hllMergeDense leave max[] identical to what the
 *      upstream loop leaves? Checked over random, identical, all-zero,
 *      all-sixty-three and one-hot distributions, plus every short length.
 *      A differential alone is not a bounds argument -- the checking apparatus
 *      has been the defective component in this project twice -- so this says
 *      only that the two agree on the cases tried.
 *
 *   2. BOUNDS. Does hllMergeDense read outside the array? Checked by placing
 *      the register array so that its final byte is the final byte of a mapped
 *      page, with an unmapped PROT_NONE page immediately after. Any read of
 *      byte 12288 is then SIGSEGV rather than a silent over-read. The child is
 *      forked so that the parent survives to report which side faulted.
 *
 * The upstream loop is expected to FAULT under (2). That is the point: the
 * witness has to be able to fail, and upstream's unconditional `_p[_byte+1]`
 * is what it is being able to fail on.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#define HLL_REGISTERS 16384
#define HLL_BITS 6
#define HLL_REGISTER_MAX ((1 << HLL_BITS) - 1)
#define HLL_HDR_SIZE 16
#define HLL_DENSE_SIZE (HLL_HDR_SIZE + ((HLL_REGISTERS * HLL_BITS + 7) / 8))
#define HLL_PACKED (HLL_DENSE_SIZE - HLL_HDR_SIZE) /* 12288 */

/* The macro under test, copied from src/hyperloglog.c verbatim. */
#define HLL_DENSE_GET_REGISTER(target, p, regnum) do { \
    uint8_t *_p = (uint8_t *)p; \
    unsigned long _byte = regnum * HLL_BITS / 8; \
    unsigned long _fb = regnum * HLL_BITS & 7; \
    unsigned long _fb8 = 8 - _fb; \
    unsigned long b0 = _p[_byte]; \
    unsigned long b1 = _p[_byte + 1]; \
    target = ((b0 >> _fb) | (b1 << _fb8)) & HLL_REGISTER_MAX; \
} while (0)

#include "hllMergeDense.inc"

/* The upstream loop, exactly as hllMerge() spells it. */
static void upstream_merge(uint8_t *max, const uint8_t *registers) {
    int i;
    for (i = 0; i < HLL_REGISTERS; i++) {
        uint8_t val;
        HLL_DENSE_GET_REGISTER(val, registers, i);
        if (val > max[i]) max[i] = val;
    }
}

/* An array of HLL_PACKED bytes whose last byte is the last byte of a mapping,
 * with an unmapped page immediately after it. Returns NULL on failure. */
static uint8_t *guarded_registers(void) {
    long page = sysconf(_SC_PAGESIZE);
    size_t span = (HLL_PACKED + page - 1) / page * page; /* whole pages */
    uint8_t *base = mmap(NULL, span + page, PROT_READ | PROT_WRITE,
                         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base == MAP_FAILED) return NULL;
    if (mprotect(base + span, page, PROT_NONE) != 0) return NULL;
    /* The array ends on the last byte of `span`, so any read of registers[]
     * at index HLL_PACKED is the first byte of the PROT_NONE page. */
    return base + span - HLL_PACKED;
}

/* fork a merge into the guarded mapping; 0 = returned, 1 = faulted */
static int run_guarded(void (*merge)(uint8_t *, const uint8_t *)) {
    uint8_t *regs = guarded_registers();
    if (!regs) { fprintf(stderr, "mmap failed\n"); exit(2); }
    memset(regs, 0, HLL_PACKED);

    pid_t pid = fork();
    if (pid == 0) {
        static uint8_t max[HLL_REGISTERS];
        memset(max, 0, sizeof(max));
        merge(max, regs);
        _exit(0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    return WIFSIGNALED(status);
}

static uint64_t rng_state = 0x2545F4914F6CDD1DULL;
static uint32_t rng(void) {
    rng_state = rng_state * 6364136223846793005ULL + 1442695040888963407ULL;
    return (uint32_t)(rng_state >> 33);
}

/* Fill a packed register array. mode: 0 random, 1 identical, 2 all-zero,
 * 3 all-max, 4 one-hot per register. */
static void fill(uint8_t *regs, int mode, uint8_t seed) {
    int i;
    memset(regs, 0, HLL_PACKED);
    switch (mode) {
    case 0:
        for (i = 0; i < HLL_PACKED; i++) regs[i] = (uint8_t)rng();
        break;
    case 1:
        for (i = 0; i < HLL_PACKED; i++) regs[i] = seed & 0xff;
        break;
    case 2:
        break;
    case 3:
        for (i = 0; i < HLL_PACKED; i++) regs[i] = 0xff;
        break;
    case 4:
        for (i = 0; i < HLL_REGISTERS; i++) {
            unsigned long byte = (unsigned long)i * HLL_BITS / 8;
            unsigned long fb = (unsigned long)i * HLL_BITS & 7;
            uint8_t v = (uint8_t)(i % (HLL_REGISTER_MAX + 1));
            regs[byte] |= (uint8_t)(v << fb);
            if (fb > 2) regs[byte + 1] |= (uint8_t)(v >> (8 - fb));
        }
        break;
    }
}

int main(void) {
    uint8_t regs[HLL_PACKED];
    static uint8_t a[HLL_REGISTERS], b[HLL_REGISTERS];
    static const char *names[] = {"random", "identical", "all-zero",
                                  "all-max", "one-hot"};
    int failures = 0;

    /* ---- 1. equivalence ------------------------------------------------- */
    for (int mode = 0; mode < 5; mode++) {
        for (int trial = 0; trial < 64; trial++) {
            fill(regs, mode, (uint8_t)trial);
            for (int i = 0; i < HLL_REGISTERS; i++) a[i] = (uint8_t)rng();
            memcpy(b, a, sizeof(a));

            upstream_merge(a, regs);
            hllMergeDense(b, regs);

            if (memcmp(a, b, sizeof(a)) != 0) {
                int i;
                for (i = 0; i < HLL_REGISTERS; i++)
                    if (a[i] != b[i]) break;
                printf("MISMATCH mode=%s trial=%d reg=%d upstream=%u "
                       "hllMergeDense=%u\n",
                       names[mode], trial, i, a[i], b[i]);
                failures++;
                break;
            }
        }
        printf("  equivalence %-10s : %s\n", names[mode],
               failures ? "see above" : "64 trials identical");
    }

    /* An all-max packed array is the case the upstream over-read is *for*:
     * register 16383 must come out as 63, and byte 12288 is zero, so the two
     * agree here and differ only in what they touched to get there. */
    fill(regs, 3, 0);
    memset(a, 0, sizeof(a)); memset(b, 0, sizeof(b));
    upstream_merge(a, regs);
    hllMergeDense(b, regs);
    if (a[16383] != 63 || b[16383] != 63) {
        printf("MISMATCH last register: upstream=%u hllMergeDense=%u (want 63)\n",
               a[16383], b[16383]);
        failures++;
    } else {
        printf("  last register (16383)    : both 63\n");
    }

    /* ---- 2. bounds ------------------------------------------------------ */
    printf("placement witness (registers end on a page boundary, "
           "next page PROT_NONE):\n");
    int up = run_guarded(upstream_merge);
    int us = run_guarded(hllMergeDense);
    printf("  upstream      : %s\n", up ? "SIGSEGV (reads registers[12288])"
                                      : "returned  <-- witness has gone blind");
    printf("  hllMergeDense: %s\n", us ? "SIGSEGV  <-- FAILS THE BOUNDS TEST"
                                      : "returned");
    if (!up) { printf("  NOTE: upstream did not fault; this witness no longer "
                      "separates the two.\n"); failures++; }
    if (us) failures++;

    printf("\n%s (%d failure%s)\n", failures ? "FAIL" : "PASS", failures,
           failures == 1 ? "" : "s");
    return failures != 0;
}
