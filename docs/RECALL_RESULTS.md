# Recall gaps — workstream record

The D3/H2 lists four recall gaps. Their current status (2026-09-17),
established by re-running each corpus row through the pipeline:

| Gap | Kernel | Current status |
|---|---|---|
| u8 predicate All ("accumulator width") / dynamic extent | `w07_all_min`; constant-bound `w06_count_mismatch_const`, v3 `p04_count_ge_const_u32`, v5 `p07_count_eq_const_bound` | **Partially closed (2026-09-17, constant extents)**: the lowerer now promotes a `Pointer` parameter to a fixed `Array { element, length: K }` view when EVERY access is proven within one common constant K — each access is either a constant index or the carried counter of a strict counted loop (`counter = 0`, successor `counter+1`, termination exactly `counter < K`/`!= K`/`<= K-1`, NO early-exit conjunct); any uncovered/loop-varying-without-constant-bound access leaves the pointer type (no fabricated extent). The promoted view feeds the existing array structural recognizers, so w06/p04/p07 now **rewrite and execute natively clean** (24/24 cases each; +3 native rewrites, 22 total, 0 mismatches). **Still open:** runtime extents (`w07`, `p01`–`p06`, …) abstain by design — no sound fixed-width encoding exists yet. |
| map-then-sum | `w03_sum_plus_one`, v5 `p11_map_then_sum` | **Closed at recognition**: `sum(x ^ const)` maps and clang's reassociated `s += (e + 1)` shape (`tmp = acc + 1; next = tmp + e`) both derive a distinct `MappedSumReduction` concept — never `SumReduction`, so D5 raw-sum strictness is preserved. The reassociated form is detected by a new `sum_offset` reduction kind in the loop analysis (additive only). Follow-on: a mapped-sum candidate/recipe that preserves the map (recognition milestone only so far). |
| two-loop lowering | `w08_two_reductions`, v5/v4 `p12_two_loops`, v3 `p12_two_reductions`, v6 `p08_two_loops_const` | **CLOSED with native assurance (2026-09-17).** The lowerer composes sequential self-latching loops (first loop's exit walk stops at the second's guard; shared exit/return once; failures fall back to the explicit refusal) AND the SIR→C emitter composes sequential loops: carrier/output variables are namespaced by loop ordinal (`c{ord}_{i}`/`o{ord}_{i}`) and TupleExtract/FieldAccess resolve to the producing loop's outputs. Native differential: v2 `w08`, v3/v4/v5 `p12`, v6 `p08` are all clean; full sweep **103 clean / 0 mismatched / 45 lower-refused** across v2–d4 plus v6's **16 clean / 0 mismatched / 6 lower-refused**. Recognition: v5 and v3 both 11/12 positives, 0 false positives, 0 unsafe candidates/rewrites; Gate 6B 10/10. **Whole-function multi-loop candidates (follow-up, same day):** the composer now skips blocks consumed by earlier loops (the duplicate body access blocked constant-extent promotion); v6 p08 promotes to `[u8; 48]`, derives 2 regions / 15 truths, and applies **1 native-clean whole-function rewrite** (count loop → mask+popcount, sum loop unchanged), the first multi-loop rewrite. Runtime-bound two-loop kernels still abstain (no proven extent). History (preserved): the first composition attempt emitted wrong native C (w08 21/24, p12 21–24/24) and was reverted fail-closed until the emitter was fixed. Per-region outlining remains unusable as a composition basis (the w08 outline drops `add(%19,%6)`). |
| early-exit gep | `x08_early_exit_write`, v6 `p09_first_set_const` (96), v7 `p09_first_set_early_return` (80) | **CLOSED for the canonical search CFG (2026-09-17)**: clang's early-return search is a header/latch/merge CFG (`for (i…) if (a[i]) return i;`). The lowerer synthesizes the canonical found-flag SIR loop (`found' = found \| hit`, `index' = hit && !found ? i : sentinel`, termination `!found && i < BOUND`), the scans recognizer derives FirstOccurrence, and the bitscan recipes apply. Non-matching CFGs still fall back to the explicit refusal. v6 p09 (96) and v7 p09 (80) lower, promote to `[T; N]`, **rewrite to `ctz(pack)` and are native-clean** (rewrites=1 each). Assurance: extents ≤ 64 are bit-blast discharged (ConcreteSolverChecked); the 80/96 extents exceed the 64-bit concrete solver and are discharged by the symbolic scan identity (`FirstTrue(seq) → TrailingZeros(Pack(seq))`), honestly issued **SchemaChecked**. A sentinel guard was added: the no-hit result must equal the extent (position-select form, or the successor-as-result bound) — an over-strict first version broke the PS001 successor-as-result shape, which the fallback now covers. While landing the lowering, the native differential also caught that the emitter had no `BoolAnd`/`BoolOr`/`BoolNot` arms (emitted `0`); they now emit `&&`/`||`/`!`. |

v7 (2026-09-17) added two data points to the multi-loop row: a
three-sequential-loop constant-extent kernel (p07) lowers, derives 20
truths / 5 candidates and applies 1 native-clean rewrite, and a
two-loop kernel with different constant extents (40 and 64) promotes
the buffer to the maximum view (64) with 2 candidates and 0 rewrites.
The post-tested do-while reconstruction was re-validated at strides 2
and 8 (native clean); a runtime-step do-while refuses loudly. See
docs/GATE6A_V7_RESULTS.md.

## Change landed with this analysis

Predicate-collection reductions (Cardinality / Disjunctive /
Conjunctive / Exclusive) over a **declared** array now receive a
`DynamicBooleanSequence { length }` structural description plus the
`PredicateCollectionReduction` (or `BooleanCollectionReduction`) role,
even when the region has no `LogicalSequence` truth of its own. Without
a description, `derive_roles` had nowhere to attach the collection role
and inference never formed a context, so generation produced zero
candidates.

The fallback is deliberately **extent-guarded**: a region whose
collection is a runtime-length pointer gets no description at all.
Inventing the historical stub extent (64) would fabricate a bound the
source does not have — the exact class of error the D3/D5 gates exist to
prevent.

Regression test:
`sir/crates/sir_semantics/tests/recognizers.rs::predicate_reduction_over_declared_array_gets_a_structure`.

## Next candidates

1. Map-then-sum: recognize a certified element map (e.g. `x ^ k`,
   `x + k`) and give it a candidate that preserves the map natively.
2. Two-loop lowering: extend the lowerer to stitch sequential Loop
   nodes (the region extractor already bounds the work).
3. Early-exit loops: model the break/early-return edge explicitly.

## v8 held-out data points (2026-09-17)

The v8 generation tested the early-exit/scan frontier blindly:

- Early-exit searches rewrite at extents 48 (concrete solver), 96/128
  (u8) and 80 (u16) (symbolic identity, SchemaChecked) — 7 native-clean
  rewrites in the corpus.
- **Sentinel guard confirmed**: a sub-range search (48 accesses, 128
  no-hit result) and a zero-sentinel search are recognized
  (`FirstOccurrence`, 2 candidates) but apply 0 rewrites; the lowering
  itself is native-clean.
- **Predicate extraction (follow-up)**: the position pack helper now
  extracts the hit comparison from the position select instead of
  assuming implicit non-zero, so the scalar-eq search (`Eq(elem, key)`)
  rewrites to `mask_cmp(Eq, key)` + `ctz` and is native-clean; negation
  and swapped ordered operands are normalized, compound predicates are
  refused. v8 p04 moved from recognized-only to rewritten (8 native
  rewrites in v8; 41 cumulative).
- **New recorded limitations**: a runtime-extent search and a
  *search-then-second-loop* shape are refused at lowering (the counted
  successor test and the merge-to-continuation composition are not
  modeled). Multi-loop + early-exit composition is the next frontend
  item after runtime extents.

## v9 held-out data points (2026-09-17)

The v9 generation blind-tested the position-predicate extraction:

- **All ten predicate probes rewrote and ran native-clean**: Eq/Ne/Lt/
  Ge/Gt over u8/u16/u32, against zero, literal constants and
  parameters, with swapped operands (`key < elem` → `Gt`,
  `key >= elem` → `Le`), at extents 40–128 (concrete solver ≤64 and
  symbolic >64). The extracted mask is the exact hit predicate.
- **Refusal discipline confirmed blind**: a compound predicate
  (`(x & 1) && x > 3`) and a sentinel-mismatched sub-range search both
  recognize `FirstOccurrence` with candidates but apply 0 rewrites.
- **Assurance boundary pinned**: a strict `ConcreteSolverChecked`
  policy quarantines the >64 symbolic scan proof (new test in
  `scan_extent_assurance.rs`), so the weaker assurance can never be
  silently upgraded.
- Cumulative native evidence: 185 clean / 0 mismatched / 66
  lower-refused, 54 native-clean rewrites.

### Runtime-extent search lowering (follow-up)

The lowerer's early-exit synthesis now accepts clang's **guarded**
runtime-extent search: an `n == 0` entry guard, the header/latch
search, and a merge that clamps the phi with `llvm.umin(phi, n)` before
returning. The merge's post-instructions are emitted after the
synthesized loop (so the clamp is not assumed to be a no-op) and the
extra zero-trip predecessor is validated against the guard
(`sentinel == 0`). The pointer stays a pointer — no extent is
fabricated — FirstOccurrence derives, the candidate proof fails (no
collection role), and v8 `p14` / v9 `p16` are native-clean with 0
rewrites (lower-refused totals drop by one in each corpus; cumulative
187 clean / 0 mismatched / 64 lower-refused).
