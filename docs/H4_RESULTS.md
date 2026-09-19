# H4 — blind evaluation of the remediated Gate 4B.1 pipeline (closure record)

**Status (2026-09-16): Gate 4B.1 soundness PASSED — closed.** The remediated
per-rewrite application pipeline (`CheckedApplication` /
`EndToEndVerificationArtifact`, docs/H3_RESULTS.md, d4/README.md) passed a
pre-registered, implementation-blind, sealed H4 evaluation with **zero
corrupt commits, zero panics, and zero differential mismatches**.

This record closes the item H3 opened: "a genuinely independent H4 corpus
(evaluator that does not probe the fixed implementation) replaces this
role" (docs/H3_RESULTS.md).

## Corpus series and outcomes

| Corpus | Role | Result |
|---|---|---|
| `h4/` | first seal | **corpus-construction failure** (fixture grammar; no optimizer result). Archived, raw committed before inspection. |
| `h4b/` | corrected seal, blind run | Tier A: 17/17 eligible positives rewrote, 18/18 required abstentions refused, 0 mismatches. Tier B: **0/4 positives** — found F10; 9/9 negatives abstained, no panic. |
| `h4b/` (regression, post-F10) | brown-box regression | Tier B positives 4/4 rewrote; negatives unchanged; D4 outcomes byte-identical to `d4/raw/run2.txt`. |
| `h4c/` | **fresh closure corpus** (sealed at post-F10 state `1da3b28`) | Tier A: 10/10 eligible positives rewrote, 7/7 abstentions refused, 0 mismatches. Tier B: 3/3 positives rewrote (0 mismatches), 7/7 negatives abstained, no panic. |

Gate criteria applied per corpus: a positive that does not commit is a
coverage failure, a negative or safety row that commits is a soundness
failure, and any panic is a robustness failure. All pre-registered
expectations in `expected.csv` were met by `h4c`.

## Finding F10 (H4b) and remediation

Every H4b tier-B positive was blocked at application binding with
"counted loop does not cover the complete collection extent". Root cause:
`sir_lower` declared every referenced global as `Array<u8, 256>` (hardcoded
in the globals scan), so any global whose declared extent differed was
unbindable — invisible in H3/D4 because that corpus used only 256-byte
globals.

Fix (commit `1da3b28`): the GEP source element type (`inbounds [N x T]`)
supplies the global parameter's real extent and element type; the
historical 256-byte fallback remains only for flat scalar-element pointer
GEPs. New regression test
`global_array_extent_follows_the_gep_source_type`
(sir_lower/tests/lowering_regressions.rs).

Evidence that the fix did not disturb the frozen corpora: the D4 regression
run reproduces `d4/raw/run2.txt` outcome-for-outcome (byte-identical row
lines), and H4b became a passing regression corpus.

## H4c (closure corpus) detail

Tier A (17 rows: 10 P / 5 N / 2 S), `h4c/raw/run1.txt` + `exec1.txt`:

- Positives all rewrote and were differentially clean over enumerated,
  boundary and deterministic-random patterns (32/14/14/14/24/6/6/6/6/24
  cases; 0 mismatches), covering extents 5–96, element widths
  u8/u16/u32/i16/i64/u64, lt/ge/le/eq/ne, param and literal scalars
  (`const:1000`, `const:-7`), accumulator slots 0/1, and
  field/TupleExtract/select consumers.
- Near-miss negatives refused with the intended gates: nonzero start,
  partial bound, reverse traversal (stride contract), whole-tuple return
  (unclassified use), two accumulator consumers.
- Safety rows refused at the S1 gate ("accumulator identity is not the
  recurrence's monoid identity") and the S2 gate ("predicate scalar
  depends on a loop-carried value (not invariant)").

Tier B (10 kernels: 3 P / 7 N), `h4c/raw/run1.txt` + `tbexec1.txt`:

- h4c01/h4c02/h4c03 (extents 96/160/24) rewrote; differential SIR
  execution 10/50/10 patterns, 0 mismatches.
- h4c04 pointer scan: 0 candidates. h4c05 reverse scan: refused at
  lowering (no panic). h4c06 partial bound over a 96-byte global: refused
  by the extent binding (the F10 counterpart). h4c07 two loops: refused at
  lowering. h4c08 signed comparison: refused at the recipe ("integer
  collection without a certified element predicate"). h4c09
  index-dependent predicate scalar (S2 class): refused at lowering.
  h4c10 store in loop: refused at lowering.
- `TOTAL_REWRITES=13` (10 tier A + 3 tier B); no PANIC lines in either
  mode.

## Protocol and provenance

- Authoring protocol (all corpora): rows and kernels written from the
  documented C3 dialect and the D4 record only; the optimizer was not
  executed on corpus rows during authoring; expectations recorded in
  `expected.csv` before the run; corpus hashed in `manifest.sha256` and
  committed before the one-shot run; raw output committed before
  inspection. See each corpus README.
- Apparatus: the frozen H3 harness
  (`sir/crates/sir_benchmarks/src/bin/h3_run.rs`, sha256
  `b4cef2de…`, unchanged since the D4 remediation), invoked with
  `ABSAC_CORPUS_DIR=h4|h4b|h4c`.
- Seals: `h4/manifest.sha256`, `h4b/manifest.sha256`,
  `h4c/manifest.sha256`. Archive commits: `2c22179`, `99b47ad`,
  `e6acd9e`, `a45cbcb`, `af12ec8`, `7ad3bda`, plus the F10 fix
  `1da3b28`.

## Scope honesty

- **SIR-level, `SchemaChecked`.** This closes the soundness of the
  per-rewrite application pipeline at the SIR level. It is not a
  machine-checked proof of the pipeline (that is what `sir_mech` /
  Gate 4B did for three kernels), not solver-backed application
  assurance, and not native/LLVM equivalence. Emitter defects F6–F8 were
  closed on 2026-09-17 (docs/EMITTER_F6F8_RESULTS.md); the P0A queue
  item 3 (`ConcreteSolverChecked` definitions) remains open.
- **Independence caveat.** The corpus author is the same agent that
  performed the D4/F10 remediation, so this is an
  implementation-blind, spec-driven, pre-registered evaluation — not a
  third-party audit. The operative blindness criterion (no probing of the
  fixed implementation during authoring; sealed expectations; one-shot
  run; raw archives committed before inspection) is satisfied and
  documented.
- **Not covered at H4 time:** All/Parity/Popcount binding
  generalization (closed 2026-09-17: all collection reduction recipes
  consume the authorized ProposalBinding — docs/IMMEDIATE_PROGRAM.md),
  solver-backed application equivalence, LLVM/native execution (F6–F8
  closed 2026-09-17 — docs/EMITTER_F6F8_RESULTS.md), performance claims.

## Consequences

- Gate 4B.1 is flipped to PASSED in `docs/IMMEDIATE_PROGRAM.md`; the
  H3/D4 "awaiting an independent H4" notes are annotated.
- Any's quarantine condition ("until an independent H4 corpus passes",
  docs/H3_RESULTS.md) is satisfied: the Any recipe returns to the trusted
  default registry (status unchanged: `SchemaChecked`).
- The blindness protocol demonstrated here (spec-driven authoring,
  pre-registered expectations, sealed manifest, one-shot run, raw
  archives committed before inspection) is reusable for Gate 6B's fusion
  corpus; Gate 6B itself remains reserved (it is about the fusion
  automation, not the Any slice).
- Verification after the quarantine lift: `cargo test --workspace`
  **591 passed / 0 failed / 3 ignored** (one more test than the Gate 4B
  baseline: the new F10 lowerer regression).
