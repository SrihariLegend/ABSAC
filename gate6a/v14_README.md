# Gate 6A-v14 — search role-binding native corpus

**Status:** frozen before evaluation (classification in
`v14_expected.csv`, hashes in `v14_manifest.sha256`). Raw output goes to
`v14_raw/` and `emitter_native/diff_gate6a_v14.txt`.

## Frontier

Created 2026-09-18 after the PositionSearch role fixes. v13 covers merged
arithmetic/mask occurrence recovery but contains no array searches, so the
new body-derived collection/scalar binding had no native corpus coverage.
v14 blind-tests:

- `s01`: a zero search over the first array while a second array of the
  same promoted extent is also accessed. The old role scan bound the last
  array parameter; the search must bind the first.
- `s02`: an equality search with a trailing extra integer parameter. The
  old role scan bound the last integer parameter; the search must bind the
  compared key.

Both functions are constant-extent (64), so a mis-bound rewrite would be
authorized and natively observable rather than failing closed.

## Evaluation

```bash
gate6a_run --corpus gate6a/v14_corpus.ll --expectations gate6a/v14_expected.csv
emit_c_diff gate6a/v14_corpus.ll --cases 48
```

## Results (2026-09-18)

- `gate6a_run`: positives 2/2 recognized, recall misses 0; `GATE6A_VERDICT PASS`.
- `emit_c_diff`: 2 clean / 0 mismatched / 0 lower-refused, one rewrite each.

**Discriminating power:** re-running the corpus with the old function-wide role
selection did not produce a native mismatch. `s01` abstained (0 rewrites)
instead of mis-binding the second array, and `s02` still rewrote correctly
because its unused trailing parameter is not materialized by the lowerer, so
the last materialized integer parameter was already the key. The red-verified
evidence for the role fixes remains the role-level and rewritten-graph
regressions in `sir_semantics/tests/scan_role_binding.rs` and
`sir_optimizer`/native suites; v14 is fresh native coverage of the fixed
behavior, not a corpus that independently reproduced the defect.

## Profile note (2026-09-18)

The "one rewrite each" result is the default neutral policy. Under the bundled
conservative measured profile (`emit_c_diff --cost-profile conservative`),
both constant-extent array searches are rejected (0 rewrites) because the
measured array-bitscan survey found up to ~87x regressions on early-exit data;
recognition and differential correctness are unchanged. The frozen
classification above is not rewritten.
