# Optimizer blockers and execution plan

Recorded 2026-09-17, baseline commit `2cae1fd`.

## Goal and scope

Make ABSAC reliably recover semantic structure, generate profitable implementations,
and preserve behavior on unfamiliar programs. “Best optimizer” is not a universal
objective: performance depends on target, input distribution, code size and compile
budget. Measure those dimensions explicitly rather than equating IR compression
with machine speed.

This is a staged engineering/research backlog, not a claim that all items can be
fixed in one patch. Preserve the architecture freeze: extend existing knowledge
interfaces; keep SIR inspection in the layers already responsible for it. Never
weaken recognition/proof gates to manufacture benchmark wins.

## Baseline evidence

- `emitter_native/README.md`: 227 native-clean functions, 72 lowering refusals,
  61 rewrites, zero observed mismatches. These are corpus results, not universal
  correctness or 227 optimized functions.
- Computed-sentinel search lowering now works, but runtime-extent variants still
  have zero rewrites. `docs/GATE6A_V11_RESULTS.md`, `docs/RECALL_RESULTS.md`.
- Gate 6B automated shared-traversal fusion: 1.70–2.07x over independent Engine-0
  vectorization on five positive kernels. This is an expanded action-space win,
  not a controlled demonstration of deeper search. `docs/GATE6B_RESULTS.md`.
- Gate 5A candidate performance varies greatly even on the same ISA:
  `k43_sum_ascii`, 4096 bytes, AVX2 LaneAccumulation U2 = 232.9 ns versus
  Psadbw U1 = 25.2 ns. `gate5a_results.csv`. The `speedup_vs_orig` field is
  candidate/original runtime (lower is better). Engine 0 already chose Psadbw;
  choosing it again is not a new optimization.
- Stronger theorem proofs do not imply end-to-end equivalence: application
  assurance is capped at SchemaChecked, and native lowering is outside it.
  `sir/crates/sir_verification/src/application_artifact.rs`.

## Work packages, in priority order

### P0 — Correct local cost accounting

**Observed:** `CostDeriver::compute_region_cost` ignores the region ID and charges
all function nodes and whole-function depth to every structural region.

**Plan:** carry explicit semantic region membership into structural knowledge via
an additive interface. Count only region members; dependencies outside the region
are boundaries, not recursively charged work. Use dataflow rather than loop
containment edges for depth. Do not invent membership when unavailable.

**Acceptance:** two regions in one function receive distinct costs; adding unrelated
work changes neither local profile; external dependencies do not inflate depth;
missing/invalid membership cannot fall back to whole-function costs; loop depth
computation terminates; existing relevant suites and workspace tests are checked.

**Limits:** node membership is not a proof of exclusive ownership or removable work.
This remains an abstract SIR cost, not a latency/throughput or dynamic loop model.
Shared work, per-candidate replacement boundaries, and actual lowering costs need
separate follow-up.

**Status (2026-09-17):** fixed.
`CostDeriver::compute_region_cost` now uses explicit semantic membership recorded
on `StructuralDatabase` (`set_region_nodes`, maintained by `SemanticEngine::derive`);
depth is a topological, region-local dataflow computation. Missing, empty, dangling,
or cyclic membership yields no profile instead of whole-function costs; membership
follows region rekeying. Regression tests live in `sir_semantics/src/cost_deriver.rs`
(`independent_regions_have_local_costs`, `external_work_does_not_inflate_local_depth`,
`unknown_or_invalid_membership_is_not_costed`, `loops_use_dataflow_depth_and_cycles_are_uncosted`,
`membership_survives_rekeying`). The first test was confirmed red before the fix
(a one-node region was charged five nodes) and green after; workspace: 697 passing,
0 failing. This corrects cost scope only — not latency/throughput prediction, and
not the selector/beam objective (P1).

### P1 — Make the optimization objective consistent

**Observed:** default selection uses illustrative, target-independent savings;
beam pruning/final selection use return-reachable node count. Estimated replacement
costs are handwritten. Neutral/worsening steps and same-region alternatives are
removed before exploration.

**Plan:** define an explicit objective and report components; introduce target-aware
costing through additive extension points; retain alternatives under a bounded
compile budget. Score actual lowered replacements where possible. Audit region
ordering/truncation and enforcement of configured rewrite limits.

**Acceptance:** controlled cases where fewer nodes are slower select the faster
implementation; unrelated region order does not decide explored profitability;
compile budget is enforced; regressions and compile time are reported. Compare
search methods on identical action sets before investing in deeper search.

**Status (increment 1):** rewrite-limit enforcement fixed. The optimizer checks
`max_total_rewrites` before expanding each path and reports the existing
`IterationLimitReached` reason when exhausted. Zero skips all pipeline passes;
`None` remains unlimited. This is a per-path rewrite ceiling, not a global
speculative-work/compile-time budget. Tests on actual rewritable arithmetic were
red first: budget zero allowed one rewrite; budget one allowed two. Both now pass,
and a remaining-budget test confirms normal convergence. Workspace: 700 passed,
0 failed, 3 ignored.

**Status (increment 2):** selected-branch ordering fixed in
`sir/crates/sir_optimizer/src/optimizer.rs`. Region winners are ranked by descending
selector score, then ascending candidate ID, before applying the beam-width limit.
The production path uses `ranked_branch_indices`; its tests cover score priority,
ties, reordered inputs, empty/single inputs, and integer extremes. The original
ordering was extracted without behavioral change and three tests were confirmed
red before implementing the sort. Optimizer tests and the workspace pass:
704 passed, 0 failed, 3 ignored. `sir_selection`, authorization, and rewrite checks
are unchanged. This is an ordering regression test, not a measured speedup or a
new end-to-end multi-region performance fixture. Candidates rejected during
application still consume an attempt slot; fallback expansion is not added here.

Whole-path objective changes remain deferred. Neither cumulative predicted
savings (which may double-count overlapping work) nor a sum of recognized region
costs (which changes with recognition coverage) is automatically an accurate
whole-program objective. Define and test that contract before replacing node-count
beam pruning/final scoring. Same-region alternatives remain open.

**Status (increment 3):** zero beam width and explicit path stop reasons fixed.
`Some(0)` now skips all pipeline passes, preserves the input, and reports the new
`BeamWidthLimitReached` variant (takes precedence when another budget is also zero).
Terminal paths carry their stop reason alongside their state, so convergence on
the last allowed pass reports `FixedPoint`, not `IterationLimitReached`. Active
paths left at iteration exhaustion and rewrite-budget stops retain
`IterationLimitReached`. A path stopped by the existing panic boundary now reports
`PanicContained` rather than convergence; no new panic-injection test was added
in this increment. Consumers exhaustively matching the public enum must handle
the two added variants. Whole-path scoring and authorization/proof gates are unchanged.

Both targeted regressions were red before the fix and green after; a zero-iteration
budget regression also passes. Workspace: 707 passed, 0 failed, 3 ignored.
`FixedPoint` is documented as no rewrite applied under the current pipeline policy,
not global optimality (the final iteration outcome explains refusals/failures).
**Status (increment 4):** failed-application fallback fixed in
`sir/crates/sir_optimizer/src/optimizer.rs`. The beam-width limit now counts
successful successors: an authorization/binding/rewrite rejection no longer
discards that beam slot; the next ranked candidate is attempted. Attempts remain
bounded by the selected region winners; score ordering, authorization, proof and
structural checks are unchanged. End-to-end regressions in
`sir_optimizer/tests/branch_fallback.rs` use a real multiply-by-constant function,
the production recipe, and a refusal-injecting wrapper recipe (no forged proofs or
sentinel bypasses): refusal-then-success fills the beam, all-refused exhausts with
`RewriteFailed`, and a first-try success attempts nothing further. The two fallback
tests were red before the change (the second region was never attempted) and green
after.

**Correctness fix uncovered by the fallback tests:** with two independent multiply
regions, `derive_roles` bound every region's `ArithmeticOperation` role to the
last `Mul` node in the arena (the v0.1 single-region assumption). Both regions'
candidates therefore patched the same node while authorization binding still
referred to their own region. `derive_roles` now restricts the MultiplyPowerOfTwo
role search to `region.nodes`. Direct regression tests in
`sir_semantics/tests/multiply_role_binding.rs` assert per-region binding against
SemanticDatabase output (operator within region, region's own Mul, distinct
operators); the main test was red-verified by temporarily restoring the old
whole-function scan and green after. Follow-up: modulo, division and shift-mask
role scans were subsequently confirmed region-blind and fixed (see P3 below).
This is not a complete audit of every role family.

**Status (increment 5):** applied-score diagnostics added, not whole-path ranking.
`IterationRecord::applied_score` records the successful candidate's complete
`TransformationScore` (candidate ID, strategy, total and component deltas). It is
`None` on passes without a successful rewrite, including recipe refusals. IDs are
pass-local; callers should interpret the score alongside that pass's candidates.
Existing callers constructing complete `IterationRecord` literals need the new
field (or a default). Scores are estimates, not measured savings or an additive
whole-program objective. The attempted cumulative-savings ranking change was
removed before validation; beam pruning and final selection still use node count.

A real-pipeline regression refuses the first candidate, then independently derives
the successful fallback candidate's source role and score, verifying that the
record refers to the successful operator rather than the refused one. It was red
with an unpopulated score and green after recording it. All-refused passes report
no applied score. Workspace: 714 passed, 0 failed, 3 ignored.

**Status (increment 6):** the whole-path objective and compile budget are now
explicit, additive contracts. `sir_optimizer/src/objective.rs` defines
`SearchObjective`, `PathView`, `PathScore`, and `ScoreComponent`; beam pruning and
final selection both rank by the configured objective, and
`OptimizationResult::objective` reports its name, the number of terminals
considered, and the winning score/components. `NodeCountObjective` reproduces the
historical return-reachable-node-count rule exactly (negative total, stable sort
on ties); `Optimizer::new` keeps it with `DefaultCostModel`.
`Optimizer::with_objective` replaces only path ranking, while
`Optimizer::with_strategy` replaces both the region candidate cost model and the
path objective. The cost model is adapted inside `sir_optimizer`, so the frozen
`sir_selection` interface is unchanged.

`OptimizerConfig::max_pipeline_expansions` adds a global deterministic
speculative-work ceiling across all beam paths; one expansion is one full
pipeline pass. Zero runs no pass, the per-path rewrite ceiling is checked first,
and zero beam width retains precedence. Paths stopped by the ceiling terminate
with the new `CompileBudgetReached` reason (exhaustive consumers must handle it).
This bounds work units, not wall-clock time or candidate attempts.

`sir_optimizer/tests/objective.rs` controls both extension points: the default
report matches `NodeCountObjective`; a synthetic weighted target-cost objective
selects the path with MORE return-reachable nodes because its modeled target cost
is lower (52 versus 250 for the node-count default's choice); and a custom cost
model reverses the region winner on a merged modulo/shift-mask region.
`sir_optimizer/tests/compile_budget.rs` covers zero budget (no pass), a
one-expansion budget (one rewrite, differential equivalence), a generous budget
(two rewrites over three expansions), and beam-width precedence. Workspace: 739
passed, 0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: these are extension points exercised by synthetic target models. No real
target/latency model ships; replacement costs remain generation-time estimates;
neutral/worsening steps are still dropped before exploration; and the expansion
budget is not a wall-clock deadline. `OptimizationResult` gained fields, so
exhaustive constructors/matchers must be updated.

**Status (increment 7):** same-region alternatives are retained under the
bounded beam. `Selector::rank_all_improving` is an additive `sir_selection`
entry point with the same scoring, strict-improvement, and tie-break policy as
`select_all`, but it returns every improving candidate in global rank order
instead of discarding each region's non-winners. `select_all` keeps its
one-winner-per-region behavior for existing callers. The optimizer's selection
stage now uses the ranked list, so a second (or later) candidate from one region
can occupy a beam slot instead of being removed before exploration. The number
of attempted successors per expansion is unchanged: the beam width still caps
attempts, and the global pipeline-expansion budget still caps speculative
compile work. `IterationRecord::candidates_selected` now means "candidates
admitted to ranked exploration", including alternatives that the beam may not
reach.

A controlled end-to-end regression (`objective.rs`) uses a custom cost model
that ranks the modulo candidate first on a merged modulo/shift-mask region. With
one beam slot only the modulo rewrite is attempted; with two beam slots the
shift-mask alternative is retained, the node-count objective selects its
smaller result, and both terminals are reported. The test was red-verified by
temporarily collapsing ranked selection back to the first winner per region (one
terminal instead of two). Two `sir_selection` unit regressions pin ranked
retention and the unchanged `select_all` winner policy. Workspace: 742 passed,
0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: alternative retention is now on by default for optimizer callers; the
per-region limit arrives in increment 8. Neutral/worsening steps are still
filtered before exploration; retaining them requires an explicit cycle/budget
policy and remains open, as do real target models and measured
lowered-replacement costs.

**Status (increment 8):** `OptimizerConfig::region_alternative_limit` exposes
the per-region alternative policy. `Some(1)` is winner-only search (the v0.1
behavior), `None` retains every strictly-improving candidate (current default),
and `Some(0)` selects none. The optimizer applies the limit after global
ranking; the beam width and pipeline-expansion budget still bound attempted
successors and speculative work.

Search methods are now compared on an identical action set in
`sir_optimizer/tests/objective.rs`. On a merged modulo/shift-mask fixture where
both candidates prove, two runs use beam width two and differ only in
`region_alternative_limit`: winner-only selects one candidate and returns the
modulo path (9 return-reachable nodes, objective -9); ranked selection keeps
both candidates and returns the shift-mask path (8 nodes, objective -8). The
test asserts both runs discharged the same two proofs, then reports rewrites,
nodes, and scores for each method. A second regression pins `Some(0)` to
`NoSelection` with one expansion and no rewrite. Workspace: 747 passed, 0
failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: the comparison uses deterministic in-process counters (proofs,
candidates, rewrites, expansions, objective score), not wall-clock compile time
or native runtime. The controlled regression covers one fixture; the
corpus-scale follow-up below broadens it. Real target models, measured
lowered-replacement costs, and neutral/worsening retention remain open.

**Corpus-scale search comparison (same day):** `emit_c_diff` now accepts
`--winner-only` and `--beam N`, so the native differential can run the same
corpus functions under different search policies on identical action sets. All
15 development corpora (227 clean functions, 48/24 cases each) were run three
ways: default ranked selection with beam 3, winner-only selection with beam 3,
and ranked selection with beam 1.

| Policy | Clean | Mismatched | Lower-refused | Native rewrites | Per-function rewrite differences |
|---|---:|---:|---:|---:|---:|
| ranked, beam 3 (default) | 227 | 0 | 72 | 61 | — |
| winner-only, beam 3 | 227 | 0 | 72 | 61 | 0 |
| ranked, beam 1 | 227 | 0 | 72 | 61 | 0 |

The per-function comparison joins the `NATIVE <name> ... rewrites=N` lines from
each run; every function has the same rewrite count under all three policies.
On this corpus, retaining same-region alternatives and widening the beam
therefore adds no rewrite coverage, while all three runs re-confirm
0 native mismatches after the arithmetic/mask/role changes. This is development-
corpus evidence, not fresh held-out evidence; P5 remains open, and deeper search
still needs a corpus where it demonstrably wins before further investment.

**Status (increment 9):** proof-obligation attempts are now globally accounted
and bounded. One expansion can otherwise verify an unbounded number of
generated candidates even under a tiny expansion budget.
`OptimizerConfig::max_proof_attempts` counts verification attempts across all
beam paths; `None` is unlimited and `Some(0)` verifies nothing. A pass truncated
by the ceiling reports the new total in `OptimizationResult::proofs_attempted`
and terminates with `CompileBudgetReached` only when the truncation leaves it
unable to advance; expansions that still produced a proven branch continue, and
the next expansion stops at the exhausted ceiling. Per-pass records keep their
own attempted count, so a successful truncated pass reports what it verified.

`sir_optimizer/tests/compile_budget.rs` covers zero attempts (NoProof,
CompileBudgetReached, one expansion, no rewrite), a one-attempt ceiling (one
proof, one rewrite, two expansions, differential equivalence), and an unlimited
run that converges and reports the total. The one-attempt test was red-verified
by temporarily disabling the ceiling check (4 attempts instead of 1). Workspace:
750 passed, 0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: proof attempts are bounded; rewrite applications remain bounded by the
beam width per expansion. This is still a deterministic work-unit budget, not
wall-clock compile time, and it applies to the optimizer's in-process
verification loop, not to native lowering/emission. Real target models, measured
lowered-replacement costs, neutral/worsening retention, and a broader
search-method study remain open.

**Status (increment 10):** rewrite-attempt accounting and wall-clock compile
time are now reported. `OptimizationResult::rewrite_attempts` counts every
application attempt across all paths, successful or refused (bounded by
expansions × beam width); `OptimizationResult::compile_time` reports the
in-process wall-clock duration of `optimize` (analysis, semantics, inference,
generation, verification, selection, rewrite), excluding native
lowering/emission. This closes the "compile time is reported" acceptance item
without introducing a wall-clock budget or using time in any decision.

Regressions pin the accounting: the identical-action-set comparison reports
1 attempt for winner-only versus 2 for ranked search (with per-run compile
times), the all-refused fallback test asserts two refused attempts, the
fallback-success test asserts one refusal plus one success, and the zero
alternative limit asserts zero attempts. All 17 corpora re-run: 241 clean /
0 mismatched / 72 lower-refused / 96 native rewrites with zero per-function
rewrite differences. Workspace: 762 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: compile time is informational and nondeterministic; it includes solver
proof work but not native emission or execution, and no budget or ranking
decision reads it. Real target models, measured lowered-replacement costs,
neutral/worsening retention, and a broader search-method study remain open.

**Status (increment 11):** measured-retention tooling and the first isolated
rewrite-performance data. `emit_c_diff` gained `--time` (original IR vs emitted
C), `--time-baseline` (unoptimized emitted C vs optimized emitted C, both
`clang -O1`), and `--measured-retain` (per-definition-family trials). The timing
driver runs three alternating rounds and takes each side's minimum; trials that
applied zero rewrites are rejected without reading their timers, and the
retained composition is re-run through the differential harness.
`OptimizerConfig::excluded_definitions` is the execution-policy hook; its
regression was red-verified (2 rewrites without the filter, 0 with it).

Findings (Ryzen 5 3600-class host, `clang -O1`, fixed random buffers, 50k–200k
repetitions):

- same-backend control: 67 unchanged functions report geomean 1.004, so the
  harness resolves roughly 5% effects;
- 58 rewritten functions report geomean 0.441 (2.3x slower), with 11 improving
  more than 5% and 32 regressing more than 5%. Reduction families win (up to
  1.84x); array bitscan/`FirstOccurrence` rewrites regress (up to ~87x) when the
  source loop exits early on random data;
- measured retention across 7 corpora / 104 functions with a 5% margin keeps 8
  of 59 default rewrites (all `reduction`, 1.08–1.81x) and rejects the rest;
- end-to-end against the original IR compiled by clang, 49 rewritten functions
  report geomean 0.296 and 59 unchanged functions 0.801, so the emitted-C
  baseline is itself a large factor on these kernels.

Scope: single-host, workload-specific numbers under one fixed distribution.
They justify a retention policy but are not a production cost model. Production
selection still uses the static `DefaultCostModel`; wiring measured or
target-aware weights into `Optimizer::with_strategy`, and testing no-hit /
long-scan distributions, remain open.

**Status (increment 12):** distribution-controlled timing. `emit_c_diff
--time-fill V` fills both measurement buffers with a fixed byte (for example
`0x00` to force no-hit full scans in "first set/nonzero" searches, or `0xFF`
for equality searches), so the same rewrite can be scored under early-exit and
full-scan distributions.

Findings on v8/v9 search corpora (`clang -O1`, 100k repetitions):

- no-hit full scans are mostly break-even: extent-72/56/64 cases measure
  0.89–0.95x for the rewrite and extent-80/96/128 measure 0.96–1.01x; only
  extent-48 measures 1.13–1.16x in favor of the bitscan rewrite;
- measured retention under the no-hit fill keeps 3 of 48 functions (two
  small-extent bitscan rewrites at ~1.13x plus one reduction), versus 8 of 104
  on the early-exit random fill (all reductions);
- the earlier early-exit result (bitscan up to ~87x slower) and this full-scan
  result are both real: array bitscan/nonzero-search profitability is marginal
  and extent-dependent on this host, not a universal win.

(Superseded by the TSC/null/composition-guarded harness below: the extent-48
bitscan composition re-measures at 0.76–0.89x and is rejected; the corrected
no-hit survey retains only the reduction family.)

Scope: the retain/reject decision is now demonstrably distribution-sensitive,
which is the intended behavior of a measured policy. It is still benchmark-level
retention, not production selection; wiring a profile/target model into
`Optimizer::with_strategy` remains open.

**Status (increment 13):** measured weights are now a production selection
input. `sir_selection::WeightedCostModel` wraps any `CostModel` and divides each
candidate's modeled replacement cost by a per-`DefinitionId` factor
(`baseline_time / replacement_time`, `1.0` neutral, `<1.0` a measured
regression). A candidate whose rescaled total stops being strictly positive is
rejected by the existing selector policy. `Optimizer::with_weighted_cost`
constructs the default model plus `NodeCountObjective`, so callers can feed
measured or profile-derived factors without replacing the pipeline; an empty
factor map reproduces the default behavior exactly.

`sir_optimizer/tests/objective.rs::measured_weights_can_reject_a_definition`
was red-verified by temporarily ignoring the factor map (2 rewrites instead of
0): a factor of `0.0` for definition 100 rejects both chained modulo
occurrences while the unweighted control still rewrites them. Scope: this is
the policy hook, not a shipped cost profile; choosing factors per target and
validating them against held-out distributions remains open, as does P2.

**Status (increment 14):** factor export/import and production validation.
`emit_c_diff --measured-retain --write-factors FILE` exports per-definition
geometric-mean speedups; `--cost-factors FILE` feeds them through
`Optimizer::with_weighted_cost` on the normal differential path. Retained
compositions are also re-timed: over 7 corpora the 8 retained functions report
geomean 1.36 (min 1.01, max 1.81) while the 96 rejected compositions report
geomean 0.997 and median 1.000, i.e. no-rewrite noise.

Production-path validation on v8 (8 default rewrites, 0 mismatches):

- early-exit random profile: exported bitscan factor 0.0449 and reduction factor
  1.1186; the production path applies 3 rewrites (all reduction), rejecting the
  five bitscan rewrites. The exclusion-based harness retains 2; the difference
  is p08, whose aggregate factor clears the bar while its per-function margin
  did not.
- no-hit profile: exported bitscan factor 0.9757 and reduction 0.7112; the
  production path keeps all 8 rewrites even though the harness (per-function,
  per-extent measurement) retains 2. Per-definition aggregate factors cannot
  express extent- or function-level variation; per-candidate/profile features
  are required to close that gap.

The first inverse-scaling attempt (multiplying modeled savings by 0.0449)
rounded a +19 saving to +1 and changed nothing; dividing the modeled replacement
cost by the factor is what makes measured regressions reject. Scope: factor
export/import round-trips and the production hook now works end to end, but the
shipped policy is still opt-in and no profile is bundled.

**Status (increment 15): extent-aware measured weights.** The increment-14 gap
(per-definition aggregates cannot express extent variation) is closed on the
candidate feature that is actually available at selection time:

- `Candidate::source_extent()` reads the static extent from
  `LogicalSequence`, `DynamicBooleanSequence`, `PackedLogicalSequence`, and
  `BooleanMatrix` structures;
- `WeightedCostModel::with_extent_factors` consults per-definition extent rules
  `(inclusive upper bound, factor)` before the per-definition fallback, and
  `Optimizer::with_extent_weighted_cost` wires them into production selection;
- factor files accept three-column `definition extent_bound factor` rules in
  addition to two-column fallbacks; `--write-factors` now exports extent-
  specific geometric means and applies the retention margin as a policy
  (non-winners become `0.0`, a hard reject) so production reproduces the
  measured decision instead of a raw ratio.

Validation on v8 (default: 8 rewrites, 0 mismatches, 23 clean / 5 lower-refused):

- no-hit profile: exported rules are bitscan `<=48 → 1.16`, `64..128 → 0` and
  reductions `<=40 → 1.28`, `56/72 → 0`; production applies exactly
  `p01_first_set_48` (bitscan) plus `p06_two_loops_count_sum_const`
  (reduction) = 2 rewrites, matching harness retention and staying
  differential-clean;
- early-exit random profile: bitscan rules are `0` at every measured extent,
  reductions are `<=40 → 1.30`, `56 → 0`, `72 → 1.28`; production applies
  exactly `p06` plus `p09_count_le_const_u32_72` = 2 rewrites, matching harness
  retention.

Scope: extent rules key on the applied candidate's first-pass structure, so
same-extent functions still share a bucket; dynamic-loop trip counts and
non-collection structures (for example scalar multiply chains) have no extent
feature and use fallbacks. Selecting per-function/per-loop profiles remains
open. Default behavior is unchanged: factors are opt-in.

### P2 — Turn recognized runtime operations into implementations

**Observed:** runtime-length FirstOccurrence searches can lower and recognize but
lack an authorized fixed-extent mask replacement.

**Proposed experiment:** bounded chunked-mask search, preserving early exit,
in-bounds reads, zero length, scalar tails, predicate widths, and no-hit sentinels.
Do not promote a runtime pointer into a fictitious fixed-size array.

**Acceptance:** pre-register a fresh held-out corpus, including first/last/no hits,
length zero, vector boundaries/tails, alternate predicates and widths. Require
proof obligations, native differential checks, sanitizers, and speed comparisons
against equally configured Clang/GCC and relevant library implementations. Treat
profitability as a hypothesis until measured; retain scalar fallbacks.

Map-then-sum and recognized-but-unrewritten wider-element reductions are additional
backlog items; investigate their exact missing candidate/recipe obligations first.

**P2 investigation (exact obligations).** New diagnostics bin
`sir_benchmarks/src/bin/inspect_concepts.rs` prints a lowered function's region
concepts, truths, and generated candidates. Findings on the existing corpora
(and on synthetic fixtures in `sir_optimizer/tests/unrewritten_reductions.rs`):

- raw-element `SumReduction` (`p01_sum_u8_const_100`, `u8[64]` synthetic):
  recognized with `PredicateMap`, `SumReduction`, `LogicalSequence`,
  `FiniteCollection`, `ElementSequence`; the only generated candidate is
  definition 2 `PackedBitfield` citing `LogicalSequence`/`FiniteCollection` — a
  representation proposal, not a sum implementation. Definition 2 has no
  registered recipe and no verification definition, so the candidate is
  filtered and the optimizer refuses (0 rewrites). Missing: a
  `SumReduction`-specific strategy/generator entry, theorem, and recipe, plus a
  profitability case — measured scalar reduction loops are already competitive;
- `MappedSumReduction` (`p10_map_then_sum_const`, xor-map then sum):
  recognized, but **zero candidates** are generated. Missing: a strategy table
  entry for the concept, then theorem and recipe.

Three regressions pin the gaps: recognition is present, no candidate cites the
reduction concept, the def-2 packing candidate is never applied and has no
verification obligation, and mapped-sum generates no candidate and rewrites 0.
They are expected to change only when a real reduction family lands.

Follow-up: the raw-element family has landed end to end. The verification
value model gained `ArithmeticSequence`, `ElementAt`, and `Sum` expressions
plus interpreter and concrete-solver semantics; definition 400 `UnrolledSum`
adds the generator strategy, reduction binding, frame extension, and recipe.
`unrewritten_reductions.rs` now asserts the 400 candidate, its proven
obligation, and one applied rewrite; the mapped-sum test remains the
authoritative pin for the remaining gap.

Scope: this resolves "investigate the exact missing obligations first"; it does
not implement the reductions. The runtime-length chunked-mask FirstOccurrence
experiment also remains unimplemented: measured no-hit and early-exit results
show fixed-extent mask rewrites are marginal or regressive on this host, so it
needs a target-model case before code and a pre-registered corpus.

**P2 safety hardening (runtime extent).** A new regression
(`sir_optimizer/tests/runtime_extent_safety.rs`) exposed that runtime-extent
`FirstOccurrence`/`LastOccurrence` regions were given a fabricated
`LogicalSequence { length: 64 }` structural stub and therefore generated a
definition-200/201 candidate. The candidate lacked the `FixedLength`
constraint, so verification/application already rejected it (all v10/v11/v16
runtime searches apply zero rewrites) — but the fabricated structure is exactly
the "promote a runtime pointer into a fictitious fixed-size array" hazard.

The stub is removed: runtime search regions now have no structural description,
so no candidate is generated at all. Recognition is unchanged (`FirstOccurrence`
concept and `PositionSearch` roles still derive), constant-extent searches keep
their real length + `FixedLength` descriptions (v14 still rewrites both
functions), and v16 stays 11 clean / 0 mismatched / 1 lower-refused with
sanitizer violations 0. The new test asserts recognition, no definitions
200–203 on the candidate list, and zero rewrites; it failed with a def-200
candidate before the fix and passes after. All 18 prior corpora stay
244/0/71/98 with no status or per-function rewrite changes.

This closes the safety half of the runtime-extent item. The replacement itself
still needs a theorem form for runtime-bounded scans (the current theorem
language has no runtime-extent sequence operator), so it remains unimplemented
rather than half-landed.

**Bundled conservative profile (P1 increment).** The measured-retention policy
is now available without external files:

- `sir_optimizer::profile::conservative_measured_profile()` encodes the
  measured decision for array-like bitscan: definitions 200–203 with a known
  source extent get factor `0.0` (reject). Scalar bitscan candidates carry a
  `BitMask` structure with no extent and stay neutral, so hardware
  `tzcnt`/`lzcnt` replacements remain enabled. Reduction and permutation
  families are unchanged;
- `Optimizer::with_conservative_profile` wires the preset through the existing
  `WeightedCostModel` extent rules; `emit_c_diff --cost-profile conservative`
  exposes it for corpus measurement;
- regression
  `conservative_profile_rejects_array_bitscan_but_keeps_scalar_bitscan` pins
  both halves: the constant-extent array search goes from rewritten to 0 under
  the profile, while the scalar zero-count loop keeps its default rewrite count.

Corpus evidence (all 0 mismatches):

| corpus | default rewrites | conservative rewrites |
|---|---|---|
| v8 array searches + reductions | 8 | 3 (5 array bitscan rejected) |
| v6 mixed | 6 | 5 |
| v14 constant-extent searches | 2 | 0 |
| h3 global reductions | 4 | 4 (unchanged) |

Scope: the preset is a single-host, single-distribution measured policy, and it
is opt-in — `Optimizer::new` keeps the historical neutral behavior so the
documented corpus baselines stay reproducible. Choosing a profile remains a
target/distribution decision, and no claim is made that rejecting every
array-bitscan extent is optimal off this workload.

**Distribution-specific preset (same increment).** The no-hit measurements are
now a second bundled preset rather than a hand-written file:
`nohit_measured_profile` was intended to keep array bitscan at extent 48 with
the pre-hardening 1.15 factor. The corrected TSC/null/composition-guard harness
re-measured that composition at 0.76–0.89x, so the preset now rejects every
array-bitscan extent; it remains a distinct distribution entry point, but its
only corrected-harness retention is the reduction family. Regression
`nohit_profile_rejects_array_bitscan_after_harness_hardening` pins the
rejection, and `emit_c_diff --cost-profile nohit` selects it. On v8:

| v8 policy | rewrites |
|---|---|
| default neutral | 8 |
| conservative (early-exit) | 3 |
| nohit preset (corrected) | 3 (reductions only; array bitscan rejected) |

The measured-retention harness retains 1 of those 3 on v8 (p06 1.27x); the
other two reductions fall below the per-function margin, while the preset
keeps the reduction family neutral.

All three runs are 0-mismatch. The pair of presets makes the distribution
dependency explicit: the same rewrite is accepted by one and rejected by the
other, matching the measured evidence. Default behavior is still unchanged.

### P3 — Generalize semantic recovery

**Observed:** fixed recognizers and normalization gates still depend on induction,
indexing, traversal, and source shapes. Lowering refusal is a substantial boundary.

**Plan:** catalog refusals by reason; add reusable normal forms and compositional
reasoning, not fixture-name dispatch. Expand a concept across independently
written variants before adding more isolated concepts. Integrate target-plan and
fusion experiments into a reusable pipeline rather than leaving benchmark-only
capabilities disconnected from default optimization.

**Acceptance:** one change unlocks several unseen equivalent implementations;
negative/effectful cases remain refused; improvements measured on fresh inputs,
not only fixtures used to implement the feature.

**Correctness prerequisite completed:** region-local binding for ModuloPowerOfTwo,
DividePowerOfTwo and ShiftMask in `SemanticEngine::derive_roles`. Six direct tests
in `sir/crates/sir_semantics/tests/arithmetic_role_binding.rs` check two independent
regions and a recognized region followed by an unrecognized distractor for each
family (divisor 3, or unequal shift amounts). All six were red: the bound operator
was outside the recognized region. Restricting each role scan to region members
makes them green, including exact operator/lhs/rhs/result assertions. Workspace:
720 passed, 0 failed, 3 ignored. Recognizers, authorization and proof gates unchanged.

Scope of that increment: cross-region binding only, not a comprehensive audit of
all role families. These are correctness tests, not evidence of broader source
recall or faster native code. Those P3 acceptance criteria remain open.

**Merged-region follow-up:** a bounded generated probe in
`sir_semantics/tests/merged_arithmetic_roles.rs` covers 32 cases: all ordered pairs
of modulo/divide/multiply/shift-mask, chained or sharing a computed input. All
merge and exposed ambiguous ArithmeticOperation roles before the guard. Same-family
pairs silently selected the last operator; mixed-family pairs emitted multiple
roles while recipe accessors select the first without a family discriminator.
This is structural evidence, not a demonstrated wrong-code result or an exhaustive
search over arbitrary SIR.

Role derivation now emits these arithmetic roles only when exactly one possible
operator exists across the region's supported arithmetic concepts. Counting stops
at two because only uniqueness matters. The original per-family shape checks
remain; unrelated role families are unchanged. Ambiguous cases retain semantic
recognition but provide no arithmetic role for application. Occurrence-specific
binding is needed to recover this coverage safely.

**Explicit coverage loss:** chained multiplies previously rewrote and now refuse
with MissingRole. The existing rewrite-budget fixture caught this loss (isolated
rerun confirmed it); it now uses two independent regions and retains its unlimited
2 / limited 1 assertions. The original chained case is retained as an end-to-end
refusal regression asserting a successful proof, zero rewrites, no containment
failure and unchanged Function. Unambiguous-region tests remain green. Final
workspace: 722 passed, 0 failed, 3 ignored.

**Multiply occurrence recovery follow-up:** a shared semantic resolver now selects
one supported multiply by lowest NodeId within authorized region coverage,
independent of provenance-vector order. Multiply proof construction and production
rewrite binding use the same resolver. The rewrite handoff checks that the supplied
proof theorem equals the bound occurrence theorem, then gives the recipe a local
ArithmeticOperation role with that occurrence's exact operands/result. The general
merged-role ambiguity guard remains intact. Legacy unit-test authorization paths
retain their existing behavior.

The recipe emits the already-validated literal shift amount rather than
TrailingZeros(constant); otherwise the next pass's definedness checker cannot
classify the generated shift amount and chained rewriting stops after one rewrite.
The regression `32 * (x * 8)` progressed from zero rewrites (MissingRole), to one
rewrite, to two. It asserts the inner shift is 3 and outer shift is 5, and compares
source/result on 65,536 inputs plus overflow boundaries. The former same-constant
chain refusal regression now asserts two rewrites. Resolver order/coverage tests
were red before implementation. Workspace: 724 passed, 0 failed, 3 ignored.

Scope: this recovers deterministic one-occurrence-at-a-time multiply rewriting;
it does not enumerate every occurrence as a candidate, fix other arithmetic
families, or add occurrence identity to immutable authorization records. Theorem
comparison is an additional targeting check, not complete proof-artifact validation
or a higher scalar application-assurance claim. General end-to-end assurance,
including the existing fingerprint/proof-authority limitations, remains P4 work.

**Modulo/divide/shift-mask occurrence recovery:** the multiply-only resolver is
now the shared `arithmetic_occurrence` module. `resolve_pow2` selects the
lowest-ID supported multiply, remainder, or division occurrence inside the
candidate's authorized region coverage (unknown, dangling, or signed
remainder/division occurrences refuse; multiply keeps its unsigned-bit-pattern
semantics). `resolve_shift_mask` selects the lowest-ID `(x << k) >> k`
occurrence only when both shift amounts are the same in-range constant and the
operand is unsigned. `ModuloAndDefinition`, `DivideShiftDefinition`, and
`ShiftMaskDefinition` now build `obligation_bound` from that occurrence and
prefer it in `obligation_with_roles`; the previous role-scanning binding remains
as the fallback for descriptor-only unit fixtures. `RewriteEngine` binds the
same occurrence for defs 100/101/102/103, compares the supplied theorem against
that exact bound obligation, and hands the recipe the occurrence's operands; a
resolver miss leaves the structural description untouched and fails closed.

Four real-pipeline regressions in
`sir_optimizer/tests/arithmetic_occurrence.rs` cover chained `%` and `/` with
distinct constants, a merged modulo/divide region sharing a computed input, and
a merged pair of shift masks; each asserts two rewrites, no containment
failures, and differential equivalence over 14 boundary/representative inputs
(two-argument cases over the sample cross-product). All four were confirmed red
with the resolver temporarily disabled (0 rewrites, `FixedPoint`) and green
after. Resolver unit tests cover lowest-ID selection, coverage order, family
mismatch, constant dividends, unequal shift amounts, and signed/width refusal.
Two stale quarantine-era verification tests that asserted modulo/divide could
never bind were replaced with the real fail-closed condition (no authorized
coverage) plus positive coverage-bound proof tests for modulo, divide, and
shift-mask. Workspace (with `ASAN_OPTIONS=detect_leaks=0`, required for the
native sanitizer tests under this sandbox's ptrace): 732 passed, 0 failed,
3 ignored.

Scope: this recovers deterministic one-occurrence-at-a-time rewriting for
these four arithmetic families in merged and chained regions. It still does not
enumerate every occurrence as a candidate (regions are rewritten one pass at a
time), does not extend occurrence identity to other role families, and does not
add occurrence identity to immutable authorization records. It does not change
whole-path ranking, target-aware costing, or the schema-level application
assurance cap.

**Mask-algebra occurrence recovery (wrong-code fix):** the same last/first-node
binding class was live for `MaskOperation`. `derive_roles` pushed one role per
mask concept but each role selected the first `And`/`Or` in the region, so a
merged region exposed duplicate roots. Recipes consume the first
`MaskOperation`, while proof construction already bound each definition's own
pattern from `region_nodes`; a candidate could therefore prove one pattern and
apply its intrinsic to another. Reproduced with `shared = x + y`,
`clear = shared & (shared - 1)`, and `isolate = shared & -shared`: the region
exposed `[(shared, clear), (shared, clear)]`, and with the isolate candidate
ranked first the optimizer returned `2 * isolate` (value 2 instead of 1 for a
case where the source is 1). This was a real wrong-code path, not only an
ambiguity warning.

New `sir_semantics::mask_occurrence::resolve_mask` selects the lowest supported
root for each of the four mask families (clear lowest set bit, isolate lowest set
bit, isolate lowest clear bit, set lowest clear bit). `derive_roles` uses it, so
each concept emits its own pattern-specific `MaskOperation`. `RewriteEngine`
binds mask definitions 300–303 to the same occurrence, compares the supplied
theorem against the exact bound obligation, and injects that occurrence's role
before the recipe runs. `bind_mask` now also selects the lowest matching root
instead of the first coverage entry, so proof and application agree regardless
of provenance order. Broadening to all four families exposed a second latent
mismatch: `LowestClearBitMask` binding returned the `~x` node as the operand
while roles and recipes treat the base `x` as the operand; both sides now bind
the base.

Tests: `sir_semantics/tests/merged_mask_roles.rs` requires four distinct
pattern-specific roles in one merged region; `sir_optimizer/tests/mask_occurrence.rs`
prefers each definition first in turn and differentially compares the optimized
function against the source over 169 input pairs per family. Both were red
before the fix (duplicate roles; value 2 versus 1) and green after. The stale
`COMP001` benchmark expectation ("non-optimizable because ClearLowestSetBit is
Stub-quarantined") was replaced: the definition is solver-checked, its local
`blsr` substitution legitimately fires, and the composition candidate still
refuses at application binding. Workspace: 745 passed, 0 failed, 3 ignored
(with `ASAN_OPTIONS=detect_leaks=0`).

Scope: the four mask-algebra families are audited. Bitscan, bit-permutation,
rotate, and collection/reduction roles have not received the same occurrence
resolver treatment. A follow-up inspection found their definitions and recipes
read the same structural role (filtered by kind/direction), so the mask-style
proof/application split does not apply there; merged regions may still expose
only one role per permutation and defer remaining coverage to later passes.
Authorization records still carry no occurrence identity, and the mask rewrite
remains capped at schema-level application assurance.

**Fresh v12 corpus and zero-count scan soundness fix:** a new six-function
`gate6a/v12_corpus.ll` was written as fresh inputs for the occurrence-recovery
work: merged mask algebra (u32, u64, u16), merged remainder/division/shift-mask,
chained power-of-two operations, and a mixed mask/remainder/shift-mask loop. The
functions are counted loops because the lowerer requires loop CFGs; each returns
the body-computed accumulator.

The first full run isolated a real miscompile. After three valid arithmetic
rewrites, `m02_merged_pow2_u32` mismatched on all 48 cases while the same
function with `--beam 0` (no rewrites) was clean. The optimized emission was
`__sir_clz(p1)` — the accumulation loop had been replaced wholesale by a
LeadingZeroCount candidate. Root cause: the v0.1 heuristic in
`position_search.rs` classified ANY loop body containing a `Shr` and an `And`
as Trailing/LeadingZeroSearch, so once the arithmetic rewrites introduced
`Shr(shared, 3)` and `And(shared, 15)` into the loop, it matched a scan that the
loop never implemented. This is the same class of unsound shortcut as the mask
role binding: a structural co-occurrence treated as semantic evidence.

`detect_zero_count_scan` now requires the canonical shape: a loop-carried value
shifted by constant one, an `And` with a loop-carried operand whose result feeds
the loop's termination condition, and a trailing (`& 1`) versus leading (carried
mask) probe classification. The canonical SIR zero-count loops still rewrite
(`rewritten_zero_count_loops_execute_natively` stays green), and two new
semantics unit tests pin the accumulation-loop rejection and canonical trailing
acceptance. The v12 corpus is now 6/6 native-clean with 15 rewrites.

Re-running all 16 corpora: 233 clean / 0 mismatched / 72 lower-refused / 76
native rewrites. The per-function rewrite counts on the original 15 corpora are
unchanged (zero differences), so the tightened gate cost no observed coverage.
`emit_c_diff` now also supports `--winner-only`, `--beam N`, and
`EMITC_VERBOSE=1` for unsuppressed optimizer traces.

Scope: v12 is fresh relative to the development fixtures but was authored after
the fixes and contains only six functions; it is not an independently generated
or pre-registered held-out suite, so P5 remains open. The scan gate is
fail-closed: non-canonical zero-count loops that previously matched the loose
heuristic may now abstain, though no such coverage loss appeared on the existing
corpora.

**Pre-registered v13 corpus and LLVM-form recognition gap:** `gate6a/v13_corpus.ll`
was frozen with `v13_expected.csv` and `v13_manifest.sha256` before evaluation:
five positives for merged occurrence recovery, plus three negatives pinning the
accumulation loops that must never become zero-count scans. The first evaluation
scored 3/5 positives. The two merged-mask rows missed `LowestSetBit` and
`LowestClearBitMask` because LLVM lowers unary minus to `Sub(0, x)` and bitwise
not to `Xor(x, all-ones)`, while recognition, proof binding, and the occurrence
resolver only accepted `Neg` / `Not`. The three negatives were already safe.

The shared `mask_occurrence` helpers now expose `is_negation_of` and
`bitwise_not_operand` (with width-aware all-ones detection). The semantic
recognizer, `bind_mask` proof binding, and `resolve_mask` application binding all
consume those helpers, so the LLVM forms recover the same occurrence on all
three sides. Two recognizer unit tests pin the `0 - x` and `xor -1` forms.
After the fix the frozen corpus passes: `gate6a_run` reports 5/5 positives,
0 recall misses, 3/3 safe negatives; `emit_c_diff` reports 8 clean /
0 mismatched with 16 native rewrites (`a01` and `a04` rise from 2 to 4).

Across all 17 corpora (15 development + v12 + v13): 241 clean / 0 mismatched /
72 lower-refused / 96 native rewrites. The original 15 corpora have zero
per-function rewrite differences; v12's two merged-mask rows also rise from 2 to
4 rewrites because recognition now covers the LLVM forms. Workspace: 754 passed,
0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: v13 was authored after the fixes it pins (a regression/fresh-input
corpus, not blind discovery), so P5 still requires independently generated,
pre-registered held-out programs with external baselines. The negative rows
cover scalar zero-count misclassification only; other loose recognizer
heuristics have not been audited.

**PositionSearch scalar binding fix (second wrong-code path):** `derive_roles`
selected the PositionSearch `scalar` as the LAST integer parameter in the whole
function, with no region check. A canonical trailing-zero scan of `x` in a
function that also has an unused integer parameter `y` therefore bound `y`.
Proof construction and the recipe both consume that role, so the obligation
still discharged and the loop was replaced with `ctz(y)`.

Reproduced at SIR level with a two-parameter trailing-zero loop: the role
regression failed with `scalar = NodeId(1)` (`y`) instead of `NodeId(0)` (`x`).
The native regression emitted `["0","64"]` (ctz of the wrong parameter) with the
old binding and `["4","8"]` (ctz of `x`) after the fix; the old-binding failure
was red-verified before restoring the fix.

`detect_zero_count_scan` now returns `ZeroCountScan { leading, scalar }`: it
identifies the loop-carried value shifted by one, uses it directly for trailing
scans, and for leading scans uses the non-probe `And` operand. `derive_roles`
consumes that scalar for `TrailingZeroSearch` / `LeadingZeroSearch` regions and
sets no collection for them. First/LastOccurrence array-search role selection is
unchanged; its function-wide last-parameter heuristic remains an open risk.

Tests: `sir_semantics/tests/scan_role_binding.rs` (red/green) and
`two_parameter_scan_binds_the_scanned_value_natively` in the native emitter
suite. All 17 corpora re-run after the change: 241 clean / 0 mismatched /
72 lower-refused / 96 native rewrites, with zero per-function rewrite
differences. Workspace: 756 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

**Array-search role binding fix (third wrong-binding path):** First/Last
Occurrence roles used the same function-wide scan: collection was the last
array parameter and scalar the last integer parameter, regardless of which
array or compared value the loop actually used. Reproduced with a forward
search over `arr` in a function that also has a second bool array `other`:
the role bound `other` (red role test), and the end-to-end rewrite consumed
`other` instead of `arr` (red rewritten-graph regression, verified by
temporarily restoring the old selection). An equality search `arr[i] == key`
with a trailing extra integer parameter bound `extra` as the scalar.

`search_roles` now derives both from the loop body: distinct `ArrayAccess`
bases give the collection; when several arrays are accessed, only the base
reached from the element predicate is accepted; the scalar is the non-memory
operand of that comparison when it is a parameter. Ambiguous cases return
`None` (fail closed). Role derivation uses this for First/LastOccurrence;
zero-count scans keep the earlier `ZeroCountScan` path.

Three role regressions in `scan_role_binding.rs` were red before and green
after, and `two_array_search_rewrites_the_searched_array` pins the recipe
path. All 17 corpora re-run: 241 clean / 0 mismatched / 72 lower-refused /
96 native rewrites with zero per-function rewrite differences. Workspace:
759 passed, 0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

**Region-scoped reduction collections (coverage unblock):** reduction role
derivation collected every boolean-array parameter in the function as a
collection candidate, then required uniqueness. A declared but unrelated bool
array therefore suppressed the reduction role entirely (false ambiguity), even
when the loop accessed only one array. Reproduced with a count reduction over
`board` plus an unused `other` bool array: no `BooleanCollectionReduction` role,
while the single-array fixture kept its role. Collection candidates now come
from `ArrayAccess` bases inside the region; unrelated parameters are not
evidence. A loop that genuinely accesses two arrays still refuses (new
safety regression), so this is coverage recovery, not a weakened ambiguity
gate.

`reduction_role_binding.rs` pins both directions (unrelated array no longer
blocks; single array still binds), and the hardening suite replaces the old
"any second array refuses" test with "declared but unused array does not
block" plus a real two-access refusal test. All 17 corpora re-run: 241 clean /
0 mismatched / 72 lower-refused / 96 native rewrites with zero per-function
rewrite differences. Workspace: 762 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

**Pre-registered v14 corpus (native search-role coverage):** v13 contains no
array searches, so the body-derived collection/scalar binding had no native
corpus coverage. `gate6a/v14_corpus.ll` was frozen with expectations and hashes
before evaluation: `s01` searches the first array while also accessing a second
array of the same promoted extent, and `s02` performs an equality search with a
trailing extra integer parameter. `gate6a_run` reports 2/2 recognized and
`emit_c_diff` reports 2 clean / 0 mismatched with one rewrite each.

Honest discriminating-power note: re-running v14 with the old function-wide role
selection did not produce a native mismatch. `s01` abstained (0 rewrites)
instead of mis-binding the second array, and `s02` still rewrote correctly
because its unused trailing parameter is not materialized by the lowerer, so the
last materialized integer parameter was already the key. The red-verified
evidence for these fixes remains the role-level tests in `scan_role_binding.rs`
and the rewritten-graph regression; v14 is fresh native coverage of the fixed
behavior, not an independent reproduction.

Also added this turn: a negative regression (`reduction_predicate_mismatch.rs`)
where a loop accesses an array but counts an index-derived predicate. It must
not rewrite as a collection popcount; the optimizer preserves the loop (0
rewrites). All 18 corpora: 243 clean / 0 mismatched / 72 lower-refused / 98
native rewrites, zero per-function differences on the prior 17. Workspace: 763
passed, 0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

**P4 increment: scalar candidate frame check.** Arithmetic and mask rewrites
(definitions 100–103 and 300–303) carry no reduction `ProposalBinding`, so they
previously skipped every application-frame check. `RewriteEngine` now runs
`check_scalar_candidate_frame` for those production rewrites:

- only pure scalar operators and the pure bit-manipulation intrinsics
  `blsr`/`blsi`/`blsmsk` may appear in the patch;
- every source-node input must be reachable from the single bound occurrence
  role's operands (the replaced operator/root is not a legal input);
- every replacement must target the bound occurrence, and every replacement
  root must exist in the patch.

This validates patch shape against the conservative frame, not theorem-to-patch
semantic equivalence; application artifacts are still not issued for scalar
rewrites and the assurance label remains SchemaChecked. Unit-test authorization
paths keep their legacy behavior. Four direct `sir_rewrite` unit tests cover the
accept case plus refusals for a forbidden `ExternalCall`, an out-of-boundary
input, and a wrong replacement target. Existing arithmetic/mask end-to-end tests
and all 18 corpora still rewrite identically: 243 clean / 0 mismatched / 72
lower-refused / 98 native rewrites, zero differences. Workspace: 767 passed,
0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`). Rotate, permutation,
bitscan, and reduction patches are not yet covered by this scalar check.

**P4 follow-up: bit-permutation frame coverage.** The same check now covers
rotate, byte-swap, and bit-reverse rewrites (definitions 310–313, which carry
`BitPermutation` roles rather than scalar arithmetic roles). Allowed inputs are
the permutation operand plus the rotation amount; allowed patch nodes add
`Rol`/`Ror` and the `bswap`/`rbit` intrinsics. A fifth unit regression accepts a
canonical rotate patch, and the native rewritten suite (rotate, byte swap, bit
reverse, bitscan, masks, arithmetic) still passes. All 18 corpora remain
243 clean / 0 mismatched / 72 lower-refused / 98 native rewrites with zero
differences. Workspace: 768 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: this is still patch-shape validation, not theorem-to-patch equivalence or
native-code semantics; bitscan patches intentionally replace a tuple consumer
rather than the role result and therefore need a different boundary rule, and
reduction patches already use the `ProposalBinding` frame path. Scalar rewrites
still do not issue end-to-end application artifacts.

**P4 follow-up: bitscan frame coverage.** Bitscan rewrites (definitions
200–203, `PositionSearch` roles) now use the checker with their own boundary
rule: allowed inputs are the role's collection and/or scalar; the replacement
may target the loop result or a direct `TupleExtract`/`FieldAccess` projection
of it; allowed patch nodes add `Pack`, `ArrayCmpMask`, `TrailingZeros`,
`LeadingZeros`, `BitScanReverse`, and `Tuple`. A sixth unit regression accepts a
scalar `tzcnt(scalar)` patch targeting the loop projection.

Landing this uncovered a real role gap: `search_roles` only bound a parameter
as the predicate scalar, so comparisons against constants (e.g. `buf[i] != 0`)
left `scalar = None` and the frame check refused a legitimate bitscan patch.
`search_roles` now binds any non-memory comparison operand, whether parameter,
constant, or computed. The early-exit 48-element native search, forward/reverse
bitscan, and zero-count native tests all pass again; all 18 corpora remain
243 clean / 0 mismatched / 72 lower-refused / 98 native rewrites with zero
differences. Workspace: 769 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: still patch-shape validation only. Theorem-to-patch equivalence and
end-to-end application artifacts for scalar/permutation/bitscan rewrites remain
open, as do P2/P5 and the P1 cost-model items.

**P4 increment: end-to-end artifacts for scalar and permutation rewrites.**
Definitions 100–103, 300–303, and 310–313, which previously produced
`application: None`, now issue a matched theorem/application artifact:

- `derive_scalar_source_frame` certifies the replaced root as pure and total
  (constant nonzero divisors, in-range constant shifts); its operands are
  opaque boundary values, so an enclosing loop or memory access does not
  invalidate the substitution;
- `build_scalar_end_to_end` binds the proof to the occurrence role digest,
  replacement live-out digest, source/candidate frame digests, and assumptions
  digest, then issues an `ApplicationChecker` artifact capped at
  `SchemaChecked` and constructs the `EndToEndVerificationArtifact`;
- `OptimizationResult::end_to_end_artifacts` reports how many applied rewrites
  carried one.

Narrowing the frame mattered: an initial whole-region version refused the
COMP001 clear-lowest-bit rewrite because the occurrence sits inside a loop.
The final root-based frame keeps that rewrite (and its artifact) while still
failing closed on effectful, trapped, or non-total roots. Three
`scalar_artifacts.rs` regressions assert arithmetic and mask rewrites issue one
artifact each and bitscan rewrites still issue none. All 18 corpora remain
243 clean / 0 mismatched / 72 lower-refused / 98 native rewrites with zero
differences. Workspace: 772 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: the artifact is schema-level. The digests are identity keys, the
application checker has no machine-equivalence backend, and this does not
establish theorem-to-patch semantic equivalence. Bitscan rewrites (200–203)
initially remained artifact-free pending their projection/live-out story
(array bitscan coverage arrives in the follow-up below); reduction rewrites
already used the `ProposalBinding` frame path.

**P4 follow-up: array bitscan artifacts.** Array bitscan rewrites (200/201,
`PositionSearch` with a fixed-length collection) now issue artifacts through a
`BitscanSourceFrame`: the X06 `PositionSearch` authorization certificate is the
totality/binding witness (ordinary `LoopFact::is_finite` does not cover
predicate-bounded searches), the collection must have a declared fixed extent,
the loop fact must exist and not be nested, and the loop effects must be
read-only. All 18 corpora remained 243 clean / 0 mismatched / 72 lower-refused /
98 native rewrites with zero differences at that point.

**P4 follow-up: scalar zero-count bitscan artifacts.** Scalar zero-count scans
(202/203) have no collection extent; their boundedness witness is the canonical
shift-by-one scan shape combined with the operand's integer width, plus the same
non-nested read-only loop constraint. They now issue artifacts too, so all four
`scalar_artifacts.rs` regressions assert one issued artifact: arithmetic, mask,
array bitscan, and scalar zero-count bitscan. All 18 corpora remain 243 clean /
0 mismatched / 72 lower-refused / 98 native rewrites with zero differences.
Workspace: 773 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: artifacts remain schema-level identity bindings, not equivalence proofs.
Theorem-to-patch equivalence remains open.

**P4 increment: solver-checked theorem-to-patch obligation for arithmetic.**
The four arithmetic families (100–103) now close the gap between the proof and
the actual replacement. After the recipe builds its patch:

- `patch_expression` converts the replacement graph into the verifier's
  expression language (boundary operands become variables; constants are
  normalized to u64 values; `And`/`Shl`/`Shr` map to the corresponding
  `SemanticExpression` nodes);
- `arithmetic_patch_obligation` builds `bound theorem.rhs == patch_expression`
  using the bound obligation's domain;
- `verify_arithmetic_patch` discharges it with the concrete solver inside
  `rewrite_checked`; failure refuses the rewrite.

Together with the existing proof (source == RHS), this yields a solver-checked
equality chain source == RHS == patch for arithmetic rewrites. A direct
regression proves that `x % 8` patched with mask 7 discharges the obligation
while mask 6 does not. All 18 corpora remain 243 clean / 0 mismatched /
72 lower-refused / 98 native rewrites with zero differences; existing
arithmetic/chained/merged tests still pass. Workspace: 774 passed, 0 failed,
3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: only arithmetic (100–103) has the converter so far. Mask, permutation,
and bitscan patches involve intrinsics/Pack/scan nodes and still rely on shape
checks plus schema-level artifacts; extending the converter family by family
remains open, as does any independent solver-soundness validation.

**P4 follow-up: mask and rotate theorem-to-patch obligations.** The converter
now covers three mask families and both rotations:

- `blsr(x)` → `ClearLowestSetBit(x)` and `blsi(x)` → `LowestSetBit(x)`;
- `Not`/`blsi` (isolate lowest clear bit) maps through `BitwiseNot` and
  `LowestSetBit`;
- `Rol`/`Ror` map to `RotateLeft`/`RotateRight`, and boundary constants now
  resolve to constants (needed to match the certified rotation amount).

Rotations compare the patch against the certified theorem LHS (the abstract
operation) with a structural fast path, because comparing against the
shift/or RHS hit the concrete solver's `DomainTooLarge` limit at 32 bits; the
existing proof already connects that LHS to the source expression. Arithmetic
and mask patches continue to compare against the RHS. `rewritten_constant_rotate`
and `rotates_execute_natively_with_width_relative_amounts` pass again, and a
direct converter test pins the blsr/rotate mappings.

All 18 corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 native
rewrites with zero differences. Workspace: 775 passed, 0 failed, 3 ignored
(with `ASAN_OPTIONS=detect_leaks=0`).

Scope: set-lowest-clear-bit (`blsmsk`), byte swap, bit reverse, and bitscan
still have no theorem-to-patch converter; their artifacts remain shape/schema
level. Solver-soundness validation remains external.

**P4 follow-up: remaining permutation converters.** Three more families join
the theorem-to-patch converter:

- set-lowest-clear-bit (303): the whole `x | blsmsk(~x)` patch is recognized as
  `SetLowestClearBit(x)` and compared against the certified theorem LHS;
- byte swap (312) and bit reverse (313): `bswap`/`rbit` map to
  `ByteSwap`/`BitReverse`, the optional alignment shift maps through
  `ShiftRight`, and both compare against the certified LHS (which already
  includes the type-width/perm-width shift).

A converter regression pins the bswap, rbit, and blsmsk shapes.
`merged_mask_patterns_rewrite_their_own_occurrences` (303),
`rewritten_byte_swap_executes_natively`, and
`rewritten_bit_reverse_executes_natively` all pass with the obligations
enforced. All 18 corpora remain 243 clean / 0 mismatched / 72 lower-refused /
98 native rewrites. Workspace: 776 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: bitscan (200–203) is the only remaining family without a
theorem-to-patch converter; its `Pack`/`ArrayCmpMask`/scan expression mapping
is still open, as is independent solver-soundness validation.

**P4 follow-up: scalar bitscan converters.** The zero-count bitscan families
(202/203) now join the theorem-to-patch converter: `TrailingZeros` and
`LeadingZeros` patch nodes map to `SemanticExpression::TrailingZeros` /
`LeadingZeros`, and the obligation compares the patch against the certified
LHS, which is exactly the same abstract operation over the bound scalar, so it
discharges structurally. A converter regression pins both mappings;
`scalar_artifacts` and the native zero-count rewrite tests pass with the
obligation enforced. All 18 corpora remain 243 clean / 0 mismatched /
72 lower-refused / 98 native rewrites. Workspace: 777 passed, 0 failed,
3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: array bitscan (200/201) is now the only family without a
theorem-to-patch converter; mapping `Pack`/`ArrayCmpMask`/`Filter` against the
`FirstTrue(Pack(seq))` theorem remains open, as does independent
solver-soundness validation.

**P4 follow-up: array bitscan converters complete.** Definitions 200/201 now
join the theorem-to-patch converter. `Pack` maps to
`Pack(LogicalSequence{collection})`; `ArrayCmpMask` also maps to
`Pack(LogicalSequence{collection})` because the predicate mask is the packed
form of the predicate map; `BitScanReverse` maps through; single-element tuple
wraps forward to their element. Definition 200 compares the patch against the
theorem RHS, 201 against the LHS.

Landing this caught a real coverage regression: the first `ArrayCmpMask`
mapping produced a bare `LogicalSequence`, so the patch expression dropped the
`Pack`, structural equality failed, and the >64-extent symbolic searches
returned `DomainOverflow` and stopped rewriting (98 → 89 native rewrites).
Mapping the mask to `Pack(LogicalSequence)` restored per-function parity with
the known-good baseline: 243 clean / 0 mismatched / 72 lower-refused / 98 native
rewrites with zero per-function differences. A converter regression pins the
Pack/ArrayCmpMask/BitScanReverse mappings, and the native forward/reverse
bitscan, early-exit, equality-search, and two-array tests pass. Workspace: 778
passed, 0 failed, 3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: every scalar arithmetic, mask, permutation, and bitscan recipe family
now has a theorem-to-patch converter. Reduction rewrites still rely on the
`ProposalBinding` frame path without a theorem-to-patch expression obligation,
and independent solver-soundness validation remains open.

**P4 follow-up: reduction theorem-to-patch obligations.** The reduction
families (0 popcount, 4 any, 5 all, 6 parity) now join the converter:

- `Popcount`, `Ne(...,0)`, `Eq(...,all-ones)`, and `And(Popcount,1)` map to
  `Popcount`, `NotEqualZero`, `EqualFullMask`, and `BitwiseAndOne`;
- reduction definitions abstract the collection as variable 0, so patch
  variables are normalized to that template variable;
- scalar bitset paths (Kernighan parity, scalar popcount) wrap their bare
  operand as `Pack(LogicalSequence{0})`, and Bool parity's outer `!= 0`
  truthiness wrapper is stripped for template comparison;
- reduction patches build a theorem-to-patch obligation like every other
  family; the former table-lookup exemption was removed in the increment below.

Two integration mistakes were caught while landing this: the reduction IDs were
first added to the frame-check hook (which forbids `Popcount` and blocked the
HD012 parity rewrite), and scalar parity/popcount patches hit `DomainOverflow`
until the template normalization above was added. Both are fixed. Twelve
checker/converter tests pass, including a reduction mapping regression; the
boolean-reduction, any-hardening, and hacker's-delight suites pass; all 18
corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 native rewrites
with zero per-function differences. Workspace: 779 passed, 0 failed, 3 ignored
(with `ASAN_OPTIONS=detect_leaks=0`).

Scope: theorem-to-patch coverage now spans every recipe family with an
expression converter, with no family-level exemption; independent-algorithm
solver validation is covered by the increment below, and third-party z3
cross-checking now covers the QF_BV expression subset (see below).

**P4 increment: reduction theorem-to-patch exemption removed.**
`verify_patch_theorem` previously returned `Ok` for definitions 0/4/5/6 when no
obligation could be built, so a reduction patch the expression converter could
not express skipped the theorem obligation and relied on frame checks alone.
Instrumenting that branch showed **zero hits across 15 hand-written corpora** —
the table-lookup popcount path is not exercised by any current corpus or test.

The exemption is deleted: every applied rewrite must build and discharge a
theorem-to-patch obligation, and a reduction patch the converter cannot express
is refused (fail closed) instead of being silently exempted. Evidence:
workspace 801 passed / 0 failed / 3 ignored after removal, and all 18 corpora
remain 244 clean / 0 mismatched / 71 lower-refused / 98 rewrites with zero
per-function differences. If a real program needs the table-lookup path, the
converter must be extended; no current coverage was lost.

**P4 increment: independent solver-soundness differential.**
`sir_verification/tests/solver_soundness_differential.rs` cross-checks
`ConcreteSolverVerifier` against a test-local reference evaluator: plain `u64`
wrapping/masking arithmetic with SMT-LIB shift semantics that enumerates all
256 states of two 4-bit variables and shares no code with either solver.

- corpus: 9 identities, 6 invalid pairs, and 300 deterministic random
  expression pairs over add/subtract/multiply/and/or/not/left-right shift;
- a `Proven` verdict paired with an oracle counterexample is an "UNSOUND PROOF"
  failure; a `Rejected` verdict with no counterexample is a "SPURIOUS
  REJECTION" failure; `Unknown` is allowed;
- observed: proven agreements ≥6, rejected agreements ≥30, 0 unknowns, 0
  disagreements;
- a second pass compares `ConcreteSolverVerifier` with `ExhaustiveVerifier` on
  200 random pairs from the sub-language the exhaustive backend supports
  (add/and/or/not), requiring ≥20 agreeing verdicts and no `Proven`/`Rejected`
  disagreement.

The harness was red-verified twice by corrupting the oracle: `Add` as subtract
triggered the UNSOUND PROOF path on `x+y == y+x`, and a corrupted shift arm
triggered a spurious-rejection disagreement; restoring the oracle turns the
suite green. Scope: both the oracle and the exhaustive backend are first-party
implementations. Third-party validation is now supplied by the z3 increment
below.

**P4 increment: third-party z3 cross-check.** z3-solver 5.1.0.0 is installed
under the gitignored `vendor/z3py` directory
(`pip3 install --target vendor/z3py z3-solver`). The new
`concrete_solver_agrees_with_external_z3` test emits QF_BV SMT-LIB for each
obligation (`x`, `y` : `BitVec 4`) and compares z3's `sat`/`unsat` verdict with
the concrete solver:

- a concrete `Proven` with a z3 counterexample fails as an unsound proof;
- a concrete `Rejected` with a z3 proof fails as a spurious rejection;
- `unknown` from either side is tolerated;
- corpus: 8 fixed identities/invalid pairs, modulo/divide/multiply/shift-mask
  theorem shapes, and 100 deterministic random pairs; the test requires at
  least 4 proven and 20 rejected agreements. All pass.

The test skips with a message when z3 is absent (override with `Z3_BIN`), so
environments without the vendored solver stay green. Scope: z3 is only asked
about obligations in the QF_BV expression subset
(variables/constants/add/sub/mul/and/or/not/shifts/divide/remainder);
obligations involving `FirstTrue`, rotations, permutations, or mask-algebra
forms are not cross-checked, so external validation remains partial.

**P4 follow-up: packed-sequence reduction theorems are z3-validated.**
`z3_validates_reduction_definition_theorems` builds the real registry
obligations for definitions 0/4/5/6 (Popcount, Any, All, Parity) and translates
their packed-sequence expressions (`LogicalSequence`, `Pack`, `Filter`,
`Count`, `Popcount`, `Exists`, `NotEqualZero`, `All`, `EqualFullMask`,
`Parity`, `BitwiseAndOne`) to SMT-LIB. z3 proves every theorem at lengths
32/64/96, including the 96-element domain the 64-bit concrete solver cannot
bit-blast. This closes the reduction-family gap in third-party validation;
scan/permutation/mask theorem shapes remain outside the external emitter.

**P4 follow-up 2: mask-algebra obligations are z3-validated.**
`mask_algebra_theorems_are_z3_valid` reuses the existing
`mask_pattern_function` / candidate fixtures and asks z3 to prove the real
registry obligations for definitions 300–303 (`ClearLowestSetBit`,
`IsolateLowestSetBit`, `IsolateLowestClearBit`, `SetLowestClearBit`). The SMT
translation mirrors the concrete solver's bit-vector identities
(`x & (x-1)`, `x & -x`, `~x & (x+1)`, `x | (x+1)`); all four are `unsat`.
Remaining outside the external emitter: scan (`FirstTrue`/`LastTrue`/
trailing/leading-zero) and permutation (`Rotate*`/`ByteSwap`/`BitReverse`)
theorem shapes.

**P4 follow-up 3: scan-family obligations are z3-validated.**
The sequence emitter now translates `FirstTrue`, `LastTrue`,
`TrailingZeros`, `LeadingZeros`, `BitScanReverse`, and `Reverse`:

- array bitscan definitions 200/201 are checked on real 96-element registry
  obligations (`FirstTrue(seq) == TrailingZeros(Pack(seq))`,
  `BitScanReverse(Pack(seq)) == LastTrue(seq)`);
- scalar zero-count definitions 202/203 are checked
  (`TrailingZeros(x) == FirstTrue(x)`,
  `LeadingZeros(x) == FirstTrue(Reverse(x))`).

All four are `unsat` under z3, closing the scan family in third-party
validation.

**P4 follow-up 4: permutation obligations are z3-validated.**
The bitvector emitter now translates `RotateLeft`/`RotateRight` (constant
amounts), `ByteSwap`, `BitReverse`, and `ShiftLeft`/`ShiftRight`. The real
registry obligations for definitions 310–313 are checked:

- constant rotations 310/311 against their `Or(Shl, Shr)` complements;
- byte swap 312 (partial 16-of-32-bit width) against its shift/mask stage;
- bit reverse 313 (8-of-32-bit width) against its bit placement chain.

All are `unsat` under z3. Combined with the earlier increments, **all six
recipe families** — reductions, arithmetic, mask algebra, scan, and
permutations — now have their registry theorem obligations cross-checked by a
third-party solver.

Scope of the external check: it validates the definition theorems in their
translated expression forms. It does not validate patch construction or
theorem-to-patch conversion, does not cover `Filter` predicates other than
`True`, and skips obligations outside the implemented emitter forms; those
remain covered by the first-party solver/oracle differentials.

**P3 increment: occurrence identity in immutable authorization records.**
`TransformationAuthorization` now carries `occurrences:
Vec<(SemanticConcept, NodeId)>`, derived per region with the same resolvers used
by proof construction and application binding (modulo/divide/multiply/shift-mask
plus the four mask families). `AuthorizationRef` copies the certified map,
`exact_binding_matches` compares it against the database record, and
`RewriteEngine::candidate_occurrence_binding` requires the candidate-local
resolver to land on the exact certified node before injecting a role. The
UNIT_TEST authorization sentinel keeps its documented escape for hand-built
legacy fixtures; production candidates always carry real records.

A new `authorization_occurrences.rs` regression builds a merged
remainder/clear-lowest-bit region and asserts the issued authorizations name
`(ModuloPowerOfTwo, rem)` and `(ClearLowestSetBit, clear)`. Landing the
enforcement initially broke the direct theorem-to-patch unit test because its
UNIT_TEST candidate had an empty occurrence map; the sentinel escape fixed that
without touching production behavior.

All 18 corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 native
rewrites with zero per-function differences. Workspace: 780 passed, 0 failed,
3 ignored (with `ASAN_OPTIONS=detect_leaks=0`).

Scope: occurrence entries currently cover the arithmetic and mask families;
scans, permutations, and reduction concepts still rely on structural roles and
the `ProposalBinding` path. The record is an identity, not a proof.

**P4/P5 evidence: sanitizer-instrumented corpus run.** All 18 development/fresh
corpora were re-run through `emit_c_diff --sanitize` (ASan + UBSan,
`detect_leaks=0`): 243 clean / 0 mismatched / 72 lower-refused / 0 harness
errors and **0 sanitizer violations**. This covers every rewrite family after
the occurrence, artifact, and theorem-to-patch changes; it is complementary
evidence, not a proof or an external baseline.

**P3 follow-up: permutation occurrence identity.** `certified_region_occurrences`
now also records the permutation truth's output node for
`CircularPermutation`, `BytePermutation`, and `BitPermutation`. The rewrite
engine resolves each permutation definition (310–313) to its certified concept
and requires the structural role's result node to appear in the immutable
authorization record. A new regression builds a canonical rotate and asserts
`(CircularPermutation, or_node)` is certified. All 18 corpora remain
243 clean / 0 mismatched / 72 lower-refused / 98 native rewrites with zero
per-function differences; workspace: 781 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: scan and reduction occurrence identity remain open; UNIT_TEST
authorizations keep the documented escape.

**P3 follow-up: scan and reduction occurrence identity.** Authorization records
now also certify the loop occurrence for scan and reduction concepts, matching
the exact selection rules in `derive_roles`:

- scan concepts (`FirstOccurrence`, `LastOccurrence`, `TrailingZeroSearch`,
  `LeadingZeroSearch`) certify the highest-NodeId loop in the region, which is
  the loop the scan role derivation keeps;
- reduction concepts (`Cardinality`, `Disjunctive`, `Conjunctive`,
  `ExclusiveReduction`, `BitsetIteration`) certify the region's unique loop;
  multi-loop regions emit no entry, mirroring `unique_node`'s refusal.

The rewrite engine enforces both: bitscan definitions 200–203 must match the
`PositionSearch` role's loop, and reduction definitions 0/4/5/6 must match the
`ProposalBinding`'s loop node (any of the definition's supported concepts may
match). Two regressions build a canonical zero-count scan and a boolean count
loop and assert their loops are certified. All 18 corpora remain 243 clean /
0 mismatched / 72 lower-refused / 98 native rewrites with zero per-function
differences. Workspace: 783 passed, 0 failed, 3 ignored (with
`ASAN_OPTIONS=detect_leaks=0`).

Scope: occurrence identity now spans arithmetic, mask, permutation, scan, and
reduction concepts; UNIT_TEST authorizations retain their documented escape.
Identity is not a proof, and solver-soundness validation remains external.

### P4 — Strengthen assurance across the whole transformation

**Observed:** some solver obligations bind recognized roles rather than translating
the complete source graph. The application checker remains schema-level and later
native lowering has separate empirical assurance.

**Plan:** connect source/replacement semantics, live outputs, frame conditions,
definedness, and emitted-code semantics through checkable obligations. Preserve
honest assurance labels and fail-closed behavior; expand one supported family at a
time. Previously quarantined definitions have been lifted; do not describe their
historical stubs as current defects.

**Acceptance:** transformation-specific source/application obligations are checked;
incorrect role/bound/frame mutations are refused; native tests and sanitizers
remain complementary evidence, never relabeled as universal proofs.

### P5 — Establish independent performance/generalization evidence

**Plan:** independent held-out programs, strong compiler/library baselines, multiple
architectures and input distributions. Publish refusal rate, rewrite rate,
correctness evidence, speedup/regression distribution, code size and compile cost.
Keep recognition, rewriting, verification and performance gates distinct.

**Acceptance:** reproducible wins beyond development kernels with all unsuccessful
cases disclosed. Gate 6B's machine-specific results and same-author corpus caveat
remain explicit until broader evidence exists.

**Evidence (first measured pass, increment 11):** `emit_c_diff --time-baseline`
isolates the rewrites from the emitted-C baseline (both `clang -O1`), and
`--measured-retain` keeps only families whose solo rewrite clears a 5% margin.
On 7 development corpora / 104 functions, 8 of 59 rewrites are retained (all
`reduction`, 1.08–1.81x) and 51 are rejected on the measured distribution,
including every array bitscan rewrite under early exits. Same-author corpora,
one host, one input distribution: this is progress on "score actual lowered
replacements", not independent held-out evidence.

`--time-fill` now controls the measurement distribution. Under a no-hit fill
the retain set changes to 3 of 48 functions (including two small-extent bitscan
rewrites at ~1.13x), confirming that the measured policy is distribution-
sensitive rather than universally rejecting a family.

**Fresh corpus v16 (P5 increment).** `gate6a/v16_*` was pre-registered before
its first run and covers edges the v12–v14 sets do not: runtime-extent searches
with a zero-length guard and alternate predicates, wider-element and
map-then-sum reductions, and scalar bit permutations.

- `gate6a_run`: 12 positives, 4 recognized, 0 recall misses, 8 pre-registered
  known-gap misses, `GATE6A_VERDICT PASS`;
- the first frozen native run found a real **miscompile**: the descending
  runtime search (`r03`) mismatched 38/48 cases with zero rewrites because the
  synthesized loop aliased the no-hit sentinel with the mutable counter
  carrier. `sir_lower` now freezes the descending index seed into a distinct
  pre-loop snapshot; `descending_search_lowers_with_a_sub_successor` asserts the
  seed separation and was red-verified by restoring the aliased lowering;
- post-fix `emit_c_diff`: 7 clean / 0 mismatched / 5 lower-refused / 0
  rewrites; `--sanitize` (ASan+UBSan) reports 0 violations; same-backend timing
  is 0.99–1.02x because no rewrite applied;
- disclosed refusals: the zero-length guard shape, three random-access
  permutation shapes, and the search-then-sum composition; recognized-but-
  unrewritten sums/map-sums remain pinned by the P2 investigation tests.

Same-author corpus caveat applies; this run strengthens correctness and refusal
evidence, not rewrite-rate or external-baseline evidence.

Enabler follow-up (see the execution log): the three permutation intrinsics now
lower and rewrite, changing v16 to 10 clean / 0 mismatched / 2 lower-refused /
3 rewrites, with rotate/bswap/bitreverse measuring 1.00x / 1.03x / 1.00x and
all three rejected by the 5% measured-retention margin. Prior corpora rewrite
counts are unchanged (98).

**Randomized differential fuzzing (P5 increment).** `gen_random_corpus`
(committed) deterministically generates valid LLVM IR over known-lowering
templates: scalar i32 expression trees, constant-extent u8/u16/u32 sums,
conditional sums, and count/any/parity reductions. Two frozen corpora were
run end to end (`gate6a/v17_*`):

- seed `20260918`, 60 functions: 60 clean / 0 mismatched / 0 lower-refused,
  11 rewrites; sanitizer violations 0;
- seed `777`, 120 functions: 120 clean / 0 mismatched / 0 lower-refused,
  30 rewrites; sanitizer violations 0.

Combined 180 randomized programs, 41 rewrites, 0 mismatches, 0 violations.
Same-author generator and restricted CFG templates: correctness evidence, not
recognition coverage or external baselines.

**Extended randomized differential corpora (v19).** The generator gained i64
straight-line trees, randomized comparison predicates, and guarded
runtime-extent reductions (the v16 `r01` tail-phi shape) in sum and
predicate-count variants. Two frozen corpora:

- seed `4242`, 160 functions: 160 clean / 0 mismatched / 0 lower-refused,
  15 rewrites; sanitizer violations 0;
- seed `31337`, 200 functions: 200 clean / 0 mismatched / 0 lower-refused,
  17 rewrites; sanitizer violations 0.

Combined 360 programs and 32 rewrites with no mismatches, refusals, or
violations; regenerating either seed reproduces the manifest hashes. This is
the strongest single piece of correctness evidence for the guarded runtime
lowering, but remains same-author, restricted-template fuzzing.

**Cross-compiler differential (P5 increment).** `emit_c_diff --cc gcc` compiles
the emitted C with GCC 14 while the reference IR is compiled by clang 19 (IR is
first compiled to an object because GCC cannot read LLVM IR). This validates the
emitted C — including rotate, byte-swap, bit-reverse, bit-count, scan, and
reduction helpers — against a second, independent C implementation:

- v16: 11 clean / 0 mismatched / 1 lower-refused; `--cc gcc --sanitize` reports
  0 violations;
- v19 fuzz seed 4242: 160 clean / 0 mismatched / 0 lower-refused;
- v19b fuzz seed 31337: 200 clean / 0 mismatched / 0 lower-refused.

That is 360 randomized programs additionally clean under GCC. A GCC-native
regression (`rewritten_bit_reverse_executes_natively_with_gcc`) pins the
bit-reverse output, skipped when GCC is absent. Same architecture (x86-64) and
no 32-bit multilib is installed, so this is compiler portability, not
multi-architecture evidence.

GCC sanitizer runs (ASan+UBSan, `-fno-sanitize-recover=all`) over the fuzz
corpora add: v17 60 clean / 0 mismatched / 0 lower-refused, v19 160 clean,
v19b 200 clean — **420 randomized programs with 0 sanitizer violations**.

**Cross-architecture execution (P5 increment).** No AArch64 cross toolchain is
installed and `apt-get` cannot add one (no root; sudo is blocked by
no-new-privileges), so the available second ISA is WebAssembly: clang 19
`--target=wasm32`, linked with the rustup-shipped `wasm-ld`, executed by
Node 22. `wasm32_cross_architecture_executes_emitted_scalar_ops` compiles the
emitted C for wasm32 and checks:

- rewritten bit-reverse (`0x01/0x0F/0xA5/0` → `128/240/165/0`);
- width-relative rotate (`rotl(0x80000001,3)=0xC`,
  `rotl(0x12345678,8)=0x34567812`);
- popcount (`0/0xF/0xFFFFFFFF` → `0/4/32`);
- trailing-zero count (`1/8/0` → `0/3/32`).

The test skips when Node or `wasm-ld` is unavailable (`WASM_LD` override).
Scope: scalar emitted-C operations on a second ISA, not corpus-scale
pointer/loop differential and no meaningful timing under the JS runtime;
AArch64 remains untested because the root install is blocked.

**Corpus-scale cross-architecture differential.** `wasm_diff` compiles the
optimized emitted C natively and for wasm32, feeds both identical deterministic
inputs — including 2048-byte buffers for pointer parameters — and compares the
return value plus an FNV-1a checksum of every used buffer.

Results over the 22 development/fresh/fuzz corpora (24 inputs each):

- **481 clean / 0 mismatched / 72 lower-refused / 0 skipped /
  0 harness errors**.

Adding the 200-function v19b fuzz seed brings the total to **681 clean
cross-architecture function runs** with 0 mismatches. This now covers
pointer/loop/reduction/search functions, not just scalars.
Three harness issues were found and fixed during bring-up, none of them codegen
defects: a 64-bit value printed unsigned natively but signed through JS
`BigInt` (normalized with `BigInt.asUintN(64, …)`); the `-nostdlib` wasm build
needing a stub `string.h` plus `memcpy`/`memset` and `<stddef.h>`; and the
buffer-checksum export needing `export_name` rather than a linker
`--export=bufsum` flag. Scope: wasm32 as the second ISA, no meaningful timing
through the JS runtime, and AArch64 remains untested (root-only install).

**Cross-compiler timing reverses some decisions.** The same baseline-vs-
optimized emitted-C measurement under GCC 14 `-O1` disagrees materially with
clang 19 `-O1` (baseline/replacement speedup, higher is better):

| function | clang | gcc |
|---|---|---|
| `p01_rotl13_u32` | 1.00x | 0.69x |
| `p02_bswap_u32` | 1.03x | 1.10x |
| `p03_bitrev_u32` (SWAR) | 1.00x | 0.93x |
| v6 `p02_count_eq_const_u8` | ~1.73x | 1.28x |
| h3 `h3b03_any_eq_global` | ~1.81x | 1.10x |
| v7 `p07_three_loops_const` | ~1.33x | **0.81x** |

The three-loop reduction is a clear clang win and a GCC loss: profitability is
compiler-specific, not just distribution-specific. Consequences now encoded:

- factor files record their provenance (`cc`, `time_fill`, retention margin),
  so a clang-measured profile cannot be mistaken for GCC data;
- the bundled presets carry a compiler-specificity warning and must be
  re-measured per code generator/target;
- cross-compiler differential remains a separate gate from cross-compiler
  profitability.

**Library-implementation baselines (P5).** `gate6a/v20_library_baseline.c`
(raw outputs in `v20_raw/`) times library/builtin and naive scalar versions of
the targeted kernels on the same host at clang/gcc `-O1`:

| kernel / distribution | clang lib | clang naive | gcc lib | gcc naive |
|---|---|---|---|---|
| first zero, n=64, hit at 3 | 1.49 | 1.72 | 1.78 | 1.97 |
| first zero, n=64, no hit | 1.50 | 17.63 | 2.13 | 18.02 |
| first zero, n=4096, hit at 3 | 1.44 | 1.35 | 1.57 | 1.94 |
| first zero, n=4096, no hit | 21.17 | 560.95 | 20.90 | 559.28 |
| popcount (u64) | 1.32 | 18.04 | 1.06 | 18.12 |
| ctz (u64) | 0.19 | 1.31 | 0.60 | 1.45 |

Implications: library/builtin baselines dominate naive scalar kernels where the
work is long or no-hit (`memchr` ~26x on a 4096-byte no-hit scan, ~8–12x at 64
bytes; popcount ~14–17x; ctz ~2.4–7x). On an immediate hit the library search
is only marginally better or slightly worse, so distribution still decides.
Critically, the proposed scalar chunked-mask runtime search measured only
0.89–1.01x on no-hit scans — nowhere near a vectorized library search. Closing
P2 properly requires a vectorized/library-search representation and proof,
which is beyond the current recipe and theorem capabilities.

**Compiler provenance is now enforced.** Because the same rewrite can win under
one compiler and lose under another, measured factors may not be silently
reused across code generators:

- `MeasuredProfile` carries `measured_with` (`"clang"` for both bundled
  presets, pinned by a unit test);
- `--cost-profile <name> --cc <other>` exits with a clear error before running;
- factor files record `cc=` in their provenance header, and a mismatched
  `--cc` is refused the same way;
- `--allow-foreign-factors` is the explicit escape hatch for
  cross-compiler experiments, never the default.

Verified manually: clang preset + `--cc gcc` exits 2; the same command with
`--allow-foreign-factors` runs; a `cc=clang` factor file under `--cc gcc` exits
2; the matching combination runs.

**Optimization level is a third provenance dimension.** `SIR_OPT_LEVEL`
selects the clang/GCC `-O` level for the harness (verified IR and emitted C
both use it). Factor files record `opt=-O<level>`, presets carry
`measured_opt`, and applying a profile at a different level is refused unless
`--allow-foreign-factors` is passed. Re-running the measured-retention survey
at clang `-O2` over v6/v8/h3/v12 (53 functions) retained **zero** functions:
the `-O1` reduction wins disappear once clang optimizes the baseline loops
harder. The measured `-O2` decision is therefore to disable rewrites
(`max_total_rewrites = Some(0)`) rather than reuse `-O1` factors.

**Measured profile selection.** `select_measured_profile(compiler, opt)` and
`Optimizer::with_selected_profile` turn the provenance tables into an explicit
selection:

- clang `-O1` → conservative measured profile (array bitscan rejected);
- gcc `-O1` → GCC conservative profile;
- clang `-O2` → disable rewrites (the measured `-O2` decision);
- unmeasured combinations (for example gcc `-O2`, clang `-O3`) return `None`,
  so callers cannot silently borrow another target's data. The no-hit
  distribution remains an explicit `nohit` choice rather than an auto default.

`emit_c_diff --cost-profile auto` wires the selection to `--cc` and
`SIR_OPT_LEVEL`. Validation: v8 clang `-O1` keeps 3 reduction rewrites; v8
clang `-O2` applies 0; v6 gcc `-O1` keeps 1 (extent-96 reduction); gcc `-O2`
exits 2 with “no measured profile”. All runs 0 mismatches.

**Timing-harness robustness (P1/P5).** Repeated runs on *identical* baseline
code exposed two measurement artifacts: wall-clock `clock_gettime` showed
discrete 1.54/1.00/0.65 ratios under CPU frequency scaling, and cross-symbol
address layout aliasing produced stable 1.5–2.2x deviations that min-of-three
rounds did not remove. The timing driver now uses the invariant TSC
(`rdtsc`), five alternating rounds with per-side minima, a **null control**
that measures the no-rewrite baseline against itself to normalize each
function and reject it as `UNSTABLE` when the null ratio leaves [0.75, 1.33],
and a **composition guard** that discards a retained set whose final mixed
speedup does not clear the margin (motivated by a gcc `-O2` case where a
family measured 1.91x solo but its 4-rewrite composition ran at 0.52x).

Re-measured under the corrected harness:

- clang `-O1`: v6 2/17 retained (1.71x/1.25x solo, final 1.75x/1.25x), h3 2/7
  (1.82x/1.28x), v7 2/19 (1.31x/1.33x), 1 unstable of 43 functions;
- gcc `-O1` under the recheck + 20% final margin: v6 1/17 (solo 1.30x, final
  1.32x), h3 0/7 (1 unstable; the earlier per-function 1.10x no longer clears
  the final margin), v8 0/23 (1 unstable), v12 0/6. The GCC preset’s extent-96
  factor is updated 1.2942 → **1.3012**; its reject-everything-else shape is
  confirmed.
- no-hit v8: only the reduction benchmark survives (p06 1.27x, final 1.27x).
  The extent-48 bitscan family measures above the margin solo but its
  composition is 0.76–0.89x, so the composition guard discards it and no
  nonzero bitscan factor is exported. This supersedes the pre-hardening
  1.13–1.16x extent-48 claim.

Two further hardening steps followed the first corrected run: a family win
must **reproduce** (the trial is run twice and both must clear the solo margin
before it is retained), and the composition guard uses a separate
`--final-margin`, default **0.20**, because repeated runs showed residual
~20% noise for small effects. Authoritative clang `-O1` results:

| corpus | retained | final speedups |
|---|---|---|
| v6 | 2/17 | p02 1.73x, p08 1.24x |
| v7 | 2/19 (1 unstable) | p02 1.33x, p07 1.29x |
| h3 | 2/7 | h3b03 1.79x, h3b04 1.30x |
| v8 | 2/23 | p06 1.27x, p09 1.30x |
| v9 | 0/25 | — |

Every retained function is a reduction; array bitscan, arithmetic, mask, and
permutation families retain nothing on this set. Re-validated matrix points:
clang `-O2` 0 retained of 53 (0 unstable), gcc `-O2` 0 retained of 53
(9 unstable), clang `-O3` only sporadic non-reproducible small wins, so all
three keep the conservative `DisableRewrites` mapping. These retention
numbers predate the canonical-loop emission fix and are kept as history;
under canonical emission no family clears the margin (see below).

**Downstream (“enabling”) comparison.** `--measured-retain` also reports
`vs_original`: the original IR compiled by the selected compiler versus the
final emitted C. This is the end-to-end question that `final_speedup` alone
does not answer, because `final_speedup` compares only against our own
no-rewrite emission. `--require-downstream-win` (default off) keeps a retained
composition only when it beats the downstream compiler outright **and**
improves on the no-rewrite pipeline; a missing measurement fails the gate
closed.

Before the emission fix below, this measurement showed the problem clearly:
the retained reductions improved our own baseline by 1.26–1.81x, but the
pipeline still measured only 0.58–1.09x of clang because the no-rewrite loop
emission was itself 0.46–0.63x of clang. The rewrites were compensating for an
emission defect, not enabling the downstream compiler.

**Canonical-loop emission.** Root cause: `emit_loop` lowered every SIR loop to
`while (1) { if (!term) break; ... }` with a `c_i` → `o_i` copy chain for
carried values and no expression inlining for the guard. Clang could not
recover the canonical loop at `-O1`. The emitter now:

- renders the guard as a `while (cond)` condition whenever the whole
  termination graph is a pure integer/bool expression (a recursive
  `emit_pure_expr` whitelist; anything with calls/stores/bitvectors/nested
  loops keeps the explicit pre-test form);
- makes the loop outputs the carriers themselves, so each iteration updates
  `o_i` in place instead of `o_i = value; c_i = o_i;`;
- keeps the pre-test semantics for guards with dependencies, including
  zero-trip correctness.

Effect on the no-rewrite pipeline, clang `-O1`, same corpora and timing
driver (pre-fix → post-fix, `vs_original`):

| function | before | after |
|---|---|---|
| v6 p02 count | 0.62x | 1.02x |
| v7 p02 count | 0.63x | 1.00x |
| h3b03 any | 0.61x | 1.01x |
| h3b04 any | 0.60x | 1.03x |
| v8 p06 count+sum | 0.46x | 1.00x |
| v8 p09 count | 0.63x | 0.99x |

Across all **91** measured functions (v6/v7/v8/v9/h3, clang `-O1`): mean
`vs_original` 0.96x, 73/91 at ≥0.95x and 60/91 at ≥1.0x. Re-running the
retention survey on the fixed emission keeps **0/91** rewrite families: every
previously “retained” reduction now measures neutral or slower against the
canonical loop (for example v6 p02’s mask rewrite is 0.96x, h3b03’s is
1.07x, both below the 20% margin). The measured profiles were therefore
re-calibrated: `select_measured_profile` maps clang `-O1/-O2/-O3` and gcc
`-O1/-O2` to `DisableRewrites`; the old O1 presets remain only for explicit
experiments. `--cost-profile auto` now applies zero rewrites and runs clean on
clang `-O1` and gcc `-O1` (17/17 v6 functions each, 0 mismatches).

**First-occurrence early-exit emission.** The remaining losses were the
`first_*` search kernels in v9. The generic emitter carried
`[found=false, result, index]` and produced a compare/select/or per iteration
(two cmovs, setcc, or); clang’s code for the same source is a tight
compare/branch loop. The emitter now recognizes the shape — found output
`found || match`, result output `match ? index : result`, termination
`!found && bound(index)`, `found` initialized false — and emits:

```c
while (bound) {
    if (match) { result = index; found = true; break; }
    index = step;
}
```

The pattern requires the match and bound graphs to be pure (recursive
`emit_pure_expr`), the found flag to start false, and no live external read of
the index output (its final value differs by one iteration between the two
forms); otherwise the generic pre-tested emission is kept. Regression:
`first_occurrence_loop_emits_early_exit` lowers the shape, asserts the
early-exit C, and executes hit/miss cases natively. Measured effect within
v9 (`vs_original`): p04_first_lt_key 0.30→**1.00**, p13_first_eq_key_u32
0.36→**0.99**, p01_first_eq_key_u16 0.44→**0.96**, p10_first_key_ge_elem
0.47→**1.07**, p06_first_eq_zero 0.53→**1.00**, p08_first_eq_literal
0.58→**1.00**.

Across the final 91-function clang `-O1` survey: mean `vs_original` **1.021x**,
74/91 at ≥1.0x, 86/91 at ≥0.95x, **0/91 below 0.8x** (the canonical-loop-only
stage was 0.96x mean with 9/91 below 0.8x). The retention survey still keeps
**0/91** rewrite families: with both emission fixes the extensions are
redundant on these corpora. The P2 runtime/library performance prize is
addressed at the emitter layer in the next subsection; its proof-carrying SIR
form — a runtime-bounded operator plus an induction rule — remains open, and
the fixed-extent scalar search is now at parity.

**Byte-search library idiom (P2 performance).** The emitter now recognizes a
byte-equality first-occurrence search and lowers it to the library idiom:

- match is `Eq(ArrayAccess u8 at the index carry, scalar)` where the scalar is
  `u8`-typed or a constant ≤255;
- the termination bound is a strict `index < bound` whose bound graph contains
  no memory reads (so the length can be cached safely);
- the index starts at zero and advances by one, the loop body has no effects
  beyond reads, and only the found/result outputs are live after the loop;
- otherwise the scalar early-exit loop (or the generic pre-tested loop) is
  kept.

Emitted form:

```c
uint64_t __sir_n = bound;
if (__sir_n >= 32u) {
    const void *hit = memchr(base, (int)key, (size_t)__sir_n);
    if (hit) { result = hit - base; found = true; }
} else {
    /* scalar early-exit loop */
}
```

The `n >= 32` threshold keeps short scans on the no-call path; library
baselines measured 8–26x on no-hit scans and ~parity (7% slower at most) on
immediate hits with large n. Measured `vs_original` (clang `-O1`):
p06_first_eq_zero **12.46x**, p04_first_eq_key **11.44x**, p08_first_eq_literal
**11.04x** (previously 1.00x). The final 91-function survey now averages
**1.373x**, with 80/91 at ≥1.0x, 87/91 at ≥0.95x, 6/91 at ≥1.2x and 0/91
below 0.8x. Optimizer retention is still 0/91: this is an emission-layer
library idiom, not an optimizer candidate.

Validation for the idiom: clang 18-corpus 244 clean / 0 mismatched / 71
refused; fuzz v17/v17b/v19/v19b 60/120/160/200 clean; GCC v8/v9 23/25 clean;
wasm32 v9 25 clean with a stub `memchr`; ASan+UBSan v9 25 clean / 0 violations;
native regressions `byte_search_uses_memchr_fast_path` (runtime bound, hit,
miss, short scan, `n = 0`) and `first_occurrence_loop_emits_early_exit`.
Scope note: this closes the *performance* part of the P2 search blocker at the
emitter layer, validated differentially rather than by a proof-carrying SIR
recipe. The SIR-level runtime-bounded operator plus induction rule remains
open, and non-equality predicates (`<`, `!=`, compound) and wider elements
stay on the scalar path.

The timing driver also had a global-initialization gap: in ref mode, original
IR read zero-initialized injected globals while the emitted C received filled
buffers, so global-reading functions reported `vs_original=none`. The driver
now copies the corresponding buffers into the injected globals.

**GCC-measured preset.** The same seven-corpus measured-retention survey was
re-run under GCC 14.2 `-O1` (`--cc gcc --measured-retain`, 30k reps), and its
aggregate policy is materially stricter than clang's:

These GCC numbers were also measured before the canonical-loop emission fix;
they remain as history and are no longer used by `select_measured_profile`.

- per-corpus retained functions under the recheck + 20% final margin: v6
  1/17 (2 unstable), h3 0/7 (1 unstable; the per-function 1.10x no longer
  clears the final margin), and 0 for v12/v7/v8/v9/h4;
- only one measured boundary cleared the margin in aggregate: reduction
  extent 96 at 1.30x (exported factor 1.3012). Every arithmetic/mask candidate, every array-bitscan
  extent, and all other measured reduction extents were neutral or slower;
- permutation spot measurements: rotate-left 0.69x, byte-swap 1.10x,
  bit-reverse 0.93x.

`profile::gcc_conservative_measured_profile()` (provenance `"gcc"`) encodes
those boundaries: arithmetic/mask families rejected, array bitscan rejected,
reductions accepted only at extent 96, rotate-left/bit-reverse rejected,
byte-swap kept, rotate-right unmeasured/neutral.
`Optimizer::with_gcc_conservative_profile` and
`emit_c_diff --cost-profile conservative-gcc --cc gcc` expose it. Results:
v6 1 rewrite (the extent-96 reduction); v12/v7/v8/v9/h3/h4/v14 and the v17
fuzz corpus 0 rewrites; every run 0 mismatches, and applying the preset under
clang is refused by the provenance guard. This is a real compiler-specific
policy, not a rescaled clang profile.

## Execution log

- Baseline focused checks: `cargo test -p sir_lower
  peeled_search_with_computed_sentinel_lowers` and `cargo test -p sir_benchmarks
  --test peeled_search_semantics` both pass (one test each). Existing warnings
  remain. No evidence from these tests that the latest lowerer commit is broken.
- P0 implemented (see above); a workspace run before the fix showed 39 sir_semantics
  unit tests with the new test red and everything else passing; after the fix the
  workspace reports 697 passing, 0 failing.
- P1 increment 1 complete: per-path rewrite budget enforced, with red/green
  regressions and full workspace validation (700 passed, 0 failed, 3 ignored).
- P1 increment 2 complete: selected branch attempts sorted by score and candidate
  ID before truncation; workspace 704 passed, 0 failed, 3 ignored.
- P1 increment 3 complete: zero beam-width handling and explicit per-path stop
  reasons; workspace 707 passed, 0 failed, 3 ignored.
- P1 increment 4 complete: rejected candidates no longer consume beam slots;
  region-blind MultiplyPowerOfTwo role binding fixed and red-verified with a
  direct test; workspace 712 passed, 0 failed, 3 ignored.
- P1 increment 5 complete: per-pass applied selector score exposed for diagnostics;
  workspace 714 passed, 0 failed, 3 ignored. No whole-path ranking change.
- P3 correctness prerequisite: modulo/divide/shift-mask roles restricted to their
  semantic regions, six red/green regressions; workspace 720 passed, 0 failed,
  3 ignored.
- Merged arithmetic-role ambiguity guarded: 32-case generated red/green probe
  plus end-to-end chained-multiply refusal; budget test separated from binding
  policy. Workspace 722 passed, 0 failed, 3 ignored. Coverage loss disclosed above.
- Multiply occurrence recovery: shared resolver for proof/application, literal
  shifts, chained-multiply recovery and distinct-constant differential regression;
  workspace 724 passed, 0 failed, 3 ignored. General ambiguity guard retained.
- Modulo/divide/shift-mask occurrence recovery: shared arithmetic resolver for
  proof/application, chained and merged end-to-end regressions (red/green),
  coverage-bound verification tests; workspace 732 passed, 0 failed, 3 ignored
  (with `ASAN_OPTIONS=detect_leaks=0` for the native sanitizer tests). Remaining
  P3 role families and the broader P2/P4/P5 goals remain pending.
- P1 increment 6 complete: explicit `SearchObjective` contract, configurable
  cost model, objective/components report, and global pipeline-expansion budget;
  controlled target-cost and budget regressions; workspace 739 passed, 0 failed,
  3 ignored.
- P1 increment 7 complete: ranked same-region alternatives retained under the
  beam (`Selector::rank_all_improving`), red/green end-to-end and selection
  regressions; workspace 742 passed, 0 failed, 3 ignored. Real target models,
  measured replacement costs, neutral/worsening retention, candidate-attempt
  accounting, and identical-action-set search comparison remain pending, along
  with P2/P4/P5.
- P3 mask-algebra occurrence recovery: fixed a reproduced wrong-code path where
  merged mask regions exposed duplicate `MaskOperation` roles and recipes could
  apply an intrinsic to another pattern's root; shared occurrence resolver,
  proof/application agreement, base-operand fix for `LowestClearBitMask`, and
  stale COMP001 expectation update. Workspace 745 passed, 0 failed, 3 ignored.
  Follow-up inspection found bitscan/permutation/rotate/reduction definitions
  and recipes share one role, so they lack the mask-style split; merged regions
  may still defer permutation coverage to later passes. P1/P2/P4/P5 remain open.
- P1 increment 8 complete: `region_alternative_limit` exposes winner-only versus
  ranked same-region search, and a controlled identical-action-set comparison
  reports proofs, rewrites, reachable nodes, objective score, and expansions for
  both methods; workspace 747 passed, 0 failed, 3 ignored. Wall-clock compile
  time, real target models, measured replacement costs, neutral/worsening
  retention, P2/P4/P5, and the remaining role-family audit stay open.
- P1 increment 9 complete: global proof-attempt accounting and budget
  (`max_proof_attempts`, total in `OptimizationResult::proofs_attempted`), with
  red/green zero/one/unlimited regressions; workspace 750 passed, 0 failed,
  3 ignored. Wall-clock compile time, rewrite-attempt accounting beyond the
  beam bound, real target models, measured replacement costs,
  neutral/worsening retention, P2/P4/P5, and the remaining role-family audit
  stay open.
- Corpus-scale search comparison: all 15 development corpora re-run under
  ranked/beam-3, winner-only/beam-3, and ranked/beam-1 via the new
  `emit_c_diff --winner-only/--beam` flags. Every policy reports 227 clean /
  0 mismatched / 72 lower-refused / 61 native rewrites with zero per-function
  rewrite differences. No regression from the semantic/search changes, but also
  no corpus-level evidence that deeper per-region search contributes; fresh
  held-out evidence (P5) remains the gate for further search investment.
- Fresh v12 corpus + scan soundness fix: six new loop functions exposed a real
  miscompile where any loop containing `Shr` + `And` was misclassified as a
  zero-count scan and replaced by `clz`. `detect_zero_count_scan` now requires
  the canonical carried-shift/probe/termination shape; two semantics regressions
  and the canonical native rewrite test pin it. Combined corpus: 233 clean /
  0 mismatched / 72 lower-refused / 76 native rewrites, with zero rewrite-count
  differences on the original 15 corpora. Workspace 752 passed, 0 failed,
  3 ignored. P5 still needs independently generated held-out inputs.
- Pre-registered v13 corpus: five occurrence positives + three zero-count-scan
  negatives, frozen with expected classifications and hashes. The first run
  scored 3/5 and exposed a real recognition gap for LLVM-lowered `Sub(0,x)` /
  `Xor(x,all-ones)`. Shared negation/not helpers now feed recognition, proof
  binding, and application; v13 passes 5/5 with 3/3 safe negatives, 8 clean /
  0 mismatched and 16 rewrites. All 17 corpora: 241 clean / 0 mismatched /
  72 lower-refused / 96 rewrites. Workspace 754 passed, 0 failed, 3 ignored.
- PositionSearch scan-scalar fix: zero-count scans bound the last function-wide
  integer parameter, so a two-parameter trailing-zero loop proved and applied
  `ctz(y)` instead of `ctz(x)`. `ZeroCountScan` now carries the scanned value
  from the canonical shift/probe shape; red/green semantics and native
  regressions cover it. Corpus unchanged at 241 clean / 0 mismatched / 72
  lower-refused / 96 rewrites; workspace 756 passed, 0 failed, 3 ignored.
  First/LastOccurrence array-search role selection still uses the old
  function-wide parameter heuristic.
- Array-search role binding fix: First/LastOccurrence collection/scalar now come
  from the loop's own array accesses and element comparison instead of the last
  function-wide array/integer parameters. Three role regressions and a
  rewritten-graph native regression were red/green verified; all 17 corpora
  remain 241 clean / 0 mismatched / 72 lower-refused / 96 rewrites with zero
  per-function differences. Workspace 759 passed, 0 failed, 3 ignored.
- Reduction collection scoping: reduction roles no longer treat every declared
  boolean array as a collection candidate, so an unrelated parameter no longer
  blocks a valid reduction (false ambiguity). Two role regressions and a
  two-accessed-array refusal test pin both directions; all 17 corpora unchanged
  at 241 clean / 0 mismatched / 72 lower-refused / 96 rewrites. Workspace 762
  passed, 0 failed, 3 ignored.
- P1 increment 10 complete: `rewrite_attempts` (successful and refused
  applications) and in-process `compile_time` are reported on
  `OptimizationResult`; fallback, identical-action-set, and zero-limit tests pin
  the counters. All 17 corpora unchanged at 241 clean / 0 mismatched / 72
  lower-refused / 96 rewrites; workspace 762 passed, 0 failed, 3 ignored.
  Compile time is informational only (no time-based budget). Real target models,
  measured replacement costs, neutral/worsening retention, broader search
  evidence, P2/P4/P5, and authorization occurrence identity remain open.
- Pre-registered v14 corpus + negative reduction probe: v14 natively covers the
  search collection/scalar role fixes (2/2 recognized, 2 clean / 0 mismatched,
  1 rewrite each); a red run with the old role selection abstained rather than
  miscompiling, so the role-level/rewritten-graph regressions remain the
  red-verified evidence. A mixed loop that counts an index-derived predicate
  while accessing an array is pinned as no-rewrite. All 18 corpora: 243 clean /
  0 mismatched / 72 lower-refused / 98 native rewrites, zero per-function
  differences on the prior 17. Workspace 763 passed, 0 failed, 3 ignored.
- P4 scalar frame check: production arithmetic/mask rewrites (100–103,
  300–303) now validate their patch shape against a conservative frame — pure
  scalar operators plus `blsr`/`blsi`/`blsmsk`, occurrence-boundary inputs only,
  correct replacement target/root — with four direct unit regressions. All 18
  corpora are unchanged at 243 clean / 0 mismatched / 72 lower-refused / 98
  rewrites; workspace 767 passed, 0 failed, 3 ignored. This is shape validation,
  not theorem-to-patch equivalence, and rotate/permutation/bitscan/reduction
  patches remain outside the check.
- P4 follow-up: the frame check extends to rotate/byte-swap/bit-reverse
  (310–313) via `BitPermutation` roles, allowing the operand/amount boundary,
  `Rol`/`Ror`, and `bswap`/`rbit`. A fifth unit regression and the native
  rewritten suite pass; all 18 corpora remain 243 clean / 0 mismatched / 72
  lower-refused / 98 rewrites. Workspace 768 passed, 0 failed, 3 ignored.
  Bitscan (different replacement boundary) and theorem-to-patch equivalence
  remain open.
- P4 follow-up: bitscan (200–203) joins the frame check with a projection
  boundary rule, and `search_roles` now binds constant/computed predicate
  scalars as well as parameters. Six checker regressions plus the native
  bitscan/zero-count/early-exit suites pass; all 18 corpora remain 243 clean /
  0 mismatched / 72 lower-refused / 98 rewrites. Workspace 769 passed, 0 failed,
  3 ignored. Theorem-to-patch equivalence and scalar/permutation/bitscan
  end-to-end artifacts remain open.
- P4 artifact issuance: scalar/permutation rewrites (100–103, 300–303,
  310–313) now issue schema-level end-to-end artifacts from a root-based pure/
  total source frame, with `OptimizationResult::end_to_end_artifacts` reporting
  them. Three regressions cover arithmetic/mask issuance and bitscan abstention;
  COMP001 still rewrites. All 18 corpora remain 243 clean / 0 mismatched / 72
  lower-refused / 98 rewrites. Workspace 772 passed, 0 failed, 3 ignored.
  Bitscan artifacts and theorem-to-patch equivalence remain open.
- P4 follow-up: array bitscan (200/201) now issues artifacts from a fixed-extent,
  non-nested, read-only frame witnessed by the X06 PositionSearch certificate;
  scalar zero-count bitscan (202/203) initially abstained and now issues
  artifacts from the canonical shift-by-one shape plus operand width. Four
  `scalar_artifacts` regressions all assert issuance; all 18 corpora remain
  243 clean / 0 mismatched / 72 lower-refused / 98 rewrites. Workspace 773
  passed, 0 failed, 3 ignored. Theorem-to-patch equivalence remains open.
- P4 theorem-to-patch obligation: arithmetic rewrites (100–103) now discharge
  `bound theorem.rhs == patch_expression` with the concrete solver before the
  patch is applied, giving source == RHS == patch. Positive and wrong-constant
  regressions cover it; all 18 corpora remain 243 clean / 0 mismatched / 72
  lower-refused / 98 rewrites. Workspace 774 passed, 0 failed, 3 ignored. Mask,
  permutation, and bitscan converters remain open.
- P4 follow-up: masks 300–302 and rotations 310/311 join the theorem-to-patch
  converter. Rotations compare against the certified LHS with a structural fast
  path after the RHS path hit `DomainTooLarge` at 32 bits; boundary constants
  now resolve to constants. Converter-mapping and native rotate tests pass; all
  18 corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 rewrites.
  Workspace 775 passed, 0 failed, 3 ignored. `blsmsk`, byte swap, bit reverse,
  and bitscan converters remain open.
- P4 follow-up: `blsmsk` (303), byte swap (312), and bit reverse (313) convert
  through `SetLowestClearBit`, `ByteSwap`, and `BitReverse` and compare against
  their certified LHS. Mask, byte-swap, and bit-reverse native tests pass; all
  18 corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 rewrites.
  Workspace 776 passed, 0 failed, 3 ignored. Bitscan is the only remaining
  theorem-to-patch converter gap.
- P4 follow-up: scalar zero-count bitscan (202/203) converters map
  `TrailingZeros`/`LeadingZeros` nodes to the matching expression and discharge
  structurally against the certified LHS. Converter, artifact, and native
  zero-count tests pass; all 18 corpora remain 243 clean / 0 mismatched / 72
  lower-refused / 98 rewrites. Workspace 777 passed, 0 failed, 3 ignored. Array
  bitscan (200/201) is the only remaining converter gap.
- P4 follow-up: array bitscan (200/201) converters complete. `Pack` and
  `ArrayCmpMask` both map to `Pack(LogicalSequence)` (the predicate mask is the
  packed predicate map), `BitScanReverse` maps through, and single-element
  tuple wraps forward. An initial bare-`LogicalSequence` mapping silently
  dropped `Pack` and cost 9 rewrites on >64 extents; the fix restored
  per-function parity with the known-good baseline (243 clean / 0 mismatched /
  72 lower-refused / 98 rewrites, zero differences). Workspace 778 passed,
  0 failed, 3 ignored. Reduction theorem-to-patch obligations and solver
  soundness validation remain open.
- P4 follow-up: reduction families (0/4/5/6) join the theorem-to-patch
  converter with template-variable normalization and scalar-bitset wrapping.
  Misplacing the IDs on the frame-check hook and scalar `DomainOverflow` were
  caught and fixed; table-lookup patches remain exempt. All 18 corpora stay
  243 clean / 0 mismatched / 72 lower-refused / 98 rewrites with zero
  per-function differences. Workspace 779 passed, 0 failed, 3 ignored.
  Independent solver-soundness validation remains open.
- P3 occurrence identity: immutable authorizations now certify the exact
  arithmetic/mask occurrence per concept; candidates copy it, exact binding
  comparison includes it, and the engine requires a matching resolver result.
  Merged-region regression added; UNIT_TEST sentinel escape retained. All 18
  corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 rewrites with
  zero per-function differences. Workspace 780 passed, 0 failed, 3 ignored.
  Scan/permutation/reduction occurrence identity remains open.
- Sanitizer corpus evidence: all 18 corpora re-run with ASan+UBSan, 243 clean /
  0 mismatched / 72 lower-refused / 0 sanitizer violations.
- P3 follow-up: permutation occurrence identity added (truth output per
  permutation concept, enforced in the rewrite engine) plus rotate regression;
  all 18 corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98
  rewrites, zero per-function differences. Workspace 781 passed, 0 failed,
  3 ignored. Scan/reduction occurrence identity remains open.
- P3 follow-up: scan and reduction occurrence identity added with
  `derive_roles`-matching loop selection (highest loop for scans, unique loop
  for reductions) and engine enforcement; scan/count regressions added. All 18
  corpora remain 243 clean / 0 mismatched / 72 lower-refused / 98 rewrites with
  zero per-function differences. Workspace 783 passed, 0 failed, 3 ignored.
  Identity now spans all recipe concept families; solver soundness remains
  external.
- P1 increment 11: `emit_c_diff` gained `--time`, `--time-baseline`, and
  `--measured-retain`; `OptimizerConfig::excluded_definitions` added as the
  execution-policy hook (red-verified test: 2 rewrites without the filter, 0
  with it). Same-backend control geomean 1.004; 58 rewritten functions geomean
  0.441 (11 wins, 32 losses); measured retention keeps 8 of 59 rewrites across
  7 corpora (all reductions, 1.08–1.81x). Default behavior unchanged: 243
  clean / 0 mismatched / 72 lower-refused / 98 rewrites, zero per-function
  differences; workspace 784 passed, 0 failed, 3 ignored.
- P1 increment 12: `--time-fill` controls the measurement distribution. No-hit
  full scans measure 0.89–1.01x for array bitscan rewrites except extent-48 at
  1.13–1.16x; retention under the no-hit fill keeps 3 of 48 (two small-extent
  bitscan plus one reduction) versus 8 of 104 under early-exit random data.
  Measured retention is distribution-sensitive; no production profile model is
  wired into selection yet.
- P1 increment 13: `WeightedCostModel` + `Optimizer::with_weighted_cost` wire
  measured per-definition factors (baseline/replacement time) into the
  production selection path; empty map is neutral. The regression was
  red-verified (ignoring factors: 2 rewrites; factor 0.0 for definition 100:
  0 rewrites). Choosing/validating target profiles remains open.
- P1 increment 14: `--write-factors` exports per-definition measured speedups
  and `--cost-factors` feeds them through `Optimizer::with_weighted_cost`;
  retained compositions re-time at geomean 1.36 over 8 functions (rejected
  sets: 0.997). Production v8 early-exit run drops 8 rewrites to 3 (all
  reduction, 0 mismatches); the no-hit factor file keeps all 8 because
  per-definition aggregates cannot express per-extent variation. Savings
  scaling must divide replacement cost (inverse factor), not multiply savings.
- P4 increment: independent solver-soundness differential added
  (`solver_soundness_differential.rs`). A test-local u64/SMT-LIB reference
  oracle over all 256 states of two 4-bit variables agrees with the concrete
  solver on 9 identities + 6 invalid + 300 random pairs (proven≥6, rejected≥30,
  0 unknowns, 0 disagreements); the exhaustive backend cross-check adds 200
  supported-fragment pairs. Red-verified via corrupted `Add` (UNSOUND PROOF) and
  shift arms. No external SMT solver is installed, so third-party validation
  remains open. Workspace 787 passed, 0 failed, 3 ignored.
- P1 increment 15: extent-aware measured weights. `Candidate::source_extent`,
  `WeightedCostModel::with_extent_factors`, and
  `Optimizer::with_extent_weighted_cost` add `definition extent_bound factor`
  rules; `--write-factors` exports per-extent geomeans with the retention
  margin applied. v8 validation reproduces harness retention exactly in both
  profiles (no-hit: p01 bitscan + p06 reduction; early-exit: p06 + p09
  reductions), 2 rewrites each, 0 mismatches. Default behavior unchanged;
  per-function/dynamic-trip profiles remain open. Workspace 789 passed,
  0 failed, 3 ignored.
- P2 investigation: added `inspect_concepts` diagnostics and three gap-pinning
  regressions. Raw-element `SumReduction` generates only the def-2
  `PackedBitfield` collection proposal (no recipe, no verification definition,
  0 rewrites); `MappedSumReduction` generates zero candidates. Missing
  obligations are a reduction-specific strategy, theorem, and recipe for each,
  plus a profitability case. Runtime-length chunked-mask search stays
  unimplemented pending a target-model case. Workspace 792 passed, 0 failed,
  3 ignored.
- P5 increment: fresh pre-registered v16 corpus (runtime-extent guards,
  wider/map sums, permutations). First run found and post-fix closed a
  descending-search miscompile: sentinel/counter carrier aliasing made no-hit
  searches return the last index (38/48 native mismatches, 0 rewrites). Lowerer
  snapshots the sentinel, with a red-verified regression. Post-fix: 7 clean /
  0 mismatched / 5 lower-refused, sanitizer violations 0, timing 0.99–1.02x
  (no rewrites). `inspect_concepts` also gained `--print-sir`. All 18 prior
  corpora unchanged (243/0/72/98, zero per-function differences); workspace
  792 passed, 0 failed, 3 ignored.
- P3/P5 follow-up: permutation intrinsics now lower end-to-end. Single-block
  straight-line functions are admitted, and `llvm.fshl`/`llvm.fshr`,
  `llvm.bswap`, and `llvm.bitreverse` lower to the canonical shift-pair /
  masked-swap stage patterns. v16 post-enabler: 10 clean / 0 mismatched /
  2 lower-refused / 3 rewrites (defs 310/312/313), sanitizer violations 0.
  Lowerer regressions cover all three intrinsic shapes. Prior corpora:
  244 clean / 0 mismatched / 71 lower-refused / 98 rewrites, zero per-function
  rewrite differences (only `n09_self_recurrence` moved refused→clean).
  Measured timing: rotate 1.00x, bswap 1.03x; bitreverse was 0.21x with the
  per-bit `__sir_rbit` loop, now a branchless SWAR chain at 1.00x. Measured
  retention rejects all three at the 5% margin, so default neutral retention
  remains open. Workspace 795 passed, 0 failed, 3 ignored.
- P5 increment: committed deterministic fuzz generator and two frozen corpora
  (`v17` 60 functions seed 20260918, `v17b` 120 functions seed 777). Both run
  end to end over scalar expression trees and sum/conditional/count/any/parity
  reduction templates: 180 clean / 0 mismatched / 0 lower-refused / 41 rewrites;
  sanitizer violations 0. Same-author generator, restricted templates:
  correctness fuzzing, not recognition or external-baseline evidence.
  Workspace 795 passed, 0 failed, 3 ignored.
- P2/P5 follow-up: the runtime zero-length guard shape now lowers. The lowerer
  models clang's `umin` clamp plus tail phi as a select between the entry-guard
  value and the clamped result; v16 `r01_runtime_first_zero_guard` is clean,
  identifies as `FirstOccurrence` with PositionSearch roles, and stays at 0
  rewrites (runtime extent still has no authorized replacement). Added
  `guarded_runtime_search_lowers_with_a_tail_select`. v16 is now 11 clean /
  0 mismatched / 1 lower-refused (only the search-then-sum composition) /
  3 rewrites, sanitizer violations 0; 18 prior corpora unchanged
  (244/0/71/98, zero rewrite differences); v17 fuzz seed still 60 clean.
- P2 safety hardening: removed the fabricated `LogicalSequence { length: 64 }`
  stub for runtime-extent searches. New `runtime_extent_safety` regression
  failed with a def-200 candidate before the fix; after it, runtime regions
  generate no candidate at all while still recognizing `FirstOccurrence` and
  `PositionSearch` roles. Constant-extent v14 still rewrites both functions;
  v16 11/0/1, v17 60 clean, 18-corpus parity unchanged 244/0/71/98. Workspace
  797 passed, 0 failed, 3 ignored. The runtime replacement itself still needs a
  runtime-bounded theorem form.
- Lowering-coverage increment: `llvm.ctpop`, `llvm.cttz`, and `llvm.ctlz`
  (the well-defined `is_zero_undef = false` forms) now lower to the SIR
  `Popcount`, `TrailingZeros`, and `LeadingZeros` operations, with i32 builder
  counts widened to the IR result width. The poison (`true`) form is refused.
  New v18 corpus (popcount/cttz/ctlz u32/u64): 6 clean / 0 mismatched /
  0 lower-refused / 0 rewrites — clang already emitted the target intrinsic, so
  this is input coverage and differential correctness, not speed. Sanitizer
  violations 0; two lowerer regressions added. All 18 prior corpora unchanged
  (244/0/71/98, no status or rewrite changes).
- P1 increment: bundled conservative measured profile
  (`profile::conservative_measured_profile` + `Optimizer::with_conservative_profile`
  + `emit_c_diff --cost-profile conservative`). Array-like bitscan (defs
  200–203 with a source extent) is rejected; scalar bitscan stays enabled
  (unit-tested). Corpus deltas: v8 8→3, v6 6→5, v14 2→0, h3 4→4, all
  0 mismatches. Opt-in; default behavior and corpora unchanged. Workspace
  800 passed, 0 failed, 3 ignored.
- P5 increment: extended fuzz generator (i64 straight-line, randomized
  comparison predicates, guarded runtime reductions) and two frozen corpora,
  v19 seed 4242 (160 functions) and v19b seed 31337 (200 functions). Results:
  360 clean / 0 mismatched / 0 lower-refused / 32 rewrites, sanitizer
  violations 0; deterministic regeneration verified. Same-author,
  restricted-template correctness fuzzing. Workspace 800 passed, 0 failed,
  3 ignored.
- P1 increment: second bundled distribution preset. `nohit_measured_profile`
  keeps extent-48 array bitscan (1.15x measured) and rejects larger extents;
  `Optimizer::with_nohit_profile` and `emit_c_diff --cost-profile nohit`
  expose it. Regression pins extent 48 accepted / 64 rejected. v8 policy
  comparison: default 8, conservative 3, nohit 4 rewrites, all 0 mismatches.
  Default unchanged. Workspace 801 passed, 0 failed, 3 ignored.
- P4 increment: removed the reduction theorem-to-patch exemption. An
  instrumented scan found zero uses across 15 corpora; deletion keeps
  workspace 801 passed and 18-corpus parity 244/0/71/98 with zero per-function
  differences. Reductions the converter cannot express now fail closed instead
  of skipping the obligation. Table-lookup coverage must be re-earned through
  the converter if a real program needs it.
- P5 increment: `emit_c_diff --cc gcc` cross-compiler differential (GCC 14
  emits the C; clang 19 compiles the reference IR to an object). Results:
  v16 11/0/1 with gcc sanitizer violations 0; v19 160/0/0; v19b 200/0/0 —
  360 randomized programs additionally clean under GCC. New guarded
  `rewritten_bit_reverse_executes_natively_with_gcc` native regression.
  Same architecture; no 32-bit multilib. Workspace 802 passed, 0 failed,
  3 ignored.
- P5 follow-up: cross-compiler timing shows profitability is compiler-specific
  (v7 three-loop reduction ~1.33x under clang vs 0.81x under GCC; rotate
  1.00x vs 0.69x). Factor files now record `cc=`, `time_fill=`, and retention
  margin; the bundled presets warn that they are clang/x86-64 measurements and
  must be re-measured per code generator. No code-selection default changed.
- P5 hardening: compiler provenance enforced. Bundled presets carry
  `measured_with="clang"`; factor files record `cc=`; applying either under a
  mismatched `--cc` exits 2 unless `--allow-foreign-factors` is passed.
  Manual checks cover preset/file refusals, the override, and the matching
  path; unit tests pin preset provenance and the match policy.
- P5 increment: GCC-measured preset from a fresh seven-corpus survey
  (`--cc gcc --measured-retain`). GCC retained almost nothing at 5%: v6 1/17,
  h3 1/7, others 0; only reduction extent 96 (1.29x) cleared the margin.
  `gcc_conservative_measured_profile` (provenance gcc) + optimizer constructor
  + `--cost-profile conservative-gcc` encode the boundaries; corpus runs give
  v6 1 rewrite, all other surveyed corpora plus v17 0 rewrites, 0 mismatches,
  and clang use is refused by the provenance guard. Workspace 805 passed,
  0 failed, 3 ignored.
- P4 increment: third-party z3 validation. Installed z3-solver 5.1.0.0 into
  gitignored `vendor/z3py`; new `concrete_solver_agrees_with_external_z3` emits
  QF_BV SMT-LIB for 8 fixed identities/invalid pairs, modulo/divide/multiply/
  shift-mask shapes, and 100 random pairs, and requires z3 to agree with the
  concrete solver (≥4 proven, ≥20 rejected; unknown tolerated). Scope is the
  QF_BV expression subset; Popcount/Pack/FirstTrue/wide domains are not
  cross-checked. Test skips cleanly when z3 is absent (`Z3_BIN` override).
  Workspace 806 passed, 0 failed, 3 ignored.
- P4 follow-up: packed-sequence reduction theorems (defs 0/4/5/6) are now
  z3-validated through a sequence-level SMT emitter at lengths 32/64/96,
  including a 96-element domain the 64-bit concrete solver cannot bit-blast.
  Scan/permutation/mask theorem forms remain outside the external emitter.
  Workspace 807 passed, 0 failed, 3 ignored.
- P4 follow-up 2: real mask-algebra obligations (defs 300–303) are z3-validated
  through the SMT translation of their bit-vector identities; all four unsat.
  Remaining outside the external emitter: scan and permutation theorem shapes.
  Workspace 808 passed, 0 failed, 3 ignored.
- P4 follow-up 3: scan-family obligations (array bitscan 200/201 on a
  96-element sequence; scalar zero-count 202/203) are z3-validated through
  `FirstTrue`/`LastTrue`/`TrailingZeros`/`LeadingZeros`/`BitScanReverse`/
  `Reverse` translations. All unsat; only the permutation family (310–313)
  remains outside the external emitter. Workspace 810 passed, 0 failed,
  3 ignored.
- P4 follow-up 4: permutation obligations (rotations 310/311, byte swap 312,
  bit reverse 313) are z3-validated via `Rotate*`/`ByteSwap`/`BitReverse` and
  shift translations. All six recipe families now have third-party-validated
  registry theorems. External check does not validate patch construction,
  non-`True` filters, or emitter-unmodelled forms. Workspace 813 passed,
  0 failed, 3 ignored.
- P5 increment: cross-architecture execution via wasm32. Clang
  `--target=wasm32` + rustup `wasm-ld` + Node 22 run the emitted C for
  rewritten bit-reverse, width-relative rotate, popcount, and ctz with pinned
  expected values; test skips without the toolchain (`WASM_LD` override).
  AArch64 is blocked by the root-only apt install, so wasm32 is the available
  second ISA; scope is scalar ops, not corpus-scale loops/pointers or timing.
  Workspace 814 passed, 0 failed, 3 ignored.
- P5 follow-up: corpus-scale cross-architecture differential tool
  (`wasm_scalar_diff`) compiles optimized emitted C natively and for wasm32 and
  compares 24 deterministic scalar inputs per function across 22 corpora:
  99 clean / 0 mismatched / 72 lower-refused / 382 skipped non-scalar /
  0 harness errors. Five initial "mismatches" were signed-vs-unsigned print
  formatting in the harness, not codegen. No timing under JS; AArch64 still
  untested. Workspace 814 passed, 0 failed, 3 ignored.
- P5 follow-up 2: `wasm_diff` now marshals pointer parameters (2048-byte
  buffers + FNV checksums), covering loops/searches/reductions. Full run over
  22 corpora × 24 inputs: 481 clean / 0 mismatched / 72 lower-refused /
  0 skipped / 0 harness errors. Three harness bring-up fixes (u64 sign
  normalization, stub string.h + memcpy/memset for -nostdlib, export_name vs
  linker flag); no codegen defects found. AArch64 still untested.
- P5 evidence consolidation: added v19b (200 functions) to the
  cross-architecture run (681 clean function runs total, 0 mismatches) and ran
  GCC ASan+UBSan over the fuzz corpora (v17 60, v19 160, v19b 200 = 420
  programs) with 0 sanitizer violations and 0 mismatches. Workspace unchanged
  at 814 passed, 0 failed, 3 ignored.
- P5 gap: real library baselines added (`gate6a/v20_library_baseline.c`,
  clang/gcc -O1 raw outputs). `memchr` is ~26x faster than a naive loop on a
  4096-byte no-hit scan and ~8–12x at 64 bytes, but only parity on an immediate
  hit; builtin popcount ~14–17x and ctz ~2.4–7x faster than naive loops. The
  scalar chunked-mask proposal (0.89–1.01x no-hit) would leave nearly all of
  that win on the table, so a real P2 closure needs a vectorized/library-search
  representation and proof. Docs/README updated; no pipeline change.
- P1/P5 harness hardening: timing switched to invariant TSC ticks, five
  alternating min rounds, a per-function null control (identical-code ratio
  outside [0.75,1.33] marks the function UNSTABLE and it retains nothing), and
  a composition guard that discards retained sets whose final mix fails the
  margin. Re-measured surveys: clang O1 reductions corroborated (v6 1.71/1.25,
  h3 1.82/1.28, v7 1.31/1.33; 1 unstable of 43), gcc O1 (v6 1.33 at extent 96,
  h3 1.10 per-function, v8/v12 zero; 2 unstable of 53). GCC preset factor
  updated to 1.3012 after the recheck/20%-final-margin revalidation (h3’s
  1.10x no longer clears the final margin). Selector extended with measured clang O3 / gcc O2
  disable-rewrites decisions; gcc O3 remains unmeasured/refused. Validation:
  clang O3 auto 0 rewrites, gcc O2 auto 0 rewrites, gcc O3 auto exits 2, all
  0 mismatches. Workspace 815 passed, 0 failed, 3 ignored.
- P1/P5 hardening follow-up: solo wins must reproduce (two-of-two) and a
  separate `--final-margin` (default 0.20) governs the composition guard.
  Authoritative clang -O1 survey: v6 2/17, v7 2/19 (1 unstable), h3 2/7,
  v8 2/23, v9 0/25; every retained function is a reduction, all
  bitscan/arith/mask/permutation families rejected; retained final speedups
  1.24–1.79x. TSC re-validation: clang -O2 0/53 (0 unstable), gcc -O2 0/53
  (9 unstable), clang -O3 only non-reproducible small wins — all keep
  DisableRewrites. Workspace 815 passed, 0 failed, 3 ignored.
- P1/P5 downstream-enabling measurement: `--measured-retain` reports
  `vs_original` (final emitted C vs the selected compiler on the original IR)
  and `--require-downstream-win` keeps a composition only if it beats clang
  outright and improves on the no-rewrite pipeline, failing closed when
  unmeasured. Pre-fix this exposed that retained rewrites measured 1.26–1.81x
  vs our own emission but only 0.58–1.09x vs clang. Ref-mode timing driver now
  initializes injected globals from the same buffers as the emitted C.
  Workspace 815 passed, 0 failed, 3 ignored; 18-corpus differential 244 clean
  / 0 mismatched / 71 refused.
- P1/P5 emission canonicalization: `emit_loop` now emits `while (cond)` for
  fully pure guards (recursive `emit_pure_expr`) and uses the loop outputs as
  carriers, removing the `while (1) { if (!term) break; }` plus `o`/`c` copy
  pattern. Clang `-O1` 91-function survey: mean `vs_original` 0.96x, 73/91 at
  ≥0.95x, 60/91 at ≥1.0x (was 0.46–0.63x on the reduction kernels); zero
  rewrite families retained across v6/v7/v8/v9/h3, so the measured presets are
  superseded and `--cost-profile auto` applies 0 rewrites under clang `-O1`
  and gcc `-O1` (17/17 clean each). Remaining losses are the `first_*` search
  kernels (0.30–0.58x), sharpening P2 to vectorized/library search. Workspace
  815 passed, 0 failed, 3 ignored; 18-corpus clang 244 clean / 0 mismatched /
  71 refused; GCC differential v6 17/0/5 and h3 7/0/5; wasm32 v6 17/0,
  h3 7/0, v8 23/0; ASan+UBSan v6 17 clean / 0 violations.
- P1/P5 first-occurrence early-exit emission: the emitter recognizes the
  carried `[found=false, result, index]` search shape and emits
  `if (match) { result = index; found = true; break; }` with pure-graph and
  consumer-liveness guards, falling back to the generic loop otherwise. v9
  `first_*` measurements: p04_lt 0.30→1.00, p13_eq_u32 0.36→0.99,
  p01_eq_u16 0.44→0.96, p10_ge_elem 0.47→1.07, p06_eq_zero 0.53→1.00,
  p08_eq_literal 0.58→1.00 (all `vs_original`). Final 91-function clang -O1
  survey: mean 1.021x, 74/91 ≥1.0x, 86/91 ≥0.95x, 0/91 <0.8x; still 0/91
  rewrites retained. Workspace 816 passed, 0 failed, 3 ignored (new native
  regression `first_occurrence_loop_emits_early_exit`); 18-corpus 244 clean /
  0 mismatched / 71 refused; GCC v9 25/0/4; wasm32 v9 25/0/4; ASan+UBSan v9
  25 clean / 0 violations.
- P2 byte-search library idiom: the emitter recognizes u8 equality searches
  (Eq on a u8 ArrayAccess at the index carry, strict `<` bound with no memory
  reads, zero start, unit step, side-effect-free body, only found/result
  outputs live) and emits a `memchr` fast path for `n >= 32`, otherwise the
  scalar early-exit loop. Measured `vs_original`: p06_first_eq_zero 12.46x,
  p04_first_eq_key 11.44x, p08_first_eq_literal 11.04x; final 91-function
  survey mean 1.373x (80/91 ≥1.0x, 87/91 ≥0.95x, 6/91 ≥1.2x, 0/91 <0.8x).

## Code-size pruning — first pass (2026-09-19)

Baseline: 92,889 Rust LOC, 19 crates, 26 binaries, 44 build warnings.

Removed from the working tree (all were git-tracked and clean at deletion
time, so `git show HEAD:<path>` / `git checkout HEAD -- <path>` restores
them):

- ten unreferenced probe/diagnostic binaries (`v07_debug`, `show_rewrite`,
  `probe_semantics`, `probe_real_loop`, `probe_real`, `probe_ct`,
  `lower_test`, `gate6b_probe`, `check_verify`, `bench_all`);
- the `sir_gate4b` crate (LCF/linear-arithmetic checker plus its two test
  binaries) — Gate 4B results and proofs remain in `docs/` and
  `gate4/proof/`;
- the Gate 5A/6B harness modules and runners (`gate5a.rs`, `gate6b.rs`,
  `gate5a_bench`, `gate6b_run`) — results and raw evidence remain in
  `docs/GATE5A_RESULTS.md`, `docs/GATE6B_RESULTS.md`, and `gate6b/`;
- vector plan/derive/emit modules and `emit_vectorized`, plus the
  historical `gate6a_v3_run` and `ps002_inspect` runners.
- the legacy `sir_tests` integration crate (881 LOC across six suites);
  its builder/printer/module/regression/verifier wiring is covered by the
  per-crate suites.

No active code referenced the removed modules; the emitters, optimizer,
verification pipeline, corpora runners, and proof kernel are untouched.

Result: **83,651 LOC (−9,238), 17 crates, 10 binaries, 0 build warnings,
~1,341 public items** (from 1,467). Workspace tests: **781 passed / 0 failed
/ 3 ignored** (50 historical tests were removed with `sir_gate4b` and
`sir_tests`); v6/v8 corpus differentials remain 17/0/5 and 23/0/5.
`sir_mech::kernel`/`arith` were deliberately kept: they are the foundation
for the planned induction/array-theory work.

Next pruning targets: merge leaf crates (`sir_printer`, `sir_transform`,
`sir_nodes`, `sir_types`, `sir_builder`, `sir_inference`, `sir_tests`),
consolidate the remaining 10 binaries into one CLI with subcommands, reduce
the ~1.4k public items toward `pub(crate)`, and deduplicate the test/harness
code (~28k LOC).

### Second pass (same day, target 20k)

The first pass was not close to the 20k target, so the second pass replaced
the large curated suites with a small proof/end-to-end core and deleted
unreachable subsystems:

- **Tests:** kept 9 files (~2.2k LOC) — the `sir_mech` bitvector lemmas,
  the z3 solver differential, the runtime-bounded-search and integer-sum
  theorem regressions, the unrewritten-reduction end-to-end test, the
  byte-equality role test, and the memchr family test. The other 68 test
  files (~17.3k LOC) and their explicit `[[test]]` manifest entries were
  removed.
- **Binaries:** kept `emit_c_diff` and `wasm_diff`; the remaining 8
  historical runners (`h3_run`, `gate6a_run`, `report`, `batch_run`,
  `gen_random_corpus`, `inspect_concepts`, `emit_c_ll`, `emit_c`) were
  removed.
- **Dead corpus stack:** the old Engine-0 benchmark modules
  (`framework`, `graph`, `hackers_delight`, `boolean_reductions`,
  `positional_search`, `failures`, `non_optimizable`, `std`,
  `composition`, `real_kernels`) and the `sir_printer` crate are gone;
  `sir_benchmarks` is now just `emit.rs` plus the two runners.
- **Unused proof machinery:** `sir_mech`'s LCF `kernel` and integer
  `arith` modules and their tests were removed; the concrete bitvector
  path (`bv`/`equiv`/`cnf`/`sat`/`term`) remains. The induction work will
  reintroduce a smaller kernel rather than carrying the old one.

Result: **58,564 LOC (−34,325, −37%), 16 crates, 2 binaries, ~1,233 public
items, 0 build warnings**. Retained tests: **425 passed / 0 failed / 3
ignored**; v6/v8 corpus differentials remain 17/0/5 and 23/0/5.

Still to cut for 20k (≈38k remaining): drop the non-retained families
(arithmetic/mask/permutation machinery, ~10k), collapse verification to the
concrete solver (~4k), simplify the rewrite engine/application artifacts
(~4k), flatten the semantics/authorization/binding pipeline (~7k), prune
unused analysis passes (~2k), and merge crates/compress the remaining
harness (~2k).

### Third pass (target 20k)

The third pass kept deleting by subsystem rather than abstracting:

- **Inline unit-test modules:** stripped 8,074 LOC of `#[cfg(test)]`
  modules from 71 product files (the retained `tests/` proof/end-to-end
  core is the guardrail) plus one orphan test file.
- **Analysis framework:** `sir_analysis` is now `facts` + `graph` +
  `loops` + a ~70-line manager. Eleven analyses
  (alias/cache/constants/dependency/dominance/escape/purity/ranges/
  use_def/value_numbering and the trait/cache layer) were removed;
  only loop facts are consumed by the live pipeline.
- **Permutation family:** rotate/byte-swap/bit-reverse recognizers,
  generators, definitions, recipes, closure rules, concepts, roles,
  `SourceStructure`/`Representation` variants, and the permutation
  inference source are gone (~2.4k).
- **Arithmetic power-of-two/shift-mask family:** modulo/divide/multiply
  power-of-two and shift-mask recognizers, generator, occurrence
  resolver, definitions, symbolic rules, recipes, concepts, roles and
  structures are gone (~2.5k).
- **Set algebra/membership:** recognized but never generated or
  rewritten; deleted with its concepts (~450).

Result: **42,858 LOC (−15,706 this pass; −50,031 / −54% from the 92,889
baseline), 16 crates, 2 binaries, ~1,055 public items, 0 build warnings**.
Retained tests: **34 passed / 0 failed / 3 ignored**. Fourteen corpora
(v2–v14, v16) remain **0 mismatched / 0 harness errors**.

Remaining to 20k (≈23k): verification still carries the
symbolic/exhaustive/interpreter stack and 15 family definitions;
`sir_rewrite` still carries application artifacts/frames; the semantics
pipeline still has the authorization/binding/closure split; `sir_lower`
is a 4.7k monolith. Those are now the rewrite targets, not deletion
targets — the retained family set is reductions (Any/All/Parity/
Cardinality), sums/mapped sums, and byte-search/memchr.

### Fourth pass (target 20k)

- **Concrete-only verification:** added `Exists`/`All`/`Parity`/
  `NotEqualZero`/`EqualFullMask`/`BitwiseAndOne`/`Popcount`/`Count`
  lowering to the concrete solver, moved Any/All/Parity/Popcount to
  `ConcreteSolverChecked`, and deleted the symbolic backend, normalizer,
  semantic rule set, interpreter, exhaustive backend, report/statistics
  API, and the two tests that existed only to exercise the deleted
  backends. The z3-vs-concrete differential test is retained as the
  independent third-party check.
- **Dead-code harvest (~1.7k):** removed zero-caller helpers from
  `sir_analysis::graph`, `sir_lower` (the orphaned loop-extraction
  helper set plus `LoopRegion`), `sir_builder`, `sir_mech`,
  `sir_rewrite`, `sir_optimizer` profile constructors, `sir_inference`
  explanation API, `sir_nodes` (`Module`, unused function setters),
  `sir_types` (float/aggregate helpers), `sir_semantics`
  (`ConceptKind`, dead structure/authorization/binding helpers), and
  `sir_verification` (unused constructors).

Result: **38,745 LOC (−4,113 this pass; −54,144 / −58% from the 92,889
baseline), 16 crates, 2 binaries, ~914 public items, 0 build warnings**.
Tests: **23 passed / 0 failed / 3 ignored**. The 14-corpus differential
remains fully clean: **221 clean cases, 64 rewritten cases, 0
mismatched**.

Deletion is now exhausted: every remaining module is reachable from the
retained pipeline. Closing the last ~18.7k requires replacing layers, not
pruning them. The target is one `sir_opt` pass over SIR that owns
recognition → obligation → concrete proof → patch → emit for the retained
families, with `sir_semantics`/`sir_inference`/`sir_generation`/
`sir_selection` and most of `sir_rewrite`/`sir_verification` orchestration
folded into it (~8k combined), keeping `sir_lower`, `sir_mech`, the
retained recognizer logic, and the C emitter.

### Performance pass (priority: performance > size > readability)

Method: release build, `valgrind --tool=callgrind` on the sum workload
(`v6 p04_sum_u8_const_u64`, `--cases 1`), child compiler processes excluded
so the counts are the SIR pipeline itself. Instruction counts are the
stable metric; wall time is dominated by per-case `clang` invocations.

| stage | instructions | delta |
|---|---:|---:|
| baseline | 21,350,939 | — |
| streaming FNV digests (no `format!`/`join` Strings) | 19,632,176 | −8.1% |
| drop write-only `Proof::normalized_theorem` + verifier clone | 18,100,157 | −15.2% |
| structural `SemanticExpression` digest | 17,374,464 | −18.6% |
| direct discriminant/candidate-binding digests | 16,822,711 | −21.2% |
| structural `ReplacementPatch` digest (hashable `NodeKind`/`Type`) | 12,892,936 | −39.6% |
| `NodeArena` BTreeMap → ID-indexed `Vec<Option<Node>>` | 11,338,299 | −46.9% |
| FNV/FxHash-backed `FastHashMap`/`FastHashSet` in solver + `sir_mech` | 10,837,595 | −49.2% |
| drop unused report clones + `Bv` capacity hint | **10,741,551** | **−49.7%** |
| emitter `push_str(&format!)` → `write!` (22 sites) | 10,359,129 | −51.5% |
| `Arc<Theorem>` in `Proof`/`ProofObligation` (no deep theorem clones) | **9,454,615** | **−55.7%** |
| semantics/structure/authorization/cost/obligation DB maps → `FastHashMap` | 9,459,518 | −55.7% (flat; memchr −1.1%) |

Memchr proof (`v8 p04_first_eq_key_u8_64`): 8,037,325 (mid-pass
measurement) → **4,459,583** after the `Arc<Theorem>` and DB-map changes
(−44.5%); the `Arc<Theorem>` change alone was −43.9%.

Key changes:

- `sir_types::fnv::Fnv1a64` is a streaming `fmt::Write`/`Hasher`; all
  artifact digests hash numeric fields directly instead of building
  `format!("{:?}")` strings and `join`ing them. The former
  `String::write_str`/`memcpy` hotspots dropped from 2.49M/1.80M to
  negligible values.
- `Proof::normalized_theorem` was write-only and caused two deep theorem
  clones per proof; removed. Verifier issuance now moves the proof instead
  of cloning it.
- `SemanticExpression::digest_into` and the `ReplacementPatch` digest walk
  structure directly; `candidate_frame_digest` alone was 26% inclusive of
  the baseline and is now negligible.
- `NodeArena` lookup is O(1) and iteration stays ID-ordered; `NodeKind` and
  `Type` derive `Hash` for structural digests.
- `FastHashMap`/`FastHashSet` use FNV for identity maps and an FxHash-style
  word hasher for lookup tables; SipHash no longer appears in profiles.
- `Bv` reserves from the obligation's declared domain, cutting growth
  reallocations in sequence proofs.
- The C emitter writes directly into the output buffer instead of
  allocating a `String` per `format!`+`push_str`.
- `Proof` and `ProofObligation` now carry `Arc<Theorem>`, so checker
  artifacts and obligation databases share the theorem instead of
  deep-cloning its `SemanticExpression` tree.

Correctness: **23 passed / 0 failed / 2 ignored** (including the
`concrete_solver_agrees_with_external_z3` differential); 14-corpus
differential **64 rewrites / 221 clean / 0 mismatched**; sanitized v6
**17 clean / 0 mismatched / 5 lower-refused** with
`ASAN_OPTIONS=detect_leaks=0` (environmental LSan requirement). LOC:
**39,312** (+567 for the performance machinery, accepted under the stated
priority).

Remaining profile: allocation/free/memcpy is now ~18% of the sum path, and
~38% of the memchr path (`7.95M` instructions absolute). The allocation
callers are diffuse: per-candidate function clones in
`RewriteBuilder::apply`, frame/diff temporaries, SAT clause vectors, and
the rewrite patch builder. Next performance targets are clone-once/reuse
buffers in the rewrite engine, `Arc<Theorem>`/`Arc<Proof>` for artifact
sharing, and CNF vector reservation.

### Logic-redo pass 1 (moon-shot target: 10k LOC)

Started replacing layers with direct data flow, keeping the performance
gates above:

- **`sir_inference` deleted (630 LOC).** Evidence aggregation existed to
  predict which representations a region supports; the semantics
  pipeline already knows the region concepts. `sir_optimizer::contexts`
  now maps the exact former evidence-source concept sets (including
  closure truths) to `TransformationContext`s.
- **`sir_selection` deleted (555 LOC).** Only `rank_all_improving` was on
  the product path. The cost model, score types, and ranked selection
  moved into `sir_optimizer::selection` (~230 LOC); report/database/select
  APIs were unused and are gone.

Important lesson: the first context mapping was over-broad and raised the
sum workload from 9.46M to 15.79M instructions (+67%) by generating extra
candidates. Tightening it to the evidence-source concept sets restored
**9,444,444** instructions (slightly better than before the fold) while
keeping 14-corpus parity.

Result: **38,563 LOC, 14 crates** (from 39,312 / 16), tests **23 passed /
0 failed / 2 ignored**, 14-corpus **64 rewrites / 221 clean / 0
mismatched**. Memchr remains 4.43M instructions.

Next: fold `sir_generation` into the optimizer, trim the harness, then
port the sum family into `sir_opt` (recognize → concrete proof → patch →
emit) and delete `sir_semantics`/`sir_verification`/`sir_rewrite`
orchestration once parity holds. The 10k target requires that full
single-pass consolidation, not further micro-folds.

### Logic-redo pass 2: `sir_opt` vertical slice

Created `crates/sir_opt` (582 LOC) as the single-pass replacement
architecture. It has no dependency on semantics/verification/rewrite:

- `patch.rs`: local-id replacement graph, fresh-global-id import, SSA
  use rewriting, and mark/sweep dead-code elimination.
- `prove.rs`: proves the wrapping-add monoid laws (associativity and both
  identities) with the `sir_mech` SAT kernel.
- `sum.rs`: recognizes a fixed-extent raw-element sum loop, reads the
  certified extent/element/accumulator/index roles, requires the
  unit-stride finite trip count, proves the monoid laws, builds the
  straight-line add chain, and replaces the loop's tuple projection.

`sir_opt::optimize_sum` lowers `v6 p04_sum_u8_const_u64`, recognizes the
loop, proves the laws, removes the loop, and emits the add chain; the new
test passes. Workspace: **24 passed / 0 failed / 2 ignored**, 39,145 LOC,
15 crates (the +582 is the new slice; the old pipeline is still present
until the remaining families port).

Next port order: mapped sums (401), byte search (402), reductions
(0/4/5/6), bit scans (200–203), mask ops (300–303). Once `sir_opt` covers
those, wire the lean CLI to it, delete `sir_semantics` /
`sir_verification` / `sir_rewrite` / `sir_generation`, and shrink the
remaining core/lower/harness toward 10k.

Progress after the first slice:

- Mapped sums (401) ported: recognition finds the certified element access
  inside the map expression, and the patch re-instantiates
  Constant/Convert/Xor/Add/Sub/Mul/And/Or per constant index. The
  `v6 p10_map_then_sum_const` test passes.
- The differential runner now routes `sir_opt::optimize_sum` first and
  falls back to the old pipeline. Sums in v6/v7/v8/v10 execute through
  `sir_opt` with clang-differential validation.

Current: **39,290 LOC, 15 crates, 25 passed / 0 failed / 2 ignored**.
14-corpus differential: **69 rewrites / 221 clean / 0 mismatched** (was 64
rewrites; the new engine recognizes several sums the cost-gated old path
declined). Before deleting the old sum definitions/recipes, port the
measured-profile retention rule so those extra rewrites remain
performance-safe, then continue with byte search and reductions.

Follow-up in the same pass:

- The CLI routes sums through `sir_opt` only in static mode (no measured
  factors) and only when definitions 400/401 are not explicitly excluded;
  profile-aware and exclusion runs keep the historical pipeline until the
  measured gate ports.
- Deleted the cross-arch `wasm_diff` runner (416 LOC, recoverable) since
  AArch64/wasm work is not on the active path.

Current: **38,881 LOC, 15 crates, 25 passed / 0 failed / 2 ignored**;
14-corpus **69 rewrites / 221 clean / 0 mismatched**. Next: port byte
search (402) and reductions (0/4/5/6), then delete the old sum
recognizer/definitions/recipes and continue with the larger layers.

Byte-search slice ported:

- `sir_opt::memchr` recognizes the canonical fixed-extent
  found/result/index byte-equality loop, requires a constant bound equal
  to the u8 array length, and emits the `sir_memchr` primitive.
- `prove::first_match_laws` proves the found-flag scan and the
  reverse-overwrite scan compute the same first-match index through
  `sir_mech`; the differential runner routes 402 through `sir_opt` in
  static mode.

Current: **39,235 LOC, 15 crates, 26 passed / 0 failed / 2 ignored**;
14-corpus **69 rewrites / 221 clean / 0 mismatched** (v8 memchr now runs
through `sir_opt`). Next: reductions (0/4/5/6), measured-retention
gating, then delete the old sum/memchr paths and continue with the
larger layers.

Any reduction slice ported:

- `sir_opt::any` recognizes the fixed-extent integer-array OR reduction
  (the v6 `p07_any_or_const_u32` shape), proves the OR-monoid laws
  (associativity, identity, commutativity) with `sir_mech`, and replaces
  the loop projection with a straight-line OR chain.
- The differential runner routes definition 4 through `sir_opt` in
  static mode; profile/exclusion modes still use the old pipeline.

Current: **39,552 LOC, 15 crates, 27 passed / 0 failed / 2 ignored**;
14-corpus **69 rewrites / 221 clean / 0 mismatched**. `sir_opt` is 1,316
LOC and now covers definitions 400, 401, 402, and 4 with no dependency on
the old semantic/verification/rewrite layers.

Next: All/Parity/Cardinality slices (bool-array pack paths), the
measured-retention gate, then delete the old sum/memchr/Any
recognizers/definitions/recipes and start on the larger layers.

Cardinality slice ported:

- `sir_opt::count` recognizes the fixed-extent comparison-predicate
  count loop (`Convert(ZeroExtend(cmp))` accumulated as a sum), proves
  the add-monoid laws, re-instantiates the predicate per constant index,
  and builds the indicator add chain.
- External parameters keep their original NodeIds during patch import —
  cloning a Parameter node would renumber it and mis-emit parameter
  names.

Current: **39,922 LOC, 15 crates, 28 passed / 0 failed / 2 ignored**;
14-corpus **76 rewrites / 221 clean / 0 mismatched / 0 harness errors**
(cardinality adds seven clean rewrites). `sir_opt` is 1,658 LOC and
covers definitions 400, 401, 402, 4, and 0.

Next: All/Parity slices, then the measured-retention gate, then the
first deletions of the old recognizer/definition/recipe paths.

All slice ported:

- `sir_opt::all` recognizes the fixed-extent `select_reset` indicator
  loop (the reduction fact stores the reset value, so the per-iteration
  indicator is read from the matching loop output slot), proves the
  bitwise-AND monoid laws plus the 0/1 indicator identity, and builds the
  AND chain.
- The differential runner routes definition 5 through `sir_opt` in
  static mode; v6 `p06_all_ne_const_u8`, v7 `p04_all_ge_const_u8`, and
  v8 `p10_all_ge_const_u16_40` now execute through the new engine.
- Parity (6) has no fixture in the gate corpora, so it is deferred until
  there is a corpus to validate it against.

Current: **40,321 LOC, 15 crates, 29 passed / 0 failed / 2 ignored**;
14-corpus **79 rewrites / 221 clean / 0 mismatched / 0 harness errors**.
`sir_opt` is 2,029 LOC and covers definitions 0, 4, 5, 400, 401, and
402.

The porting phase has now produced more coverage than the old static path
for these six definitions. The next phase is the first deletion pass:
remove old recognizer/definition/recipe code only where `sir_opt`
strictly supersedes it, and port the measured-retention gate so the
profile mode can use the new engine too. After that, delete the larger
`sir_semantics` / `sir_verification` / `sir_rewrite` layers once the
remaining families are ported.

### Deletion passes 1–2

A trace mode (`SIR_OPT_TRACE`, `SIR_OPT_DISABLE`) was added to the runner
to map every corpus rewrite to its engine and definition before deleting
anything. The old path's applied definitions were 0, 4, 200, 300, 400,
401, 402; the new engine covers 0, 4, 5, 400, 401, 402.

- **Any/All old paths deleted** (they are fully superseded by the new
  slices): verification definitions, rewrite recipes, bit-set generation
  strategies, registrations, and the z3 Any/All checks. Parity preserved:
  79 rewrites / 221 clean / 0 mismatched.
- **Parity old path deleted** (the old definition never fired in any
  gate corpus): definition, recipe, generation strategies, and
  registrations.
- **Any/All/Parity recognizers deleted** from `sir_semantics`
  (`conjunctive_reduction`, `disjunctive_reduction`,
  `exclusive_reduction`).

Result so far: **39,137 LOC** (from 40,321 before the deletions), 15
crates, **29 passed / 0 failed / 2 ignored**, and unchanged corpus
parity **79 rewrites / 221 clean / 0 mismatched / 0 harness errors**.

Still old-only after the coverage map: bit scans (200, nine rewrites),
mask ops (300, seven rewrites), and multi-loop sum/count shapes. Those
need ports before their old code can be deleted. Next deletion targets
are the harness measurement machinery and the `sir_verify` fold, which
are independent of family coverage.

Harness trim:

- Removed the `--measured-retain` mode and its null-control/retention
  machinery, the manual `--cost-factors`/`--write-factors` file I/O, and
  the orphaned `RETAIN_FAMILIES`/`applied_extent` helpers (about 640 LOC).
- Kept the differential runner, `--sanitize`, basic `--time` timing, and
  `--cost-profile` presets. The measured-retention gate will be
  reintroduced directly in `sir_opt` rather than in the runner.
- Removed the cross-arch `wasm_diff` runner earlier; its work is
  recoverable from git history.

Current: **38,498 LOC, 15 crates, 29 passed / 0 failed / 2 ignored**;
14-corpus **79 rewrites / 221 clean / 0 mismatched / 0 harness errors**.
Binaries are down to 1,387 LOC.

Next: fold `sir_verify` into `sir_nodes` (compact structural validator),
then port bit scans/mask ops so their old families can be deleted.

### Bounded-search port + bitscan deletion

- `sir_opt::search` generalizes the memchr slice to any fixed-extent
  comparison predicate (`Eq`/`Ne`/`Lt`/`Le`/`Gt`/`Ge`, one or two array
  accesses). Equality over u8 keeps the `sir_memchr` primitive (402);
  everything else lowers to the reverse-overwrite select chain (200),
  proven equivalent to the found-flag scan by `first_match_laws`.
- Coverage trace before deletion confirmed the new engine covers **every**
  old definition-200 case (p01/p02/p03/p04/p09/p10/s01 family).
- Deleted the old bitscan family: verification definitions 200–203,
  rewrite recipes, generation strategies, registrations, engine
  occurrence/frame branches, and the corresponding z3 tests.

Result: **37,404 LOC** (from 38,498), 15 crates, **28 passed / 0 failed /
2 ignored**, 14-corpus **88 rewrites / 221 clean / 0 mismatched / 0
harness errors**. The search slice itself added nine clean rewrites while
removing roughly 1,100 LOC of old path.

Remaining old-only work: mask ops (300, seven rewrites) and multi-loop
sum/count shapes. Mask ops measured neutral, so the next iteration will
port or drop them and then continue deleting the old families.

### Mask-family deletion

The old mask-op family was deleted rather than ported: generation
strategy, definitions 300–303, `mask_bind`, rewrite recipes,
registrations, engine occurrence/frame branches, and the scalar frame
machinery that only served them (~1,500 LOC). The semantics mask
recognizer and bitset closure were kept because the popcount/Kernighan
path can still consume `ClearLowestSetBit` through the bitset-iteration
closure rule.

Trade-off: mask ops measured neutral in the survey, and the mask-heavy
v12/v13 corpora now report zero rewrites (they previously applied ten and
nine mask rewrites). All cases remain differential-clean; no miscompile
or correctness regression. This can be revisited by porting masks into
`sir_opt` later.

Current: **35,905 LOC, 15 crates, 28 passed / 0 failed / 2 ignored**;
14-corpus **69 rewrites / 221 clean / 0 mismatched / 0 harness errors**.

Remaining old-only work: multi-loop sum/count shapes (six corpus
functions). Porting those lets the old definitions 0/400/401 and their
recipes be deleted as well.

### Multi-loop driver + old reduction deletion

- Family recognizers now pick the first eligible loop instead of
  requiring a single-loop function, and `sir_opt::optimize` runs every
  allowed family to a bounded fixed point. This covers the two/three-loop
  sum/count corpus shapes.
- Coverage trace confirmed the driver covers **every** old definition
  0/400/401 case, so their verification definitions, rewrite recipes,
  generation strategies, engine theorem-to-patch machinery, and obsolete
  tests were deleted.
- The CLI now routes through the single `sir_opt::optimize` entry point
  in static mode; profile modes still fall back to the historical
  pipeline until the measured gate ports.

Result: **33,565 LOC** (from 35,905), 15 crates, **24 passed / 0 failed /
2 ignored**, 14-corpus **76 rewrites / 221 clean / 0 mismatched / 0
harness errors**.

The historical pipeline now registers only definition 402 (memchr), and
`sir_opt` covers it too. Next iteration removes the old-pipeline fallback
entirely and deletes `sir_semantics`, `sir_verification`, `sir_rewrite`,
`sir_generation`, and `sir_optimizer` — the largest remaining deletion,
expected to take the workspace to roughly 15k LOC before further core
trimming.

### Old-pipeline purge (moon-shot milestone)

The historical pipeline is gone:

- Deleted **`sir_semantics`, `sir_transform`, `sir_generation`,
  `sir_verification`, `sir_rewrite`, and `sir_optimizer`** (six crates).
- The runner now calls `sir_opt::optimize` directly with an allow-all
  gate; the old-pipeline fallback, measured-factor/`--cost-profile`
  machinery, old trace path, and obsolete tests were removed.
- Rewrote the last old-pipeline test (`memchr_search_family`) to assert
  the `sir_memchr` intrinsic through `sir_opt`.

Result: **15,610 LOC, 9 crates, 0 build warnings**; workspace tests
**19 passed / 0 failed / 2 ignored**; 14-corpus **76 rewrites / 221 clean
/ 0 mismatched / 0 harness errors** — identical corpus results to the
pre-purge engine.

Remaining crates: `sir_types`, `sir_nodes`, `sir_builder`, `sir_verify`,
`sir_analysis`, `sir_mech`, `sir_lower`, `sir_opt`, `sir_benchmarks`.

Assurance note: deleting `sir_verification` also removed the
z3-vs-concrete-solver differential test. The new engine's obligations are
discharged by `sir_mech` and its algebraic-law tests; restoring a
third-party z3 cross-check for `sir_opt` proof laws is the next assurance
task. Profile-aware retention likewise moves into `sir_opt` next.

Remaining path to 10k: trim `sir_lower` (4.2k), deduplicate the shared
recognizer/patch helpers across the `sir_opt` family modules (~0.5k),
fold `sir_verify` into `sir_nodes` (~0.4k), and compress the
emitter/harness.

### Functionality/extensibility audit (post-purge)

The purge did remove capability; the small core is a family-specific
engine, not yet the general framework the project needs. Preserved:
LLVM lowering + structural verification, SIR core/arena/builder, loop
analysis/graph, the `sir_mech` bitvector SAT kernel, six family
recognizers with algebraic/first-match proofs, the patch engine, the C
emitter, and the differential/sanitize harness (76 rewrites / 221 clean /
0 mismatched).

Restored immediately: `sir_opt::patch::apply` now runs
`sir_verify::Verifier` after every patch, so malformed SIR (dangling
refs, cycles, bad loop/type shapes) is rejected before emission. All 76
corpus rewrites still pass the gate.

Known losses to restore, in priority order:

1. **Extensibility**: no `Family`/definition/recipe registry. Every new
   family is a bespoke module. Restore a small `Family` trait + `Plan`
   (recognition, obligation, patch) registry (~200–300 LOC).
2. **Third-party assurance**: the z3 solver differential was deleted with
   `sir_verification`. Restore as a z3 cross-check of the `sir_opt`
   algebraic laws and generated bitvector encodings.
3. **Gates**: authorization/occurrence binding, live-out/frame checks,
   and memory-base certificates are simplified to recognizer shape
   checks. Restore as conservative plan-bound checks.
4. **Measured retention**: profile/factor gating is gone; rewrites are
   unconditional in recognized shapes.
5. **Families**: parity (6), mask ops (300–303), arithmetic
   (100–103), permutations (310–313), set algebra, Kernighan bitset
   popcount, and runtime-bound search are unsupported; v4/v12/v13 have
   zero rewrites.
6. **Vision tracks**: induction/array theory, interaction-aware global
   search, predictive cost model/autotuning/runtime dispatch.

All deleted code remains in git history (`git show HEAD:<path>`) and the
corpora/evidence are untouched, so restoration is additive rather than a
rewrite from zero.
  Validated: clang 18-corpus 244/0/71; fuzz v17/v17b/v19/v19b 60/120/160/200
  clean; GCC v8/v9 23/25 and GCC-sanitized v17 60 clean; wasm32 v9 25 and
  v19b 200 clean with a stub `memchr`; ASan+UBSan v9 25 clean / 0 violations;
  new native regression
  `byte_search_uses_memchr_fast_path` (runtime bound, hit, miss, short scan,
  n = 0). Scope: emission-layer idiom validated differentially; the
  proof-carrying SIR runtime-bounded representation remains open.
- P2 Sum/MappedSum value-model stage: added
  `VariableKind::ArithmeticSequence { length, element_width }`,
  `SemanticExpression::ArithmeticSequence / ElementAt / Sum`,
  `Value::ArithmeticSequence`, interpreter wrapping semantics (bitvector
  addition now masks to width), and concrete-solver lowering to per-element
  bitvector variables with a wrapping fold. `integer_sequence_sum.rs`: Proven
  for the right-nested unroll at element widths 64/16/8, Rejected for a
  mutated unroll and a constant claim, exhaustive agreement on a 2×2 domain.
  Workspace 820 passed / 0 failed / 3 ignored.
- P2 raw SumReduction implementation (definition 400 `UnrolledSum`):
  `SourceStructure::ArithmeticSequence` recognition (single-loop, one integer
  array), generator strategy citing `SumReduction`, canonical reduction
  binding (sum recurrence, zero identity, +1 stride, certified collection),
  frame permission limited to constant in-bounds reads of that collection,
  a candidate-bound concrete-solver theorem (`Sum == left fold`, discharged
  by hash-consed term identity), and the unroll recipe. `SubgraphBuilder`
  gained `array_access`/`convert`; `Bv` now structurally interns terms;
  trip-count-aware costing is scoped to arithmetic-sequence regions;
  multi-loop functions refuse the structure after v9 p12 regressed. Measured
  composition (clang -O1): v6 p04 extent 128 2.82x (vs clang 2.83x), v7 p01
  extent 100 3.05x (vs clang 3.10x), v8 p07 extent 112 2.95x (vs clang
  2.79x). Auto clang -O1 now selects a profile enabling exactly those three
  extents; all other measured targets stay `DisableRewrites`. Validation:
  820 workspace tests, 18-corpus 244/0/71, GCC v6/v7, ASan+UBSan v6, wasm32
  v6 all clean.
- P2 MappedSumReduction implementation (definition 401
  `UnrolledMappedSum`): `SemanticExpression::MapBinary` + `MapOp` with
  interpreter and concrete-solver evaluation, generator strategy citing
  `MappedSumReduction`, region-resolved certified constant map, and a recipe
  that clones the authorized map expression per constant index (builder
  gained `bitwise_xor`; other-array reads and unsupported nodes refuse).
  Measured composition (clang -O1): v6 p10 extent 64 3.13x (vs clang 3.10x),
  v7 p10 extent 48 3.40x (vs clang 3.52x), v8 p11 extent 64 3.13x (vs clang
  3.04x); clang -O1 profile enables exactly those extents. Updated tests:
  `unrewritten_reductions` now asserts both sum candidates, obligations and
  applied rewrites; `integer_sequence_sum` adds mapped theorem proofs and a
  wrong-constant rejection. Validation: 18-corpus 244/0/71; corpus mapped
  functions clean at 8 cases; workspace 821 passed / 0 failed / 3 ignored.
- P1/P5 sum-matrix extension: measured definitions 400/401 on the six
  representative sum functions across clang `-O2/-O3/-Os/-Oz` and gcc
  `-O1/-O2/-O3/-Os`. Retention: clang O2 400{100:1.35,128:1.20}
  401{48:1.28,64:1.29}; clang O3 400{100:1.33,128:1.24} 401{64:1.29};
  clang Os 400{100:3.04,112:2.98,128:2.76} 401{48:1.45,64:1.41};
  clang Oz 400{100:2.99,112:3.00,128:2.93} 401{48:3.63,64:3.49};
  gcc O1 400{100:1.92,112:1.69,128:1.54} 401{48:2.51,64:2.00};
  gcc O2 400{100:1.79}, 401 rejected; gcc Os 400{100:1.82,112:1.65,128:1.46},
  401 rejected; gcc O3 none. `select_measured_profile` now returns these
  profiles (clang `-Oz` included, previously refused) and leaves gcc `-O3`
  on `DisableRewrites`; auto-mode runs across six target/corpus pairs are
  clean (0 mismatches). Workspace 821 passed / 0 failed / 3 ignored.
- P1/P5 sum-coverage expansion: full-corpus factor surveys found three more
  retained functions — v6 `p05_sum_u16_const_u32` (definition 400, extent
  40) and masked mapped sums v7 `n07` (401, 48), v8/v9 `n03` (401, 64).
  Measured clang `-O1`: 4.62x, 3.40x, 2.94x/3.00x. Across targets, masked
  sums measured 7.92x/10.73x at clang `-O2`, 8.01x/10.60–10.82x at `-Os`,
  and 12.25x/15.61–15.67x at `-Oz`; extent 40 measured 6.29x at clang `-Oz`,
  2.36x at gcc `-O1`, 3.85x at gcc `-Os`, and was rejected at clang
  `-O2/-O3/-Os`. Profile rules updated with conservative measured minima;
  auto-mode differentials (clang O1/Oz, gcc O1/Os over v6–v9) are clean.
  Workspace 821 passed / 0 failed / 3 ignored.
- P1/P5 O0/Og matrix: measured the ten representative sum functions at
  clang/gcc `-O0` and `-Og`. All retained except clang `-Og` extent 40
  (unstable): clang `-O0` 1.42–1.84x, clang `-Og` 2.79–3.65x, gcc `-O0`
  1.38–1.84x, gcc `-Og` 1.50–3.97x over our own no-rewrite emission.
  `select_measured_profile` now returns measured profiles for these four
  targets; auto runs rewrite the measured extents and auto-mode
  differentials for clang `-O0`/`-Og` and gcc `-O0`/`-Og` on v6/v7/v8 are
  clean (0 mismatches). Note clang/gcc `-O0` pipelines still trail the
  compiler on the original IR (0.5–0.9x) even after the unroll; the
  composition gate measures the improvement over our own emission.
  Workspace 821 passed / 0 failed / 3 ignored.
- P2 bounded runtime-search proof rule (stage 1): added
  `BoundedFirstEqual` plus `Equal`/`LessThan`/`Ite` to the theorem language,
  with interpreter (including Integer↔BitVector coercion), normalizer, and
  concrete-solver lowering. The solver discharges the bounded-unroll rule for
  a symbolic bound against an explicit guarded-select chain up to the
  certified maximum length, with width-adaptive operand matching. Regression
  `runtime_bounded_search.rs`: Proven for lengths 2/3/4 at 8/16-bit elements,
  Rejected for a mutated chain, exhaustive agreement on a 2×2 domain, and
  sentinel behavior. Added `Memchr` as the library primitive with a proven
  contract (`Memchr == BoundedFirstEqual == guarded chain`, mutated contract
  rejected) — 6 tests total. The SIR `sir_memchr` intrinsic and emitter helper
  landed with a native runtime-length regression; the optimizer
  definition/recipe/frame wiring remains. Induction for arbitrary extents and
  the library-call frame path are the next stages. Workspace 828 passed /
  0 failed / 3 ignored.
- P2 SIR library primitive: added the `sir_memchr` intrinsic and the
  `__sir_memchr` emitter helper (first index of key below a runtime length,
  sentinel = length). Builder/emitter native regression covers hit, miss,
  short length, and `len = 0`. The optimizer definition/recipe/frame wiring
  is the remaining stage. Workspace 828 passed / 0 failed / 3 ignored.
- P2 proof-carrying byte-search family (definition 402): added the
  `RegionRoles::ByteEqualitySearch` certificate (semantics; duplicate body
  ids deduped; equality predicate, unit stride, constant bound),
  `Constraint::ByteEqualitySearch`, the `MemchrSearch` generator (fixed
  extents only; the bitscan generator defers on certified shapes), the
  `MemchrSearchDefinition` obligation (`Memchr == guarded chain`,
  ConcreteSolverChecked), the recipe emitting `sir_memchr`, a dedicated
  frame requiring the certified collection/scalar/extent operands, and
  end-to-end artifact reuse. Tests: `byte_equality_search_role` (2),
  `memchr_search_family` (candidate + Proven + applied + `__sir_memchr`),
  updated `emit_c_native` expectation. Measured v8 `p04` 11.66x
  (1,052k → 91k ticks); 18-corpus 244/0/71; GCC/ASan+UBSan/wasm32 v8 clean.
  Workspace 831 passed / 0 failed / 3 ignored.
- Sum-era re-validation (definitions 400/401 active): v16 11/0/1; fuzz
  v17/v17b/v19/v19b 60/120/160/200 clean; GCC v16 11/0/1; ASan+UBSan v16 11
  clean / 0 violations; GCC-sanitized v17 60 clean / 0 violations; wasm32 v16
  11/0/1 and v19b 200 clean; 18-corpus differential 244/0/71.
- P1/P5 target-matrix extension: measured gcc `-O3` (0/53 retained, 8
  unstable), clang `-Os` (0/53, 0 unstable) and gcc `-Os` (0/53, 7 unstable)
  on v6/v8/h3/v12 under the fixed emission. `select_measured_profile` now
  maps clang `-O2/-O3/-Os` and gcc `-O1/-O2/-O3/-Os` to `DisableRewrites`
  (clang `-Oz` remains refused) and clang `-O1` to the measured unrolled-sum
  profile; auto runs on gcc `-O3` and clang `-Os` execute v6 17/0/5 with 0
  rewrites instead of exiting 2. Workspace 816 passed, 0 failed, 3 ignored.
- P5/P1 increment: optimization level is now a measured-provenance dimension.
  `SIR_OPT_LEVEL` drives both clang/GCC flags in the harness; factor files
  record `opt=`, presets carry `measured_opt`, and mismatched levels are
  refused unless `--allow-foreign-factors`. Clang `-O2` measured-retention on
  v6/v8/h3/v12 (53 functions) retained zero rewrites, so the measured `-O2`
  policy is to disable rewrites rather than reuse `-O1` factors. Unit test
  updated; default `-O1` behavior unchanged. Workspace 814 passed, 0 failed,
  3 ignored.
- P1/P5 increment: measured profile auto-selection.
  `select_measured_profile`/`Optimizer::with_selected_profile` map measured
  (compiler, opt) pairs to conservative/gcc/conservative or disable-rewrites;
  unmeasured pairs return None. `--cost-profile auto` validates: v8 clang -O1
  3 rewrites, v8 clang -O2 0, v6 gcc -O1 1, gcc -O2 refused (exit 2), all
  0 mismatches. Workspace 815 passed, 0 failed, 3 ignored.

## Completion audit — 2026-09-18

Audited against the P0–P5 acceptance criteria using current evidence.

### Met

- **P0 local cost accounting:** region-scoped costs, no whole-function
  fallback, cycle handling; regressions in `cost_deriver.rs`. Still an
  abstract SIR cost, not a dynamic/latency model (documented scope).
- **P1 objectives/budgets/accounting:** `SearchObjective`, configurable cost
  model and strategy, ranked same-region alternatives, rewrite/proof/expansion
  budgets, `rewrite_attempts`/`compile_time`/`proofs_attempted`, and the
  identical-action-set comparison are implemented and tested
  (`objective.rs`, `compile_budget.rs`, `optimizer_tests.rs`).
- **P1 measured replacement costs and retention:** `emit_c_diff`
  `--time`/`--time-baseline`/`--measured-retain`, extent-aware weighted costs,
  conservative/nohit/GCC presets, compiler+opt-level provenance guards, and
  `select_measured_profile`/`--cost-profile auto`; validated against corpus
  deltas (v8 8→3 conservative, 4 nohit; v14 2→0; v6 gcc 1; clang -O2 0).
  After the canonical-loop emission fix the retention survey kept 0/91
  families; the later UnrolledSum implementation (definition 400) cleared the
  gate at extents 100/112/128, so clang `-O1` now selects the measured sum
  profile while every other measured target stays on `DisableRewrites`. The
  older O1 presets remain for explicit experiments only.
- **P2 investigation and safety:** exact missing obligations for
  `SumReduction`/`MappedSumReduction` are pinned; the fabricated
  `LogicalSequence{64}` runtime stub is removed with a red-verified regression;
  guarded runtime search lowering (tail phi + `umin`) is implemented and
  differentially clean (v16 `r01`, 96-element scans, fuzz).
- **P2 raw SumReduction implementation:** definition 400 `UnrolledSum` now has
  the full path — `ArithmeticSequence` structural recognition (single-loop,
  one integer array), a generator strategy citing `SumReduction`, the
  canonical reduction binding (sum recurrence, zero identity, +1 stride,
  certified collection), a frame that permits only constant in-bounds reads
  of that collection, a concrete-solver theorem `Sum == left fold` discharged
  through the hash-consed term identity, and the unroll recipe. Measured
  under the corrected clang `-O1` harness: v6 `p04` extent 128 **2.82x**
  (final 2.82x, vs clang 2.83x), v7 `p01` extent 100 **3.05x** (final 3.15x,
  vs clang 3.10x), v8 `p07` extent 112 **2.95x** (final 2.95x, vs clang
  2.79x), and v6 `p05_sum_u16_const_u32` extent 40 **4.62x** (vs clang
  3.10x). Multi-loop functions refuse the structure (a partial unroll
  regressed v9 `p12`), and the clang `-O1` auto profile enables exactly the
  three measured extents.
- **P2 MappedSumReduction implementation:** definition 401
  `UnrolledMappedSum` adds the `MapBinary` theorem form, a generator citing
  `MappedSumReduction`, region resolution of the certified constant map
  (xor/add/sub/mul/and/or, operand order preserved), and a recipe that
  re-instantiates the authorized map expression at each constant index
  (rejecting any other array read or unsupported node). Measured clang `-O1`
  composition: v6 `p10` extent 64 **3.13x** (vs clang 3.10x), v7 `p10`
  extent 48 **3.40x** (vs clang 3.52x), v8 `p11` extent 64 **3.13x**
  (vs clang 3.04x). Masked mapped sums also clear: v7 `n07` extent 48
  **3.40x**, v8 `n03` extent 64 **2.94x**, v9 `n03` extent 64 **3.00x**.
  The clang `-O1` profile enables exactly those extents; unmeasured extents
  refuse.
- **P2 runtime-bounded proof rule (bounded stage):** the theorem language now
  has `BoundedFirstEqual` with a symbolic bound, plus `Equal`, `LessThan`,
  and `Ite`. The concrete solver discharges the bounded-unroll rule by
  bit-blasting the operator against an explicit guarded-select chain for a
  certified maximum sequence length (lengths 2/3/4 at 8/16-bit elements
  tested), including mutation rejection; the exhaustive interpreter agrees on
  a 2×2 domain and sentinel behavior is pinned
  (`runtime_bounded_search.rs`). `Memchr` was added as the library primitive:
  the solver proves `Memchr == BoundedFirstEqual` and
  `Memchr == guarded chain`, and rejects a mutated contract (6 tests total).
  The SIR now also carries a `sir_memchr` intrinsic: the emitter lowers it
  through a `__sir_memchr` helper (first index below a runtime length, or the
  length), with a native runtime-length regression. This is the bounded,
  non-inductive rule the blocker asked to design first; the optimizer
  definition/recipe/frame wiring and an induction rule for arbitrary extents
  are still pending.
- **P2 proof-carrying byte-search family:** `RegionRoles::ByteEqualitySearch`
  certifies the exact found/result/index u8 equality shape (unique array
  access deduped across duplicate body entries, equality predicate,
  unit-stride induction, `!found && i < bound` termination), and
  `Constraint::ByteEqualitySearch` feeds a generator (definition 402,
  `MemchrSearch`), a frame that admits only the certified `sir_memchr`
  operands, a concrete-solver obligation `Memchr(seq, N, key) == guarded
  chain`, and a recipe emitting the intrinsic. On v8 `p04_first_eq_key_u8_64`
  the optimizer now emits `__sir_memchr` and measures **11.66x** vs clang on
  the original IR (1,052k → 91k ticks); GCC, ASan+UBSan, and wasm32 v8 runs
  are clean, and full 18-corpus parity remains 244/0/71. Because the emitter
  already lowers this shape to memchr, measured retention sees the optimizer
  family as neutral against its own baseline; the profiles leave 402 neutral
  (applied unless rewrites are disabled).
- **P3 recognition/occurrence identity:** region-scoped role binding,
  occurrence recovery for arithmetic/mask, and immutable authorization
  occurrence identity across arithmetic, mask, permutation, scan, and
  reduction concepts; ambiguity fails closed.
- **P4 assurance:** theorem-to-patch obligations span every recipe family; the
  reduction exemption is removed (instrumented scan found zero uses);
  first-party truth-table and exhaustive differentials pass; z3 validates all
  six families’ registry theorems (including packed sequences and 96-element
  domains); harness red-verified.
- **P5 evidence:** v12–v14/v16 pre-registered corpora, deterministic fuzz
  corpora (v17/v19/v19b), clang and GCC sanitizer runs, GCC cross-compiler
  differential, wasm32 cross-architecture differential (681 function runs),
  same-backend timing distributions, and library/builtin baselines
  (`memchr`/popcount/ctz) for the targeted kernels.

### Not met, with exact blockers

1. **P2 runtime-extent replacement.** Fixed-extent certified byte-equality
   searches are now proof-carrying (definition 402; 11.66x measured on v8
   `p04`). The bounded rule, the `Memchr` contract, and the SIR `sir_memchr`
   intrinsic/emitter lowering also exist. What remains: runtime-bound searches
   over pointers (the certified role is derived, but the definition/recipe
   refuse non-constant bounds until a trusted-primitive or induction rule is
   approved); an induction rule for arbitrary/unbounded extents; and
   performance for non-equality predicates and wider elements.
   Performance evidence: the emitter-level `memchr` idiom already reaches
   11–12.5x of clang on u8 equality searches and lifts the 91-function mean to
   1.373x, so the *performance* prize is realized at the emission layer;
   non-equality predicates (p16-style first-nonzero) and wider elements stay
   scalar at ~parity. Fixed-extent mask rewrites still measured only
   0.89–1.01x on no-hit scans and up to ~87x slower on early-exit data, so the
   chunked-mask recipe remains rejected.
2. **Independently authored held-out programs.** Library baselines are now
   covered for the targeted kernels (v20: `memchr`/builtin popcount/ctz versus
   naive and versus the SIR timing harness). The corpora and fuzzer remain
   same-author and no external program suite is available offline. Next action:
   user-provided external programs.
3. **AArch64.** The toolchain install requires root; `sudo` is blocked by
   no-new-privileges. Next action: root install or user-supplied toolchain.
   wasm32 is the current second ISA.
4. **Production default retention.** Presets remain opt-in to keep historical
   baselines reproducible; choosing a default profile is a per-target policy
   decision. Next action: adopt `with_selected_profile` in the production call
   path once the target matrix is approved. The measured part of that matrix
   now covers clang `-O0/-Og/-O1/-O2/-O3/-Os/-Oz` and gcc
   `-O0/-Og/-O1/-O2/-Os` with measured sum profiles; gcc `-O3` stays
   `DisableRewrites` and unmeasured combinations stay refused.
5. **Broader target matrix.** Sum-family retention is now measured on every
   compiler/opt pair in the matrix. Composition factors per extent:
   clang `-O2` 400 {100:1.35, 128:1.20}, 401 {48:1.28, 64:1.29};
   clang `-O3` 400 {100:1.33, 128:1.24}, 401 {64:1.29};
   clang `-Os` 400 {100:3.04, 112:2.98, 128:2.76}, 401 {48:1.45, 64:1.41};
   clang `-Oz` 400 {100:2.99, 112:3.00, 128:2.93}, 401 {48:3.63, 64:3.49};
   gcc `-O1` 400 {100:1.92, 112:1.69, 128:1.54}, 401 {48:2.51, 64:2.00};
   gcc `-O2` 400 {100:1.79}, 401 rejected;
   gcc `-Os` 400 {100:1.82, 112:1.65, 128:1.46}, 401 rejected;
   gcc `-O3` no retention → `DisableRewrites`. Clang `-Oz` is now a measured
   target rather than refused. Auto-mode differentials across
   clang `-O2/-Os/-Oz` and gcc `-O1/-O2/-Os` are clean (0 mismatches).
   Expanded follow-up measurement (u16 sum extent 40 and masked mapped sums
   at extents 48/64) added: clang `-O1` 400 {40:4.62, 100:2.91, 112:2.95,
   128:2.73}, 401 {48:3.37, 64:2.94}; clang `-Oz` 400 {40:6.29}; gcc `-O1`
   400 {40:2.36}, 401 {48:2.43}; gcc `-Os` 400 {40:3.85}. Masked sums
   measured **7.9–10.8x** at clang `-O2`, **8.0–11.3x** at `-Os`, and
   **12.2–16.0x** at `-Oz` (family-minimum factors kept in the profile).
   `-O0`/`-Og` added: clang `-O0` 400 {40:1.84, 100:1.78, 112:1.66,
   128:1.80}, 401 {48:1.48, 64:1.42}; clang `-Og` 400 {100:3.01, 112:2.90,
   128:2.79}, 401 {48:3.26, 64:2.97}; gcc `-O0` 400 {40:1.82, 100:1.73,
   112:1.73, 128:1.70}, 401 {48:1.40, 64:1.38}; gcc `-Og` 400 {40:3.97,
   100:1.79, 112:1.62, 128:1.50}, 401 {48:2.22, 64:1.73}. Note that at
   `-O0` the emitted pipeline still trails the compiler on the original IR
   (0.5–0.9x); the unroll improves the pipeline relative to its own
   no-rewrite emission, which is what the composition gate measures.
   Still unmeasured: other compilers, other ISAs (wasm32 is the second ISA),
   and non-x86-64 hosts.

### Current verification state

- Workspace: 831 passed / 0 failed / 3 ignored (with
  `ASAN_OPTIONS=detect_leaks=0`); changed regions rustfmt-clean.
- Default behavior intentionally changed with the UnrolledSum family: the
  optimizer now applies definition 400 on the measured single-loop sums.
  The full 18-corpus differential is still clean (244 / 0 / 71), and the
  old 98-rewrite baseline no longer describes default rewrite counts.
- Target matrix: sum profiles are measured for clang
  `-O0/-Og/-O1/-O2/-O3/-Os/-Oz` and gcc `-O0/-Og/-O1/-O2/-Os`; gcc `-O3`
  retains nothing and maps to `DisableRewrites`. Auto-mode differentials
  across clang `-O0/-O2/-Os/-Oz/-Og` and gcc `-O0/-O1/-O2/-Os/-Og` are clean
  (0 mismatches). Clang `-Oz` and the `-O0`/`-Og` targets are no longer
  refused.
- v16: 11 clean / 0 mismatched / 1 lower-refused; v17/v19/v19b fuzz: 60/160/200
  clean; GCC sanitized fuzz: 420 clean / 0 violations; wasm32 cross-architecture:
  681 clean / 0 mismatched.
- Byte-search `memchr` idiom: clang 18-corpus 244/0/71, fuzz
  v17/v17b/v19/v19b 60/120/160/200 clean, GCC v8/v9 23/25 clean, wasm32 v9
  25 clean with a stub `memchr`, ASan+UBSan v9 25 clean / 0 violations.
- UnrolledSum family: measured clang `-O1` composition 2.73x (v6 p04,
  extent 128), 2.91x (v7 p01, 100), 2.96x (v8 p07, 112) and 4.62x (v6 p05,
  extent 40), each at or above clang on the original IR; auto clang `-O1`
  applies exactly those extents. Extent 40 was also measured at clang `-Oz`
  6.29x, gcc `-O1` 2.36x and gcc `-Os` 3.85x, and rejected at clang
  `-O2/-O3/-Os`.
  Differential validation: clang 18-corpus 244/0/71, GCC v6/v7 17/19 clean,
  ASan+UBSan v6 17 clean / 0 violations, wasm32 v6 17/0/5. Workspace 821
  passed / 0 failed / 3 ignored.
- UnrolledMappedSum family (definition 401): measured clang `-O1`
  composition 2.94–3.13x over v6 p10 (64), v7 p10/n07 (48), v8 p11/n03 (64)
  and v9 n03 (64), each at or above clang on the original IR; the clang
  `-O1` profile enables exactly those extents. Masked sums measured
  7.9–10.8x at clang `-O2`, 8.0–11.3x at `-Os` and 12.2–16.0x at `-Oz`;
  the profiles keep conservative family minima. Validation: 18-corpus 244/0/71; auto-mode
  mapped functions clean; GCC/sanitizer/wasm v6 clean. Workspace 821 passed /
  0 failed / 3 ignored.
  Measured `vs_original` on the equality searches 11.0–12.5x; 91-function
  survey mean 1.373x (80/91 ≥1.0x, 87/91 ≥0.95x, 6/91 ≥1.2x, 0/91 <0.8x).
