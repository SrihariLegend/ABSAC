# C3 Freeze Record — narrow Any vertical slice

Freeze date: 2026-09-02 (run `h3-c3-freeze`). Frozen commit and tag:

| Field | Value |
|-------|-------|
| Tag | `c3-freeze-any` (annotated) |
| Commit | `49999a4 1932535063d21b87916cb05031f121653` — "C3 freeze + hardening: Any-only registry, binding/frame gates, lowerer defect fixes" |
| Commit time | 2026-09-02 14:52:43 +0800 |
| Workspace | `/home/tom/dev/experiments/ABSAC` (library-only cargo workspace under `sir/`) |
| Test state | all test blocks green at freeze time (99 `test result: ok` blocks, exit 0) |
| Cargo.lock | sha256 `88aab1eab45d1777…` (`sir/Cargo.lock`) |
| rustc | `1.96.0 (ac68faa20 2026-05-25)` |
| clang (corpus generation) | Debian clang 19.1.7 (3+b1), target `x86_64` |
| Host arch | x86_64 |
| Verifier policy | `Verifier::new()` defaults at the frozen commit: minimum issued assurance `SchemaChecked` (research mode, honest labeling), symbolic-first then exhaustive backend, `VerificationLimits { max_states: 1_048_576 }` |
| Registry | `sir_rewrite::registry::any_only_registry()` — DefinitionId 4 (`Any`) only; exploratory `default_registry()` NOT used by C3 |
| Authorization | immutable `AuthorizationDatabase` with production ids; optimizer config `allow_unit_test_authorizations: false` |
| Optimizer config | `OptimizerConfig::default()`: max_iterations 10, beam width 3 |
| Build flags | `cargo build`/`cargo test` (debug); corpus C→LLVM: `clang -O1 -emit-llvm -S` |
| H3 harness | `sir/crates/sir_benchmarks/src/bin/h3_run.rs` (harness, not part of the frozen crates) |

Scope (frozen): the SIR-level role-bound `Any` transformation over typed-array collections —
lowering `.ll → SIR` is in scope, but LLVM/native execution equivalence and machine-level
claims are explicitly NOT claimed by this freeze. See `docs/C3_FREEZE_ANY.md` for the full
scope statement and `docs/C3_ANY_HARDENING_REPORT.md` for the hardening record.

## Corpus hashes at freeze

Prior corpora (hash of the sealed `.ll`/`.c` artifacts, sha256 first 16 hex):

| Corpus | sha256 (truncated) |
|--------|--------------------|
| `corpus/kernels.ll` | see `gate6a` records |
| `gate6a/v1_corpus.ll` | recorded in `gate6a/archive_v1.sh` |
| `gate6a/v2_corpus.ll` | recorded in `gate6a/archive_v2.sh` |
| `gate6a/heldout_corpus.ll` | recorded in `h3/` era archives |

H3 corpus hashes are sealed in `h3/manifest.sha256`.

## Freeze discipline

Any subsequent modification to the frozen crates (`sir_types`, `sir_nodes`, `sir_analysis`,
`sir_semantics`, `sir_inference`, `sir_transform`, `sir_builder`, `sir_printer`,
`sir_verify`, `sir_generation`, `sir_verification`, `sir_selection`, `sir_rewrite`,
`sir_optimizer`) = C4 and invalidates this record. Harness tooling under
`sir/crates/sir_benchmarks/src/bin/` and corpus artifacts under `h3/` are evaluation
apparatus, not frozen implementation.

## Findings promoted from the H3 engineering phase (pre-sealed evidence)

Recorded here so they survive the sealed run. Full reproductions and the official
archive are under `h3/`.

- **S1 — identity=true OR-reduction rewrites incorrectly.** A boolean/predicate any loop
  whose accumulator identity is the constant `true` (result is constant `true`) is
  rewritten to `pack/mask != 0`. The all-false input is corrupted: original `true`,
  rewritten `false`. Rows `h3a23`, `h3a24`.
- **S2 — induction-derived predicate scalar rewrites incorrectly.** A predicate whose
  scalar is derived from the induction counter (`values[i] > i`) is rewritten against a
  fixed scalar mask. Witness found over a small value domain (`orig=false rewritten=true`
  for `[0,1,2,1,2,1,4,1]`). Row `h3a21`.
- **R1 — reverse (descending) loops crash the constants analysis.** A reverse scan
  panics in `sir_analysis/src/constants.rs` (`attempt to add with overflow`) instead of
  abstaining. Row `h3b09`.

These are D4 promotion candidates and must not be remediated under this freeze.
