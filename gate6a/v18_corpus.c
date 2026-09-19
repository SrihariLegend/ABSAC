// Gate 6A-v18 — scalar bit-count intrinsics (fresh, post-freeze probe)
//
// Frontier: clang lowers scalar popcount/ctz/clz builtins to
// llvm.ctpop/cttz/ctlz. The lowerer now models the well-defined forms so the
// existing verified scalar bitscan/popcount recipes can apply to real IR.
// The ternary forms pin the zero-undef flag to false.

#include <stdint.h>

uint32_t q01_popcount_u32(uint32_t x) {
    return (uint32_t)__builtin_popcount(x);
}

uint32_t q02_popcount_u64(uint64_t x) {
    return (uint32_t)__builtin_popcountll(x);
}

uint32_t q03_ctz_u32(uint32_t x) {
    return x ? (uint32_t)__builtin_ctz(x) : 32u;
}

uint32_t q04_ctz_u64(uint64_t x) {
    return x ? (uint32_t)__builtin_ctzll(x) : 64u;
}

uint32_t q05_clz_u32(uint32_t x) {
    return x ? (uint32_t)__builtin_clz(x) : 32u;
}

uint32_t q06_clz_u64(uint64_t x) {
    return x ? (uint32_t)__builtin_clzll(x) : 64u;
}
