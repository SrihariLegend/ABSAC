# Gate 6A-v20 — library-implementation baselines

**Status:** frozen source (`v20_library_baseline.c`) with clang/GCC `-O1` raw
outputs in `v20_raw/`. This is external-baseline evidence for P5, not part of
the compilation pipeline.

## Methodology

The benchmark compiles one C file that contains both a library/builtin
implementation and a naive scalar implementation of the same kernel, then times
a tight call loop with `clock_gettime(CLOCK_MONOTONIC)` and a volatile sink.
Each iteration carries an empty `asm volatile("" ::: "memory")` barrier so the
compiler cannot hoist loop-invariant `memchr`/builtin calls.

Kernels:

- first zero byte over a `uint8_t` buffer (`memchr` vs scalar loop), measured
  with an early hit at index 3 and with no hit, at n = 64 and n = 4096;
- 64-bit population count (`__builtin_popcountll` vs Kernighan loop);
- 64-bit trailing-zero count (`__builtin_ctzll(x|1)` vs shift loop).

Commands:

```bash
clang -O1 -w gate6a/v20_library_baseline.c -o /tmp/v20_clang && /tmp/v20_clang
gcc   -O1 -w gate6a/v20_library_baseline.c -o /tmp/v20_gcc   && /tmp/v20_gcc
```

## Results (ns per call, this host)

| kernel / distribution | clang library | clang naive | gcc library | gcc naive |
|---|---|---|---|---|
| first zero, n=64, hit at 3 | 1.49 | 1.72 | 1.78 | 1.97 |
| first zero, n=64, no hit | 1.50 | 17.63 | 2.13 | 18.02 |
| first zero, n=4096, hit at 3 | 1.44 | 1.35 | 1.57 | 1.94 |
| first zero, n=4096, no hit | 21.17 | 560.95 | 20.90 | 559.28 |
| popcount (u64) | 1.32 | 18.04 | 1.06 | 18.12 |
| trailing zeros (u64) | 0.19 | 1.31 | 0.60 | 1.45 |

## Implications

- Library/builtin baselines dominate naive scalar kernels where the work is
  long or no-hit: `memchr` is **~26x** faster on a 4096-byte no-hit scan and
  ~8–12x on the 64-byte no-hit scan, while `__builtin_popcountll` is ~14–17x
  faster than Kernighan and `ctz` is ~2.4–7x faster than a shift loop.
- On an immediate hit, library search is only marginally better or slightly
  worse (1.49 vs 1.72 clang; 1.44 vs 1.35 at n=4096). Distribution decides.
- The proposed scalar chunked-mask runtime search measured only **0.89–1.01x**
  on no-hit scans, nowhere near the 8–26x of a vectorized library search. A
  runtime replacement that merely builds scalar masks would leave most of the
  available win on the table; closing P2 properly would require a
  vectorized/library-search representation and proof, beyond the current
  recipe and theorem capabilities.
