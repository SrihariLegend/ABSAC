# C3 Freeze: narrow Any vertical slice

**Status:** freeze candidate after the hardening commit

## Scope frozen

C3 covers only the SIR-level, role-bound `Any` transformation:

```text
source SIR
  -> analyses
  -> semantic/structural reduction truth
  -> authorization
  -> ProposalBinding
  -> Any candidate
  -> SchemaChecked theorem
  -> SchemaChecked application
  -> matched EndToEndVerificationArtifact
  -> verified SIR rewrite
```

The freeze registry is `sir_rewrite::registry::any_only_registry()`. It
contains only definition `4` (`Any`). The exploratory `default_registry()`
remains available for legacy and non-C3 tests, but it is not a C3 acceptance
configuration.

All/Parity/Popcount are not part of this freeze. Their recipes and theorem
construction must consume the same `ProposalBinding` before they can enter a
later freeze.

> **Post-annotation (2026-09-17):** the recipe side of that condition is
> now met — All/Parity/Popcount (and both BitScan recipes) consume the
> authorized `ProposalBinding` through shared helpers, and the
> structural `emit_pack` with its hardcoded `Gt` is deleted (see
> `docs/IMMEDIATE_PROGRAM.md`). C3 remains an Any-only freeze; widening
> it is a separate decision covering candidates, theorem obligations,
> application checks and artifacts.

## Hardening gates in this freeze

The Any acceptance suite must continue to pass:

```bash
cargo test -p sir_optimizer --test any_vertical_slice_hardening
cargo test -p sir_optimizer --test any_vertical_slice_differential
cargo test -p sir_verification --test verifier_quarantine_tests
cargo test --workspace
cargo run -q -p sir_benchmarks --bin batch_run -- --any-only ../corpus/kernels.ll
cargo run -q -p sir_benchmarks --bin batch_run -- --any-only ../gate6a/v2_corpus.ll
git diff --check
```

The hardening suite refuses independent mutations of:

- collection and element access;
- predicate operator and scalar;
- induction start, bound, and stride;
- accumulator, identity, loop/result identity, and reduction position;
- classified live-out replacement site and transitive transparent-use closure evidence;
- candidate collection reads, undeclared/dangling inputs, and effects;
- source regions with a second live-out;
- authorization after the source function has been rewritten;
- exact counted-loop provenance (zero start, full extent, forward unit stride,
  comparison direction, integer semantics, and finite normal termination);
- theorem/application artifact source, candidate, definition,
  authorization, role-map, region, and assumptions identities.

The differential suite executes both the original and rewritten SIR for
lengths `0, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128`, including empty,
all-false, all-true, first-hit, last-hit, vector-boundary, alternating, and
deterministic pseudo-random inputs from four fixed seeds per length. Predicate
cases cover all six comparison operators and the same boundary/random protocol.

## Explicitly not frozen

This C3 freeze makes no claim about:

- LLVM translation or emitted machine code;
- solver-checked application assurance;
- vector lowering, AVX2 alignment, over-read, or tail safety;
- machine equivalence;
- performance versus `clang -O3`;
- All, Parity, or Popcount;
- a general Arena/MCTS implementation.

The current end-to-end assurance is the minimum of theorem and application
assurance and is `SchemaChecked` for this path. The executed boundary report
is [`C3_ANY_HARDENING_REPORT.md`](C3_ANY_HARDENING_REPORT.md); it records the
current zero corpus rewrite frontier and the unrun performance/H3 stages.

## Independent H3 handoff

H3 must be generated and sealed by an evaluator who did not author the Any
recipe or this acceptance test. The evaluator must receive the frozen
configuration and source corpus through a one-way handoff, preserve eligible
cases and near misses, and return a report with one row per input and these
boundary fields:

1. lowering: lowered / abstained + reason;
2. SIR verification: passed / failed + invariant;
3. semantic recognition: concept + structural role set;
4. authorization: issued / refused + authorization ID;
5. proposal binding: issued / refused + role-map and live-out digests;
6. candidate generation: proposed / refused + candidate ID;
7. theorem verification: proven / failed + checker-issued assurance;
8. application frame: passed / refused + frame reason;
9. matched artifact: constructed / refused + end-to-end assurance;
10. rewrite: committed / refused + source-version fingerprint;
11. differential execution: passed / failed + seed/case;
12. performance: calibrated timings, confidence interval, CPU pinning,
    assembly hash, and comparison to `clang -O3`.

Failures and abstentions are first-class H3 results. No H3 result may be
converted into a witness without correctness, artifact identity, and
source-version replay evidence.
