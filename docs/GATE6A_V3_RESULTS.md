# Gate 6A-v3 — fresh blind recognition evaluation and closure

**Status (2026-09-17): the Gate 6A-v3 cycle is closed.** The sealed v3
corpus found one real false positive; the D5 remediation fixed it; the v3
corpus became the D5 regression set; a corpus-classification defect in the
first validation generation (v4) was identified; and the fresh v5 corpus
passed with **zero false-positive recognitions, zero unsafe candidates,
zero unsafe rewrites**, and all non-known-gap positives recognized.

## Evidence chain

| Artifact | Commit | Result |
|---|---|---|
| v3 corpus + harness sealed (`gate6a/v3_*`) | `c738167` | pre-registered |
| v3 one-shot raw run | `689004f` (`v3_raw/run1.txt`) | **FAIL: 1 false positive** |
| D5 remediation + regression tests | `f631202` | Sum strictness |
| v3 re-run as D5 regression set | — | 0 false positives |
| v4 corpus sealed + raw run | `0760956`, `0f3660b` | **FAIL: corpus-classification defect** |
| v5 corpus sealed + raw run | `cdfbba1`, `3258662` | **PASS (10/12 positives, 0 safety violations)** |

## v3 sealed run (H3-recog generation)

24 fresh kernels; 12 positives, 12 metamorphic negatives covering the H2
failure classes plus the standing abstention classes.

- Positives: **10/12 recognized**; the two pre-registered known-gap probes
  missed as expected (map-then-sum, two-loop lowering).
- Negatives: **11/12 safe abstentions**, 0 candidates, 0 rewrites — but
  one **false-positive recognition**:

> `n08_conditional_sum`: `if (buf[i] & 1) s += buf[i];` was recognized as
> `SumReduction`. The loop lowers to `s += select(cond, zext(buf[i]), 0)`;
> the recognizer classified the select result as a raw element value. A
> naive vector plan would compute `sum(buf[i])` — a live semantic
> corruption.

Containment held below recognition (the candidate layer produced nothing),
but recognition is the gate's safety surface, so v3 is a **failure** and
the corpus is promoted to regression set **D5**.

## D5 remediation (`f631202`)

`SumReduction` now requires the accumulator's invariant to be the element
itself through transparent conversion nodes (`Convert` chains ending in an
`ArrayAccess`/`Load`). Masked, conditional, shifted, or scaled values no
longer qualify. Regression tests:

- `conditional_sum_is_not_recognized_as_raw_element_sum` — the v3 shape
  yields zero SumReduction truths;
- `plain_element_sum_is_still_recognized` — the control shape still
  recognizes.

Post-fix regressions: v3 corpus → 0 false positives; v2 (H2) corpus → 0
false positives; `k43_sum_ascii` plan derivation unchanged; full workspace
suite green.

## v4 — the classification defect (recorded, preserved)

The first validation generation (v4, fresh kernels) failed on
`n03_sum_if_ne`: `if (buf[i] != 0) s += buf[i];`. Inspection of the
lowered IR shows clang canonicalizes the guard away
(`x != 0 ? x : 0 ≡ x`), so the kernel *is* a plain element sum and
`SumReduction` is **correct**. The pre-registered expectation was wrong:
the metamorphic difference was semantically vacuous. The other four
D5-class negatives (mask guard, threshold ternary, shift, scale) all
refused correctly, and the contained signed-nsw row stayed contained.

v4 is therefore recorded as a **corpus-classification defect**, not a
pipeline safety failure; its raw run is preserved. The lesson (adopted in
v5): classification must be justified against the *lowered* semantics of
the guard, not its source syntax.

## v5 — fresh validation frontier (PASS)

24 new kernels; every conditional/masked guard is non-vacuous.

- Positives: **10/12 recognized** — Sum (u8→u32, u32→u32), Cardinality
  (u32 `>=`, u16 `<`, u8 `==` constant bound, do-while u16 `!=`), All
  (u16 `==`, u8 `!=`, u16 `>=`), Disjunctive (u32 raw OR). The two known
  gaps (map-then-sum, two-loop) missed as pre-registered.
- Negatives: **12/12** — 0 false-positive recognitions, 0 unsafe
  candidates, 0 unsafe rewrites. The D5-class rows (mask guard, threshold
  ternary, high-nibble mask, index-conditional sum) all refused; running
  max, two-array relation, may-alias store, volatile loads, stride 4,
  atomics, and self-recurrence all abstained.
- The `contained_abstain` row (signed nsw sum) shows the intended
  two-layer behaviour: the concept *is* recognized (it is a true sum) and
  **zero candidates and zero rewrites** are produced — overflow semantics
  still gate authorization.

`GATE6A_SUMMARY positives=12 recognized=10 recall_misses=0
known_gap_misses=2`; `GATE6A_SAFETY negatives=12 contained_rows=1
false_positive_recognitions=0 unsafe_candidates=0 unsafe_rewrites=0`;
`GATE6A_VERDICT PASS`.

## Generational history (recognition line)

```
Generation  Negatives reaching recognition   False positives
H0          6                                3   (50%)
H1          4                                1   (25%)
H2          7                                1   (~14%)
v3 (H3r)    12                               1   (masked-sum class; fixed by D5)
v4          —                                corpus-classification defect (guard vacuous)
v5 (H4r)    12                               0
```

Sample sizes remain too small for statistical claims; the defensible
statement is qualitative: the D5 rule generalizes to four fresh
masked/conditional forms and does not regress the recognized positives,
and no new false-positive class appeared on the v5 frontier.

## Protocol and provenance

- v3, v4 and v5 corpora were classified and sealed (SHA-256 manifests)
  before their one-shot runs; raw outputs were committed before
  inspection (`689004f`, `0f3660b`, `3258662`). The v3 harness
  (`gate6a_v3_run`) is frozen and hashed in `gate6a/v3_manifest.sha256`;
  v4/v5 use the generation-agnostic `gate6a_run` (hashed in their
  manifests).
- Independence caveat (disclosed): the corpora are authored by the same
  agent that implemented the remediation; the operative criteria are
  pre-registration, sealing, the frozen harness, and the one-shot runs.
- Performance and rewrite-assurance claims are out of scope for this
  gate: it evaluates *recognition* generalization (false positives,
  abstention, positive recall) exactly as the H0–H2 gates did.

## Consequences

- Gate 6A-v3 is closed through remediation D5 and the v5 validation; the
  gate table records v3 FAILED (preserved), D5, and v5 PASSED, with no
  open 6A gate remaining.
- The masked-sum false-positive class is now impossible at recognition
  (D5) and the `contained_abstain` expectation is part of the harness for
  future generations.
- Recall gaps unchanged and still queued: map-then-sum, two-loop
  lowering; the v3/v4/v5 corpora are regression sets for the next
  generation.

## Post-remediation re-measurement (2026-09-17)

The frozen v5 one-shot result (10/12 recognized, 2 pre-registered
known gaps) is preserved. With the current code:

- **v5: 11/12 positives recognized, 0 false positives, 0 unsafe
  candidates, 0 unsafe rewrites — VERDICT PASS.** `p12_two_loops`
  (known frontend gap) now lowers, verifies and recognizes
  (`SumReduction` + `ConjunctiveReduction` + `PredicateMap` truths).
  The remaining known gap is `p11_map_then_sum`, which now derives
  `MappedSumReduction` (the D5-safe concept) but has no
  candidate/recipe yet and is scored against the frozen
  `SumReduction` expectation.
- **v3: 11/12 recognized** (was 10/12), 0 false-positive
  recognitions, 0 unsafe candidates/rewrites — PASS.
- Both re-measurements are remediation runs on frozen regression
  sets, not new generations.
