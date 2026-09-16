# Gate 6B — sealed multi-reduction fusion corpus

Status: **sealed before automation** (see `manifest.sha256`). This corpus
exists to close the two open halves of Gate 5B:

- **5B-Automation** — the system must generate the fused single-pass
  implementation from the primitive actions (per-loop semantic
  recognition + target-plan derivation + a shared-traversal composition),
  not from hand-written fused C.
- **5B-Search** — search over the composition action space must select the
  fusion (with a cost model that accounts for memory-traffic savings)
  where the deterministic per-region pipeline (Engine 0) does not.

## Blindness protocol

Per `docs/IMMEDIATE_PROGRAM.md` ("seal Gate 6B or independent post-freeze
corpus creation … must not be inspected while automating fusion"), this
corpus is authored and sealed *before* any fusion automation exists. The
authoring inputs were the documented Gate 5B challenge
(`docs/GATE5B_RESULTS.md`), the recognized reduction dialect of Gate 5A
(`sir/crates/sir_benchmarks/src/gate5a.rs`), and the kernel shapes of
`corpus/kernels.c` (k18 all-equal, k43 sum, k50 count-masked).

`expected.csv` records, per kernel, the pre-registered expectation
(`fuse` / `no_fuse`), the expected reduction set, and the minimum
fused-vs-3vec speedup. The one-shot evaluation, its harness, and its raw
output follow the H4 protocol: harness frozen and hashed before the run;
raw output committed before inspection.

Residual caveat (disclosed): the corpus author is the same agent that
implements the fusion; the operative criteria are the pre-registration,
the no-probing-before-seal discipline, and the one-shot evaluation.

## Inputs

- `corpus.c` — 10 kernels, two or three sequential counted loops each,
  every accumulator observable through the returned value.
- `corpus.ll` — compiled with the C3 freeze flags
  (`clang -O1 -emit-llvm -S`, clang 19.1.7, x86_64).

## Pass criteria (pre-registered)

P rows (5):

1. A fused plan covering every recognized reduction of the kernel is
   produced by the composition stage (no hand-written fused source).
2. Differential correctness: each fused per-reduction result equals an
   independent scalar reference; the combined reference equals the
   original kernel compiled by clang, over the pattern set
   {empty, n = 1, 31, 32, 33, 63, 64, 65, 256, 4096, 65536} ×
   {random, zeros, all-equal to the predicate value, alternating,
   single-hit at head/tail}.
3. Performance: median fused time ≤ 1/`min_speedup_vs_3vec` of the
   system-generated 3vec baseline (each recognized loop vectorized
   independently by the plan emitter and called in sequence) at
   n = 4096 and n = 65536, CPU-pinned, ≥15 trials, medians reported.

N rows (5):

1. No fused plan is produced; the refusal reason is recorded.
2. Independent per-loop vectorization may still occur; that is not a
   failure.

All rows:

1. No panics; emitted C compiles cleanly under the harness flags.
2. Any fused plan that is emitted must pass the differential suite —
   correctness failures are soundness failures, not coverage failures.

## One-shot result procedure (to be frozen at evaluation time)

```bash
# harness frozen + hashed first; then, once:
cargo run -q -p sir_benchmarks --bin gate6b_run -- \
  --corpus gate6b/corpus.ll --expectations gate6b/expected.csv \
  > gate6b/raw/run1.txt
```

The run is one-shot: a corpus/expectation defect is a recorded finding,
not an invitation to retune the corpus.
