#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

extern void __sir_ffi_reset(void) __attribute__((weak));

extern uint64_t find_first__base(const uint8_t *, uint64_t, uint8_t);

extern uint64_t find_first(const uint8_t *, uint64_t, uint8_t);

static uint64_t now_ns(void) {
    // Invariant TSC ticks: unlike clock_gettime wall time, this is
    // not distorted by CPU frequency scaling between phases. Only
    // ratios are consumed, so the historical ref_ns/new_ns labels
    // are kept even though the unit is ticks.
    unsigned lo, hi;
    __asm__ __volatile__("rdtsc" : "=a"(lo), "=d"(hi));
    return ((uint64_t)hi << 32) | (uint64_t)lo;
}

int main(int argc, char **argv) {
    int zero_state = 0, ref_only = 0;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--zero")) zero_state = 1;
        if (!strcmp(argv[i], "--ref-only")) ref_only = 1;
    }
    static _Alignas(64) uint8_t bufA[8][2048 + 32];
    static _Alignas(64) uint8_t bufB[8][2048 + 32];
    static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};
    static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};
    uint64_t seed = 0x9E3779B97F4A7C15ull;
    for (int i = 0; i < 8 * (2048 + 32); i++) {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        ((uint8_t *)bufA)[i] = zero_state ? 0u : (uint8_t)seed;
        ((uint8_t *)bufB)[i] = zero_state ? 0u : (uint8_t)seed;
    }
    uint64_t s0 = (uint64_t)(lens[4] + keys[0 % 9]);
    uint8_t s1 = (uint8_t)(lens[4] + keys[1 % 9]);
    if (__sir_ffi_reset) __sir_ffi_reset();
    uint64_t left_ret = (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    if (ref_only) { printf("EMITTIME_REF_OK\n"); return 0; }
    if (__sir_ffi_reset) __sir_ffi_reset();
    uint64_t right_ret = (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    if (left_ret != right_ret) {
        printf("EMITTIME_BASE_MISMATCH find_first\n");
        return 2;
    }
    volatile uint64_t sink = 0;
    uint64_t t0, t1, ref_ns, new_ns, b0, b1, b2, b3, b4, o0, o1, o2, o3, o4;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); b0 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); o0 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); o1 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); b1 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); b2 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); o2 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); b3 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); o3 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first__base((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); b4 = t1 - t0;
    t0 = now_ns();
    for (uint64_t r = 0; r < 200000ull; r++) {
        sink += (uint64_t)find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
    }
    t1 = now_ns(); o4 = t1 - t0;
    ref_ns = b0 < b1 ? b0 : b1; ref_ns = ref_ns < b2 ? ref_ns : b2;
    ref_ns = ref_ns < b3 ? ref_ns : b3; ref_ns = ref_ns < b4 ? ref_ns : b4;
    new_ns = o0 < o1 ? o0 : o1; new_ns = new_ns < o2 ? new_ns : o2;
    new_ns = new_ns < o3 ? new_ns : o3; new_ns = new_ns < o4 ? new_ns : o4;
    printf("EMITTIME_BASE find_first reps=200000 state=%s ref_ns=%llu new_ns=%llu sink=%llu\n",
        zero_state ? "zero" : "random",
        (unsigned long long)ref_ns, (unsigned long long)new_ns, (unsigned long long)sink);
    return 0;
}
