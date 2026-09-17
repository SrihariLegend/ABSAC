# Recall gaps — workstream record

The D3/H2 lists four recall gaps. Their current status (2026-09-17),
established by re-running each corpus row through the pipeline:

| Gap | Kernel | Current status |
|---|---|---|
| u8 predicate All ("accumulator width") / dynamic extent | `w07_all_min`; constant-bound `w06_count_mismatch_const`, v3 `p04_count_ge_const_u32`, v5 `p07_count_eq_const_bound` | **Partially closed (2026-09-17, constant extents)**: the lowerer now promotes a `Pointer` parameter to a fixed `Array { element, length: K }` view when EVERY access is proven within one common constant K — each access is either a constant index or the carried counter of a strict counted loop (`counter = 0`, successor `counter+1`, termination exactly `counter < K`/`!= K`/`<= K-1`, NO early-exit conjunct); any uncovered/loop-varying-without-constant-bound access leaves the pointer type (no fabricated extent). The promoted view feeds the existing array structural recognizers, so w06/p04/p07 now **rewrite and execute natively clean** (24/24 cases each; +3 native rewrites, 22 total, 0 mismatches). **Still open:** runtime extents (`w07`, `p01`–`p06`, …) abstain by design — no sound fixed-width encoding exists yet. |
| map-then-sum | `w03_sum_plus_one`, v5 `p11_map_then_sum` | **Closed at recognition**: `sum(x ^ const)` maps and clang's reassociated `s += (e + 1)` shape (`tmp = acc + 1; next = tmp + e`) both derive a distinct `MappedSumReduction` concept — never `SumReduction`, so D5 raw-sum strictness is preserved. The reassociated form is detected by a new `sum_offset` reduction kind in the loop analysis (additive only). Follow-on: a mapped-sum candidate/recipe that preserves the map (recognition milestone only so far). |
| two-loop lowering | `w08_two_reductions`, v5/v4 `p12_two_loops`, v3 `p12_two_reductions`, v6 `p08_two_loops_const` | **CLOSED with native assurance (2026-09-17).** The lowerer composes sequential self-latching loops (first loop's exit walk stops at the second's guard; shared exit/return once; failures fall back to the explicit refusal) AND the SIR→C emitter composes sequential loops: carrier/output variables are namespaced by loop ordinal (`c{ord}_{i}`/`o{ord}_{i}`) and TupleExtract/FieldAccess resolve to the producing loop's outputs. Native differential: v2 `w08`, v3/v4/v5 `p12`, v6 `p08` are all clean; full sweep **103 clean / 0 mismatched / 45 lower-refused** across v2–d4 plus v6's **16 clean / 0 mismatched / 6 lower-refused**. Recognition: v5 and v3 both 11/12 positives, 0 false positives, 0 unsafe candidates/rewrites; Gate 6B 10/10. **Whole-function multi-loop candidates (follow-up, same day):** the composer now skips blocks consumed by earlier loops (the duplicate body access blocked constant-extent promotion); v6 p08 promotes to `[u8; 48]`, derives 2 regions / 15 truths, and applies **1 native-clean whole-function rewrite** (count loop → mask+popcount, sum loop unchanged), the first multi-loop rewrite. Runtime-bound two-loop kernels still abstain (no proven extent). History (preserved): the first composition attempt emitted wrong native C (w08 21/24, p12 21–24/24) and was reverted fail-closed until the emitter was fixed. Per-region outlining remains unusable as a composition basis (the w08 outline drops `add(%19,%6)`). |
| early-exit gep | `x08_early_exit_write` | Still refused at lowering: `loop with a separate latch block (multi-block back-edge) not modeled`. Needs an early-exit control-flow model (SIR has no break/early-return inside loops). |

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
