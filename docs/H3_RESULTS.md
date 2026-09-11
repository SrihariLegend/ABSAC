# H3 — Sealed post-freeze adversarial evaluation of the frozen C3 (Any slice)

> **STATUS CORRECTION (advisor review): H3 is a FAILED SAFETY GATE, not an H3
> pass.** The corrected status matrix is below. H3 is a *sealed, post-freeze,
> white-box adversarial evaluation* — valuable because the implementation could
> not be tuned after observing results and raw results were archived before
> inspection — but it is NOT fresh held-out generalization, and it is NOT blind:
> corpus construction required stage-level probing of the frozen commit. After
> remediation, a genuinely independent H4 corpus (evaluator that does not probe
> the fixed implementation) replaces this role; H3 is promoted to **regression
> corpus D4** for the remediation phase.

Run: `h3-c3-freeze` (2026-09-02). Frozen commit `49999a4`, git tag `c3-freeze-any`.
Evaluation apparatus (harness + corpus + sealed archives) tagged `h3-eval-1`.

## Corrected status

```text
C3 reproducible freeze:       PASSED
SIR synthetic capability:     12/12 rewrote and passed differential tests
LLVM-source capability:       FAILED — 0 commits (frontier unreachable from clang loops)
Safety:                       FAILED — 2 corrupt rewrite classes committed (S1, S2)
Robustness:                   FAILED — reverse scan panicked (R1)
Blindness:                    NOT OBTAINED
Performance:                  NOT RUN
Gate 4B.1 implementation:     complete
Gate 4B.1 soundness:          FAILED under adversarial evaluation
```

The gate is a soundness failure of the *specification*, not of the machinery:
ABSAC now has a complete artifact pipeline, but that pipeline issued application
assurance for transformations whose semantic preconditions were incomplete.

Freeze configuration: see `docs/C3_FREEZE_RECORD.md`. Corpus, expected
classifications (recorded before the sealed run), sealed manifest, and the raw
archives (`h3/raw/run1.txt`, `h3/raw/exec1.txt`, committed before inspection):
see `h3/`.

## Headline result

The frozen C3 rewrote **15 committed rewrites across 14 unseen inputs**, all in the
SIR-source tier. Every committed rewrite executed differentially equivalent over
boundary and deterministic pseudo-random inputs **except three near-miss rows that
should have abstained and instead committed corrupt rewrites** (Safety failures S1a/S1b/S2).
The LLVM-source tier produced **zero committed rewrites**: all five positive-shaped
kernels generated the correct Any candidate and were blocked late at application
binding. One LLVM input crashed the frozen analysis instead of abstaining (Robustness R1).

**Outcome matrix (per input row, two-dimensional):**

| Tier | Inputs | Expected | Actual | Result |
|------|--------|----------|--------|--------|
| A (SIR) eligible positives | 12 | rewrite + equivalent | 12/12 committed, 0/… execution mismatches | Capability OK |
| A (SIR) near-miss negatives | 10 | abstain | 10/10 abstained (binding/verification gates) | Safety OK |
| A (SIR) safety near-misses | 3 (h3a21, h3a23, h3a24) | abstain REQUIRED | 3/3 REWROTE + differential corruption | **Safety FAIL (S1a, S1b, S2)** |
| B (LLVM) positive-shaped | 5 | rewrite at SIR (dialect cap) | 0 committed — candidate generated, blocked at binding (`termination does not compare the induction counter`) | Coverage gap, blocked late |
| B (LLVM) near-miss negatives | 6 | abstain | 5 abstained cleanly (lower/verify/candidates); **h3b09 crashed** | **Robustness FAIL (R1)** |
| B (LLVM) signed variant | 1 | abstain@binding | blocked at binding (as expected) | OK |

## Fresh-run evidence (sealed archives)

- `h3/raw/run1.txt` — the one-shot run over the sealed corpus (`h3_run run`, 92 lines),
  committed before inspection. Stage boundary rows per input (lowered / verify /
  candidate count / rewrite count / outcome), plus the engine's raw gate messages for
  every refused candidate.
- `h3/raw/exec1.txt` — execution of committed rewrites (`h3_run exec`, then `s1`),
  committed before inspection: differential SIR execution per rewritten input over
  enumerated patterns (extent ≤ 8), boundary patterns (all-false/all-true/first/last/
  alternating), and deterministic pseudo-random seeds.

## Rewrite-capable frontier (empirically established, disclosed)

Under the frozen C3 the rewrite-capable input language is a hand-built SIR dialect:
typed-array counted loops with zero-based unit-stride induction, `lt(carry, bound)`
termination, bound == collection extent, false OR-identity, and exactly one consumer
(a `FieldAccess` of the reduction slot; `TupleExtract` consumers abstain). The
lowerer's `.ll → SIR` output is structurally outside this dialect for every counted
loop: clang normalizes exits to `icmp eq next, bound` (binder requires `lt(carry, bound)`),
and lowering always materializes a second (dead) `TupleExtract` of the index, so the
loop tuple has two users and live-out classification refuses. Corpus engineering
therefore required stage-level probing of the frozen commit (disclosed); the H3 result
is an *implementation-tuned* measurement, not a blind one. A genuinely independent
H4 corpus, produced by an evaluator that does not probe the fixed implementation,
is required before Gate 6B sealing.

## Capability outcomes (positive tier)

- **12/12 eligible positive SIR sources** committed SchemaChecked Any rewrites:
  zero-trip (extent 0), extent 1/8/64/128/256 (wide pack, word-chunked execution),
  index-first tuple layouts (acc slot 1), select consumers downstream of the acc value,
  predicate forms across element widths u8/u16/u32/u64 and signed i8/i32, all six
  comparison operators, parameter scalars and literal scalars (incl. 0 and 100).
- Execution: 0 mismatches across all enumerated/boundary/random inputs (h3a01: 1 pattern
  for extent 0; h3a03: 256/256 enumerated; h3a04–06: 14 boundary patterns each at extents
  64/128/256; predicate rows: 6–24 inputs each).
- **LLVM-source frontier: 5/5 positive-shaped kernels** (fixed-extent extern-global
  scans, clang `-O1`) lowered, verified, recognized, and generated exactly the Any
  candidate — then were blocked at application binding by the eq-next termination
  shape. This is the "correct candidates blocked late → binding/application coverage
  problem" outcome; the binder/lowerer do not yet normalize compiler loop syntax.
- The frozen slice therefore does not yet recover the mathematical object from any
  compiler-produced loop: its end-to-end success path is reachable only from SIR
  sources that already use the slice's own loop dialect.

## Safety outcomes

Three safety near-miss rows committed rewrites that corrupt semantics. All three are
the "unstable accumulator/scalar" family the C3 near-miss list flags; none is refused
by the frozen gates.

- **S1a — boolean any with identity=true (h3a23).** OR-reduction seeded `true` is
  constant-true; the rewrite to `pack != 0` returns false on the all-false input.
  1/256 enumerated patterns mismatch (`orig=true rewritten=false`).
- **S1b — predicate any with identity=true (h3a24).** Same corruption on the predicate
  form; 9/24 boundary/random inputs mismatch.
- **S2 — induction-derived predicate scalar (h3a21, `values[i] > i`).** The per-index
  comparison cannot be expressed by the rewritten single-scalar mask; a small-domain
  witness search found 2/4000 corrupting arrays (`orig=false rewritten=true` for
  `[0,1,2,1,2,1,4,1]`).

Root-cause hypotheses (for D4, not remediated here): the Any theorem/application path
does not bind the accumulator identity (must be the monoid identity, false) as a
precondition, and it does not verify that the predicate scalar is a stable live-in
(unaffected by the induction counter). The recognition layer additionally certifies a
reduction role for accumulators that do not actually accumulate the collection.

## Robustness outcome

- **R1 — reverse scan crashes the constants analysis (h3b09).** A descending
  (`for i = 255; i >= 0; i--`) global scan lowers and verifies, then panics in
  `sir_analysis/src/constants.rs` (`attempt to add with overflow`) instead of
  abstaining. The run isolates per-input panics; the crash is captured in the archive.

## Boundaries not obtained under this freeze (open, recorded)

| Boundary | Status |
|----------|--------|
| LLVM/native execution of committed rewrites | Not obtained. Known emitter defects F6–F8 (post-loop emission order, buffer element-width typing, loop-control polarity/entry-guard) make C-emit of loop sources unsound at HEAD, and Pack/ArrayCmpMask emission is unexercised. SIR-interpreter differential execution is the frozen slice's execution bridge and is what H3 used. |
| Blind evaluation | Not obtained — authoring required stage-level probing (disclosed above); a genuinely independent H4 is required before sealing Gate 6B (H3 is not reusable as blind evidence). |

## Strategy interpretation (advisor)

H3 is not evidence that the semantic-optimization thesis failed. It is evidence
that the first ApplicationAssurance schema omitted two crucial facts: *what is the
reduction's exact initial state?* and *is every broadcast predicate operand
loop-invariant?* Those are generalizable architectural lessons — not
benchmark-specific patches. The sealed evaluation found these defects before Any
was generalized into All/Parity/Popcount. Fix the semantic contract, prove it
through the artifacts, and use a truly independent H4 to determine whether the
architecture is converging.

## D4 promotions — defect register for the remediation phase

This corpus is promoted to **regression corpus D4**; it must pass before any C4
freeze. Defects registered from this run (plus the deeper F9 found while
preparing the canonical-domain work):

1. **S1 — Any reduction identity not part of the contract.** The complete
   reduction law (operation = Or, identity = false, recurrence = acc ∨ predicate,
   result = final accumulator) is not validated: an OR-seeded `true` is rewritten.
   Fix: bind identity into the reduction contract; theorem/application must refuse
   identity ≠ monoid identity (false for Or) and carry identity in the artifact
   digest.
2. **S2 — broadcast predicate scalar not proven loop-invariant.** For
   `values[i] > i` the scalar depends on the induction counter (per-iteration
   value) but is treated as a fixed live-in. Fix: transitive dependency
   classification (element / constants / certified loop-invariant live-ins vs
   induction variable / accumulator / loop-carried / memory-derived / unknown);
   refuse unless invariant.
3. **R1 — analyses must be total.** Reverse scan panics in the constants
   analysis. Fix: checked/wrapping folds (Unknown on overflow) + a containment
   boundary so an optimizer capsule failure preserves baseline compilation.
4. **F9 — rotated eq-next clang loops lower to dead SIR.** `icmp eq next, bound`
   with continue-while-true SIR semantics = zero iterations. The LLVM frontier
   needs canonical iteration-domain normalization at the lowerer (constant-bound
   rotations → `Lt(carry, bound)`); unnormalizable guard forms must refuse, not
   mis-lower.
5. **Dead frontend tuple extractions count as observable users.** Lowered loops
   expose a dead index `TupleExtract` (zero users); classification must not count
   a provably dead projection as a second consumer.
6. **Duplicate body entries break element-access binding.** Lowered loop bodies
   may list one access node twice → spurious "element access ambiguous".
7. **Quarantine.** Until the above land and the D4 corpus passes, Any is removed
   from the default registry; it remains available only through the explicit
   experimental registry.

### Remediation status (D4) — all registered items closed

Items 1–7 above were remediated in the D4 pass; two further defects found while
running the promoted corpus were closed in a completion pass:

- **D4.1 truthful-observable classification:** a lowered loop's integer
  accumulator slot is classified through its unique `Ne(slot, 0)` truthiness
  comparison (the theorem result's type), never through the raw integer
  projection.
- **D4.2 implicit element-nonzero predicate:** raw-element OR reductions
  (`acc |= x[i]`) lower to the vectorized `x[i] != 0` mask, certified by the
  role map (`predicate == element_access`, `bitwise_or`); projected elements
  refuse.

Deterministic verification after both passes: tier A 26/26 conformance —
15/15 eligible-positive rows rewrote and 11/11 required-abstention rows (9
near-miss negatives + 2 safety poisons) refused, all committed rewrites
differentially clean (including the three rows whose H3 abstention was a fixture
artifact, and with the three corrupt H3 rows now refusing via the S1/S2 gates);
tier B h3b01–04 rewrites committed and differentially clean (10/50/50/50 patterns,
0 mismatches), h3b09 no longer panics, all near-miss negatives abstain/refuse;
workspace suite 546 passed / 0 failed. Full record: `d4/README.md`,
`d4/raw/{run1,exec1,run2,exec2,tbexec1}.txt`, `d4/manifest.sha256`. Any remains
quarantined from the trusted registry until an independent H4 corpus passes.

## Provenance, classification, and hashes

- Corpus: `h3/tier_a.tsv` (26 SIR-source rows: 12 P / 12 N / 2 S1 — semantics column
  documented in `h3/README.md`), `h3/tier_b.c` + `h3/tier_b.ll` (12 kernels, compiled
  with the freeze flags). Expected outcomes and expected safety recorded in
  `h3/expected.csv` before the sealed run.
- Sealed manifest: `h3/manifest.sha256` (generated before the sealed run; hashes for
  every artifact including the harness source).
- Evaluation chain: run exactly once over the sealed corpus → raw archive committed
  before inspection → committed rewrites executed (differential SIR) → archive
  committed before inspection → this report.
