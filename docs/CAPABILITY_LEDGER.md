# Capability Ledger — old vs current

Source of truth for what the pruning removed. The old tree is at
`git HEAD` (`2cae1fd`), extracted at `/tmp/old_sir/sir`, and cloned as a
writable reference at `/tmp/absac_head_clone` (a real worktree is
impossible because `.git` is read-only). Recover any file with
`git show HEAD:<path>`.

Important: the session’s final old tree (~92.9k LOC) was **never
committed**. `HEAD` is an earlier commit (~76.9k LOC). Uncommitted
session additions — exact sum/memchr definitions and recipes, measured
profile tables, later gate fixes — are not recoverable as source;
their high-value algorithms were ported into `sir_opt` before deletion,
and their corpora/evidence remain in the working tree.

Rule going forward: **delete a capability only when the new architecture
has an equivalent or better path, or when it is measured-neutral and its
recovery path is recorded here.**

## Crates

| old crate | status | recovery / replacement |
|---|---|---|
| `sir_types` | kept | current |
| `sir_nodes` | kept | current |
| `sir_builder` | kept | current |
| `sir_verify` | kept | current; post-patch gate restored |
| `sir_analysis` | reduced | loops/graph kept; 11 dead analyses dropped |
| `sir_mech` | kept | BV/SAT/term; LCF kernel + QF integer linear arithmetic restored from `HEAD`; kernel-model bug fixed |
| `sir_lower` | kept | current |
| `sir_semantics` | **missing** | concepts/roles/certificates/binding/authorization; port per family |
| `sir_transform` | **missing** | contexts/roles/constraints/structures; re-express in `sir_opt::plan` |
| `sir_generation` | **missing** | candidate/strategy layer; replaced by `Family::propose` |
| `sir_verification` | **missing** | theorem/obligation/backends/artifacts; restore per family + z3 |
| `sir_rewrite` | **missing** | engine/frames/artifacts/recipes; patch engine replaced, gates to restore |
| `sir_optimizer` | **missing** | driver replaced; profile/objective/budgets/measured retention to restore |
| `sir_selection` | removed | needs return for global search/cost ranking |
| `sir_inference` | removed | contexts simplified; re-add only if evidence needs it |
| `sir_printer` | removed | diagnostics; optional |
| `sir_tests` | removed | legacy cross-crate tests |
| `sir_gate4b` | removed | Gate 4B checker; evidence in `docs/GATE4B_PROOF.md` |

## Families

| definition | family | current status |
|---|---|---|
| 200–203 | bitscan/scan | 200 and 402 shapes ported; reverse/zero-count/scalar variants missing |
| 400/401 | sums/mapped sums | fixed-extent, multi-loop, and conditional/masked maps ported; folded-offset/runtime shapes missing |
| 402 | byte search | equality/u8 fixed-extent ported; runtime bounds missing |
| 0 | popcount/cardinality | comparison-predicate count ported; bool-pack/Kernighan path missing |
| 4 | Any | integer OR reduction ported; bool-pack path missing |
| 5 | All | integer AND-chain ported; bool-pack path missing |
| 6 | parity | **restored (XOR-reduction path)** as `sir_opt::parity`; `HEAD` bitset/pack/popcount path still to mine |
| 7 | Kernighan popcount | **restored for widths ≤16** as `sir_opt::popcount`; 32/64 await the kernel induction proof |
| 100–103 | arithmetic | **restored** as `sir_opt::arithmetic` (divide→shift, modulo→mask, multiply→shift, shift-mask) |
| 300–303 | mask algebra | dropped as measured-neutral |
| 310–313 | permutations | **missing** |
| set algebra | — | **missing** |

## Assurance and tooling

| capability | old | current |
|---|---|---|
| structural verifier | `sir_verify`, post-rewrite | kept; post-patch gate restored |
| family obligations/theorems | definitions + obligation DB | missing; `Family` proof hook to land |
| concrete/symbolic/exhaustive backends | verification crates | BV/SAT + law checks + restored LCF kernel with `replay`, induction, and QF integer linear arithmetic |
| z3 differential | solver/family level | kernel random differential restored; family level missing |
| application gates | authorization/occurrence/live-out/frame/base | simplified shape checks only |
| measured profiles/retention | optimizer + timing harness | missing; rewrites unconditional |
| timing/differential harness | rich `emit_c_diff` modes | differential + sanitize + basic time |
| wasm differential | `wasm_diff` | removed (recoverable) |
| AArch64 differential | toolchain installed | harness never written |
| external corpora | planned/fetched in parts | not yet active |
| proof records/replay | partial | LCF kernel `replay` + kernel-induction proof restored; not yet wired into family obligations |

## Recovery plan (mine `HEAD` selectively)

Rules:

- Mine `HEAD` for semantics, recognizers, definitions, recipes, and
  tests; do not resurrect the old framework or copy whole crates.
- Port one capability at a time into `sir_opt` `Family` modules on the
  lean core. Each port carries its own proof, z3 differential, gate,
  tests, and measurement.
- No deletion without a replacement, a measurement, or a ledger entry.
- Back up and commit before and after every milestone.

Milestones:

| # | milestone | exit criteria |
|---|---|---|
| M0 | **Safety** | writable `.git` or external mirror; current and `HEAD` both committed; backups verified |
| M1 | **Measurement** | timing/retention tool restored outside core; 14-corpus + callgrind baselines reproducible; rewrite counts recorded |
| M2 | **Assurance** | family-level z3 differential + plan-bound gates (authorization/occurrence, live-outs, frames, memory bases); every supported family has an obligation and fails closed |
| M3 | **Top-value families** | Kernighan bitset popcount, runtime-bounded search/memchr, parity; each has proof + z3 + corpus/fuzz + measured win |
| M4 | **Breadth families** | arithmetic 100–103, permutations 310–313, bitscan variants; masks 300–303 only if measurement justifies; 15+ definitions with proofs and measurements |
| M5 | **Evidence and targets** | external corpora with manifests/held-out splits; `aarch64_diff`; wasm differential rebuilt; independent reproduction |
| M6 | **Research** | array/quantifier theory + proved `memchr`; global interaction-aware optimizer; predictive cost model + autotuning + runtime dispatch (ROADMAP pillars A–G) |

Acceptance for any port:

- zero mismatches, differential and sanitizers clean, zero warnings;
- a small `Family` module (roughly 100–400 LOC, not a framework);
- a machine-checked obligation plus an independent z3 cross-check;
- a measured speedup, or an explicit measured-neutral justification;
- a ledger entry that names the `HEAD` source files mined.

Non-goals:

- no wholesale `git checkout` of old crates;
- no pruning before replacement/measurement;
- no LOC target (leanness is a constraint, not a number);
- no claim that `HEAD` is simply “better”: session-final old was ~92.9k
  uncommitted, `HEAD` is 76.9k with 47 warnings and mismatches on
  v12–v16 where current is clean.

## HEAD reference comparison (2026-09-19)

| metric | old `HEAD` (`2cae1fd`) | current |
|---|---:|---:|
| Rust LOC | 76,919 | ~15.9k + restored kernel/arith |
| crates / binaries | 19 / 23 | 9 / 1 |
| build warnings | 47 | 0 |
| verification definitions | 24 | 13 |
| rewrite recipes | 21 | 8 families |
| semantics recognizers | 23 | 5 family modules |
| test files / test LOC | 63 / 15,217 | **9 tests**, case bodies under `tests/cases/` |
| `sir_mech` proof kernel | LCF kernel + `arith.rs` + induction proof; 38 tests pass | restored; 28 proof cases run under 5 `sir_mech` tests |
| corpora known | v1–v11 | v2–v14, v16 (v12–v20 added in session) |
| corpus result | v1–v11: 0 mismatched | 14 corpora default: 221 clean / 82 rewrites / 0 mismatched (85 with runtime search opt-in); with fuzz v17–v19b: 761 clean / 290 rewrites / 0 mismatched |

Corpus comparison on `HEAD`’s own v1–v11: clean/mismatch counts are
**identical** between `HEAD` and current on every corpus. On the
session-added v12–v20 corpora, `HEAD` mismatches (v12=1, v13=4, v16=1)
because it predates session gate fixes; current is 0-mismatch.

Correction to earlier notes: `HEAD` **did** have an induction kernel
(`sir_mech/src/kernel.rs`, `tests/kernel_induction.rs`) and
quantifier-free integer linear arithmetic (`sir_mech/src/arith.rs`).
There was **no array theory and no quantifier support** (runtime-pointer
library search was blocked on exactly that). The kernel and arithmetic
have now been restored into current `sir_mech` and tested.

## Progress log

- **2026-09-19 — H0 safety**: workspace mirror `recovery.git` holds
  `head` (`2cae1fd`) and `current`; current includes the lean core,
  restored kernel/arith, parity, and the consolidated test suite.
- **2026-09-19 — H1 tests**: workspace-wide test count is **9**
  (`cargo test --workspace`: 9 passed / 0 failed / 0 ignored), with no
  coverage dropped: proof cases, all six families, the z3 kernel
  differential, the patch gate, and the end-to-end corpus harness.
- **2026-09-19 — H1 gate**: fixed a real gate hole — `return_node` was
  not remapped by patch replacements, and a missing return operand was
  not reported as a dangling reference.
- **2026-09-19 — H2 first mine**: parity definition 6 ported from
  `HEAD`'s exclusive-reduction semantics as an XOR-reduction `Family`;
  `xor_laws` discharged by `sir_mech`.
- **2026-09-19 — H1 measured retention**: `emit_c_diff
  --measured-retain` times baseline and optimized code and reports
  `EMITRETAIN retain=base|new` plus `retained_base`. It immediately
  retained the baseline for inequality search at extent 64 (82ns vs
  244ns); definition 200's select chain is now gated to short extents
  until a cost model exists. Base-14 rewrites fell 76 → 59 with zero
  mismatches: the removed rewrites were not measured wins.
- **2026-09-19 — H1 gate hardening**: added a live-out check (a
  replaced node must have no remaining references after use rewriting),
  initial-value gates for sum/Any/count (zero accumulators), and a
  zero-offset gate for folded `sum_offset` shapes.
- **2026-09-19 — H1 family-level z3**: the z3 differential now also
  checks the sum/Any/All/XOR identities and the first-match scan
  contract directly, in the same single solver invocation as the 80
  random kernel queries.
- **2026-09-19 — H1 proof-cost fix**: the family chains are left folds
  in loop order, so associativity and commutativity proofs were
  unnecessary. Trimming to the needed identity laws (with operand order
  matched exactly) cut the sum pipeline from **3.82B to 7.19M
  instructions** — within the 9.4M budget. The memchr pipeline is
  **29.6M** vs the 4.4M budget; the first-match scan equivalence still
  dominates and needs an induction/kernel proof or a cached lemma
  (H3).
- **2026-09-19 — H2 runtime lowering**: rotated self-latch zero-test
  loops (`while (x != 0)`, back-edge on false, entry guard) now lower to
  pre-tested SIR loops as `x != 0` / `x == 0`. The focused
  `naive_popcount` fixture (`x &= x - 1`) lowers and is
  differential-clean. This unblocks the Kernighan popcount family; its
  step lemma and loop-to-popcount obligation are the next proof item
  (H3). Separate-latch runtime loops (the v20 `naive_first_zero` shape)
  remain refused and are the next lowering target.
- **2026-09-19 — H2 runtime search**: the found-flag runtime-bound
  search shape now recognizes a non-constant bound and rewrites to the
  `sir_memchr` intrinsic (definition 402) with the runtime bound as its
  third argument. The focused `naive_first_zero` fixture is
  differential-clean and sanitizer-clean. Measured retention is
  input-dependent: 1.25× on no-hit (89,234ns → 71,186ns) but 0.82× on
  early-hit (710,492ns → 868,892ns), so the predictive cost model (H4)
  is required before this rewrite becomes the unconditional default.
- **2026-09-19 — analysis correctness**: trip-count estimation now
  requires the carried variable to be stepped by a nonzero constant.
  Previously a runtime bound sentinel was mistaken for a counter
  (`trip_count = Some(0)`), which could misclassify runtime loops; the
  fix raised base-14 rewrites 59 → 62 with zero mismatches.
- **2026-09-19 — harness symbol rename**: the differential/timing
  harness now renames exact `@name` symbols everywhere they appear
  (calls, function-pointer operands, `llvm.compiler.used` metadata),
  not only calls. Address-taken corpus functions can now be tested.
- **2026-09-19 — H1 frame/base gate v0**: synthesized patch nodes may
  be pure or read memory, but introducing writes, allocation, I/O,
  atomics, or volatile accesses now fails closed. Coverage is folded
  into the existing patch-gate test; base-14 stays 62 rewrites with
  zero mismatches. Full frame/base identity checks across regions
  remain.
- **2026-09-19 — Kernighan proof-cost evidence**: bounded symbolic
  unrolling proves the Kernighan loop-to-popcount identity at 8 bits in
  8ms and 16 bits in 285ms, but 32 bits takes 35.6s and 64 bits is
  infeasible. The production-width family therefore needs the
  induction/kernel proof (H3), not brute-force unrolling.
- **2026-09-19 — AArch64 differential mode**: `emit_c_diff --target
  aarch64` cross-compiles the emitted C and driver with
  `aarch64-linux-gnu-gcc -static`, compiles the reference IR with
  `clang --target=aarch64-linux-gnu`, and runs both under
  `qemu-aarch64`. Base-14 is **221 clean / 0 mismatched**, matching
  native, including the sum and memchr families; fuzz v17/v19 add
  **220 clean / 0 mismatched**.
- **2026-09-19 — input-domain oracle fix**: the differential driver now
  respects `!range` metadata (e.g. a `_Bool` array filled with 0/1
  instead of arbitrary bytes). This removed a false AArch64 mismatch in
  v10 `p04_runtime_search_bool`; the cross-arch gate is now trustworthy.
- **2026-09-19 — external corpus probe**: Embench and PolyBench were
  cloned to `/tmp/external`. Self-contained functions compare clean with
  zero mismatches, but most real functions are still `lower_refused`
  (globals, allocas, external calls, multi-loop/nested CFG). This is the
  concrete evidence for the next lowering-breadth workstream.
- **2026-09-19 — H2 arithmetic families 100–102**: unsigned
  power-of-two division, remainder, and multiplication now lower to
  shifts/masks, each discharged by `sir_mech` at the operand width.
  v12/v13 went from zero rewrites to 7/6, base-14 rose 62 → 75 with
  zero mismatches, and AArch64 matches native. This also required the
  patch applier to rewrite values inside `Loop` bodies (`replace_in_kind`
  gained the Loop container arm).
- **2026-09-19 — H2 shift-mask 103 + collision gate**: `(x << k) >> k`
  now lowers to `x & ((1 << (W-k)) - 1)`. The first version reused patch
  local ids 0/1, which collided with parameter NodeIds and silently
  mapped a parameter to the mask constant in v17b `f000`. The family
  now allocates local ids above the function maximum, and `patch::apply`
  rejects duplicate or colliding local ids. Native and AArch64
  differentials are clean.
- **2026-09-19 — H4 runtime hybrid rejected**: a scalar-prefix +
  library-memchr hybrid was measured against both regimes and lost:
  early-hit 831,546ns vs 764,552ns baseline, no-hit 90,062ns vs 71,186ns
  plain memchr. The hybrid code was removed; runtime search stays on
  plain `sir_memchr`, and predictive retention remains open.
- **2026-09-19 — lowering breadth**: LLVM's `ret void` operand is now
  treated as a Unit return (previously only an empty operand list was).
  External Embench probes drop one refusal each (crc32 5 → 4, edn
  12 → 11); globals, allocas, external calls, and multi-loop CFG remain
  the ceiling.
- **2026-09-19 — direct permutation lowering**: `llvm.bswap`,
  `llvm.bitreverse`, and same-operand `llvm.fshl`/`llvm.fshr` are kept
  as direct SIR `bswap`/`rbit`/`Rol`/`Ror` ops instead of expanding to
  shift/mask chains. This removed ~50 lines of lowerer code; v16 stays
  differential- and sanitizer-clean. At `-O1` clang already selected the
  same machine instructions, so this is an IR-quality/leanness win, not
  a wall-time claim.
- **2026-09-19 — bitscan evidence**: the v18 corpus (ctpop/cttz/ctlz
  u32/u64) is 6 clean / 0 mismatched on both native and AArch64; the
  lowerer already maps these intrinsics to the SIR/emitter helpers, so
  no family is needed.
- **2026-09-19 — pointer-base searches**: fixed-extent equality
  searches over a raw pointer no longer require an array length in the
  type; the constant loop bound supplies the extent (v14
  `s01_search_a_while_accessing_b` / `s02_eq_key_with_extra`). Coverage
  is folded into the search case; base-14 and fuzz counts are unchanged
  and clean.
- **2026-09-19 — compile-time hint calls**: `llvm.lifetime.*`,
  `llvm.dbg.*`, `llvm.assume`, and
  `llvm.experimental.noalias.scope.decl` are dropped during lowering;
  they have no runtime semantics, and keeping them blocked otherwise
  ordinary functions. Embench statemate drops from 12 to 11 refusals;
  base-14 is unchanged at 78 rewrites with zero mismatches.
- **2026-09-19 — H3 case-split rejected**: replacing the single
  symbolic first-match query with 65 per-first-index implication
  queries was measured at **537M instructions** vs **29.6M** for the
  original query, and was reverted. The memchr proof budget still needs
  a genuine induction/kernel proof, not naive case splitting.
- **2026-09-19 — Kernighan popcount slice**: definition 7 recognizes
  `while (x) { x &= x - 1; count++ }` and rewrites the loop result to
  `Popcount(x)`. The identity is machine-checked by bounded symbolic
  unrolling for widths ≤16 (8 bits 8ms, 16 bits 285ms); wider widths are
  refused pending the kernel induction proof. The focused i16 fixture is
  differential-clean on native and AArch64, sanitizer-clean, and emits
  `__builtin_popcount`. At `-O1` the timing fixture shows parity because
  clang already selects popcount, so this is a capability slice, not a
  speed claim.
- **2026-09-19 — Kernighan H3 cost evidence**: the single-step identity
  `popcount(x & (x-1)) = popcount(x) - 1` was measured by SAT at
  8/16/32/64 bits: 5ms / 134ms / 1.5s / 53s. Even the step lemma is too
  slow at production widths, so 32/64 still need a bit-level/kernel
  induction proof rather than SAT. The i16 slice is now a canonical
  corpus (`gate6a/v21_popcount16.ll`): 1 clean rewrite on native,
  AArch64, and sanitizers.
- **2026-09-19 — measured-retention policy**: runtime-bound memchr is
  now definition **403**, separate from fixed-extent 402. The default
  policy (`family::allow_profiled`) refuses 403 because it measured
  1.25× no-hit but 0.82× early-hit; `emit_c_diff
  --allow-runtime-search` opts in. Base-14 default drops 78 → 75
  rewrites (all clean), opt-in restores 78; fuzz stays 129.
- **2026-09-19 — conditional/masked sums**: the map re-instantiator
  covers comparisons, boolean ops, and `Select`. v16/v10 conditional and
  masked sums rewrite; base-14 default rises 75 → 82 (opt-in 85) and
  fuzz rises 129 → 208, all differential-clean on native and AArch64.
  Measured wins: conditional sum 1.90×, ternary masked sum 2.24×,
  high-mask sum 3.00×; sanitizers clean.

## Reference commands

- Extract old tree: `git archive HEAD sir | tar -x -C /tmp/old_sir`
- Old file: `git show HEAD:sir/crates/<crate>/<path>`
- Build old baseline (outside repo):
  `cd /tmp/old_sir/sir && CARGO_TARGET_DIR=/tmp/old_target cargo build --release`
