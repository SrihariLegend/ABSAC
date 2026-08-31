# Witness Frontier — Final Results (Gate 2)

## Date: 2025-07-14
## Status: FROZEN

## Experiment

Benchmark 3 kernels with 6-7 candidate implementations each, against
clang -O3 -march=native and gcc -O3 -march=native, across multiple
input distributions and sizes including tail-edge sizes.

## Hardware

- Intel Core Ultra 9 275HX (AVX2, SSE4.2, POPCNT, BMI1/2, AVX-VNNI)
- No AVX-512
- ~3.2 GHz, pinned to core 0

## Correctness

All candidates pass differential validation against the original across:
- 11 sizes (16, 17, 31, 32, 33, 63, 64, 65, 256, 4096, 65536)
- 5 distributions (k18), 3 distributions (k43), 8 distributions (k50)
- Both clang and gcc

**Bug fixed during experiment:** k50 SWAR/chunked variants had a correctness
bug — the SWAR `haszero` trick `(v - lo) & ~v & hi` has false positives from
borrow propagation and cannot count zero bytes. Fixed by using unrolled
per-byte extraction (chunked) and haszero-as-filter with per-byte fallback (swar).

## LLVM Vectorization Remarks (critical evidence)

### clang -O3 -march=native
```
k18_all_equal:  vectorized loop (vectorization width: 4, interleaved count: 4)
k43_sum_ascii:  vectorized loop (vectorization width: 4, interleaved count: 4)
k50_count_masked: vectorized loop (vectorization width: 4, interleaved count: 4)
```

**clang vectorizes all three at VF=4 (4 bytes per vector instruction).**
Despite the target supporting AVX2 (32-byte vectors), clang chooses
4-byte vectors for these reduction loops. Interleave count 4 means
4×4=16 bytes per iteration group, but using 32-bit vector operations.

### gcc -O3 -march=native
```
k18_all_equal:  loop vectorized using 32 byte vectors + 16 byte vectors
k43_sum_ascii:  loop vectorized using 32 byte vectors + 16 byte vectors
k50_count_masked: loop vectorized using 32 byte vectors + 16 byte vectors
```

**GCC uses full 32-byte AVX2 vectors** (with versioned loops for alignment).
Despite this, GCC's orig is still slower than hand-written AVX2.

## Witness Frontier at 4096 bytes (cache-resident)

### k18_all_equal (all_equal distribution)

| Candidate | clang median (ns) | gcc median (ns) | clang speedup | gcc speedup |
|-----------|------------------:|----------------:|--------------:|------------:|
| orig      | 207               | 296             | 1.00×         | 1.00×       |
| scalar    | 410               | 278             | 1.98×         | 0.94×       |
| chunked   | 75                | 56              | 0.36×         | 0.19×       |
| sse2      | 34                | 37              | 0.16×         | 0.12×       |
| avx2      | 22                | 22              | **0.105×**    | **0.075×**  |
| multi     | 24                | 22              | 0.12×         | 0.075×      |

**Headroom: 9.5× (clang), 13.3× (gcc)**

### k43_sum_ascii (random distribution)

| Candidate | clang median (ns) | gcc median (ns) | clang speedup | gcc speedup |
|-----------|------------------:|----------------:|--------------:|------------:|
| orig      | 116               | 268             | 1.00×         | 1.00×       |
| scalar    | 115               | 268             | 0.99×         | 1.00×       |
| chunked   | 81                | 90              | 0.70×         | 0.33×       |
| swar      | 106               | 152             | 0.92×         | 0.57×       |
| sse2      | 47                | 68              | 0.41×         | 0.26×       |
| avx2      | 27                | 38              | **0.23×**     | **0.14×**   |
| multi     | 27                | 34              | 0.23×         | 0.13×       |

**Headroom: 4.3× (clang), 7.2× (gcc)**

### k50_count_masked (half_match distribution)

| Candidate | clang median (ns) | gcc median (ns) | clang speedup |
|-----------|------------------:|----------------:|--------------:|
| orig      | 256               | —               | 1.00×         |
| scalar    | 256               | —               | 1.00×         |
| chunked   | 215               | —               | 0.84×         |
| swar      | 261               | —               | 1.02×         |
| sse2      | 67                | —               | 0.26×         |
| avx2      | 33                | —               | **0.13×**     |
| multi     | 33                | —               | 0.13×         |

**Headroom: 7.7× (clang)**

## Size regime analysis

### Tiny (16 bytes)
All candidates within 1.3-1.5ns. No headroom — function call overhead
dominates. Scalar is competitive.

### Small (256 bytes)
AVX2 starts winning. k18: 1.4ns (AVX2) vs 13.5ns (orig) = 9.6×.

### Medium (4096 bytes, cache-resident)
AVX2 dominates. 4-10× headroom. This is the sweet spot.

### Large (65536 bytes)
AVX2 still wins but gap narrows as memory bandwidth becomes a factor.
k18: 600ns (AVX2) vs 3324ns (orig) = 5.5×.

## Why the headroom exists

### The causal explanation (with instruction-level evidence)

The headroom is NOT merely about vector width. Both compilers leave
headroom, but for DIFFERENT reasons:

#### Clang: narrow vectors + lane-wise accumulation

clang -O3 vectorizes all three loops at **VF=4** (4-byte vectors), as
confirmed by LLVM vectorization remarks. The assembly shows:

```
vmovd    (%rdi,%rax), %xmm7      # 4-byte load
vpand    %xmm0, %xmm7, %xmm7     # mask
vpcmpeqb %xmm2, %xmm7, %xmm7     # compare
vpand    %ymm3, %ymm7, %ymm7     # widen result
vpaddq   %ymm7, %ymm1, %ymm1     # accumulate
```

Clang loads 4 bytes, compares, widens to 64-bit, and accumulates with
`vpaddq`. This is lane-wise accumulation at narrow width — 4 bytes per
iteration group, with the comparison result occupying only 4 of 32
vector lanes.

#### GCC: wide vectors + wrong reduction algorithm

GCC uses 32-byte vectors (correct width) but the wrong reduction
algorithm. The assembly for k50 shows:

```
vpcmpeqb   (%rax), %ymm5, %ymm0    # 32-byte compare (good)
vpand      %ymm4, %ymm0, %ymm0    # AND with mask
vextracti128 $0x1, %ymm0, %xmm0   # extract high lane
vpand      %xmm0, %xmm3, %xmm3    # horizontal AND reduction
vextracti128 $0x1, %ymm3, %xmm0   # extract again
vpand      %xmm0, %xmm3, %xmm0    # AND again
...                               # many more vpand/vextracti128
```

GCC compares 32 bytes correctly, then reduces the result through a
tower of `vextracti128` + `vpand` instructions — a vector-based
counting scheme. It does NOT use `vpmovmskb` + `popcnt`.

For k43 (Sum), GCC uses `vpmovzxbw` (zero-extend bytes to words) +
`vpaddq` (accumulate), NOT `vpsadbw` (packed sum of absolute byte
differences against zero). The zero-extend-and-add approach requires
many more instructions than psadbw.

### Semantic knowledge selects a better reduction algorithm

The headroom is NOT just about vector width. It's about **algorithm
selection** — choosing the right vector reduction instruction for each
semantic operation:

| Semantic operation | Clang's approach | GCC's approach | ABSAC's approach |
|---|---|---|---|
| Cardinality (count matching) | narrow compare + vpaddq | wide compare + vextracti128 tower | **vpmovmskb + popcnt** |
| Sum (byte accumulation) | narrow vmovd + vpaddq | vpmovzxbw + vpaddq | **vpsadbw + vpaddq** |
| All (universal test) | narrow compare + AND-reduction | wide compare + vextracti128 | **vpmovmskb + full-mask test** |

`vpmovmskb` extracts 32 comparison results into a 32-bit GPR in one
instruction. `popcntl` counts them in one instruction. This is the
optimal algorithm for Cardinality — but neither compiler selects it.

`vpsadbw` computes the sum of 8 unsigned bytes against zero in one
instruction, producing 16-bit sums directly. This is the optimal
algorithm for byte-Sum — but neither compiler selects it.

### Why semantic recognition enables this

A syntactic vectorizer sees a loop with a reduction variable and tries
to vectorize the loop body. Its cost model evaluates whether widening
the vector is profitable, considering:
- reduction type
- data dependencies
- tail handling
- target cost model

A semantic recognizer sees `Cardinality(PredicateMap(buf, λx. (x & mask) == target))`
and knows directly that the optimal implementation is:
```
for each 32-byte chunk:
    mask = AND(chunk, broadcast(mask))
    cmp = pcmpeqb(mask, broadcast(target))
    bits = pmovmskb(cmp)
    count += popcount(bits)
```

The semantic interpretation bypasses the cost model entirely. It maps
each reduction type to its optimal vector algorithm:
- Cardinality → movemask + popcount
- Sum → psadbw + accumulate
- All → movemask + full-mask test (with early exit)

This is the core ABSAC thesis: **semantic knowledge selects a better
reduction algorithm, not merely a larger vector width.**

## Distinct semantic endpoints (for Gate 3)

The three kernels have DIFFERENT semantic endpoints:

### k18: All(PredicateMap)
```
vector compare (32 bytes)
→ reduce.and (or movemask == full_mask)
→ no popcount needed
```

### k50: Cardinality(PredicateMap)
```
vector compare (32 bytes)
→ movemask (extract 1-bit per lane)
→ popcount (count set bits)
→ scalar accumulate
```

### k43: Sum(Map)
```
vector load (32 bytes)
→ byte-sum (psadbw against zero, or widening add)
→ vector accumulate
→ horizontal reduction at end
```

Gate 3 should implement a general family of semantic reductions, not
a benchmark-specific "emit AVX2" rule for each kernel.

## Confidence intervals

All reported medians have tight confidence intervals (MAD < 1ns for
cache-resident sizes). The 95% CI for the AVX2 candidates at 4096 bytes:
- k18 avx2: 21.7-22.0 ns (CI_lo 21.7, CI_hi 22.0)
- k43 avx2: 26.7-27.5 ns
- k50 avx2: 32.5-33.2 ns

The wins are statistically significant — the AVX2 CI does not overlap
the orig CI for any kernel at 4096 bytes.

## Conclusion

**Gate 2 is PASSED.** There is 4-10× demonstrated headroom on these
kernels that neither clang -O3 nor gcc -O3 reaches, despite both
auto-vectorizing the loops. The headroom has two distinct causes:

1. **Clang: narrow vectors + lane-wise accumulation.** Clang chooses
   VF=4 (4-byte vectors) and uses lane-wise accumulation (vpaddq).
   Both width and algorithm are suboptimal.

2. **GCC: wide vectors + wrong reduction algorithm.** GCC uses correct
   32-byte vectors but selects the wrong reduction algorithm (vector
   AND tower instead of movemask+popcount for Cardinality; zero-extend
   + add instead of psadbw for Sum). Width is correct but algorithm is
   suboptimal.

The semantic interpretation maps each reduction type to its optimal
vector algorithm, bypassing both limitations. This is aligned with
ABSAC's semantic thesis.

## Files

- `results/results_clang.csv` — full benchmark data (clang)
- `results/results_gcc.csv` — full benchmark data (gcc)
- `results/errors_clang.txt` — correctness errors (empty = all pass)
- `results/errors_gcc.txt` — correctness errors (empty = all pass)
- `results/llvm_remarks.txt` — LLVM vectorization remarks
- `results/orig_kernels_clang.s` — assembly of original kernels (clang)
- `results/orig_kernels_gcc.s` — assembly of original kernels (gcc)
- `results/orig_kernels_opt.ll` — optimized LLVM IR
