# Recall gaps — workstream record

The D3/H2 lists four recall gaps. Their current status (2026-09-17),
established by re-running each corpus row through the pipeline:

| Gap | Kernel | Current status |
|---|---|---|
| u8 predicate All ("accumulator width") | `w07_all_min` | **Refined**: lowers and recognizes `ConjunctiveReduction` + `PredicateMap`; a Reduction authorization is issued. The remaining blocker is the **dynamic extent** — the collection is a runtime-length pointer (`len` parameter), so no sound fixed-width `pack`/`ArrayCmpMask` candidate exists. No fabricated 64-element extent is created. |
| map-then-sum | `w03_sum_plus_one`, v5 `p11_map_then_sum` | **Closed at recognition**: `sum(x ^ const)` maps and clang's reassociated `s += (e + 1)` shape (`tmp = acc + 1; next = tmp + e`) both derive a distinct `MappedSumReduction` concept — never `SumReduction`, so D5 raw-sum strictness is preserved. The reassociated form is detected by a new `sum_offset` reduction kind in the loop analysis (additive only). Follow-on: a mapped-sum candidate/recipe that preserves the map (recognition milestone only so far). |
| two-loop lowering | `w08_two_reductions` | Still refused at lowering: `multiple loops sharing an exit CFG (nested/sequential loops) not modeled`. **2026-09-17 evidence:** per-region outlining is NOT a sufficient composition basis — for w08 the second region's outline returns only its own sum (`ret %19`) while the function returns the shared-exit combine `add(%19, %6)`, so composing outlines would silently drop the first loop's live-out. Composition must happen inside the loop lowerer (build each loop in block order; the first loop's exit walk already stops at the second loop's conditional guard, so its exit phis feed the second loop's pre-header; emit the shared exit/return once), with the historical refusal as the fail-closed fallback. |
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
