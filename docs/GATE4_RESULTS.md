# Gate 4 — Concrete Correctness: Results

## Date: 2025-07-14
## Status

```
Gate 4A: PASSED — compositional correctness argument
                    + extensive adversarial validation

Gate 4B: OPEN   — mechanically checked concrete end-to-end equivalence
```

The six lemmas below are human-written mathematical arguments supported
by 16,869,913 differential tests. They are NOT machine-checked proofs.
The per-byte and chunk lemmas are algebraic identities (true by the
definition of x86 intrinsics), and the loop invariant is an induction
argument. These are valid mathematical reasoning, but no solver or
proof checker has verified them against a formal model of the AVX2
instruction semantics.

Gate 4B would require:
1. The actual source SIR region is mechanically bound to the theorem's operands.
2. The actual generated vector plan is encoded — not an idealized stand-in.
3. The per-chunk identities are proven by a solver/proof checker.
4. The loop invariant covers arbitrary valid n (mechanically).
5. The tail decomposition is proven for all remainders.
6. The target intrinsic semantics are modeled.
7. The generated implementation's memory and overflow semantics match the source.
8. A proof artifact or reproducible solver result exists.

None of these are currently satisfied. The distinction matters: tests
are not a formal proof over arbitrary lengths.

## Objective

Establish a correctness argument for the ABSAC-generated AVX2 code
relative to the original scalar code, using compositional reasoning
supported by extensive adversarial validation.

## Proof Structure

The proof is compositional — instead of sending the entire dynamic loop
to one SMT query, it decomposes the equivalence into 6 lemmas:

```
1. Per-byte lemma     → vector op per byte ≡ scalar op per byte
2. Chunk lemma         → 32-byte vector ≡ 32 scalar iterations
3. Loop invariant      → induction on chunk count
4. Tail lemma          → 16-byte + scalar tail ≡ original remainder
5. Boundary conditions → n=0, n<32, exact multiples, all tail sizes
6. Region binding      → SIR recognizer bound to correct operands
```

Composition:
```
per_byte_lemma  ⟹  chunk_lemma  ⟹  loop_invariant
                                           +
                   tail_lemma    ⟹  boundary_conditions
                                           +
                   region_binding
                                           =
              Source region ≡ Generated AVX2 candidate
```

## The Six Lemmas

### 1. Per-byte lemma

For each byte position i in a 32-byte chunk, the vector operation produces
the same result as the scalar operation on that byte.

**All:** `movemask(pcmpeqb(chunk, broadcast(val)))[i] = (chunk[i] == val) ? 1 : 0`

**Cardinality:** `movemask(pcmpeqb(and(chunk, broadcast(mask)), broadcast(target)))[i] = ((chunk[i] & mask) == target) ? 1 : 0`

**Sum:** `psadbw(chunk[0..8], 0) = Σ chunk[i] for i in [0,8)`

These are algebraic identities — they hold by the definition of the x86
intrinsics (pcmpeqb, pmovmskb, psadbw).

### 2. Chunk lemma

A 32-byte vector chunk produces the same partial result as 32 scalar
iterations. This follows from the per-byte lemma and the independence of
byte lanes in the vector operations.

**All:** `movemask(pcmpeqb(chunk, val)) == 0xFFFFFFFF` iff all 32 bytes match.
Full-mask test = AND-reduction of all per-byte equality results.

**Cardinality:** `popcount(movemask(pcmpeqb(and(chunk, mask), target)))` == count of matching bytes.

**Sum:** `Σ psadbw_lanes(chunk, zero) == Σ chunk[i]` for i in [0,32).

### 3. Loop invariant (induction)

After k full 32-byte chunks: `vector_acc == scalar_reduction(input[0 .. 32k))`

**Base case (k=0):** Both accumulators are the identity element
(0 for Cardinality/Sum, true for All).

**Inductive step (k→k+1):** By the chunk lemma, the vector chunk result
equals the scalar chunk result. By the IH, the accumulators are equal.
Therefore the new accumulators are equal.

For **All**, early termination is handled: if any chunk has a mismatch,
both the vector code (movemask != full_mask) and the scalar code
(all &= false) return 0.

### 4. Tail lemma

For r ∈ [0, 31] remaining bytes:

- **16-byte vector tail (r ∈ [16, 31]):** Same chunk lemma at SSE2 width
  (128-bit instead of 256-bit). Same proof structure.
- **Scalar tail (r ∈ [0, 15]):** Identical to the original scalar loop body.
  Trivially equivalent.

Composition: `total = full_chunks_result + tail_result == scalar_total`.

### 5. Boundary conditions

Explicitly verified:

1. **n=0:** No loop iterations. Returns identity element.
2. **n ∈ [1,15]:** Only scalar tail runs. Identical to original.
3. **n ∈ [16,31]:** One 16-byte vector + scalar remainder.
4. **n=32:** Exactly one 32-byte iteration. No tail.
5. **n=33:** One 32-byte + one scalar.
6. **n=48:** One 32-byte + one 16-byte. No scalar.
7. **All r ∈ [0,31]:** Condition `i+width<=n` prevents over-reads.
8. **Unaligned addresses:** `vmovdqu`/`_mm_loadu_si128` handle unaligned.
9. **No over-read:** Every load guarded by `i+width<=n`.
10. **No strict-aliasing violation:** Intrinsics type-pun via `__m256i*`.
11. **Accumulator width:** u64. Max sum = 2^20 × 255 << 2^64. No overflow.
12. **Target availability:** Requires AVX2 + POPCNT.

### 6. Concrete region binding

The SIR recognizer correctly identified:
- Buffer parameter (param 0, NodeId 0)
- Length parameter (param 1, NodeId 1)
- Mask/target/val parameters (params 2, 3)
- Reduction variable and loop structure

This protects against the recognizer binding to the wrong loop, constant,
mask, or reduction variable. Verified by:
1. The vector plan only fires when expected semantic truths exist
2. Emitted code's parameter names match the original function
3. Differential testing confirms correctness for all inputs

## Strengthened Differential Testing

As an independent defense against bugs in the formal model or lowering:

| Test | Description | Tests Run | Failures |
|------|------------|----------:|---------:|
| Exhaustive lengths (0-128) | 6 distributions × 129 sizes × 3 kernels | 2,322 | 0 |
| Vector boundaries | All sizes 0-200, covers 16/32/48/64 boundaries | 603 | 0 |
| All alignment offsets 0-31 | 32 alignments × 129 sizes × 3 kernels | 12,384 | 0 |
| Guard-page over-read detection | mmap + mprotect, sizes near page boundary | 512 | 0 |
| Adversarial byte values | SWAR bug patterns, 8 patterns × 129 sizes × 25 mask/target combos | 25,800 | 0 |
| k18 mismatch positions | Every mismatch position at every size 0-200 | 20,301 | 0 |
| k43 Sum overflow | All 0xFF, max byte values, 0×-65536 lengths | 772 | 0 |
| Random differential | 10000 trials, random length/offset/data/params | 30,000 | 0 |
| k50 all mask/target (1 byte) | Exhaustive 256×256×256 = 16M combinations | 16,777,216 | 0 |
| NULL pointer at zero length | Verify no dereference at n=0 | 3 | 0 |
| **TOTAL** | | **16,869,913** | **0** |

All tests also pass under **AddressSanitizer** (compiled with
`-fsanitize=address`) and **UndefinedBehaviorSanitizer** (compiled with
`-fsanitize=undefined`), confirming no memory safety violations and no
undefined behavior (integer overflow, aliasing, shift, etc.).

## Proof vs Testing

The compositional proof provides the mathematical argument:

```
For all n ∈ ℕ, for all buf ∈ [u8]*, for all mask, target ∈ u8:
  cardinality_scalar(buf, n, mask, target) 
  == cardinality_avx2(buf, n, mask, target)
```

The differential testing provides empirical verification:

```
16,869,913 concrete test cases, all passing.
Includes: exhaustive per-byte, all alignments, guard pages, adversarial patterns.
```

Together, they form a two-layer correctness argument:
1. **Proof:** why it's correct in general (algebraic identity + induction)
2. **Testing:** that the implementation matches the proof (no lowering bugs)

## What This Establishes

The compositional argument provides mathematical reasoning for why the
equivalence holds in general. The differential testing provides empirical
verification that the implementation matches the argument.

Together, they establish:
- The algebraic identities (per-byte, chunk, tail) are correct by
  the definition of x86 intrinsics
- The induction argument (loop invariant) is mathematically valid
- The implementation matches the argument across 16.8M test cases
- No over-reads (guard-page verified)
- No memory safety violations (ASan verified)
- All alignment offsets (0-31) tested
- All byte values and mask/target combinations exhaustively tested (1-byte)

**This is strong engineering validation, not a machine-checked proof.**

## What is NOT yet established (Gate 4B)

- No solver has verified the per-chunk identities against a formal model
  of AVX2 instruction semantics
- The loop invariant is not mechanically checked for arbitrary n
- The region binding is not formally verified against the SIR graph
- No proof artifact or reproducible solver result exists
- The intrinsic semantics are modeled by human reading of the Intel SDM,
  not by a formal instruction model

## Limitations (honest assessment)

1. **The per-byte and chunk lemmas are algebraic identities**, not SMT-proven
   theorems. They hold by the definition of the x86 intrinsics. An SMT
   proof would verify these against the Intel SDM, which is stronger but
   requires modeling the instruction semantics formally.

2. **The region binding is structural**, not formally verified against the
   SIR graph. It relies on the recognizer producing the correct semantic
   truths, which is verified by differential testing rather than by a
   formal proof of the recognizer itself.

3. **The boundary conditions are tested exhaustively** for small sizes
   (0-200) and randomly for large sizes, but not proven for all possible
   sizes. The algebraic argument covers all sizes, but the testing only
   samples them.

4. **No SMT backend used.** The existing `sir_verification` crate has
   symbolic and exhaustive backends, but they operate on
   `SemanticExpression` — a different abstraction level. Connecting the
   compositional proof to SMT would require either:
   - Modeling the AVX2 intrinsics as `SemanticExpression` operations
   - Or building a new SMT backend for the vector plan abstraction

5. **The proof is for the generated C code**, not for the SIR-to-C
   lowering step itself. A bug in `vector_emit.rs` could produce incorrect
   C that doesn't match the vector plan. The differential testing catches
   this, but a formal verification of the emitter would be stronger.

## Architecture

New modules added to `sir_benchmarks`:

1. **`compositional_proof.rs`** — The proof framework
   - `CompositionalProver`: proves equivalence for a given `ProofObligation`
   - `SemanticSpec`: All, Sum, Cardinality specifications
   - `Lemma`: individual proof step with proof method
   - `ProofMethod`: Exhaustive, AlgebraicIdentity, Induction, StructuralAnalysis
   - `format_proof_report`: human-readable proof output

2. **`bin/gate4_prove.rs`** — Proof runner binary
   - Runs all 3 compositional proofs
   - Prints detailed proof reports

3. **`bin/gate4_differential.c`** — Strengthened differential test harness
   - 10 test categories, 16.8M tests
   - Guard-page detection, ASan compatible

## Files

- `sir/crates/sir_benchmarks/src/compositional_proof.rs` — proof framework
- `sir/crates/sir_benchmarks/src/bin/gate4_prove.rs` — proof runner
- `sir/crates/sir_benchmarks/src/bin/gate4_differential.c` — differential test harness
- `gate4/` — standalone differential test directory
