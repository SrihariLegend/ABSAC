# H4c — fresh blind evaluation corpus (Gate 4B.1 closure run)

Run: `h4c-blind-1`. This is the third corpus in the H4 series:

- `h4/` — first attempt; aborted in fixture construction (corpus defect,
  archived).
- `h4b/` — corrected corpus; blind run passed the SIR tier fully and found
  a real tier-B defect: `sir_lower` hardcoded every global array to
  `Array<u8, 256>` (F10), so non-256-byte globals were unbindable. Fixed in
  commit `1da3b28`; H4b is retained as a regression corpus (its outcomes
  after the fix: 4/4 tier-B positives rewrite, negatives unchanged).
- `h4c/` (this corpus) — a *fresh* row and kernel set sealed at the F10
  implementation state, so the closure evidence is not a re-run of the
  corpus that exposed F10.

## Authoring protocol (blindness)

1. Rows and kernels were written from the documented C3 input dialect
   (`h3/README.md`, `docs/C3_FREEZE_ANY.md`) and the D4 remediation record
   (`d4/README.md`) only. The optimizer was not executed on any H4c row
   during authoring; no stage-level probing of the implementation occurred.
2. `expected.csv` records the expected outcome and safety class for every
   row before the run.
3. The corpus and expectations are hashed in `manifest.sha256` and
   committed before the one-shot run; raw run/execution output is archived
   under `raw/` and committed before inspection.
4. The evaluation apparatus is the frozen H3 harness
   (`sir/crates/sir_benchmarks/src/bin/h3_run.rs`, unchanged since the D4
   remediation) invoked with `ABSAC_CORPUS_DIR=h4c`.

Residual independence caveat (disclosed): the corpus author is the same
agent that performed the D4/F10 remediation work, so this is an
implementation-blind, spec-driven corpus, not a third-party audit. The
no-probing and pre-registration discipline is the operative criterion
defined by H3's correction notice.

## Tier A (SIR fixtures): `tier_a.tsv`

17 rows: 10 eligible positives (P), 5 near-miss negatives (N), 2 safety
rows (S1/S2). Positives use fresh combinations: extents
5/6/7/13/15/17/40/48/63/96, element widths u8/u16/u32/i16/i64/u64,
comparisons lt/ge/le/eq/ne, parameter and literal scalars (including
`const:1000` and `const:-7`), accumulator slots 0/1, and
field/TupleExtract/select consumers. Negatives re-test nonzero start,
partial bound, reverse traversal, whole-tuple return, and duplicate
accumulator consumption; safety rows re-test S1 (identity=true) and S2
(index-derived predicate scalar).

## Tier B (LLVM sources): `tier_b.c`, `tier_b.ll`

10 kernels compiled with the C3 freeze flags
(`clang -O1 -emit-llvm -S`, clang 19.1.7, x86_64). Positives h4c01–h4c03
are fixed-extent u8 global scans at extents 96/160/24 (eq-next rotation).
Negatives cover pointer scan, reverse scan (must not panic), a partial
bound over a 96-byte global (exercises the new F10 extent binding),
two loops of different extents, signed comparison, an index-dependent
predicate scalar (S2 class), and a store inside the loop.

## Result procedure

```bash
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4c \
  cargo run -q -p sir_benchmarks --bin h3_run -- run > h4c/raw/run1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4c \
  cargo run -q -p sir_benchmarks --bin h3_run -- exec > h4c/raw/exec1.txt
ABSAC_CORPUS_DIR=/home/tom/dev/experiments/ABSAC/h4c \
  cargo run -q -p sir_benchmarks --bin h3_run -- tbexec > h4c/raw/tbexec1.txt
```

The run is one-shot: a corpus/expectation defect is a recordable finding,
not an invitation to retune the corpus.
