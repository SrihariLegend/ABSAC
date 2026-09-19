# H4 — blind evaluation corpus for the remediated Gate 4B.1 pipeline

Run: `h4-blind-1`. Status: **corpus sealed before the run** (see
`manifest.sha256`). This corpus exists to close Gate 4B.1, whose soundness
failed in the sealed H3 evaluation (`docs/H3_RESULTS.md`) and whose
registered defects were remediated in D4 (`d4/README.md`). H3 cannot be
reused as blind evidence: its authoring required stage-level probing of the
frozen implementation. H4 is authored under a no-probing protocol.

## Authoring protocol (blindness)

1. Rows and kernels were written from the *documented* C3 input dialect
   (`h3/README.md`, `docs/C3_FREEZE_ANY.md`) and the D4 remediation record
   (`d4/README.md`) only. The optimizer was NOT executed on any H4 row
   during authoring; no stage-level probing of the implementation occurred.
2. `expected.csv` records the expected outcome and safety class for every
   row *before* the run.
3. The corpus and expectations are hashed in `manifest.sha256` and
   committed before the one-shot run; raw run/execution output is archived
   under `raw/` and committed before inspection.
4. The evaluation apparatus is the frozen H3 harness
   (`sir/crates/sir_benchmarks/src/bin/h3_run.rs`, unchanged since the D4
   remediation) invoked with `ABSAC_CORPUS_DIR=h4`.

Residual independence caveat (disclosed): the corpus author is the same
agent that performed the D4 remediation work, so this is an
implementation-blind, spec-driven corpus, not a third-party audit. The
no-probing and pre-registration discipline is the operative criterion
defined by H3's correction notice.

## Tier A (SIR fixtures): `tier_a.tsv`

35 rows: 17 eligible positives (P), 14 near-miss negatives (N), 4 safety
rows (S1/S2). Positives exercise unseen combinations of the documented
dialect: extents 2/7/9/12/20/24/31/33/64/100/128, element widths
u8/u16/u32/u64/i8/i32/i64, all six comparison operators, parameter and
literal scalars, accumulator slots 0/1, and field/TupleExtract/select
consumers. Negatives re-test each registered near-miss class on fresh
parameters: nonzero starts, partial bounds, reverse traversals, unstable
bounds, two reductions, whole-tuple returns, duplicate accumulator
consumers, index-derived predicate scalars (S2), and identity=true
reductions (S1). `h4a17` re-tests the D4 dead-projection classification fix.

Expected outcomes are recorded in `expected.csv`. A positive that does not
commit is a coverage failure; a negative or safety row that commits is a
soundness failure; any panic is a robustness failure.

## Tier B (LLVM sources): `tier_b.c`, `tier_b.ll`

13 kernels compiled with the C3 freeze flags
(`clang -O1 -emit-llvm -S`, clang 19.1.7, x86_64). Positives h4b01–h4b04
are fixed-extent extern-global scans (clang eq-next rotation) at extents
32/64/128 with raw-or and predicate forms. Negatives cover the documented
refusal classes: pointer scan, volatile load, store in loop, two loops,
reverse scan (must not panic), early exit, out-parameter index, signed
comparison, and an index-dependent predicate scalar (S2 class).

## Result procedure

```bash
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4 \
  cargo run -q -p sir_benchmarks --bin h3_run -- run > h4/raw/run1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4 \
  cargo run -q -p sir_benchmarks --bin h3_run -- exec > h4/raw/exec1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4 \
  cargo run -q -p sir_benchmarks --bin h3_run -- tbexec > h4/raw/tbexec1.txt
```

The run is one-shot: a corpus/expectation defect is a finding recorded
against H4, not an invitation to retune the corpus. If remediation is
required, a new sealed corpus (H4b) must be authored.
