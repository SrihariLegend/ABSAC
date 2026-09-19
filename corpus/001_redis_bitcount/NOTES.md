# Corpus Entry 001: Redis BITCOUNT tail loop

## Provenance

- **Project:** Redis
- **File:** `src/bitops.c`
- **Function:** `redisPopcount` — the `remain:` tail loop
- **Git ref:** `unstable` branch
- **URL:** https://github.com/redis/redis/blob/unstable/src/bitops.c

## The kernel

```c
while (count--) bits += bitsinbyte[*p++];
```

Where `bitsinbyte` is a 256-entry lookup table: `bitsinbyte[b] == popcount(b)` for all byte values 0–255.

This is the fallback popcount path in Redis's `BITCOUNT` command.

## What ABSAC did

The pipeline ran end-to-end on the hand-lowered SIR:

| Layer | Result |
|-------|--------|
| Analysis (facts) | 121 facts derived; 2 sum reductions detected (counter + accumulator) |
| Semantics (truths) | `CardinalityReduction`, `PredicateMap`, `ElementSequence`, `LogicalSequence` |
| Inference (beliefs) | `BitSet` representation inferred |
| Generation (candidates) | `Popcount` candidate generated |
| Verification | Equivalence proven: `bitsinbyte[byte] == popcount(byte)` over finite domain |
| Rewrite | **Table lookup → native Popcount applied** (1 rewrite, 15→14 nodes) |

### The rewrite

```
BEFORE:  bits += bitsinbyte[buf[i]]    // memory access into 2KB table
AFTER:   bits += popcount(buf[i])      // single POPCNT instruction, no memory
```

The loop structure is preserved. The 256-entry lookup table is eliminated. Each iteration now uses a hardware popcount instruction instead of a memory access.

## What was changed in ABSAC

1. `sir_semantics/src/semantics.rs` — broadened role assignment to recognize non-boolean array collections (arrays indexed by loop carried inputs).
2. `sir_rewrite/src/recipes/popcount.rs` — added `build_table_lookup_patch`: a new rewrite pattern that replaces table-lookup popcounts with native `Popcount` instructions, keeping the loop intact.

## Delta vs clang -O3

**Status: NOT YET MEASURED.**

This kernel is a Week-1 plumbing test (does the pipeline run on real code?), not a Phase 2 delta target. We expect `clang -O3 -march=native` to auto-vectorize this loop with SIMD, and the table-lookup version may even be faster than per-byte POPCNT on modern CPUs (the 2KB table fits in L1, and SIMD can do 16 lookups in parallel). The honest expectation is that we **lose** to clang -O3 on this specific kernel.

The kernels where we expect to *win* are ones where clang is structurally constrained (e.g., OpenSSL constant-time code, where -O3 cannot introduce timing-variable rewrites). Those are the Phase 2 targets.

## Gaps logged

- #1: straight-line SWAR arithmetic popcount not recognized (needs arithmetic-shape recognizer)
- #2: SIR has no type cast/convert node (u8→u64 widening)
- #3: SIR has no pointer arithmetic (modeled as index)
