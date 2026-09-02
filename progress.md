# Progress

## Goal

find bugs

## Context

- Repo: ABSAC (Semantic IR compiler toolchain). Active work: C3 freeze/hardening commit (in progress, uncommitted on `main`, 28 modified files, +2246/−644). The freeze narrows executable rewrites to the Any family (`sir_rewrite::registry::any_only_registry()`, DefinitionId 4); the surrounding change set hardens authorization, proposal bindings, application frame checks, digest-bound artifacts, and trip-count provenance.
- All builds green (`cargo build`), full workspace `cargo test` green (97 test-result blocks, incl. new `any_vertical_slice_differential.rs` / `any_vertical_slice_hardening.rs` / `verifier_quarantine_tests.rs`).
- The Any vertical slice (binding → frame → recipe → rewrite → differential eval) is internally consistent and heavily tested: lengths 0..128, 6 predicates, boundary patterns; 0/50 corpus rewrites are intentional (docs/C3_FREEZE_ANY.md, docs/C3_ANY_HARDENING_REPORT.md).

## Findings (3 confirmed bugs)

1. **Legacy `RewriteEngine::rewrite()` is permanently dead (API regression).**
   - `rewrite()` (sir_rewrite/src/engine.rs) forwards to `rewrite_checked` with an EMPTY `AuthorizationDatabase`. Step 1.5 unconditionally requires `candidate.authorization.matches_function(function) && candidate.binding_digest_valid() && exact_binding_matches(candidate, db)`.
   - `matches_function` (sir_generation/src/candidate.rs) returns false for EVERY `AuthorizationRef::for_unit_test()` (its region is the `u64::MAX` sentinel). Real-id candidates fail `exact_binding_matches` against the empty db (no issuer record).
   - Result: `rewrite()` can never rewrite anything. The 5 integration tests in `sir_rewrite/tests/integration_test.rs` that call it pass only because they accept any error — they are vacuous; `definition_mismatch_rejected` can no longer distinguish a missing recipe from an authorization refusal.
   - Empirical probe confirmed: valid Any function + candidate + proof + structural DB → `Err(RecipeFailed("candidate authorization is stale, forged, or binding digest invalid"))` before the recipe is reached.
   - Three independent blockers: (a) `matches_function` fingerprint 0 + region sentinel; (b) `binding_digest_valid` (test candidates carry digest 0); (c) empty `FactDatabase` → binding fails at "region loop has no loop facts".

2. **`build_frame`'s `has_volatile_or_atomic` misses `Effects::VOLATILE`.**
   - `sir_semantics/src/binding.rs` `build_frame`: `has_volatile_or_atomic = effects.contains(ATOMIC) || effects.contains(IO)` — the `VOLATILE` bit (0b0010_0000, set by `sir_lower` for `load volatile`, and checked by recognizers + `RegionInterfaceCertificate`) is never tested, despite the field name, the FrameCondition docstring ("Volatile or atomic operations present (contract forbids)"), and `supported_conservative()`'s contract.
   - Probe confirmed: a loop whose element read is `Load{ptr: ArrayAccess}` with `READ_MEMORY | VOLATILE` effects yields `frame.has_volatile_or_atomic = false` and `supported_conservative() = true`.
   - Currently masked upstream (the disjunctive/cardinality recognizers and the authorization certificate both refuse volatile regions), so no end-to-end soundness hole today — but the frame layer is the documented fail-closed gate and the engine's unit-test path defaults `ConcreteFacts` (`unwrap_or_default()`), so the frame itself admits the volatile loop. Defense-in-depth gap.

3. **`is_unit_counter` misclassifies real counting accumulators as induction counters.**
   - `sir_semantics/src/authorization.rs` `is_unit_counter` returns true for ANY "sum"/"sub" reduction whose invariant is `Constant(1)` — including a genuine `count += 1` accumulator, which is not a traversal index.
   - Probe confirmed on `for i in 0..n { any |= board[i]; count += 1 }`: `count` (kind sum, invariant Const(1)) is labeled `is_unit_counter=true`, so `semantics.rs` and `binding.rs` exclude it from accumulator candidates (`non_counter` = 1). The classification contradicts the function's own docstring ("A unit counter is a contiguous-traversal index").
   - All reachable consequences today are fail-closed (multi-accumulator loops are still refused; dead-slot removal is sound), but the label is wrong and masks the true counter/accumulator structure — a latent correctness/completeness hazard for future shapes.

## Observations (non-bugs, noted for the plan)

- `WholeValue` live-out paths (classify_live_outs / recipe / `check_any_patch_shape`) are unreachable: single-output loops cannot bind (no induction counter possible), so the whole-value branch is dead code.
- `check_any_patch_shape`'s `actual_nodes != expected_nodes` comparison is vacuous: `SubgraphBuilder::finish()` places every arena node into `roots`, so the sets are identical by construction.
- `SubgraphBuilder` local ids start at 1_000_000_000; `RewriteBuilder::rewrite_kind_refs` resolves any operand id that exists in the ORIGINAL function as external. A source function with ≥ 1e9 nodes would misresolve local refs — theoretical, unreachable in practice.
- `predicate_candidates.len() > 1` produces NO reduction role (fail-closed) — intentional, loses the interval-membership case.
- Two-accumulator loops (e.g. `sum += board[i]; count += predicate`) are refused (AmbiguousRole) — intentional fail-closed.
- The 5 integration-test fixtures are constant stubs (no loops) — they never could rewrite; the differential/hardening tests carry the real coverage.

## Plan

### Fix 1 — Restore the documented unit-test escape + a functioning legacy `rewrite()`

**Files:** `sir/crates/sir_rewrite/src/engine.rs`, `sir/crates/sir_generation/src/candidate.rs`, `sir/crates/sir_rewrite/tests/integration_test.rs`

Root cause: the P0A hardening added `matches_function` (fingerprint + region-sentinel check) and `binding_digest_valid` to `rewrite_checked` step 1.5, but did not extend the documented UNIT_TEST escape to them. Three independent blockers make `RewriteEngine::rewrite()` permanently dead for unit-test candidates: (a) `for_unit_test()` has fingerprint 0 and region `u64::MAX` → `matches_function` false; (b) test candidates carry `binding_digest: 0` → digest invalid; (c) `rewrite()` passes an empty `FactDatabase` → binding fails at "region loop has no loop facts". All 5 integration tests pass only because they accept arbitrary errors (vacuous).

Changes:
1. `engine.rs` step 1.5: branch on `authorization_id == AuthorizationId::UNIT_TEST` and skip the three checks for that id (single choke point matching the documented escape in `exact_binding_matches` and `binding_for_region`). Production ids keep the full gate unchanged.
2. `engine.rs` `rewrite()`: run `AnalysisManager` over the function to populate the `FactDatabase` before delegating (sir_analysis is already a dependency), so the binding can derive on the legacy path.
3. `integration_test.rs`: rework fixtures to be honest — replace the constant-stub `make_board_function` with a real Any loop (the `build_any_loop(Some(0))` shape from `proposal_binding.rs`), and make `bs001_end_to_end_rewrite_produces_valid_sir` / `provenance_tracks_recipe_id` assert a REAL successful rewrite (no longer error-accepting). The 3 error tests stay but assert the correct failure stage (missing structural description / recipe mismatch), not the authorization refusal.

### Fix 2 — `build_frame` must reject VOLATILE loops

**File:** `sir/crates/sir_semantics/src/binding.rs`

Add `|| effects.contains(sir_types::Effects::VOLATILE)` to the `has_volatile_or_atomic` computation (currently only `ATOMIC | IO`). Confirmed by probe: a `Load{ptr: ArrayAccess}` with `READ_MEMORY | VOLATILE` yields `has_volatile_or_atomic = false` / `supported_conservative() = true`, contradicting the field name, the FrameCondition docstring, and `supported_conservative()`'s contract. Defense-in-depth: the concept recognizers + authorization certificate already refuse volatile loops, but the frame is the documented fail-closed gate and its unit-test path defaults `ConcreteFacts`.

### Fix 3 — `is_unit_counter` must require index use

**File:** `sir/crates/sir_semantics/src/authorization.rs`

A `sum`/`sub` reduction with Constant(1) invariant is only a traversal counter if its carried variable is actually used as an element-access index (or termination operand) in the loop body. Add a loop-body scan: the counter's carried variable must be the index of some `ArrayAccess` in the body, or the lhs of the termination comparison. A genuine `count += 1` accumulator that never indexes is a REAL reduction (non-counter) → multi-accumulator loops then fail `AmbiguousRole` (fail-closed) instead of being silently relabeled as counters. Matches the docstring ("contiguous-traversal index") and the canonical Any shape (`board[i]` with the raw carried var).

### Regression tests

**Files:** `sir/crates/sir_semantics/tests/proposal_binding.rs` (or new `sir_semantics/tests/` file)
- Volatile-load Any loop → frame `supported_conservative() == false` (Fix 2).
- `for i in 0..n { any |= board[i]; count += 1 }` → `count` is NOT a unit counter; binding refuses as ambiguous (Fix 3), and the canonical `any |= board[i]` shape still binds (no regression).

### Verification

- `cargo build` clean; full-workspace `cargo test` green (97 blocks today; expect more with new tests).
- `any_vertical_slice_differential.rs` / `any_vertical_slice_hardening.rs` stay green — proves the Any slice semantics unchanged by Fixes 2–3.
- `sir_rewrite` integration tests now exercise a real rewrite (assert `Ok`), no longer error-accepting.

### Non-goals

- Do not change the production authorization/optimizer path (real AuthorizationIds keep the full gate).
- Do not make the `WholeValue` path reachable (dead by design; single-output loops cannot bind).
- Do not address the theoretical ≥1e9-node id collision.
- Do not remove `RewriteEngine::rewrite()` (public documented API) or touch `sir_lower` (formatting-only diff).

### Risks

- Fix 1 step 1.5 escape: only affects UNIT_TEST ids (tests); production candidates unaffected.
- Fix 1.2 (`rewrite()` runs analyses): only affects the legacy convenience path; the optimizer passes its own facts.
- Fix 3 index-use check: canonical shapes use the raw carried var directly, so no slice regression; differential tests guard this.

### Observations kept as-is (documented, not fixed)

- `check_any_patch_shape`'s node-count comparison is vacuous (roots == arena by construction).
- `SubgraphBuilder` 1e9 local-id space vs external-reference ambiguity.

## Implementation status (done)

All three fixes implemented and verified green:

### Fix 1 — legacy `rewrite()` restored (engine.rs, integration_test.rs)
- `rewrite_checked` step 1.5 now branches on `AuthorizationId::UNIT_TEST` and skips the three P0A gates for it (single choke point; the escape was already documented in `exact_binding_matches` and `binding_for_region`). Production ids keep the full gate.
- `rewrite()` now runs `AnalysisManager` to populate the `FactDatabase` before delegating, so the binding can derive on the legacy path.
- `sir_rewrite/tests/integration_test.rs` reworked from constant stubs to a REAL Any loop; `bs001_end_to_end_rewrite_produces_valid_sir` and `provenance_tracks_recipe_id` now assert genuine successful rewrites (loop eliminated, sir_verify passes, diff non-empty). The 3 error tests assert their intended failure stages. 5/5 pass.

### Fix 2 — volatile frame (binding.rs)
- `has_volatile_or_atomic` now also tests `Effects::VOLATILE`.
- Regression test `volatile_element_read_forces_unsupported_conservative_frame` (proposal_binding.rs): a `Load{ptr: ArrayAccess}` with `READ_MEMORY|VOLATILE` makes `derive_proposal_binding` refuse with `FrameUnsupported` (pre-fix it was admitted with `supported_conservative() == true`, confirmed by probe).

### Fix 3 — unit-counter honesty (authorization.rs)
- `is_unit_counter` now requires the carried variable to be used as an element-access index or termination operand — a bare `count += 1` accumulator is a real reduction, not the traversal counter.
- `accumulators_are_reassociable` now abstains unless there is EXACTLY ONE non-counter recurrence (previously it picked the first by scan order — a heuristic the binding layer forbids; now reachable because of Fix 3).
- Regression test `counting_accumulator_is_not_certified_as_single_reduction`: the `any |= board[i]; count += 1` loop gets ZERO reduction-domain authorizations and the binding refuses as ambiguous. 8/8 proposal_binding tests pass.

### Verification
- Full workspace `cargo test`: 97 test-result blocks, 0 failures (differential + hardening suites included).
- `cargo build` clean of new warnings (all warnings pre-existing in sir_benchmarks/sir_generation/sir_lower).

## Verification (audit-time notes)

- Baseline: `cargo build` OK; `cargo test` (workspace) all green before probing.
- Probes (temporary, removed after use): volatile-frame admission, legacy-rewrite dead path, unit-counter mislabeling — all three findings reproduced empirically before the fixes.

---

# New run: fix-lowerer-defects (understand → findings)

## Context

Consumer-facing check of the C3 freeze (`batch_run --any-only gate6a/v2_corpus.ll`) shows 7/16 kernels lowered/recognized. All 9 failures are in `sir_lower` (LLVM→SIR translator), BEFORE semantics/ontology. Classified every kernel across `gate6a/v1_corpus.ll`, `v2_corpus.ll`, `heldout_corpus.ll` (via a temporary classify bin) into failure families:

| Family | Kernels | Current failure |
|--------|---------|-----------------|
| F1 select literal arms mis-typed | w07_all_min, h02_all_match (+n14 bool literal) | `TypeMismatch` (expected iN, actual I64) / `cannot resolve select ... 'false'` |
| F2 multi-block loop (separate latch) | x08_early_exit_write, n03_early_terminate, n08_find_first_mismatch | confusing `cannot resolve gep index '%N'` |
| F3 multiple loops (shared exit CFG) | w08_two_reductions, v07_count_then_sum | half-lowered → `MissingReturn` (sir_verify soundness gate) |
| F4 store pointer typing | n01_count_with_write, n10_store_alias | `TypeMismatch` Pointer{pointee Unit} |
| F5 float/other | n05_fsum, n12_count_with_call, signed-icmp kernels | unsupported-class / unresolved |
| Deliberate gates (NOT bugs) | signed icmp (w04,w05,x02,x03,h01,v05,v16,n16...), volatile store (x01), atomic load (x05) | clean `unsupported:` refusals by design |

Root causes (confirmed by temporary alloc-trace + IR inspection):
- **F1**: the `select` emit arm strips the LLVM arm type (`i8 0` → `0`) then resolves with `type_hint=None` → `get_node_id` defaults constants to `u64`. Real i8/i32 arms then mismatch in `builder.select`'s `expect_same_type`. `n14` additionally needs `true`/`false` Bool-literal parsing (`get_node_id` only parses integers).
- **F2**: the loop detector only recognizes single-block loops (a header phi whose incoming label == the header's own label). Canonical clang output with a SEPARATE latch block (back-edge from another block) has no self-referencing phi → loop undetected → `lower_straight_line` ignores phis → the header phi is never mapped → raw mid-instruction errors.
- **F3**: TWO loop blocks (sequential/nested sharing the CFG, e.g. inner scan + outer accumulator loop). The lowerer picks the first loop; the exit-chain walker only follows unconditional br chains, so the second loop is never lowered and the `ret` (inside it) never reached → structurally invalid SIR caught only downstream as `MissingReturn`.

Deliberate gates stay untouched. `n09_atomic_load` "OK" is a naming artifact (its IR uses `load volatile`, which lowers with VOLATILE effects and is refused later by semantics/binding — by design).

## Scope decision (this run)

Fix families F1, F2, F3 (all `sir_lower`, plus a new `sir_lower/tests/lowering_regressions.rs`):
- **F1 fix** (w07/h02/n14 → lower correctly): select arms carry their LLVM type as the constant hint; add `true`/`false` parsing to `get_node_id`.
- **F2 fix** (x08/n03/n08): up-front clean refusal "unsupported: loop with separate latch block (multi-block back-edge) not modeled" instead of the raw resolution error. Full multi-block loop support (with early-exit position merge) is a future feature — recorded, not attempted.
- **F3 fix** (w08/v07): up-front clean refusal "unsupported: multiple loops sharing an exit CFG (nested/sequential loops) not modeled" instead of half-lowering into MissingReturn.

Non-goals (recorded, not fixed in this run): F4 store pointer typing, F5 float constants, call-in-loop-chain resolution, the deliberate signed/volatile/atomic gates, and the latent arithmetic-literal type-hint gap (no corpus kernel currently exposes it). Each is a separate finding for a future run.

## Plan (fix-lowerer-defects)

### Files to change
1. `sir/crates/sir_lower/src/lib.rs` — three targeted changes:
   - **Fix F1** in the `"select"` emit arm: before `strip_type`, extract each operand's type token (first whitespace token after qualifier stripping → `parse_type`) and pass it as the `type_hint` to `get_node_id` for cond/true/false. Also extend `get_node_id` (after the `parse_int_constant` branch) to map `"true"`/`"false"` → `ConstantData::boolean` (Type::Bool). This types `i8 0`, `i32 0`, `i1 false` literals by their LLVM type.
   - **Fix F3** gate: count blocks whose phis reference the block's own label (reuse the existing `loop_block_idx` closure as a filter). If count > 1 → `Err("unsupported: multiple loops sharing an exit CFG (nested/sequential loops) not modeled: <fname>")` placed before the `match loop_block_idx` decision.
   - **Fix F2** gate: if the loop block count is 0 AND any `br` in the function targets an EARLIER block (back-edge, via `ir.block_map`) → `Err("unsupported: loop with a separate latch block (multi-block back-edge) not modeled: <fname>")`.
   - No change to the deliberate signed-icmp / volatile / atomic / exact-flag gates.
2. NEW `sir/crates/sir_lower/tests/lowering_regressions.rs` — self-contained mini-LLVM snippets (no corpus dependency):
   - `i8_select_literal_lowers_and_verifies` (w07 shape: i8 sticky accumulator select with literal `0` arm) → Ok + `sir_verify` passes.
   - `i1_bool_literal_select_lowers` (n14 shape) → Ok.
   - `i32_select_literal_lowers` (h02 shape) → Ok.
   - `two_loops_share_exit_cfg_refused_cleanly` (w08/v07 shape, minimal two-loop IR) → Err contains `unsupported`.
   - `separate_latch_loop_refused_cleanly` (x08 shape, header+latch) → Err contains `unsupported`.
   - Guard: an existing single-block-loop kernel snippet still lowers (no regression) — reuse a canonical sum-scan snippet.
3. Remove the two temporary probe/classify bins (`sir_benchmarks/src/bin/w07_probe.rs`, `classify_lower.rs`) before leaving implement.

### Verification strategy
- `cargo test -p sir_lower` green (new tests); full workspace `cargo test` green (97+ blocks).
- Corpus counts via `classify_lower` (before removing it): v2 7→8 OK (w07) with w08/x08 clean refusals; v1 10→11 OK (n14), v07 clean refusal; heldout 10→11 OK (h02), n03/n08 clean refusals. No previously-OK kernel may regress to FAIL.
- `batch_run --any-only gate6a/v2_corpus.ll`: 8/16 lowered, 8/16 verified, rest clean `unsupported:` refusals (never a raw error); C3 freeze unchanged (0 rewrites expected).
- No new build warnings from the changed files.

### Risks
- F1's type-hint change only affects literal arms (value_map lookups ignore the hint) — passing kernels have i64/typed-consistent literals already, so behavior is unchanged for them; empirical OK-set diff guards this.
- F2/F3 gates are conservative (refuse, never half-lower) and only fire on shapes that today fail — the OK-set diff guards against false positives.
- Lowering more kernels (w07/h02/n14) feeds new functions to semantics; all are BooleanCollectionReduction shapes already exercised by the slice tests. Any rewrite stays impossible under the Any-only freeze.

### Done criteria
All three fixes in, corpus counts as predicted (no OK→FAIL regressions), workspace green, probes removed, progress.md updated.

## Implementation status (fix-lowerer-defects) — done

All three families fixed in `sir/crates/sir_lower/src/lib.rs`, regression tests added, probes removed.

### Fix F1 — select literal arms typed by their LLVM width (+ Bool literals)
- `"select"` emit arm now extracts each operand's declared type (`operand_type` helper: qualifier-stripped token scan → `parse_type`) and passes it as the `get_node_id` type hint — `i8 0`, `i32 0`, `i1 false` literals are built at the width the IR declares instead of the u64 default.
- `get_node_id` now parses `true`/`false` → `ConstantData::boolean` (LLVM spells i1 constants that way; previously unresolvable).
- Corpus effect: w07_all_min, h02_all_match, n14_saturating_count now lower (and verify structurally inside lowering).

### Fix F2 — clean refusal for multi-block loops (separate latch)
- New pre-emission gate: if no self-referencing loop block exists but any `br` targets an EARLIER block (back-edge via `ir.block_map`), refuse `unsupported: loop with a separate latch block (multi-block back-edge) not modeled`.
- Corpus effect: x08_early_exit_write, n03_early_terminate, n08_find_first_mismatch (and n12_count_with_call) now produce an explicit class refusal instead of raw `cannot resolve gep index '%N'` / `cannot resolve return value` errors. Full header/latch loop support (incl. early-exit position merge) is future work — recorded, not attempted.

### Fix F3 — clean refusal for multiple loops sharing an exit CFG
- Loop detection now counts self-referencing phi blocks; count > 1 → refuse `unsupported: multiple loops sharing an exit CFG (nested/sequential loops) not modeled`.
- Corpus effect: w08_two_reductions, v07_count_then_sum are refused up front instead of half-lowered into SIR that only the downstream `sir_verify` soundness gate caught (MissingReturn).

### Regression tests (new: sir_lower/tests/lowering_regressions.rs, 6 tests)
- `i8_select_literal_arm_lowers_and_is_typed_i8`, `i32_select_literal_arm_lowers`, `i1_false_literal_select_lowers` — self-contained snippets; all FAIL when F1 is reverted (verified empirically).
- `two_loops_sharing_exit_cfg_refused_cleanly`, `separate_latch_loop_refused_cleanly` — assert the clean `unsupported:` message.
- `canonical_sum_scan_still_lowers` — guard that the recognized single-loop shape is untouched.

### Verification
- `cargo test`: 98 test-result blocks, 0 failures (was 97; +1 = new test file block). No new warnings from changed files (sir_lower's 2 and sir_semantics's 1 are pre-existing; `loop_until_zero.rs` is committed at HEAD, untouched).
- Corpus (classify, then removed): no previously-OK kernel regressed; v2 7→8 OK, v1 10→11 OK, heldout 10→11 OK; the F2/F3 kernels all flipped from raw errors / downstream MissingReturn to explicit `unsupported:` refusals.
- `batch_run --any-only`: v2 8/16 lowered+verified+recognized (was 7/16), heldout 11/16, 0 rewrites (C3 Any-only freeze intact). Newly-lowered kernels carry full Facts/Truths (w07: 4 truths).
- Temp bins `w07_probe.rs` / `classify_lower.rs` removed from sir_benchmarks.

## Findings surfaced by this run's verify (recorded, OUT of scope)

Runtime-differential probing of newly-lowered kernels exposed PRE-EXISTING defects in the consumer C-emit path (`sir_benchmarks/src/emit.rs`). They are independent of the F1/F2/F3 fixes — proven by reproduction on kernels that lowered fine before this run:

- **F6 — post-loop emission order.** Entry-block nodes (e.g. `zext i32 n to i64`) referenced by the loop termination are emitted AFTER the while loop. `emit_c_ll` output for v02_sum_u16 (OK since before this run) and w07_all_min does not compile: use of `v4`/`v5` before declaration.
- **F7 — buffer params modeled as `u8*` regardless of element width.** Lowered params from `ptr` are always `Pointer{u8}`; GEP element type is separate. For u16/u32 element kernels (v02_sum_u16 takes `uint16_t*` in C) the emitter reads bytes — semantically wrong data widths.
- **F8 — loop control mis-emitted.** (a) The entry guard (`if n == 0 skip loop`) is computed but never branched on, so the `while(1)` runs at least one iteration for n=0. (b) Termination polarity is inverted: `if (!(term)) break;` continues while `term` true, but SIR Loop semantics exit when termination is true. Empirical proof: emitted C for w02_sum_u32 returned `r=217` (buf[0]) for EVERY length 0..200; the original C sums the buffer.

These three make the emit_c → compile → run path unsound for lowered corpus kernels generally (bench_all's historical numbers for lowerer-based kernels were measuring ~0–1 iteration loops). Fixing them (correct entry-guard branch, polarity, pre-loop topo hoisting, element-width pointer typing) is a coherent follow-up unit in the consumer layer — separate from the lowering fixes in this run. Not attempted here (scope + the defects predate the F1–F3 changes; see Risks in the plan).

### Verification summary (fix-lowerer-defects)
- `cargo test` workspace: 98 test-result blocks, 0 failures (was 97; +1 new sir_lower test file, 6 tests).
- sir_lower regression suite: 6/6; F1 tests fail when the fix is reverted (verified).
- Corpus consumer path (`batch_run --any-only`): v1 10→11/16, v2 7→8/16, heldout 10→11/16 lowered+verified+recognized; 0 rewrites everywhere (C3 Any-only freeze intact). No previously-OK kernel regressed; all F1/F2/F3 raw errors eliminated (only deliberate gates and the recorded F4 store-typing / F5 float non-goals remain).
- No new build warnings (sir_lower's 2 and sir_semantics's 1 are pre-existing at HEAD).

## Review notes (fix-lowerer-defects) — clean

- Self-review of the full sir_lower diff: fixed one cosmetic join artifact from an earlier edit (`let ty = ...; let const_val` on one line → split). All other hunks reviewed: `operand_type` mirrors `strip_type`'s qualifier set; detection predicate preserved byte-for-byte from the original closure; gates placed before any emission; refusal messages carry the `unsupported:` class prefix matching the existing gate style.
- Sensitivity re-checked for ALL regression tests: F1 revert → 3 select tests FAIL; F2/F3 gate disable → 2 refusal tests FAIL (raw-error / MissingReturn paths re-appear); restore → 6/6 pass. The canonical-sum guard passes both with and without gates (its purpose).
- Corpus OK-set stability re-verified after the cosmetic fix: v1 11/16, v2 8/16, heldout 11/16; 0 rewrites (freeze intact); no previously-OK kernel regressed.
- Working tree: no probe/scratch files from this run remain (w07_probe.rs, classify_lower.rs removed; builder.rs trace reverted; /tmp scratch cleaned). `git status` shows only the C3 freeze change set + this run's sir_lower edits + new test file + progress.md.
- Out-of-scope findings recorded in progress.md (F4 store-typing, F5 floats, F6–F8 emit.rs ordering/element-width/loop-control) with empirical evidence, for a future consumer-layer run.

## Implementation status (h3-c3-freeze) — done

Executed the C3 freeze/tag + fresh independent H3 run per the updated immediate sequence. Full report: `docs/H3_RESULTS.md`. Freeze record: `docs/C3_FREEZE_RECORD.md`. Corpus + sealed archives: `h3/`.

### Freeze
- Tagged the exact C3 commit: `c3-freeze-any` → `49999a4` (annotated). Freeze record carries the lockfile sha256, rustc 1.96.0 / clang 19.1.7 / x86_64, verifier policy (SchemaChecked minimum, symbolic→exhaustive, max_states 1_048_576), registry (any_only_registry, DefinitionId 4), authorization config (immutable db, no unit-test auths), optimizer config, build flags, corpus hashes, and the freeze discipline (any implementation change = C4).
- Workspace green at freeze (99 test blocks). No frozen crate modified by this run; harness `sir/crates/sir_benchmarks/src/bin/h3_run.rs` is evaluation apparatus.

### H3 (fresh held-out, sealed)
- Tier A (26 SIR-source rows: 12 P, 12 N, 2 S1) + Tier B (12 clang -O1 kernels). Expected outcomes + expected safety frozen in `h3/expected.csv`; artifact hashes sealed in `h3/manifest.sha256`; both before the one-shot run. Raw run + execution archives committed to git BEFORE inspection.
- Capability: 12/12 eligible-positive SIR sources rewrote; every committed rewrite executed differentially equivalent (enumerated/boundary/pseudo-random inputs; zero-trip extent 0 through wide extent 256). LLVM-source frontier: 5/5 positive-shaped kernels generated the Any candidate then blocked at application binding (eq-next termination) — the rewrite-capable dialect is unreachable from clang-sourced loops under the frozen C3 (binder requires lt(carry,bound)); lowering also always emits a second dead index TupleExtract → 2 loop users. 0 LLVM-source commits.
- Safety FAILS (committed corrupt rewrites on near-misses that must abstain): S1a/S1b identity=true OR-reductions → pack/mask != 0 corrupts all-false (bool 1/256 patterns, pred 9/24); S2 index-derived predicate scalar (`values[i] > i`) rewritten vs fixed scalar (witness 2/4000). Root cause hypotheses recorded for D4: accumulator identity not bound as application precondition; predicate-scalar stability not verified.
- Robustness FAIL: reverse scan (h3b09) panics the constants analysis (`attempt to add with overflow`) instead of abstaining — D4 item.
- Boundaries honestly not obtained: LLVM/native execution (F6–F8 emitter defects at HEAD), performance measurement, and blind evaluation (authoring required stage-level probing of the frozen commit — disclosed; blind re-run recommended before sealing Gate 6B).
- D4 promotions listed in `docs/H3_RESULTS.md` (binder/lowerer loop-syntax normalization; Any identity + scalar-stability conditions; constants overflow; TupleExtract live-out). Remediation deliberately NOT attempted under the freeze.
