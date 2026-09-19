# H4 run 1 — corpus-construction failure (archived)

`run1.txt` was captured and committed (`99b47ad`) before inspection.
The run exited 101: fixture construction panicked at
`h3_run.rs:556:78` on row `h4a25` (`pred` + `unstable_bound`, integer
element used as a `select` condition — `TypeMismatch { expected: Bool,
actual: I32 }`). The frozen H3 harness fixture grammar admits
`unstable_bound` and `two_reductions` only for `bool` rows; `h4a25` and
`h4a27` were outside that grammar.

Consequences:
- Rows `h4a01`–`h4a24` completed before the abort (positives rewrote,
  negatives abstained, all as pre-registered).
- Rows `h4a25`–`h4a35` and all tier B kernels did not run.
- H4 is recorded as a corpus-construction failure. The corrected corpus
  is `h4b/` (see `h4b/README.md`); no optimizer-behavior observation was
  used to change any expectation.
