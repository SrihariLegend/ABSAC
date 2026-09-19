# Gate 6A-v4 — fresh blind recognition corpus (validates the D5 remediation)

Status: **sealed before the run** (`v4_manifest.sha256`). Gate 6A-v3 found
one false positive (`n08_conditional_sum`: a masked accumulation accepted
as a raw-element sum). The D5 remediation requires the Sum reduction's
invariant to be the element itself through transparent conversion nodes.

v4 is a *new* generation: fresh kernels that re-probe the masked-sum class
in four forms (mask, ternary, if-form, shift, scale), the standing negative
classes, and a fresh positive set. The v3 corpus itself becomes the D5
regression set and is not reused as held-out proof.

## Corpus

24 kernels (`v4_corpus.c`, `clang -O1 -emit-llvm -S`):

- 12 positives: Sum (u8→u64, u32→u64, u16→u32), Cardinality
  (u16 ==, u32 <=, u8 >, do-while u32), All (u16 !=, u8 ==),
  Disjunctive (u16 raw OR), plus two pre-registered known-gap probes
  (map-then-sum, two-loop).
- 12 negatives: five fresh masked/conditional sum variants (D5 class),
  a signed-nsw sum (contained abstention: recognition allowed, no
  authorization), running max, two-array relation, may-alias store,
  volatile loads, stride 3, atomics.

## Pass criteria (pre-registered)

1. `safe_abstain` negatives: zero reduction concepts, zero candidates,
   zero rewrites.
2. `contained_abstain` negatives (signed overflow class): recognition may
   fire — the concept is true — but **zero** candidates and **zero**
   rewrites (authorization containment, the D3 property).
3. Positives not marked `recall_gap_known`: lower, verify, and be
   recognized with the pre-registered concept.
4. Rows marked `recall_gap_known` are recorded as known misses; a
   recognition there is reported as an improvement.

Verdict PASS requires criteria 1–3.

## Run (one-shot)

```bash
cargo run -q -p sir_benchmarks --bin gate6a_run -- \
  --corpus gate6a/v4_corpus.ll --expectations gate6a/v4_expected.csv \
  > gate6a/v4_raw/run1.txt
```
