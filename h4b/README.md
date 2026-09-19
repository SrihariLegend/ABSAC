# H4b — corrected blind evaluation corpus (Gate 4B.1 closure run)

Run: `h4b-blind-1`. The H4 attempt (`h4/`) was sealed and run first; its raw
output is archived at `h4/raw/run1.txt` (committed before inspection). That
run aborted in *fixture construction* (not in the optimizer) on row h4a25:
the frozen H3 harness fixture grammar admits `unstable_bound` and
`two_reductions` only for `bool` rows (h3_run.rs line 556: `select`
condition must be `Bool`). H4 is therefore recorded as a
corpus-construction failure, and H4b is a newly sealed corpus with the two
invalid rows corrected:

- `h4a25`: was `pred u32 eq param ... unstable_bound` (outside the fixture
  grammar) → now `bool extent 32 acc slot 1 unstable_bound`.
- `h4a27`: was `pred u8 gt param ... two_reductions` (outside the fixture
  grammar) → now `pred u16 le param ... select partial_bound`.

All other tier A rows and all tier B kernels are unchanged from H4. No row
was altered in response to optimizer behavior; the H4 run1 archive shows
rows h4a01–h4a24 completed before the abort, and those observations did not
change any expectation.

## Authoring protocol (blindness)

1. Rows and kernels were written from the *documented* C3 input dialect
   (`h3/README.md`, `docs/C3_FREEZE_ANY.md`) and the D4 remediation record
   (`d4/README.md`). The optimizer was NOT executed on H4/H4b rows during
   authoring; no stage-level probing of the implementation occurred.
2. `expected.csv` records the expected outcome and safety class for every
   row before the run.
3. The corpus and expectations are hashed in `manifest.sha256` and
   committed before the one-shot run; raw run/execution output is archived
   under `raw/` and committed before inspection.
4. The evaluation apparatus is the frozen H3 harness
   (`sir/crates/sir_benchmarks/src/bin/h3_run.rs`, unchanged since the D4
   remediation) invoked with `ABSAC_CORPUS_DIR=h4b`.

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

A positive that does not commit is a coverage failure; a negative or safety
row that commits is a soundness failure; any panic is a robustness failure.

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
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4b \
  cargo run -q -p sir_benchmarks --bin h3_run -- run > h4b/raw/run1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4b \
  cargo run -q -p sir_benchmarks --bin h3_run -- exec > h4b/raw/exec1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4b \
  cargo run -q -p sir_benchmarks --bin h3_run -- tbexec > h4b/raw/tbexec1.txt
```

The run is one-shot: a corpus/expectation defect is a finding recorded
against H4b, not an invitation to retune the corpus.
