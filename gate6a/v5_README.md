# Gate 6A-v5 — fresh blind recognition corpus (D5 validation frontier)

Status: **sealed before the run** (`v5_manifest.sha256`).

History of the 6A-v3 cycle:

- v3 (sealed, one-shot): **FAILED** — one real false positive
  (`n08_conditional_sum`: `if (buf[i] & 1) s += buf[i];` was accepted as a
  raw-element sum).
- D5 remediation: SumReduction requires the invariant to be the element
  itself through transparent conversion nodes. v3 becomes the D5
  regression set (0 false positives after the fix).
- v4 (sealed, one-shot): **FAILED as a corpus-classification defect** —
  v4's `n03_sum_if_ne` guard `x != 0 ? x : 0` is semantically redundant
  (`= x`), so clang canonicalizes the loop to a plain sum and
  `SumReduction` was *correct*; the pre-registered expectation was wrong.
  The other four D5-class rows refused correctly.
- v5 (this corpus): fresh kernels with **non-vacuous** conditional/masked
  guards, plus fresh positives and standing negative classes.

## Corpus

24 kernels (`v5_corpus.c`, `clang -O1 -emit-llvm -S`): 12 positives
(Sum/Cardinality/All/Disjunctive across u8/u16/u32, constant bound,
do-while; two known-gap probes) and 12 negatives (masked-guard sum,
threshold ternary, high-nibble mask, index-conditional sum, contained
signed-nsw sum, running max, two-array relation, may-alias store, volatile
loads, stride 4, atomics, self-recurrence).

## Pass criteria (pre-registered)

1. `safe_abstain` negatives: zero reduction concepts, zero candidates,
   zero rewrites.
2. `contained_abstain` negatives: recognition allowed; zero candidates and
   zero rewrites.
3. Positives not marked `recall_gap_known`: lower, verify, recognized.
4. `recall_gap_known` rows are recorded; recognition there is an
   improvement.

Verdict PASS requires criteria 1–3.

## Run (one-shot)

```bash
cargo run -q -p sir_benchmarks --bin gate6a_run -- \
  --corpus gate6a/v5_corpus.ll --expectations gate6a/v5_expected.csv \
  > gate6a/v5_raw/run1.txt
```
