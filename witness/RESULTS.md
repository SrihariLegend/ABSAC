# Witness Frontier — Preliminary Results

## Date: 2025-07-14

## Experiment

Benchmark 3 kernels (k18_all_equal, k43_sum_ascii, k50_count_masked) with
multiple candidate implementations (original scalar, chunked scalar, SWAR,
SSE2, AVX2, multi-versioned) against clang -O3 -march=native.

## Hardware

- Intel Core Ultra 9 275HX
- AVX2, SSE4.2, POPCNT, BMI1/2, AVX-VNNI (no AVX-512)
- ~3.2 GHz

## Key Finding: MASSIVE HEADROOM EXISTS

At 4096 bytes (cache-resident), hand-written AVX2 vs clang -O3 original:

| Kernel | clang -O3 (ns) | AVX2 (ns) | Speedup |
|--------|---------------|-----------|---------|
| k18_all_equal (all_equal) | 210 | 22 | **9.6×** |
| k43_sum_ascii (random) | 113 | 27 | **4.2×** |
| k50_count_masked (half_match) | 258 | 33 | **7.8×** |

## Why clang doesn't close the gap

clang -O3 DOES auto-vectorize these kernels, but conservatively:

- k18_all_equal: clang vectorizes at 4 bytes/iteration (vmovd + vpcmpeqd).
  Hand-written AVX2 processes 32 bytes/iteration (vmovdqu + vpcmpeqb).
  The `all &= (buf[i] == val)` reduction pattern confuses clang's vectorizer
  into using narrow 4-byte vectors.

- k43_sum_ascii: clang vectorizes using vpmovzxbq (zero-extend byte to qword
  + vpaddq). This is 4 bytes/iteration with 64-bit accumulation.
  Hand-written AVX2 uses vpsadbw (packed sum of absolute byte differences)
  which processes 32 bytes/iteration with 16-bit lane accumulation.
  clang's approach has 8× more arithmetic operations per byte.

- k50_count_masked: Similar pattern — clang vectorizes but at lower width.

## The witness frontier

These hand-written implementations are WITNESSES, not oracles.
They prove headroom exists. They don't prove these are the best possible.

## Correctness notes

- k50_count_masked SWAR variant has a bug (counting wrong).
  All other variants pass correctness checks across all distributions.

## What this means for ABSAC

1. **Gate 2 (witness headroom) is PASSED.** There is 4-10× headroom on
   these kernels that clang -O3 does not reach.

2. **The headroom is in vectorization strategy, not instruction selection.**
   clang already picks the right instructions (vpbroadcastb, vpcmpeqb, etc.)
   but uses them at 4-32 byte granularity instead of 16-32 byte granularity.

3. **ABSAC's path to winning:** Represent the semantic operation (Cardinality,
   Reduction, etc.) and lower it to the right vector width. The semantic
   rewrite is: "this loop is a cardinality reduction → use vector compare +
   popcount at full AVX2 width." clang can't do this because it doesn't know
   the loop is a cardinality reduction — it just sees a byte loop with a
   conditional accumulate.

## Next steps

1. Fix k50 SWAR correctness bug
2. Run the full benchmark (all distributions, all sizes) with both clang and gcc
3. Check if gcc -O3 -march=native does better than clang on these patterns
4. Gate 3: Can ABSAC represent the semantic endpoint and emit the winning code?
