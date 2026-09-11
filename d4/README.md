# D4 — Remediation run over the promoted H3 regression corpus

Run: `d4-remediation` (post-H3, pre-C4). Date: 2026-09-02 (same session as the H3
advisory review). Binary: `sir/crates/sir_benchmarks/src/bin/h3_run.rs` (sha256 in
`d4/manifest.sha256`) run with `ABSAC_CORPUS_DIR=d4/`. Raw archives committed
before inspection: `d4/raw/run1.txt`, `d4/raw/exec1.txt` (pass 1) and
`d4/raw/run2.txt`, `d4/raw/exec2.txt`, `d4/raw/tbexec1.txt` (final pass).

## What changed (advisory directives → code)

| Directive | Change | Evidence |
|-----------|--------|----------|
| Quarantine Any outside explicit experimental mode | `AnyRecipe` removed from `sir_rewrite::registry::default_registry()`; `any_only_registry()` documented as the ONLY (explicit experimental) access | semantic_zoo + liveout tests updated to quarantine-aware expectations |
| R1: total analyses + containment boundary | `sir_analysis/constants.rs`: all arithmetic folds wrapping/checked; div-by-zero, `i64::MIN / -1`, `i64::MIN % -1`, negate overflow, out-of-range shifts fold to Bottom (Unknown carrier); 5 new unit tests. `sir_optimizer/optimizer.rs`: every pipeline pass runs inside `catch_unwind`; a capsule failure rejects the capsule, preserves the baseline, records `IterationOutcome::PanicContained` + `OptimizationResult::containment_failures` | constants tests green; h3b09 no longer panics (see tier B) |
| S1: identity part of the theorem/application | `monoid_identity_ok()` at binding: identity must equal the recurrence's monoid identity (`bitwise_or`/`xor`/`sum` ⇒ false/0; `bitwise_and` ⇒ all-ones; `product` ⇒ 1); identity stays in the role-map digest | h3a23/h3a24 refuse: `accumulator identity is not the recurrence's monoid identity` |
| S2: transitive predicate-invariance certificate | `certify_invariant_scalar()`: full dataflow closure of the scalar; refuses on any loop-carried input/output, memory/shape/call-derived or unknown node; only constants/parameters/convert chains pass; classification (`PredicateScalarClass`) bound into the role map + digest | h3a21 refuses: `predicate scalar depends on a loop-carried value (not invariant)` |
| Loop-domain normalization (`eq next, bound` ↔ `lt(carry, bound)`) | `sir_lower` F9: rotated clang exits (back-edge on false, `icmp eq next, bound` with `next = carry + 1`) lower to `Lt(carry, bound)`; non-counted rotated shapes refuse loudly instead of silently inverting | h3b01–04/12 now pass binding and reach recipe construction (previously blocked at binding) |
| Dead frontend tuple extractions must not count as consumers | `classify_live_outs`: pure zero-user projections excluded from the observable user set (sound: unobservable, no effects; DCE removes them post-rewrite) | h3a13/h3a25/h3a26 now rewrite and are differentially clean |
| Duplicate body entries | element-access role scan treats the body as a set | lowered loops no longer report spurious "ambiguous element access" |
| Status reframing | `docs/H3_RESULTS.md` corrected: H3 = failed safety gate, white-box adversarial, promoted to regression corpus | this file |

### D4.1/D4.2 — tier-B rewrite completion (second remediation pass)

Two further defects were fixed after the first D4 run, closing the tier-B
frontier:

1. **Truthiness-observable classification (binding).** A lowered clang loop
   exposes the raw accumulator (`u8` from `acc |= x`) as the loop slot; the
   source's only observation is `acc != 0`. The recipe replaced the *integer
   projection* with the theorem's Bool, producing ill-typed functions
   (`TypeMismatch`, Bool vs I8). `classify_live_outs` now classifies an
   integer-typed disjunction slot through its unique truthiness comparison
   `Ne(slot, 0)`: the observable boundary is that comparison (replaced by the
   Bool result; the loop and projection die by DCE). Any other downstream
   shape — other comparisons, arithmetic, escaping uses, multiple observers —
   refuses. Other recurrences (counts/sums/scans) keep their direct
   classification (regression-checked by the semantic zoo).
2. **Implicit element-nonzero predicate (recipe + frame).** `acc |= x[i]` is
   `any(x[i] != 0)`. For a collection with no explicit predicate role the
   candidate is the vectorized `x[i] != 0` mask (`ArrayCmpMask(collection,
   zero-of-element-type, Ne)`) nonzero-checked. The certification is
   fail-closed: only when the role map proves the recurrence reduces the RAW
   element (`predicate == element_access`) for `bitwise_or`; a projected
   element (`x[i] >> 7` in h3b12) refuses with "integer collection without a
   certified element predicate". The frame checker validates the exact
   4-node grammar (mask, element zero, bit-vector zero, nonzero).

Tier-B results after this pass (`d4/raw/run2.txt`, `tbexec1.txt`):

| Row | D4 pass 1 | D4 final |
|-----|-----------|----------|
| h3b01 any-or (raw u8) | structural failure (Pack over u8) | **rewrite**; 10 differential patterns, 0 mismatches |
| h3b02/03/04 predicate gt/eq/lt | structural failure (Bool vs I8) | **rewrite**; 50 patterns each, 0 mismatches |
| h3b12 signed-lt-zero | structural failure (Pack over u8) | **refused** (projected element — not a raw-or reduction) |
| h3b05/06/07–11 | abstain/refuse | unchanged abstain/refuse |

Full workspace suite after the completion pass: **546 passed, 0 failed**.

Still open (next phase): H3-register items 2–4 (generalize to All/Parity/
Popcount, solver-backed application equivalence, Gate 6B sealing with an
independent H4), plus native/LLVM-level execution (F6–F8 emitters).

## Tier A outcomes vs D4 expectations (`d4/expected.csv`)

`TIER_A_REWRITES = 15/26`. Rewrote (15): h3a01–13, h3a25, h3a26 — including the
three rows corrected N→P (`h3a13` dead-index projection, `h3a25`/`h3a26`
TupleExtract consumers; the H3 abstentions were caused by a provably-dead second
projection, a fixture artifact, not by the consumer dialect). Abstained (11):
h3a14–22, h3a23, h3a24 — every row now abstains for its classified reason and,
critically, the three rows that committed corrupt rewrites in the sealed H3 run
now refuse with the intended gates:

- h3a21 (S2 poison: predicate scalar = `Convert(carried-const)`): refuse
  `predicate scalar depends on a loop-carried value (not invariant)`.
- h3a23/h3a24 (S1 poison: OR identity = true): refuse `accumulator identity is
  not the recurrence's monoid identity`.

Differential execution (exec1): all 15 rewrites 0 mismatches (enumerated,
boundary, and random patterns; h3a03/a13/a25 at 256/256 enumerated extent-8
patterns). Zero corrupt commits, zero panics in tier A.

## Tier B outcomes (same .ll kernels; h3/tier_b.c — unchanged)

| Row | Sealed H3 (frozen) | D4 (remediated) |
|-----|--------------------|-----------------|
| h3b01–04 (eq-next counted loops) | lowered, 1 candidate, blocked at termination binding | lowered, PASS, 2 candidates, **committed rewrite** (pass 1: structural failure; fixed in pass 2 — see below) |
| h3b12 (signed-lt-zero) | not lowered (signed icmp) | lowered (clang sign-mask), PASS, 2 candidates, **refused** — projected element (`x>>7`), not a raw-or reduction |
| h3b05 (runtime-bound ptr scan) | 0 candidates | 0 candidates |
| h3b06 (volatile load scan) | 0 candidates | 1 candidate, NoProof (fact change under corrected constants analysis) |
| h3b07 (memset store loop) | not lowered | not lowered (unchanged refusal) |
| h3b08 (two loops) | not lowered | not lowered (unchanged refusal) |
| h3b09 (reverse scan) | **PANIC (caught, R1)** | **no panic**; refused cleanly at lowering by the rotated-exit gate (its exit is `eq(carry, 0)` — not the +1 counted rotation, so the domain normalization refuses instead of inverting) |
| h3b10, h3b11 (separate latch block) | not lowered | not lowered (unchanged refusal) |

## Regression status

Full workspace suite after remediation: **546 passed, 0 failed** (all crates and
integration tests; quarantine-aware expectations in `semantic_zoo` and
`liveout_binding_tests`). The H3 corpus remains byte-identical under `h3/`
(sealed, historical); `d4/` is its promoted regression copy with documented
classification corrections.
