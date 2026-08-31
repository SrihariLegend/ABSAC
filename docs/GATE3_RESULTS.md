# Gate 3 — Expressive Reachability: Final Results

## Date: 2025-07-14
## Status: PASSED

## Objective

Can ABSAC represent the semantic endpoint (vector compare + popcount at
full SIMD width), emit it as compilable code, and approach the hand-written
witness performance?

## Method

1. Lower 3 kernels from LLVM IR → SIR (using existing `sir_lower`)
2. Run semantic analysis → recognize CardinalityReduction, Sum, All
3. Derive a target-independent vector plan from the semantic truths
4. Lower the vector plan to C with AVX2 intrinsics
5. Compile with clang -O2 -march=native (NOT -O3 — let ABSAC do the optimization)
6. Differential correctness test against original (all sizes 0-128, multiple distributions)
7. Benchmark at 4096 bytes against original and hand-written witness

## Three Distinct Semantic Reductions

### k18_all_equal: All(PredicateMap(buf, λx. x == val))
- **Recognized as:** All (via ConjunctiveReduction fallback + Select pattern detection)
- **Vector plan:** Vector compare → movemask → early exit if not all match
- **Emitted intrinsics:** `_mm256_cmpeq_epi8`, `_mm256_movemask_epi8`, `if (mask != -1) return 0`
- **Assembly:** `vpcmpeqb`, `vpmovmskb`, compare to `-1`

### k43_sum_ascii: Sum(Map(buf, identity))
- **Recognized as:** Sum (CardinalityReduction without loop-body comparison)
- **Vector plan:** Vector load → psadbw (packed byte sum) → accumulate
- **Emitted intrinsics:** `_mm256_sad_epu8`, `_mm256_add_epi64`
- **Assembly:** `vpsadbw`, `vpaddq`

### k50_count_masked: Cardinality(PredicateMap(buf, λx. (x & mask) == target))
- **Recognized as:** Cardinality (CardinalityReduction with loop-body comparison)
- **Vector plan:** Vector load → AND mask → compare → movemask → popcount
- **Emitted intrinsics:** `_mm256_loadu_si256`, `_mm256_and_si256`, `_mm256_cmpeq_epi8`, `_mm256_movemask_epi8`, `__builtin_popcount`
- **Assembly:** `vmovdqu`, `vpand`, `vpcmpeqb`, `vpmovmskb`, `popcntl`

## Correctness

ALL CORRECTNESS TESTS PASSED.
- Differential testing across sizes 0-128
- Multiple distributions (random, all-equal)
- Both ABSAC and witness compared against original

## Benchmark (4096 bytes, cache-resident, clang -O3 vs ABSAC -O2 vs hand-written)

| Kernel | Distribution | orig-O3 (ns) | ABSAC-O2 (ns) | witness (ns) | ABSAC vs orig | ABSAC vs witness |
|--------|-------------|-------------:|--------------:|-------------:|--------------:|----------------:|
| k18_all_equal | all_equal | 210.4 | 36.4 | 33.6 | **5.8× faster** | 8.3% slower |
| k18_all_equal | random | 208.9 | 1.5 | 1.5 | **139× faster** | 0% (same) |
| k43_sum_ascii | random | 115.6 | 25.8 | 25.2 | **4.5× faster** | 2.4% slower |
| k50_count_masked | half_match | 256.4 | 33.0 | 30.2 | **7.8× faster** | 9.3% slower |

## Pass Condition Check

> ABSAC-generated code should be within 10-15% of the hand-written witness.

- k18 (all_equal): 8.3% slower → **PASS** (within 10%)
- k18 (random): 0% (same) → **PASS**
- k43: 2.4% slower → **PASS** (well within 10%)
- k50: 9.3% slower → **PASS** (within 10%)

## Assembly Verification

ABSAC-generated code compiles to full-width AVX2 instructions:
- `vpand` — 256-bit AND (masking)
- `vpcmpeqb` — 256-bit byte compare
- `vpmovmskb` — 256-bit movemask
- `popcntl` — popcount
- `vpsadbw` — packed sum of absolute byte differences

These are the same instructions the hand-written witness uses. clang -O3
uses only 4-byte-wide vectors (VF=4) for the original code, as confirmed
by LLVM vectorization remarks.

## Architecture

The Gate 3 implementation adds two modules to `sir_benchmarks`:

1. **`vector_plan.rs`** — Target-independent vector plan representation
   - `VectorPlan { operation, buffer_name, length_name, predicate, vector_width, tail }`
   - `VectorOp`: Cardinality, All, Sum
   - `VectorPredicate`: Equal, MaskedEqual, Identity
   - `TailPolicy`: Scalar, NarrowerVector

2. **`vector_emit.rs`** — Target-aware lowering to C with intrinsics
   - Emits AVX2 intrinsics for vector_width=32
   - Handles 32-byte main loop + 16-byte SSE2 tail + scalar tail
   - Proper parameter name resolution from SIR function

3. **`emit_vectorized.rs` (binary)** — End-to-end pipeline
   - Lowers LLVM IR → SIR
   - Runs semantic analysis
   - Derives vector plan from semantic truths
   - Emits vectorized C or falls back to scalar

## Key Findings

1. **Semantic recognition enables full-width vectorization.** clang's
   syntactic analysis vectorizes at VF=4 (4 bytes). ABSAC's semantic
   recognition (Cardinality, All, Sum) enables VF=32 (32 bytes), a 8×
   wider vector.

2. **ABSAC-generated code matches expert hand-written code within 10%.**
   The 2-9% gap is likely from instruction scheduling and tail handling,
   not from the core vectorization strategy.

3. **The early-exit optimization on k18 is a bonus.** The semantic "All"
   operation can short-circuit (return 0 on first mismatch). The original
   `all &= (buf[i] == val)` reduction cannot. ABSAC recognized this
   semantic opportunity, yielding 139× speedup on random data.

4. **Three distinct semantic endpoints, one general framework.** The
   VectorPlan abstraction handles All, Cardinality, and Sum uniformly,
   each with different vector instructions but the same plan structure.

## What This Proves

The ABSAC thesis is validated for these three kernels:
- **Semantic recognition works** — the recognizers correctly identify
  the mathematical operation (All, Cardinality, Sum)
- **Vector plan representation works** — the intermediate representation
  captures the essential structure of the vectorization
- **Target-aware lowering works** — the emitter generates correct AVX2
  intrinsic code that compiles to full-width instructions
- **The performance gap is real and closable** — ABSAC-generated code
  achieves 4-8× speedup over clang -O3, within 10% of expert code

## Limitations (honest assessment)

1. **Only 3 kernels tested.** The framework needs to generalize to more
   patterns and be tested on held-out kernels (Gate 6).

2. **The vector plan is manually derived.** The `derive_vector_plan`
   function uses pattern matching on semantic truths, not the full
   sir_generation/sir_selection pipeline. This is a proof of concept.

3. **Emitting C intrinsics, not LLVM IR.** The long-term plan is to emit
   LLVM IR with vector types and let LLVM select instructions. Currently
   we bypass LLVM's instruction selection.

4. **No formal verification (Gate 4).** The correctness is tested by
   differential testing, not by the sir_verification proof engine.

5. **k43 misidentification.** The CardinalityReduction recognizer fires
   on k43 (Sum) because both are "count/accumulate" patterns. The
   distinction (comparison in loop body) is done post-hoc in the binary,
   not in the recognizer itself.

## Next Steps

- **Gate 4 (Concrete correctness):** Use sir_verification to formally
  prove the vectorized code is equivalent to the original
- **Gate 5 (Search value):** Test whether the plan generation can be
  automated via search, or if deterministic derivation suffices
- **Gate 6 (Generalization):** Test on held-out kernels not used for
  development

## Files

- `sir/gate3_test.sh` — test harness (correctness + benchmark)
- `sir/crates/sir_benchmarks/src/vector_plan.rs` — vector plan types
- `sir/crates/sir_benchmarks/src/vector_emit.rs` — intrinsic C emitter
- `sir/crates/sir_benchmarks/src/bin/emit_vectorized.rs` — end-to-end binary
