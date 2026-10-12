#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

extern void __sir_ffi_reset(void) __attribute__((weak));

extern uint64_t find_first__ref(const uint8_t * a0, uint64_t a1, uint8_t a2);

extern uint64_t find_first(const uint8_t *, uint64_t, uint8_t);

static uint64_t val(int t, int k) {
    static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};
    static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};
    if (k == 0) return lens[t % 11];
    return keys[(t * 3 + k * 5) % 9];
}

static _Alignas(64) uint8_t __sir_ptr_scratch[4096];

int main(int argc, char **argv) {
    int zero_state = 0, ref_only = 0;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--zero")) zero_state = 1;
        if (!strcmp(argv[i], "--ref-only")) ref_only = 1;
    }
    static _Alignas(64) uint8_t bufA[8][2048 + 32];
    static _Alignas(64) uint8_t bufB[8][2048 + 32];
    uint64_t mismatches = 0;
    for (int t = 0; t < 200; t++) {
        uint64_t seed = 0x9E3779B97F4A7C15ULL ^ (uint64_t)t;
        if (zero_state) {
            memset(bufA, 0, sizeof bufA);
            memset(bufB, 0, sizeof bufB);
        } else {
            for (int b = 0; b < 8; b++) for (int i = 0; i < 2048; i++) {
                seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
                bufA[b][i + 32] = (uint8_t)seed; bufB[b][i + 32] = bufA[b][i + 32];
            }
        }
        uint64_t s0 = (uint64_t)val(t, 0);
        uint8_t s1 = (uint8_t)val(t, 1);
        if (__sir_ffi_reset) __sir_ffi_reset();
        uint64_t r_ref = find_first__ref((const uint8_t *)(bufA[0] + 32), (uint64_t)s0, (uint8_t)s1);
        if (ref_only) continue;
        if (__sir_ffi_reset) __sir_ffi_reset();
        uint64_t r_new = find_first((const uint8_t *)(bufB[0] + 32), s0, s1);
        if ((uint64_t)r_ref != (uint64_t)r_new) {
            if (mismatches < 5) printf("EMITCASE find_first t=%d ref=%llu new=%llu\n", t, (unsigned long long)r_ref, (unsigned long long)r_new);
            mismatches++;
        }
        for (int b = 0; b < 1; b++) if (memcmp(bufA[b] + 32, bufB[b] + 32, 2048)) {
            if (mismatches < 5) printf("EMITBUF find_first t=%d slot=%d\n", t, b);
            mismatches++;
        }
    }
    if (ref_only) { printf("EMITREF_OK cases=200\n"); return 0; }
    printf("EMITDIFF find_first cases=200 mismatches=%llu\n", (unsigned long long)mismatches);
    return mismatches != 0;
}
