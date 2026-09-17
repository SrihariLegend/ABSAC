# C3 Any-only hardening report

**Configuration:** SIR optimizer with `any_only_registry()` (definition 4)

**Assurance claim:** SIR-level `SchemaChecked` only. This report does not
claim LLVM correctness, vector safety, machine equivalence, or a performance
win.

## Boundary report

| Boundary | Result | Evidence / abstention |
|---|---:|---|
| Lowering | corpus: 40/50; Gate 6A v2: 7/16 | Existing lowerer gaps are preserved; no lowering capability was added here. |
| SIR verification | 40/50; 7/16 | Recognition is gated on verification in `batch_run`. |
| Semantic recognition | 40/50; 7/16 | Corpus results unchanged; Any unit fixtures recognize structurally. |
| Authorization | passed for covered Any fixtures | Authorization remains source-fingerprint and immutable-database bound. Stale/forged IDs refuse. |
| ProposalBinding | passed for covered fixtures | Canonical replay rejects role, live-out, and frame mutations; ambiguity is fail-closed. Trip-count evidence now binds zero start, `induction < bound`, unit forward stride, exact array extent, unsigned integer semantics, and finite normal termination. |
| Candidate generation | covered Any candidate produced | The freeze registry enables only Any for optimizer execution; exploratory generation may still report other proposals in the batch diagnostics. |
| Theorem verification | passed | `Verifier::bind_checked_theorem` is the theorem issuance boundary; verifier-issued theorem assurance is `SchemaChecked`; definitions cannot self-upgrade assurance. |
| Candidate frame | passed for covered candidates | Exact Any detached graph grammar (`Pack`/`ArrayCmpMask`, zero bit-vector, `Ne`, and bound replacement site), wrong collection/predicate op/scalar/extent, undeclared/dangling input, and effectful candidates refuse. LLVM/vector memory safety remains open. |
| Application artifact | passed | `ApplicationChecker::issue` is the application issuance boundary; `CheckedApplication` is digest-bound over all fields and linked to `CheckedTheorem`. |
| End-to-end artifact | passed | Source, region, candidate, definition, authorization, role-map, live-out, assumptions, theorem digest, and assurance are matched before mutation. |
| Rewrite commit | 53/53 synthetic differential functions rewrote under Any-only | The engine constructs the matched artifact before `RewriteBuilder::apply()`. Corpus rewrites remain 0/50 and 0/16. |
| SIR differential execution | passed | Boolean Any lengths: `0,1,2,31,32,33,63,64,65,127,128`; predicate Any: six operators at lengths `0,1,31,32,64,65,128`; boundary, mismatch, outcome, alternating, and deterministic random cases pass. |
| LLVM differential execution | not run | No LLVM/SIR execution bridge is claimed by this freeze. |
| Performance | not run / no claim | No calibrated CPU-pinned comparison to `clang -O3`, confidence interval, assembly inspection, AVX2 tail/alignment audit, or savings witness exists for this SIR-only path. |
| Independent H3 | pending | H3 must be generated after this freeze by an evaluator independent of the Any implementation. |

## Mutation coverage

`any_vertical_slice_hardening.rs` exercises canonical replay refusal for:

- collection, element access, induction start, bound, stride; nonzero starts and partial extents refuse trip-count binding;
- accumulator, identity, reduction position, loop/result identity;
- predicate operator and scalar;
- live-out kind, binding state, dead-use evidence, and replacement site;
- every first-slice frame field;
- candidate collection, extent, predicate, input, and effect;
- source-frame counted-loop evidence and artifact frame digests;
- a source with a second live-out;
- authorization after a real source rewrite.

`verifier_quarantine_tests.rs` additionally checks post-issuance digest
binding for every theorem/application identity field and refuses theorem or
application replay against another artifact.

## Corpus result

Commands:

```bash
cargo run -q -p sir_benchmarks --bin batch_run -- --any-only ../corpus/kernels.ll
cargo run -q -p sir_benchmarks --bin batch_run -- --any-only ../gate6a/v2_corpus.ll
```

Results:

```text
40/50 lowered, 40/50 verified, 40/50 recognized, 0/50 rewrote
 7/16 lowered,  7/16 verified,  7/16 recognized, 0/16 rewrote
```

The zero witness frontier is intentional. No corpus result is promoted to a
commercial performance claim. Lowerer gaps, signed comparisons, volatile /
atomic operations, two-loop functions, and other existing abstentions remain
reported rather than repaired opportunistically during C3.

## Freeze boundary

This is a hardening/freeze checkpoint, not a general reduction-family
milestone. All/Parity/Popcount remain outside C3 until their recipes,
candidates, theorem obligations, application checks, and artifacts consume
the same canonical binding. No Arena/MCTS implementation is introduced by
this checkpoint.

> **Post-annotation (2026-09-17):** the recipe/candidate side now consumes
> the canonical binding for All/Parity/Popcount and both BitScan recipes
> (`docs/IMMEDIATE_PROGRAM.md`); theorem obligations, application checks
> and artifacts for those families are still to be audited, so the C3
> freeze boundary is unchanged.
