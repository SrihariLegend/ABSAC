#include <stdint.h>
#include <stdbool.h>
#include <string.h>

uint64_t find_first__base(const uint8_t *p0, uint64_t p1, uint8_t p2) {
    bool o0_0 = false;
    uint64_t o0_1 = p1;
    uint64_t o0_2 = (0ULL);
    uint64_t __sir_n19 = (uint64_t)(o0_1);
    if (__sir_n19 >= 32u) {
        const void *__sir_hit19 = memchr((const void *)(p0), (int)(p2), (size_t)__sir_n19);
        if (__sir_hit19 != (const void *)0) {
            o0_1 = (uint64_t)((const uint8_t *)__sir_hit19 - (const uint8_t *)(p0));
            o0_0 = true;
        }
    } else {
        while ((o0_2) < __sir_n19) {
            if ((p0[o0_2] == p2)) { o0_1 = o0_2; o0_0 = true; break; }
            o0_2 = (o0_2 + (1ULL));
        }
    }
    bool v4 = ((bool)(((int64_t) (p1)) > ((int64_t) ((0ULL)))));
    uint64_t v20 = o0_1;
    uint64_t v21 = o0_2;
    return v20;
}
