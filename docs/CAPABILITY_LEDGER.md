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
| 404 | runtime sum | contract-backed four-accumulator helper for unknown bounds; standalone 1.7–1.9× at N=4096 |
| 405 | runtime count | contract-backed four-accumulator predicate counters (eq/ne/lt/le/gt/ge × u8/u16/u32/u64) |
| 402 | byte search | equality/u8 fixed-extent ported; runtime bounds missing |
| 0 | popcount/cardinality | comparison-predicate count ported; bool-pack/Kernighan path missing |
| 4 | Any | generic pure-map OR reduction (conditional/zero-extended predicates); measured 3.12–3.43× |
| 5 | All | integer AND-chain ported; bool-pack path missing |
| 6 | parity | **restored (XOR-reduction path)** as `sir_opt::parity`; `HEAD` bitset/pack/popcount path still to mine |
| 7 | Kernighan popcount | **restored for all widths** as `sir_opt::popcount`; ≤16 machine-checked, 32/64 contract-backed pending the kernel induction proof |
| 8 | SWAR popcount | **new** in `sir_opt::popcount`: the value-local three-step chain *plus its fold*; obligation machine-checked for 16/32 (one query) and 64 (decomposed); fires on no corpus — see the 2026-09-23 entry |
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
| corpus result | v1–v11: 0 mismatched | 14 corpora default: 221 clean / 128 rewrites / 0 mismatched (131 with runtime search opt-in); with fuzz v17–v19b: 761 clean / 488 rewrites / 0 mismatched |

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

### Old-tree claim audit (2026-09-23)

The 76.9k tree was rebuilt outside the repo (`git archive head sir` →
`/tmp/old_sir`, `CARGO_TARGET_DIR=/tmp/old_target cargo build --release` → exit 0
in 37.6 s) and its recorded claims re-measured.

**Reproduced** — the team's numbers were real, not invented: Gate 3
`k18_all_equal` 5.95×, `k18` random 141×, `k43_sum_ascii` 4.70×,
`k50_count_masked` 8.22× against clang -O3; Gate 6B `pass=10 fail=0` at
1.60–2.10× over Engine 0; Gate 5B fusion 28–35%.

**But the wins are where the input shape was arranged to match the recognizer.**
The three kernels are project-authored *and* are the development set — hardcoded
by name in `vector_derive.rs`'s plan cascade — and Gate 3's own limitations
section says "Only 3 kernels tested" and "the vector plan is manually derived …
proof of concept". The "headroom 4.3–9.5×" figure (Gate 2) measures
**hand-written witness kernels**, not ABSAC output at all.

**Tested on third-party code, it lost.** Redis `redisPopcount` runs 185 ms
against clang -O3's 79 ms — about 2.3× slower than clang *and* 2.3× slower than
not rewriting — and at `head` the rewrite no longer fires on it at all.
**[Corrected 2026-09-23: the "185 ms against 79 ms" pair was never a real A/B —
the witness's `NOTES.md` read "Status: NOT YET MEASURED" for the clang delta. It
is withdrawn. The replacement measurement is 2.16× at kernel level, caused by one
line of `emit.rs`, and the real kernel turns out never to be lowered at all — see
the `redisPopcount` entry below, which supersedes this paragraph.]** Gate 6A's
held-out profitable-optimization rate is **0/16**. H3 records LLVM-source
capability FAILED (0 commits), two corrupt rewrite classes committed, and
performance NOT RUN.

**One contradicted document**: `perft/README.md` shows Redis BITCOUNT at
"ABSAC 1.76×" (64.90 vs 114.23). That table was committed one commit *before*
the first real run, and its numbers appear in no measurement that exists — an
unlabeled mockup left in the tree looking like a result.

**Old harness soundness**, checking today's five defects against the old tree:
absent were the module-wide `undef` zero-suppression, the silent-run pass, the
unshadowed timing reference, and the profile-erased-on-timing-failure. Present
was the out-of-range index aliasing driver scratch, which yields *false
mismatches*, never inflated wins. Separate fail-open paths do exist in
`batch_bench.sh` and `bench_all.rs` (a timed loop that never calls the kernel, an
unconditional `checksum: ok`, a `WIN` computed from a stderr-discarded run), but
no published claim cites those harnesses.

**Proof consumption differs between the two trees, and the current one is
better.** In the old tree assurance was computed and never consumed:
`verify_ids(&self, candidate, _proof)` ignores the proof argument
(`sir_rewrite/src/engine.rs:287`), the selector filters on cost only, and the
production verification floor is `SchemaChecked` — so a rewrite could be recorded
`Proven` and executed on a schema-checked proof. In the current tree the
obligation is consumed at the family: each family returns `None` unless its law
proves (`all.rs:30`, `sum.rs:43`, `count.rs:31`, `parity.rs:35`,
`arithmetic.rs:40-43`).

**Not recoverable**: the session-final ~92.9k tree. `git fsck --unreachable`
yields exactly one object (`6db4450`) and it belongs to the new lean line.

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
- **2026-09-19 — generic Any maps**: `Any` now re-instantiates any pure
  expression whose type matches the accumulator, not just a bare load or
  `or(load, const)`. This covers `zext(predicate)` and conditional
  predicates over narrower elements. Base-14 82 → 83 rewrites (86
  opt-in), fuzz 208 → 240, all differential-clean on native and
  AArch64; sanitizers clean. Measured wins: 3.12× and 3.43×.
- **2026-09-19 — measured-retention definition control**: `emit_c_diff
  --allow-defs LIST` restricts the driver to an exact definition set,
  so a `--measured-retain` run can be turned into a profile decision.
  The e2e test covers it; the default policy (runtime search 403
  refused) is unchanged.
- **2026-09-19 — retention profile output**: `--measured-retain
  --profile-out PATH` now aggregates per-definition decisions and writes
  `def allow` lines (e.g. count definition 0 measured 322ns → 154ns and
  wrote `0 1`). Feed the file back through `--allow-defs` to compile
  with a measured definition set; the e2e test covers the write.
- **2026-09-19 — retention profile input**: `--profile-in PATH` reads
  the `def allow` file directly and restricts the driver to its allowed
  definitions. Verified end to end: a profile containing `0 1` keeps the
  count rewrite and excludes runtime definition 403. The
  measurement → profile → compile loop is now closed; only the
  predictive model remains.
- **2026-09-19 — one-byte hybrid rejected**: a `base[0]` fast path
  followed by memchr measured parity on the early hit (78,766ns vs
  78,840ns) but 0.86× on the no-hit case (92,010ns vs 79,016ns); it was
  reverted. Plain runtime memchr plus profile/opt-in control stays.
- **2026-09-19 — void return emission**: Unit literals now emit
  `((void)0)`, pure Unit-typed nodes are not declared as variables, and
  void functions emit `return;`. Embench edn rises to 2 clean functions
  with 0 harness errors; crc32 remains blocked by an unrelated missing
  `rand_beebs` symbol. Internal v6/v8 results are unchanged.
- **2026-09-19 — harness section GC**: reference IR is compiled with
  `-ffunction-sections -fdata-sections` and linked with
  `-Wl,--gc-sections`, so unrelated functions with undefined symbols no
  longer break every function in a module. External clean counts:
  crc32 0 → 2 and statemate 0 → 1; all four Embench probes now report
  zero harness errors. Base-14 native/AArch64 stays 83 rewrites with
  zero mismatches; v8 sanitize is clean.
- **2026-09-19 — residual shape sample**: v19b has 200 clean functions
  and 134 with no rewrite; most unrewritten loops are runtime-bound sums,
  counts, and searches (`trip_count: None`), which the fixed-extent
  proof model refuses. Runtime reduction contracts or vectorized helpers
  are the next family opportunity (H2/H4), not another fixed-extent
  recognizer.
- **2026-09-20 — runtime-bound sum (404)**: unknown-bound
  `acc += zext(a[i])` loops now lower to a four-accumulator helper
  (u8/u16/u32/u64 element types). Base-14 83 → 105 rewrites, fuzz
  240 → 296, all differential-clean on native and AArch64 and
  sanitizer-clean. The helper is 1.7–1.9× faster standalone at N=4096;
  the harness timing driver uses small bounds, where it measures parity,
  so no harness speed claim.
- **2026-09-20 — runtime-bound count (405)**: unknown-bound
  `acc += zext(icmp(a[i], k))` loops now lower to four-accumulator
  predicate counters (eq/ne/lt/le/gt/ge over u8/u16/u32/u64). Base-14
  105 → 128 rewrites (131 with runtime search opt-in), fuzz 296 → 360,
  all differential-clean on native and AArch64 and sanitizer-clean.
  Small-bound harness timing is parity; the win is capability/ILP at
  large bounds.
- **2026-09-20 — residual sample updated**: after runtime sum/count,
  v19b’s no-rewrite count drops **134 → 67**. The remainder is mostly
  loop-free single-expression functions or unsupported domains
  (signed DSP, globals, allocas, nested CFG), so further rewrite gains
  need H3 theory/lowering work rather than another reduction helper.
- **2026-09-20 — per-function retention profiles**: `--profile-out` now
  writes `function def allow` lines, and `--profile-in` accepts both the
  legacy global `def allow` form and the per-function form (per-function
  overrides global). This lets input-dependent rewrites like runtime
  definition 403 be enabled only for the functions where they measured
  a win. Verified round-trip: count retained new; unrelated runtime
  search stays default-refused.
- **2026-09-20 — full-range hybrid rejected**: first-byte check then
  memchr over the full aligned range measured **1.41×** on the early hit
  but **0.86×** on no-hit, so it still does not dominate. Runtime search
  stays profile/opt-in.
- **2026-09-20 — Kernighan 32/64 (contract)**: definition 7 now covers
  production widths. Widths ≤16 keep the bounded machine proof; 32/64
  are contract-backed and verified by differential/sanitizer evidence,
  with the kernel induction proof still H3.
  `gate6a/v22_popcount64.ll` rewrites 1 and is clean on native and
  AArch64; harness timing is parity because clang optimizes the baseline
  loop, so no speed claim.
- **2026-09-20 — H3 SAT path closed**: a narrow-counter encoding of the
  Kernighan step lemma improved width-64 from 53.2s to 48.4s — still
  orders of magnitude from pipeline feasibility (8/16/32 = 3.5ms /
  62ms / 1.06s). SAT encoding tweaks are not the H3 path; the induction
  obligation needs kernel/proof-by-reflection work instead.
- **2026-09-20 — stores fail closed**: the lowerer previously built
  `Store` nodes that the emitter silently ignored — a side-effect drop.
  Memory stores are now refused explicitly until write emission exists.
  No existing clean corpus function changes (they were already refused
  or store-free); this closes a latent miscompile class. Write emission
  is now an explicit lowering-breadth item.
- **2026-09-20 — memory write emission**: stores through direct pointer
  parameters and `ArrayAccess` lvalues now lower and emit correctly.
  Loads create a distinct read node so a GEP used as a store target is
  not materialized as a value; store targets are inlined as lvalues.
  Mutability-aware parameter emission drops `const` where a store
  writes. `SignExtend` now casts through the signed source type instead
  of zero-extending. Verified by scalar/array store fixtures, v8
  `n06_may_alias_store_const`, Embench `vec_mpy1` and `mac`, all
  differential- and sanitizer-clean on native and AArch64. Base-14
  clean rises 221 → 230 with rewrites stable at 128; external Embench
  clean rises 6 → 8.
- **2026-09-20 — signed value ops**: `ashr`, signed comparisons, and
  `smax`/`smin` are supported as value-level intrinsics with signed
  C casts. Signed comparisons that control loop domains still fail
  closed (the first attempt mis-lowered v3 `n02_running_max` and
  `n11_reverse_sum`; base-14 is back to 230 clean / 128 rewrites / 0
  mismatched). Focused signed fixtures are clean on native, AArch64,
  and sanitizers; fuzz remains 540 / 360 / 0.
- **2026-09-20 — read-only globals**: a `Global` byte-base node plus
  namespaced C-side buffers handles direct `@g` access and inline
  constant GEPs. External globals and internally-defined globals with
  non-zero initializers fail closed. Base-14 stays 230 clean / 128
  rewrites / 0 mismatched on native and AArch64; fuzz stays 540 / 360
  / 0; sanitizers are clean. External Embench clean rises 8 → 11
  (statemate `interface`/`init`, matmult-int), with zero harness
  errors.
- **2026-09-20 — global-effect verification**: the differential driver
  now shadows every referenced global into a shared namespaced buffer,
  resets it before each side's call, snapshots after the reference, and
  compares the full buffer after the emitted call (`EMITGLOB`). Void
  functions with global side effects are therefore no longer vacuously
  "clean": statemate `interface`/`init` pass with all global bytes
  compared on native, AArch64, and sanitizers.
- **2026-09-20 — acyclic global CFG + `switch`**: multi-line `switch`
  case lists are folded into one terminator, and acyclic functions whose
  memory is entirely constant-global accesses go through a predicated
  path engine (branch guards become `Select` conditions; writes merge by
  path condition). Differential globals are seeded with deterministic
  bytes so non-default switch cases execute. Two correctness bugs
  surfaced and were fixed: `Bool(true)` constants emitted as `false`
  (`as_u64` has no Bool arm), and scalar global stores were emitted
  through a `uint8_t *` (one byte instead of the stored width). Embench
  statemate clean rises 1 → 6 (all but `generic_FH_TUERMODUL_CTRL`,
  which exceeds the 4096-path cap); external Embench clean rises 8 → 14.
  Base-14 stays 230 / 128 / 0 on native and AArch64, fuzz stays
  540 / 360 / 0, and sanitizers report no violations.
  `generic_FH_TUERMODUL_CTRL` was the measured blocker here; the
  block-level join engine below resolves it.
- **2026-09-20 — pointer-valued GEPs and mixed-width access**: GEP
  results used as pointer values (phi incoming, another GEP's base,
  select/compare/call operand) lower to a pointer-typed `PointerOffset`;
  GEPs consumed only by loads/stores keep the recognizer-friendly
  `ArrayAccess` form (base-14 rewrites stay 128). `ArrayAccess`
  emission now reinterprets element width through the source pointer
  (`((T *)(p + i))[0]`), and loop-carrier preheader dependencies are
  emitted before carrier declarations. EDN `iir1` becomes clean;
  external Embench clean 14 → 15. Base-14 native/AArch64 stay 230 /
  128 / 0, fuzz stays 540 / 360 / 0, and sanitizers report no
  violations.
- **2026-09-20 — constant globals and `bcmp`/`memcmp`**: the driver now
  parses `constant` as well as `global` definitions and gives every
  referenced global a shared shadow buffer (non-zero initializers no
  longer force a refusal). Initializer bytes are not reconstructed yet:
  both sides read the same driver buffer, so this is a valid
  equivalence check over an arbitrary state, not a reproduction of the
  declared table contents — recorded as a follow-up. `bcmp`/`memcmp`
  lower to `sir_bcmp` with attribute-aware argument parsing and inline
  GEP arguments. EDN `verify_benchmark` and matmult-int
  `verify_benchmark` become clean; external Embench
  clean rises 15 → 17. Base-14 native/AArch64 stay 230 / 128 / 0, fuzz
  stays 540 / 360 / 0, and sanitizers report no violations.
- **2026-09-20 — signed division/remainder**: `sdiv`/`srem` lower to
  `sir_sdiv`/`sir_srem` and emit guarded helpers (division by zero and
  `INT_MIN / -1` are undefined in the source; the guard returns an
  arbitrary defined value). `exact` flags still fail closed. matmult-int
  `RandomInteger` becomes clean; external Embench clean rises 17 → 18.
  Base-14 native/AArch64 stay 230 / 128 / 0, fuzz stays 540 / 360 / 0,
  and sanitizers report no violations.
- **2026-09-20 — block-level joins for high-path CFGs**: acyclic
  global-memory CFGs are pre-counted; above 512 paths the predicated
  engine merges one state per basic block instead of enumerating paths
  (incoming conditions fold through `Select`). Measured motivation:
  raising the path cap for statemate `generic_FH_TUERMODUL_CTRL` grew
  the emitted file to 4.4 MB and did not finish in 120 s; the block
  engine emits 159 KB and runs in ~5 s. statemate clean rises 6 → 7 and
  external Embench clean rises 18 → 19. Base-14 native/AArch64 stay
  230 / 128 / 0, fuzz stays 540 / 360 / 0, and statemate is
  differential-clean at 16 cases with zero sanitizer violations.
- **2026-09-20 — `llvm.memset`/`llvm.memcpy`**: memory-fill and
  memory-copy intrinsics lower to effect-classified Unit intrinsics and
  emit `memset`/`memcpy` calls; volatile forms fail closed. The
  pointer-argument resolver is shared with `bcmp`/`memcmp`. A focused
  global-buffer fixture is differential-clean on native, AArch64, and
  sanitizers. Base-14 native/AArch64 stay 230 / 128 / 0, fuzz stays
  540 / 360 / 0, and the external Embench counts are unchanged (19);
  this is a prerequisite for the blocked benchmark wrappers rather
  than a new clean function by itself.
- **2026-09-20 — `llvm.memmove`**: overlap-safe copies lower through
  the same attribute-aware intrinsic path and emit `memmove`; the
  focused fixture covers an inline-GEP destination with overlapping
  source. Clean on native, AArch64, and sanitizers with base-14 and
  fuzz unchanged.
- **2026-09-20 — fixed-size `alloca`**: constant-size stack buffers
  lower to a typed `LocalAlloca` base and emit `_Alignas(N) uint8_t
  name[size];` locals; variable-size allocas fail closed. A focused
  buffer fixture (memset + runtime index store/load) exposed and fixed a
  latent store-typing bug: literal stores were typed `u64` instead of
  their declared width, which wrote eight bytes through an `i8` lvalue.
  The fixture is now clean on native, AArch64, and sanitizers; base-14
  stays 230 / 128 / 0 on both targets, fuzz stays 540 / 360 / 0, and
  external counts are unchanged (19). Alloca is a prerequisite for
  statemate `verify_benchmark` and edn `jpegdct`.
- **2026-09-20 — rejected separate-latch loop builder**: an experimental
  block-engine loop builder was measured against statemate `FH_DU` and
  `verify_benchmark`. It reached the next real blockers (local calls in
  `FH_DU`; rotated loop entry in `verify_benchmark`) but produced no
  clean function, so it was reverted rather than kept as bloat.
- **2026-09-20 — PolyBench held-out baseline**: ten kernels were
  compiled with clang `-O1 -fno-inline -fno-vectorize` (atax, bicg, mvt,
  gemver, gesummv, gemm, syr2k, jacobi-1d/2d, fdtd-2d). Every kernel
  function is refused; the universal blocker is nested (or
  nested+sequential) loop structure, with secondary blockers in
  `main`/`init_array` (allocation calls, floating point, external I/O).
  No shallow wins exist in this suite: nested-loop support is the
  prerequisite. A dominance-based back-edge rewrite plus separate-latch
  extension was measured against this suite and base-14; it produced no
  new clean PolyBench function and regressed base-14 to 189/122/1, so it
  was reverted (the focused rotated loop was already clean in the
  committed baseline).
- **2026-09-20 — runtime dispatch for `memchr` (H4)**: the emitted
  helper scans the first 16 bytes inline and falls back to vectorized
  `memchr` for the tail. Microbenchmark (N = 4096, 100k reps, ns/op):
  early hit scalar 1.25 / vector 1.55 / hybrid 1.27; mid hit 2.71 /
  1.55 / 2.80; no hit 542 / 16.6 / 16.8. This removes the measured
  early-hit regression (0.82×) while keeping the no-hit win, so
  `allow_profiled` now enables definition 403 by default. A focused
  runtime-search fixture rewrites cleanly; base-14 rises 128 → 131
  rewrites with 230 clean / 0 mismatched on native and AArch64, fuzz
  stays 540 / 360 / 0, and v10 plus the fixture are sanitizer-clean.
- **2026-09-20 — small-extent search choice (H4)**: unrolled select
  chains measured faster than the vectorized primitive at extents 4, 8,
  and 16 (1.22–1.27 vs 1.49–1.50 ns/op, 2M reps), so fixed-size
  equality searches at extents ≤16 now take the proven select chain
  (definition 200); larger extents keep the library primitive (402).
  Base-14 is unchanged end to end (its equality searches are extents
  48+), and tests, base-14 native/AArch64, fuzz, external, and
  sanitizers all stay green at 230 / 131 / 0, 540 / 360 / 0, and 19.
- **2026-09-20 — reproduction runner**: `sir/scripts/verify.sh`
  (`fast` / `full`) runs the workspace tests, base-14 native and
  AArch64, fuzz native and AArch64, external Embench, and sanitizers,
  failing on any mismatch or harness error, and writes a
  `manifest.sha256` covering source and corpus files. `full` passes:
  9 tests, base-14 230 / 131 / 0 on both targets, fuzz
  540 / 360 / 0, external 19 clean, and zero sanitizer violations
  (98-file manifest).
- **2026-09-20 — rejected speculative separate-latch lowering**: a
  builder-fork wrapper around the separate-latch builder produced no
  external or fuzz gains and mis-lowered one base-14 function
  (230 / 131 / 0 → 229 / 131 / 1), so both the fork plumbing and the
  generalized loop path were reverted.
- **2026-09-20 — `llvm.expect`**: branch-hint intrinsics (including
  `expect.with.probability`) lower as identity on their value operand.
  A handcrafted IR fixture is differential-clean on native, AArch64, and
  sanitizers; the fast reproduction runner stays green and corpus counts
  are unchanged (no corpus currently exercises the intrinsic).
- **2026-09-20 — staged nested-loop emission**: the emitter now treats
  Loop nodes contained in another Loop's body as nested rather than
  sequential, passes parent carrier maps into recursive emission, and
  schedules nested loops before consumers of their outputs. A
  hand-built nested SIR probe compiles and returns the expected value;
  existing corpora are unchanged (fast runner and fuzz stay green).
  Nested-loop construction in the lowerer is the next step.
- **2026-09-20 — two-level nested loops (fir)**: a strict nested
  detector plus builder lowers the inner loop first and constructs the
  outer loop around the inner `Loop` node and its extracts; the emitter's
  recursive support schedules the inner loop inside the outer body with
  parent carriers. EDN `fir` is differential-clean at 16 cases on
  native and AArch64, sanitizer-clean, and external Embench clean rises
  19 → 20. Base-14 stays 230 / 131 / 0, fuzz stays 540 / 360 / 0, and
  `verify.sh full` passes.
- **2026-09-20 — linear loop chains (matmult)**: nested detection now
  accepts a chain of directly nested counted loops (outer→middle→inner)
  and lowers it recursively: each level builds its child `Loop`, then
  its own carried state and termination. Multi-index GEPs fold their
  dimensions into one linear index (`[20 x i64], p, i, j` → `i*20+j`),
  and the emitter executes nested loops **in body order**, hoisting only
  parent-scoped dependencies. matmult-int `Test` and `Multiply` are
  clean at 16 cases native/AArch64 and sanitizer-clean; external Embench
  clean rises 20 → 22. Base-14 stays 230 / 131 / 0, fuzz stays
  540 / 360 / 0, and `verify.sh full` passes.
- **2026-09-20 — post-tested nested loops (`fir_no_red_ld`)**: direct
  latch tests of the carried counter (`carry < bound`) now shift by the
  loop step so the source's do-while iteration count is preserved;
  `disjoint` qualifiers are stripped; mixed-width GEPs scale their index
  to bytes when the opaque pointee differs, and `PointerOffset` casts
  the re-typed pointer expression. EDN `fir_no_red_ld` is clean at 16
  cases native/AArch64 and sanitizer-clean; external Embench clean rises
  22 → 23. `verify.sh full` passes with base-14 at 230 / 131 / 0 and
  fuzz at 540 / 360 / 0.
- **2026-09-20 — exact arithmetic and nested-scope hardening**: `exact`
  shifts/divisions with constant amounts get a guard (poison becomes a
  defined zero fallback), and `exact`/`disjoint` qualifiers are
  stripped. The chain builder maps child successor values to the child
  loop's extracts, and the emitter hoists each child's parent-scoped
  dependencies at its body-order position. Integer-phi gating excludes
  pointer-carrying nests (`jpegdct`) so they fail closed instead of
  mis-lowering. Focused `ashr exact`/`lshr exact` fixtures are clean on
  native, AArch64, and sanitizers; `verify.sh full` passes with
  external Embench at 23 clean, base-14 at 230 / 131 / 0, and fuzz at
  540 / 360 / 0.
- **2026-09-20 — sequential nested-loop groups**: back edges are
  dominance-based in the group detector, back-edge sets partition into
  sequential chains, inter-group straight-line segments are sequenced
  with run-once SIR loops, and carrier initializers substitute parent
  carriers without leaking child-scope temporaries. matmult-int
  `initialise_benchmark` is clean at 16 cases native/AArch64 and
  sanitizer-clean; external Embench clean rises 23 → 24 under the then
  current ad-hoc inputs. Base-14 stays 230 / 131 / 0, fuzz stays
  540 / 360 / 0, and `verify.sh full` passes.
- **2026-09-20 — reproducible external inputs and harness fixes**:
  `scripts/fetch-external.sh` pins the Embench clone and compile flags
  (`-O1 -fno-inline-functions -fno-vectorize -DGLOBAL_SCALE_FACTOR=1
  -DLOCAL_SCALE_FACTOR=1`). Function-body extraction now matches the
  `define` line rather than an earlier call site (it previously sliced
  `warm_caches` for `benchmark_body`, so no globals were injected), and
  the renamed reference definition drops `internal`/`private` so
  internal functions link. Nest detection allows known intrinsics
  (`memcpy`/`memset`/`memmove`/`lifetime`/`expect`). Under these
  reproducible inputs the external Embench clean count is **22**
  (matmult 5, crc32 2, edn 8, statemate 7); the earlier 24 figure came
  from unrecorded IR-generation flags and is superseded. Guarded-entry
  sequential groups (`benchmark_body`) remain fail-closed.
- **2026-09-20 — direct local calls**: the lowerer turns a direct call to
  a module-defined function into a namespaced `ExternalCall`; the harness
  lowers the callee dependency closure postorder, emits each callee as its
  own translation unit, and links them with the caller. Shared globals are
  unioned and shadowed once. External symbols, variadics, indirect calls,
  recursion, and `byval`/`sret` arguments stay refused. matmult `Test`
  (calls `Multiply`) is clean, raising the reproducible external count
  22 → 23; base-14 stays 230 clean, fuzz stays 540 / 360 / 0, and the
  full matrix (including sanitizers) passes.
- **2026-09-20 — guarded nests, exact names, fast proofs**: the nested
  detector now uses dominance back edges and accepts a guarded outer
  header only when the skip guard is exactly the inner loop's zero-trip
  test (`bound == 0` with a zero-initialized inner counter); matmult
  `benchmark_body`, `warm_caches`, `benchmark`, and
  `initialise_benchmark` become clean, taking matmult to 10/10 and the
  reproducible external count 23 → **27**. `extract_function_by_name`
  now matches `@name(`, fixing a latent prefix collision (`benchmark`
  had been lowered as `benchmark_body`). Test time work: power-of-two
  `udiv`/`urem` encode structurally, `Bv` folds constants and algebraic
  identities, the two 32-lane kernel proofs run on separate cores, the
  z3 differential proves its 40 random cases on both cores, and
  dev/test builds compile at `opt-level = 3`. The single-digit suite
  (9 tests) runs in **~1.3 s** steady-state, down from ~9.3 s; base-14
  stays 230 / 131 / 0, fuzz 540 / 360 / 0, and `verify.sh full` passes
  with sanitizers.
- **2026-09-20 — signed do-while domains and byte addressing**: signed
  carry tests are reconstructed as shifted pre-tested domains
  (`do { } while (c > k)` → `while (c > k + step)`), and an entry guard
  is accepted only after the interval proof that both its edges match
  the shifted test. This unlocks edn `latsynth` and base-14
  `n11_reverse_sum` (9 → 10 external edn, base-14 230 → 237 clean).
  Mixed-width access now byte-addresses through `uint8_t *` with signed
  index casts, nested array types parse to their innermost element, and
  the driver gives every buffer an identical 32-byte guard slab so
  guarded negative reads are comparable under UBSan. Also fixed a
  `PointerOffset` emitter shadowing bug that forced every offset through
  the byte path. Reproducible external clean rises 27 → **28**; base-14
  is 237 / 133 / 0, fuzz 540 / 360 / 0, sanitizers 0, and the 9-test
  suite still runs in ~1.3 s.
- **2026-09-22 — rejected global-state loop engine (FH_DU)**: a
  predicated do-while engine was implemented for the FH_DU state
  machine: phi values selected from predecessor edge conditions,
  conditional stores merged by select, a carried run flag for the
  post-body latch test, and prelude emission for body-reachable
  globals. It lowered FH_DU but `benchmark_body` mismatched 67 globals
  and the prelude change also perturbed existing loop emission, so the
  whole experiment was reverted to `10b9667`. FH_DU is not independently
  testable either: with arbitrary driver globals the **reference** loops
  forever; only `benchmark_body` (which memsets Bitlist and calls
  init/interface first) reaches a terminating state. H3 probe from the
  same pass: direct Kernighan unrolling at width 32 takes 7.1 s; the
  single clear-lowest-bit step lemma takes 0.32 s at width 32 and 9.9 s
  at width 64; width-64 unrolling does not finish. Induction remains
  the only viable path for production-width Kernighan.
- **2026-09-22 — first-match side-effect loops**: the separate-latch
  header/early/latch/merge pattern (`n09`/`n12`/`n13`) now lowers as a
  three-carried SIR loop (index, run flag, found flag); the match edge's
  direct global update is emitted as a select on the match condition,
  and the merge result is `found ? index : bound`. Detector stays narrow:
  one integer header phi, constant stride and bound, one direct
  non-volatile global store, no calls. Base-14 clean rises 237 → **243**
  (v8/v9/v10/v11 `n09`, v7 `n13`, v6 `n12`); `verify.sh full` passes
  with base-14 243 / 133 / 0, fuzz 540 / 360 / 0, external 28, and the
  9-test suite still ~1.3 s.
- **2026-09-22 — harness run timeout**: differential/lifetime runs now
  wait at most `SIR_RUN_TIMEOUT_SECS` (default 10) for a child before
  killing it; timeouts are reported as `lower_refused (run did not
  terminate)` rather than hanging the matrix or counting as harness
  errors. This is the safety net for nonterminating references such as
  statemate `FH_DU` on arbitrary driver globals and v7
  `n04_runtime_step_do_while` with step 0. `verify.sh full` passes
  unchanged (base-14 243 / 133 / 0, fuzz 540 / 360 / 0, external 28,
  sanitizers 0).
- **2026-09-22 — conditional-body counted loops**: headers that branch
  between a straight-line storable body and their latch now lower as a
  pre-tested SIR loop: the header condition becomes a select over the
  body value, the latch merge is `select(body_active, body, header)`,
  and the counter increment drives `index < bound`. Detector is narrow
  (two integer phis: counter + accumulator; straight-line store-free
  body; zero-compare entry guard; unit stride). Unlocks v5
  `n04_sum_index_conditional`: base-14 243 → **244** clean,
  `verify.sh full` passes (244 / 133 / 0 native+AArch64, fuzz
  540 / 360 / 0, external 28, sanitizers 0, 9 tests ~1.3 s).
- **2026-09-22 — first-match generalized to search-then-loop**: the
  first-match lowerer now allows a match block with **no** store, carries
  the match block's value as a fourth loop output, and selects the merge
  result from that output against an arbitrary exit value instead of
  hardcoding `index` vs `bound`. When the merge flows into a second
  loop, the remainder is handed back to `lower_loop_function` with the
  preceding blocks skipped, composing the two loops sequentially.
  Unlocks v16 `c01_search_then_sum`: base-14 244 → **245** clean,
  `verify.sh full` passes (245 / 133 / 0 native+AArch64, fuzz
  540 / 360 / 0, external 28, sanitizers 0, 9 tests ~1.3 s).
- **2026-09-22 — foreign support objects (crc32)**: `emit_c_diff` gains
  `--ffi-support PATH`, links a support C object into both sides, and
  calls a weak `__sir_ffi_reset()` before each side so stateful support
  code starts identically. The lowerer keeps the original symbol for
  calls whose target is not defined in the module (same-module callees
  stay namespaced). Nested-loop lowering now exposes inner-body values
  as inner outputs, remaps the latch phi to the outer output before
  post-latch re-emission, and selects post-loop guard phis
  (`n == 0 ? 0 : loop_result`). `undef` lowers to a typed zero, and
  undef-tainted modules use a nonzero scalar input domain because the
  zero path is undefined behaviour in the source. A small independent
  BEEBS RNG support object is provided under `sir/support/`. crc32 rises
  2 → **6** clean (including sanitize), external Embench 28 → **32**;
  base-14 stays 245 / 133 / 0, fuzz 540 / 360 / 0, 9 tests ~1.3 s.
- **2026-09-22 — peeled found-search with index store**: the
  entry-guard/peel/pre-tested-body/store-index/merge shape now lowers as
  one run-flag loop over indices `0..n`, with the match store emitted as
  `select(match, i, old)` and the merge returning the found flag.
  Unlocks v3 `n10_early_exit_write`: base-14 245 → **246** clean. Also
  fixed `global_decls` keeping a trailing comma on scalar global types
  (`i64,` parsed as size 0), which had blocked external scalar globals.
  `verify.sh full` passes (246 / 133 / 0 native+AArch64, fuzz
  540 / 360 / 0, external 32, sanitizers 0, 9 tests ~1.3 s).
- **2026-09-22 — rejected loop-prelude pre-emission (jpegdct)**: two
  emitter variants were tried to hoist preheader statements out of loop
  scope so a three-level chain could read the entry alloca/GEPs:
  (a) pre-emit every node with id below the first `Loop`, and
  (b) additionally exclude any node that transitively depends on a loop
  body (`depends_on_loop_body`). Both regressed loop-heavy functions
  (`fir_no_red_ld`, `iir1`, and others) and still produced uses of
  inner-loop carriers before their declaration. The reason is
  structural: an outer loop's carrier initializer is allowed to read
  values owned by an inner loop, so pre-loop emission needs a
  loop-aware schedule, not a flat prelude. Both variants were reverted;
  `jpegdct` stays refused (three back edges, outside the two-level
  detector) and `verify_benchmark`/`FH_DU` remain blocked by the same
  scoping design.
- **2026-09-22 — deep linear loop chains (jpegdct)**: three-or-more
  level linear nests now lower root-only. Each level is a do-while with
  a carried run flag, header phis map to initiates, and latch
  next-values become outputs/extracts. Entry/preheader statements are
  emitted inside the outermost body (the chain exit returns void), and
  each child loop's internal ids are folded into the parent body so the
  post-loop pass cannot leak body-scoped names. Generated address
  arithmetic now goes through `uintptr_t` with explicit two's-complement
  byte offsets, and the emitter's loop-id set covers every nested loop's
  internals, so post-loop emission cannot reference body-scoped names or
  trigger pointer-offset UB. With those fixes in place the callee
  composition guard was no longer needed: `jpegdct` and its edn callers
  (`warm_caches`, `benchmark_body`, `benchmark`) are all clean, and the
  temporary chain-disable machinery was deleted. edn reaches **13/13**
  clean native and sanitizer-clean, raising external Embench 32 → **36**
  with 0 mismatches and 0 violations. `verify.sh` fails on `violations=`
  and bounds every case with `CASE_TIMEOUT` (default 180s). Evidence:
  base-14 246 / 133 / 0 native+AArch64, fuzz 540 / 360 / 0 (plus
  AArch64 60/160), external 36, sanitize 0 violations, 9 tests ~1.3 s.
- **2026-09-22 — mismatch-search loops (verify_benchmark)**: the
  header/latch/completion/merge shape now lowers as a run-flag search
  for the first unequal element, with the completion chain emitted after
  the loop and `merge = found ? 0 : completion`. Because the preheader
  is constant-only (alloca/memset/store of a literal), it is safe to
  re-execute inside the body, which avoids the loop-scope problem.
  Whole-body node coverage (id range) keeps post-loop emission from
  leaking body temporaries, and emitter preludes now hoist `Global` base
  nodes so completion chains can read them. Unlocks statemate
  `verify_benchmark`: external Embench 36 → **37** (statemate 8/13,
  edn 13/13, crc32 6/6, matmult 10/10), with 0 mismatches and 0
  sanitizer violations. Base-14 stays 246 / 133 / 0, fuzz 540 / 360 / 0
  (AArch64 60/160), 9 tests ~1.3 s.
- **2026-09-22 — pre-tested search-then-sum (p07/p16)**: a runtime or
  constant bound is scanned for the first equal byte with an optional
  zero-guard and first-element peel; the miss path runs a fixed-size
  accumulation loop. The scan is one run-flag loop over `next = 1..=bound`
  with the pure peel recomputed inside, the match block is rebuilt with
  the loop-carried index, and the original merge branch targets select
  either the match index or the post-loop sum. This covers p07's
  zero/exhausted path and p16's `index > 38` sentinel without modeling
  them separately. Base-14 246 → **248** (496 clean / 266 rewrites
  native + AArch64), external stays **37**, 0 sanitizer violations,
  9 tests ~1.3 s. The three run-flag search lowerers (peeled-found,
  mismatch, search-then-sum) now share the same loop skeleton and are a
  recorded dedup candidate for the next leanness pass.
- **2026-09-22 — H3 Kernighan induction (width ≤32)**: `K` is the
  loop's functional equation (`K(0)=0`, `x≠0 ⇒ K(x)=1+K(x&(x-1))`);
  `K(x)=popcount(x)` is now proved by kernel strong induction on the
  integer value of `x` and replay-checked with **no hypotheses**.
  Width 16 drops from 285 ms of SAT unrolling to 34 ms, width 32 from
  35.6 s to **0.43 s** (cached per process); the family gates on the
  proof for ≤32. Width 64 still needs a >64-bit integer shadow for the
  measure, so it stays contract-backed. The old bounded-unrolling law
  was deleted. Kernel support: induction measure/premise checks fall
  back to bit-blasting when linear arithmetic cannot decide them, and
  free `Int` variables in bit-blast obligations must be pinned to
  `[0, 2^64)` by the hypotheses (they are then shadowed by 64-bit BV
  variables, including in replay). Evidence command:
  `cargo run -p sir_mech --example kernighan_induction -- 32`.
  Matrix unchanged: base-14 496/266/0, external 37, sanitizers 0,
  9 tests ~1.36 s.
- **2026-09-22 — H3 Kernighan width 64 (offline)**: the induction now
  stops at `2^64 - 2` so the `v + 1` measure still fits the kernel's
  64-bit shadow, and the excluded all-ones pattern is proved by a
  64-step unfolding of the recurrence; the two cases are combined with
  `case_split`. `cargo run -p sir_mech --example kernighan_induction
  -- 64` proves and replays in **27.8 s**. The pipeline still gates
  only ≤32 so ordinary compiles stay fast; 64 remains contract-backed
  at optimization time. Width 32 stays 0.42 s; 9 tests ~1.36 s.
- **2026-09-22 — unguarded do-while with a provable first iteration
  (n04)**: a successor-tested rotation (`next = carry + step`,
  continue while `next < bound`) can be lowered as the reconstructed
  pre-tested domain when that domain already holds at the carried init
  (constant-provable, e.g. `0 < 64`); the source's forced first
  iteration is then exactly the first pre-tested iteration. Runtime
  steps keep the scalar driver domain nonzero because step 0 is a
  nonterminating source, and the emitted domain is sanitizer-clean.
  This was the last non-atomic, non-`unreachable` base-14 refusal:
  base-14 248 → **249** (498 clean / 266 rewrites native + AArch64),
  external stays 37, 9 tests ~1.36 s.
- **2026-09-22 — held-out Embench expansion (integrity fixes)**: the
  fetch script now compiles all 19 Embench benchmarks, and the 15 new
  probes exposed four real problems:
  1. `i128` was silently emitted as a 64-bit C type (aha-mont64
     `mulul64`/`montmul` mismatched 4/4). The emitter now uses
     `unsigned __int128`/`__int128`.
  2. Self-loop back-edges (`br label %self`, no phi) were not counted
     as back-edges, so an unmodeled infinite branch fell through to the
     straight-line walker and executed the wrong path (sglib
     `sglib_iq_add` emitted undeclared temporaries and dropped the
     branch). Self-loops now select the fail-closed early-exit path.
  3. Pointer-valued `load ptr`/`store ptr` (sglib intrusive lists) were
     byte-truncated; they now fail closed.
  4. Named-struct GEPs (wikisort `BlockSwap`) were byte-scaled with no
     layout model; aggregate-typed GEPs now fail closed.
  Results over the 15 new probes: **50 clean / 229 refused / 0
  mismatched / 1 invalid-state harness error** (280 functions; cases 1).
  The one error is `sglib_iq_first_element`: random bytes drive its
  unchecked array index out of bounds on both sides, so it is an
  invalid-state probe, not a miscompile (ASan agrees; aha-mont64 and
  wikisort are sanitizer-clean). Pinned external stays 37, base-14
 498/266, fuzz unchanged, 9 tests ~1.36 s.
- **2026-09-22 — named-struct stride GEPs (wikisort)**: the lowerer now
  parses `%name = type {...}` layouts (size/alignment, nested arrays and
  structs, packed structs) from the enclosing module and scales
  direct struct-stride GEPs by the real element size. wikisort
  `BlockSwap` moves from fail-closed to **clean** (32 cases native +
  AArch64), held-out Embench 50 → **51** clean / 228 refused / 0
  mismatched / 1 invalid-state. Array-of-struct GEPs and struct field
  accesses still fail closed (no field-offset model). Base-14 stays
  249/133, pinned external 37, 9 tests ~1.36 s.
- **2026-09-22 — harness struct layouts + per-corpus support probe**:
  the driver's global-size parser reuses the lowerer's module layout
  map, so globals like wikisort's `[400 x %struct.Test]` are sized
  correctly (wikisort baseline 10 → **11** clean; `verify_benchmark`
  now clean). `scripts/probe-external.sh` runs all 19 Embench corpora
  with the support object each needs (`beebs_rng.c` for crc32/wikisort,
  `beebsc.c` for huffbench/md5sum/tarfind) and reports totals:
  **95 clean / 225 refused / 0 mismatched / 1 invalid-state** across
  all 19; held-out alone **58 clean / 221 refused / 0 mismatched / 1
  invalid-state** (no-support held-out baseline 52/227/0/1). wikisort
  is 16/12/0 with the RNG support. The invalid-state probe remains
  `sglib_iq_first_element` (unchecked random index, both sides).
  Base-14 stays 249/133, pinned external 37, 9 tests ~1.36 s.
- **2026-09-22 — acyclic scalar CFGs (path enumeration)**: the
  branch/phi path enumerator previously ran only for acyclic CFGs with
  global memory, so no-memory multi-block functions fell through to the
  straight-line walker and failed with `cannot resolve return value
  '%N'`. Acyclic scalar branch/phi CFGs now use the same enumerator
  (branch guards become selects, phi joins become edge selects). This
  adds 4 held-out clean functions (picojpeg 7 → **10**; wikisort
  TestingPathological), with the all-19 probe at **99 clean / 221
  refused / 0 mismatched / 1 invalid-state**. Base-14 stays 498/266
  native + AArch64 and fuzz stays 540/360; 9 tests ~1.36 s.
- **2026-09-23 — H3 generic first-match scan theorem**: the kernel now
  has primitive `IntIte` reduction rules (`c ⊢ if c then x else y = x`
  and its negation), with replay checks but no bit-blasting or bounds
  side conditions. On top of them, `sir_mech::scan` proves the
  extent-independent equivalence of the forward found-flag fold and
  the first-match fold, under `a <= b`, by two structural inductions:
  a symbolic peel identity, then `F(a,b) = R(a,b)`. Evidence:
  `cargo run -p sir_mech --example scan_induction` proves and replays
  in **0.000 s** (cached per process). This replaces the bounded SAT
  first-match query (29.6M instructions, extent cap 128) in the search
  family; the old `first_match_laws` table was deleted. Base-14
  498/266/0, all-19 Embench 99/221/0/1, fuzz 540/360/0, 9 tests
  ~1.36 s; pipeline/corpus behavior is unchanged apart from removing
  the proof bottleneck.
- **2026-09-23 — pointer-valued memory + same-address differential**:
  `load ptr`/`store ptr` now lower as pointer-typed `ArrayAccess`
  elements (with `null` as a typed zero), so intrusive lists/trees and
  pointer tables can be compiled. When any lowered function in the
  module touches pointer memory or returns `ptr`, the driver switches
  to a same-address sequential differential: one buffer, restart
  snapshot between the reference and emitted runs, so stored pointer
  bytes are directly comparable (pointer-carrying buffers start zeroed
  because random bytes are not valid addresses). All-19 Embench:
  **106 clean / 209 refused / 0 mismatched / 6 invalid-state** (was
  99/221/0/1); picojpeg 10 → **12**, sglib 6 → **11** clean, both
  0 mismatched; the new errors are structured-input cases where the
  zero state is not a valid list/tree, not miscompiles. Base-14 stays
  498/266, fuzz 540/360, 9 tests ~1.36 s; picojpeg is
  sanitizer-clean and `sglib_ilist_add` is clean on AArch64.
- **2026-09-23 — pointer-memory hardening**: two follow-up findings
  from the same-address runs. (1) Acyclic CFGs with branch-guarded
  memory through pointer operands were falling through to the
  straight-line walker, which executed every path unconditionally
  (sglib `dllist_add_after` dereferenced a null head and faulted); they
  now fail closed until pointer-memory predication exists. (2) Call
  arguments were not split for the escaping-GEP analysis, so a GEP used
  as a call argument was passed as its loaded value instead of its
  address (sglib `hashed_ilist_add` called the callee with null); call
  arguments are now scanned properly. All-19: **107 clean / 212
  refused / 0 mismatched / 2 invalid-state** (was 106/209/0/6); sglib
  12 clean / 1 invalid-state, 0 mismatched. Base-14 stays 498/266,
  fuzz 540/360, 9 tests ~1.36 s.
- **2026-09-23 — pointer-base path predication**: the acyclic path
  enumerator now keys memory by pointer-parameter bases and by
  entry-block pointer loads (stable across all paths), in addition to
  globals, when offsets are constant. Dynamically loaded pointer bases
  (GEPs into pointers loaded after the entry block) still fail closed.
  slre `op_len` moves to clean (sanitizer-clean); all-19 Embench is
  **108 clean / 211 refused / 0 mismatched / 2 invalid-state** (was
  107/212/0/2). Base-14 stays 498/266 and fuzz 540/360; 9 tests
  ~1.36 s.
- **2026-09-23 — guarded dynamic pointer-base predication**: the path
  enumerator now predicates loads and stores through pointer values
  that differ per path by selecting a scratch (`__sir_pred_dummy`)
  address when the path guard is false; memory entries carry both the
  value and the base pointer, and `finish` merges them with path
  selects. A small internal `sir_ptrcase` intrinsic keeps pointer
  types canonical across const/mutable spellings, and pointer-typed
  locations share one canonical memory key. All-19 Embench:
  **110 clean / 207 refused / 0 mismatched / 4 invalid-state** (was
  108/211/0/2); sglib 14 clean (+`dllist_add_after` and one more), and
  the new invalid-state entries were stack-checked to fault in the
  **reference** first (null/structured inputs), not in emitted code.
  Base-14 stays 498/266, fuzz 540/360, 9 tests ~1.36 s.
- **2026-09-23 — pointer-typed global scratch initialization**: driver
  globals declared as `ptr` are no longer random bytes (invalid
  addresses); they are set to a shared 4 KiB scratch buffer before each
  side, so pointer-table functions run in a defined state. qrduino
  `ismasked` becomes clean (native, AArch64, sanitizer-clean), and the
  qrduino invalid-state error disappears. All-19: **111 clean / 207
  refused / 0 mismatched / 3 invalid-state** (was 110/207/0/4).
  Base-14 stays 498/266, fuzz 540/360, 9 tests ~1.36 s.
- **2026-09-23 — reference-side probing + zero-state fallback**: the
  generated driver accepts `--ref-only` and `--zero`; the harness first
  runs the reference alone under random state, then under zero state if
  it faults, and either runs the full differential in the state where
  the reference is defined or classifies the input as invalid-state
  (reported as a refusal with an explicit reason). This makes
  `sglib_iq_first_element` a real clean differential case (native,
  AArch64, sanitizer-clean) and removes the last harness errors from
  the probe set. All-19: **112 clean / 209 refused / 0 mismatched / 0
  harness errors** (was 111/207/0/3). Base-14 stays 498/266, fuzz
  540/360, sanitizers 0, 9 tests ~1.36 s.
- **2026-09-23 — unguarded rotated zero-test do-while**: loops shaped
  `do { ... } while (next != 0)` with a straight-line unconditional
  entry now lower with a carried run flag whose initial value is the
  loop termination and whose successor is the test on the carried
  output. Any guard edge that can bypass the loop keeps the old loud
  refusal, so pre-tested shapes are not forced into one iteration.
  picojpeg `copyY`/`convertCb`/`convertCr` become clean (native,
  AArch64, sanitizer-clean); all-19 Embench is **115 clean / 206
  refused / 0 mismatched / 0 harness errors** (was 112/209/0/0).
  Base-14 stays 498/266 and fuzz 540/360; 9 tests ~1.36 s.
- **2026-09-23 — guarded carry-test do-while**: the run-flag model now
  covers arbitrary latch comparisons on carried values, with the run
  successor evaluated **before** the carried updates and stored in the
  first loop-tuple slot (extract indices shift by one). Entry guards
  are found through unconditional preheader chains, and any other
  pre-loop edge to the exit keeps the loud refusal. This adds qrduino
  `modnn` and wikisort `Reverse` as clean functions (native, AArch64;
  modnn sanitizer-clean). All-19 Embench: **117 clean / 204 refused /
  0 mismatched / 0 harness errors** (was 115/206/0/0). Base-14 stays
  498/266, fuzz 540/360, 9 tests ~1.36 s.
- **2026-09-23 — gate covers all 19 Embench corpora**: auditing the
  interrupted 08:59 lowerer work found that the unfinished guarded
  rotated zero-test lowering *mis-lowers* `sglib_dllist_get_first` and
  `sglib_dllist_get_last` (differential mismatch with `rewrites=0`, so
  the fault is in lowering, not in a family rewrite). The committed tree
  refuses both loudly; the interrupted state traded a refusal for a
  silent mismatch under the same shape. It stayed invisible because
  `scripts/verify.sh` gated only four external corpora — the other
  fifteen were outside the referee entirely. The interrupted work is
  preserved on `wip/guarded-rotated-zero-test` (never lost again) and
  reverted from `current`; the gate now runs **all 19** corpora native,
  and all 19 under sanitizers and AArch64 in `full`, and warns
  explicitly when corpora are absent. Lesson recorded: a corpus outside
  the gate is a corpus that cannot fail, and a loud refusal is worth
  more than a speculative acceptance.
- **2026-09-23 — arithmetic operand-position containment**: an audit of the
  restored arithmetic family found the constant-left case unsound.
  `recognize` swapped operands when the constant sat in the left slot, which
  is valid for commutative `Mul` and invalid for `Div`/`Rem`: `udiv 8, x` was
  rewritten to `x >> 3` (x=1 gives 0, not 8) and `urem 8, x` to `x & 7` (x=3
  gives 3, not 2). The law was never wrong — `prove::divide_pow2_laws` binds
  its free variable to the left slot — the *application binding* was. Latent
  rather than live: no corpus in `gate6a/` or `/tmp/external/` contains a
  constant-left `udiv`/`urem` (verified by grep), but the lowerer does map
  `udiv i32 8, %x` to `Div{const, x}`, so it was reachable. Contained by
  refusing the constant-left slot for non-commutative nodes; `Mul` keeps the
  commuted form. Regression case
  `constant_left_division_is_refused_and_commutative_mul_still_fires`
  (confirmed red before the fix, green after).
- **2026-09-23 — driver input generation and gate honesty**: two defects in the
  referee itself. (1) The differential driver disabled the zero input for every
  function in a module that contained the token ` undef` anywhere — a
  raw-text module-wide test — so one `undef` phi in `crc32.ll` removed the zero
  case from all six crc32 functions; it is now a per-function test on that
  function's own IR. (2) `verify.sh`'s `run_case` printed
  `PASS <label> no-summary` for a run that exited nonzero without an
  `EMITC_SUMMARY` line, and never inspected `clean=0`; both now fail. Known
  remaining gap, recorded rather than fixed: zero is only generated for scalar
  parameter index 0 (`k == 0` is the only branch reading `lens`), so a length
  passed as any later parameter never sees zero.
- **2026-09-23 — unresolved: timing-driver mismatches on rewritten functions**:
  the timing harness reports `EMITTIME_MISMATCH` for `crc32pseudo`,
  `getHuffTable`, and `getHuffVal` under every buffer fill tried, while the
  differential passes 200 random cases for the same functions. The cause is
  *not* established. An earlier attribution to a zero-length divergence was
  wrong: `crc32pseudo` takes no parameters, and the timing driver's own
  `lens[]` is dead code (its only consumer is `lens[4] + keys[...]`, so it
  never exercises zero either). Until this is explained, no timing number from
  that driver is reportable. Open investigation.
- **2026-09-19 — narrow-counter Kernighan proof**: a 7-bit counter
  encoding cut the width-32 bounded proof from 35.6s to 16.5s, but that
  is still pipeline-infeasible; the experiment was reverted. Production
  widths still need the kernel induction proof.
- **2026-09-23 — effects masks on synthesized memory reads**: five synthesis
  sites built nodes that read memory while declaring `Effects::empty()`: the
  `sir_sum_u*`, `sir_count_*` and `sir_memchr` intrinsics, the definition-200
  select chain's per-index `ArrayAccess`, and the `All` family's `ArrayAccess`
  — the last two through `push` helpers that hardcoded the mask. An omitted
  `READ_MEMORY` is invisible in this pipeline: the patch still applies and
  still runs, it just understates the node to every analysis downstream.
  The masks now match what the builder declares for real loads. The remaining
  root cause is narrower than first recorded here: `compute_effects`' default
  arm gives `ArrayAccess` the empty mask, so a lowered array access is
  indistinguishable from pure arithmetic. An earlier version of this entry also
  claimed `Loop` propagates nothing and that the recognizers' loop-level guards
  are therefore inert — **that was wrong**. `Builder::r#loop`
  (`sir_builder/src/builder.rs:806-812`) unions the effects of every body
  member, so a loop whose body stores carries `WRITE_MEMORY`, and the
  loop-level guards are live. The per-node check is what covers carried inputs,
  outputs and termination, which the body union does not reach.
  `patch.rs` gains a floor next to the existing cap — a patch node whose kind
  reads memory must carry `READ_MEMORY` — so a future omission refuses the
  patch instead of hiding. Shown to have teeth by reverting only the `All`
  mask, which failed the suite (`All must optimize`), then restoring it
  byte-identically. Verified: build clean, 9 tests 0 failed, `verify.sh fast`
  `VERIFY PASS` with external totals unchanged (117 clean / 204 refused / 0
  mismatched / 0 harness errors), so no family stopped firing.
- **2026-09-23 — opt-level survey: reach and firing collapse at defensible
  levels**: 19 Embench benchmarks built at `-O2` and `-O3 -march=native`
  (0 sources failed to compile) and swept with `--cases 1 --measured-retain
  --profile-out`; a fresh `-O1` re-measure with the current binary reproduces
  the committed gate exactly (117 clean / 204 refused / 0 mismatched), so the
  three levels are comparable. Accepted functions fall 117 → 75 → 74, functions
  seen 321 → 245 → 247, firing 7 → 2 → 2, and **every profile file at -O2 and
  -O3 is empty**. Every firing in the entire study is definition 102
  (multiply→shift) — no popcount, Any, All, parity, sum, search, divide or
  modulo definition fires anywhere at any level. The dominant refusals at the
  strong levels are exactly the loop shapes clang emits there: 41 "loop with a
  separate latch block (multi-block back-edge) not modeled" (early-exit
  searches), 20 "multiple loops sharing an exit CFG", 9 recursive call cycles,
  5 rotated-loop-exit forms, 5 non-constant switch indices, 5 `@strlen`,
  4 unresolved-GEP/mul operands. The `SIR_OPT_LEVEL` confound is ruled out: at
  `SIR_OPT_LEVEL=2` on the -O2 corpus the profile is still empty (clean=75
  refused=170, rewrites=2), so the collapse is coverage and firing, not a level
  handicap.
  **The only retained, rewrite-attributable win in all 57 runs** is
  `edn jpegdct`, definition 102, 4 rewrites, 6,062,328 → 5,723,516 ticks =
  1.0592×, and it is at `-O1` on a baseline no reviewer would accept. Every
  other large ratio has rewrites=0, i.e. codegen noise in both directions: the
  best are noise (nsichneu `initialise_benchmark` 2.676×) and so are the worst
  (statemate `generic_FH_TUERMODUL_CTRL` **787× slower**, `interface` 121×,
  `generic_KINDERSICHERUNG_CTRL` 10.3×) — an emission-quality problem, not a
  rewrite regression. `retain=new` is granted on ties, so most of the 30/32
  "wins" at -O2/-O3 are ≤0.05% noise.
  Consequence for the plan: two pre-registered C2 gates fired without needing
  the 3-arm run — "rewrites fire rarely / mostly refused → reach and lowering"
  and "Arm B slower than clang → emission/lowering quality first". A measured
  win report cannot be built on Embench until (1) the timing harness models
  initialized globals and bounds synthetic pointer arguments, (2) the lowerer
  covers the canonical -O2/-O3 loop shapes, (3) the baseline is a real -O2/-O3
  clang build. Recorded for the reach worklist: `tarfind::benchmark_body`
  refuses with `cannot resolve lhs '%15' in %10 = icmp eq i32 %15, 5`.
- **2026-09-23 — Kernighan recognizer: the missing effect guard was a live
  miscompile**: `popcount.rs`'s `recognize_loop` was the only family without the
  source-side effect guards the other eight apply — the loop-level
  `(loop_node.effects - READ_MEMORY).is_empty()` plus a per-node check over the
  shape's nodes. Without them a Kernighan loop carrying a store,
  `while (x) { x &= x-1; *p = count; count++ }`, was rewritten to `Popcount`:
  the `Loop` disappeared, and the orphaned `Store` survived `patch::sweep()`
  (which seeds its worklist from every non-pure node) with its count operand now
  constant 0. The program loses a per-iteration store loop and becomes a single
  straight-line store, and `patch::apply`'s verifier accepted it. Both guards
  are now in place; the per-node check chains `termination` explicitly, as
  `search.rs` does, because lowered loops do not carry their termination inside
  `body`, so without it the check would be decorative. Teeth shown by isolating
  each half: with neither, the store case is rewritten (assert fails at
  `runtime_loop.rs:131`); with the loop-level check alone, an effectful *carried
  input* still slips through (assert fails at `runtime_loop.rs:168`) — the loop
  node is pure there while the carried value carries `WRITE_MEMORY`, which is
  why both halves are needed. Cost: zero. `verify.sh fast` still
  `VERIFY PASS` with external totals byte-identical (117 clean / 204 refused /
  0 mismatched / 0 harness errors), 9 tests 0 failed, no family lost a firing.
  Cases live in `tests/cases/runtime_loop.rs` behind the existing `#[test]`, so
  the test count stays at 9.
- **2026-09-23 — emission: SIR's branchlessness flattens branchy functions
  60–380× slower**: the catastrophic `retain=base` ratios are real,
  emission-caused, and untainted. Untainted because every global in
  `statemate`, `edn` and `wikisort` is zero-initialized, so the missing-shadow
  asymmetry was byte-identical on both sides; and because `--allow-defs 999`
  (rewrites off) reproduces the same ratios, all three functions having
  rewrites=0. Callgrind instruction counts per call, 100,001 calls a side:
  `interface` 59 → 10,493 emitted instructions (**60.5×**), then
  `generic_FH_TUERMODUL_CTRL` **380.6×** and `generic_KINDERSICHERUNG_CTRL`
  **73.7×**. The mechanism is the frozen IR design: SIR has no branches, so
  every CFG merge point becomes a `Select` on the value *and on the address*,
  and the emitted function is one flattened, loop-free select DAG — **zero
  back-edges and zero indirect branches in every emitted function**, so clang
  cannot reconstruct a single loop. `interface` is the cleanest case: 33 IR
  ops, 2 `phi`, 17 `br` become 9,147 SIR values (277×), 439,645 bytes of C,
  3,587 ternaries and 79,911 bytes of x86, against a 59-instruction reference.
  **Corrected by the incidence measurement below**: those figures reproduce the
  **-O2** build. The `-O1` build of the same function is worse (592,931 bytes of
  C, 12,225 SIR values, 60 → 6,309 instructions), and the "33 IR ops" figure
  could not be reproduced from any `.ll` build — the `-O1` and `-O2` IR both
  contain 62–63 ops and 17 branches.
  The signature in the emitted C is address materialization feeding a select at
  every store site (`v27 = (v20 ? v22 : v26)`) plus 532 identity selects of the
  form `(v112 ? v82 : v82)`. **Not a cut regression**: the deleted `emit_c_ll.rs`
  was also SIR→C — it lowered a `.ll` and emitted rewritten C — so the
  flattening is inherent to the design rather than lost by the cut. Caveat on
  the absolute numbers: the reference side is strongly state-dependent
  (`ref_ns` for FH moved 2.4× between runs), so the instruction ratios are the
  defensible figures, not the tick ratios. Consequence: whole-function emission
  is not a viable output path for branchy code, and the timing harness currently
  measures "ABSAC as a whole-function compiler" rather than "ABSAC as a region
  rewriter".
- **2026-09-23 — reach scoping: the drop is mostly inlining, and class B is the
  one class worth implementing**: two corrections to the survey entry above.
  (1) Its per-class refusal counts are inflated by call-chain duplication — the
  harness logs one line per lowered *root*, so `benchmark`, `warm_caches` and
  friends each emit a refusal naming the same inner function. De-duplicated by
  inner function, the top classes are **36/37 distinct** for the separate-latch
  block class and **16/23** for the multi-loop-shared-exit class at -O2/-O3,
  against 52/51 and 41/42 log entries. (2) Those classes do **not** explain most
  of the -O1 → -O2 acceptance drop: 23 of the 46 functions accepted at -O1 but
  not -O2 are simply absent from the -O2 corpus because clang inlined them away,
  so only 23 are genuinely refused. Inlining, not rejection, is the larger half.
  `sir_lower` measures **11,469 lines in a single `lib.rs`** with no in-crate
  tests; that is the LOC overrun.
  Ranked by (acceptance gained at both levels) / (LOC cost), the class to
  implement is **B — multiple loops sharing an exit CFG**: strongest acceptance
  precedent (10 of 42 refused at -O3 were accepted at -O1, against 1 of 52 for
  class A), 75–78% of its work items sit in the counted-loop morphology the
  families consume, it holds the only function with a measured retained win
  (`edn/jpegdct`, 0.9441, refused as class B at -O3), and it **extends an
  existing pass** rather than adding machinery — a single boolean,
  `!guarded_entry` at `lib.rs:3950-3954`, excludes 15 of 16 work items at -O2
  and 19 of 23 at -O3 from a sequential emitter that is already written.
  Class A is larger in raw volume (36 vs 16 distinct functions) but is the wrong
  place to spend lines: it needs a genuinely new multi-block loop-body structurer
  (est. 500–1200 lines), three quarters of its members are pointer-chasing sglib
  structures no family can consume, and its acceptance precedent is 1 in 52.
  **Acceptance is not firing.** Measured recognizer contract: 8 of the 10
  families require an `ArrayAccess` over a `Type::Array` base with
  `fact.trip_count == Some(extent)`; `search` additionally requires
  `carried_inputs.len() == 3`, `popcount` requires 2, and only `arithmetic` has
  no array precondition. That narrows the domain and is why class B, not class A,
  is where reach pays.
  Before writing class-B code, a distinguishing run is needed to split the single
  refusal string at `lib.rs:4091` into its three documented causes (nested/
  shared-exit shapes, unresolved cross-loop values, a second return), so the
  80–350 line estimate can be pinned down. The gate at 3950 is named for
  *guarded entry*, which is the same ground as the parked
  `wip/guarded-rotated-zero-test` branch that mis-lowered
  `sglib_dllist_get_first`/`get_last` — so why that gate exists must be
  established before it is relaxed.
- **2026-09-23 — referee repair: globals modelled identically, both paths fail
  closed**: `emit_c_diff.rs` now models module globals the same way in the timing
  and differential drivers, and refuses what it cannot faithfully reproduce.
  (1) The timing reference is passed through `shadow_globals` — the step it was
  missing — so both sides read the injected buffer instead of the reference
  reading the module's real table. (2) `inject_globals` seeds shadows from the
  module's real initializer for flat integer arrays and string initializers,
  verified byte-for-byte (2048/2048 for `@crc_32_tab`), and **fails closed**
  with `unsupported: global @X has element type ... this driver cannot
  reproduce` for nested, aggregate and pointer shapes. (3) Every shadow is
  enlarged by a 256-byte guard band (`GUARD_BYTES`) so an out-of-range index
  reads zeroes that nothing writes between the two calls; the literal
  `struct { data; pad }` alias form was rejected because ASan instruments the
  alias at its declared extent and reports `global-buffer-overflow` on the pad
  read. Residual limit, stated plainly: the band is 256 bytes, so a larger
  overrun can still reach a neighbouring object. (4) The timing path now uses
  the same reference-fault probe and zero-state fallback as the differential
  path, so `sglib_iq_first_element` — a SIGSEGV inside the *reference* call,
  ASLR-dependent — becomes a deterministic `state=zero` comparison rather than
  a crash, and a refusal (`invalid-state input`) if both states fault.
  Evidence: picojpeg at `--cases 200` goes from `mismatched=54` on both
  functions to `clean cases=200`; the before-state was reconstructed by
  textually reverting the three hunks and rebuilding in an isolated workspace,
  so the defects it reproduces are genuine rather than artifacts of a missing
  parser. `verify.sh fast` → `VERIFY PASS` with external totals byte-identical
  (117 clean / 204 refused / 0 mismatched / 0 harness errors).
  **Cost, recorded**: four functions lose their timing comparison to the
  fail-closed initializer rule — `picojpeg::getHuffTable`/`getHuffVal`
  (pointer-typed switch tables) and `verify_benchmark` in matmult-int and
  wikisort (aggregate initializers). Two of those are rewrite-firing functions,
  so aggregate and pointer initializer support is a named target now, not a
  nicety. **Also recorded**: at the committed gate's `--cases 3` the phantom
  picojpeg index never lands out of range (15 clean both before and after), so
  the gate could not have caught that defect — the input set was too thin as
  well as the code being wrong.
  **Correction to the record**: an earlier note here said the harness contained
  no xorshift global fill and that the claim was unverified. That was wrong.
  `build_driver` refills every shadow with an xorshift stream before *both*
  calls (`emit_c_diff.rs:773-778`), symmetric by construction; the grep that
  "confirmed" its absence could not see the `gs ^= gs << 13` form. Same failure
  mode as the earlier family-firing claim: the search method, not the search
  result, was the problem.
- **2026-09-23 — flattening incidence: concentrated, and disjoint from the
  rewrite targets**: measured across all 266 accepted (level, function) pairs.
  **No rewrite-firing function is flattened.** At `-O1` all seven emit 0.97×–1.14×
  the reference statically (0.94×–1.09× dynamically), and `edn/jpegdct` — the one
  measured retained win — emits *fewer* instructions than the reference (190 vs
  195) while firing 4 rewrites. Every function above 5× has rewrites=0: the
  firing set and the blowup set are **disjoint**.
  The blowup is concentrated, not diffuse. At `-O1`, 81 of 117 accepted functions
  have **zero** branches, 95 of 117 have ≤2 branches and no switch, and 67 of 117
  emit exactly the reference instruction count. Two functions —
  `statemate::interface` and `statemate::generic_FH_TUERMODUL_CTRL` — carry
  **94% of all excess instructions**. Binned by branch count the cliff is sharp:
  0 br worst 2.00× (4 of 81 above 1.5×), 1–2 br worst 1.38× (none above), 3–6 br
  worst 1.58× (1 above), 7–16 br worst 10.94× (3 of 4), 17+ br worst 105.15×
  (3 of 3). SIR values per source op separate the same way: every firing function
  is ≤2.25, the catastrophic ones ≥3.3.
  So the remedy is narrow — **refuse the few functions whose emission blows up**,
  not rework emission. Two metric cautions established: the static instruction
  ratio is a **floor** for state machines with short hot paths
  (`generic_KINDERSICHERUNG_CTRL` is 1.94× static but 19.15× dynamic, because the
  reference exits early while the branch-free emission runs all 380 instructions),
  and it misleads the other way at `-O3`, where a 4-instruction reference body
  makes `depthconv::DepthwiseConvPerChannel` read 13.25× static against 1.02×
  dynamic.
  This narrows, and partly retracts, the claim in the emission entry above:
  whole-function emission is viable for the branch-light functions the rewriter
  actually targets; the catastrophic cases are heavily branchy state machines
  that no family can consume in any case.
- **2026-09-23 — termination-effect chain: checked, not needed (negative
  result)**: eight families omit `termination` from their per-node effect guard,
  and the question was whether the omission can change a verdict. It cannot.
  Every family refuses an effectful termination on its own recognizer *pin*
  first — `fact.trip_count != Some(extent)` (`all.rs:155`, `any.rs:138`,
  `count.rs:155`, `parity.rs:149`), `MAX_UNROLL` (`sum.rs:128`), `find_lt_bound`
  (`runtime_sum.rs:127`, `runtime_count.rs:173`), `bound_guard` (`search.rs:277`)
  and the `Ne`-against-zero pin (`popcount.rs:165`). `estimate_trip_count`
  accepts only Lt/Le/Gt/Ge/Eq/Ne terminations, recursing `BoolAnd`, and no
  effect-carrying SIR node kind is a comparison — so an effectful termination
  always yields `trip_count = None` and the pin rejects it long before the effect
  check could matter. `src/` is therefore unchanged; `termination` stays out of
  the chain as non-load-bearing. A synthetic probe that stamps `WRITE_MEMORY`
  onto a pure comparison's termination does show the guard is blind to a
  *stamped* node, but it was deliberately not shipped: no front-end produces one,
  and shipping it would encode the blind spot as expected behaviour. Cases for
  the pure-twin control live in `runtime_loop.rs` behind the existing `#[test]`.
- **2026-09-23 — OPEN (fix in flight): a `BoolAnd` termination silently drops its
  non-`Lt` conjunct**: the same audit found that a loop whose termination is
  `BoolAnd { Lt { i, 64 }, ArrayAccess(loaded i1 flag) }` — every node pure —
  gets `trip_count = Some(64)` and is rewritten by `sum` as definition 400. If
  the flag is false the source loop runs zero iterations while the rewritten
  unrolled sum adds all 64 elements: a miscompile of a defined program.
  `.chain(once(termination))` cannot catch it, because every node involved is
  pure and the discarded conjunct is a pure `ArrayAccess`, invisible to any
  effect guard. The missing check is structural, in
  `estimate_trip_count`/`find_lt_bound`: a non-counted conjunct must be proven
  constant-`true` or the trip count must be `None`. Reachability is latent today
  — the clang `-O1` shapes that produce fused flag terminations are all
  lower-refused (`unguarded post-tested loop`, `separate latch block`) and the
  external differential reports 0 mismatches — but the hand-written pre-tested
  shape does lower and is rewritten, so under the standing correctness policy the
  affected transformation is blocked until the applicability condition is fixed.
- **2026-09-23 — class B re-scoped: the gate is necessary, not binding, and the
  parked bug was elsewhere**: an investigation of the `guarded_entry` gate at
  `lib.rs:4065` (`git log -S` finds exactly one commit, 39568b2, whose entry says
  only "Guarded-entry sequential groups (`benchmark_body`) remain fail-closed")
  reports it as a fail-closed *routing* decision, not a response to a measured
  miscompile — it predates the parked branch by three days.
  The parked `wip/guarded-rotated-zero-test` failure was **not** a gate defect:
  it is an exit-value/phi-edge mismatch in the do-while model. The carried
  pointer is advanced in the body and the post-advance value is returned, while
  the source exit phi takes the pre-advance value (`%9 = phi ptr [ null, %1 ],
  [ %4, %3 ]`), so on a terminal single-node list the reference returns `p0` and
  the parked lowering returns NULL — one iteration off.
  **The soundness criterion this exposes is unchecked by the current code as
  well as by the parked branch**: the run-flag/do-while model is sound only if
  every post-loop use of a carried value refers to the successor (output) value.
  No evidence yet of a reachable case on the current tree — the shapes that trip
  it are refused at `lib.rs:8762` (`guarded rotated zero-test loop`) — but the
  criterion is unstated in the code and worth a deliberate test.
  Reach re-estimate: the already-written sequential emitter is reachable
  **exactly once** in the whole three-level sweep, and that one reach failed on a
  shape test ("latch is not a constant-step counted test") unrelated to the gate
  — a record of 1 reach, 0 successes. For 7 of the 47 class-B raisers the entry
  is already unguarded, so the gate is not binding for them at all; the honest
  ceiling is ≤6 at -O1, ≤15 at -O2, ≤19 at -O3 rather than 23 at -O3. Whether the
  gate is binding for the remaining 40 is **not established** — the harness
  cannot currently distinguish "gate blocked" from "detector returned None", so
  an instrumented run is required before any relaxation, and a reach delta well
  below the ceiling is the expected outcome.
  Split of the `lib.rs:4091` refusal (47 distinct raisers; 0 a second return,
  20 nested/shared exit, 24 unresolved cross-loop values as a residual bucket),
  plus a **third, undocumented cause in 3 of 47**: the self-latch phi predicate
  (`', '.join(operands).contains("%"+label)`) over-matches when a label is a
  prefix of a value — label `28` against value `%281` — inventing a spurious
  self-latch block; nettle-aes `_aes_set_key` shows the converse false negative.
- **2026-09-23 — emission-blowup refusal: three catastrophic functions now fail
  closed**: `lower_function` refuses when lowering produces more than
  `8 × source_instructions` SIR values (plus 32 values of slack for micro
  bodies), reporting `unsupported: emission blowup: lowering produced N SIR
  values from M source instructions (X.Xx, bound is 8x plus 32 values of slack)`.
  The bound is calibrated on the 266 recorded lowerings rather than chosen: the
  whole rewrite-firing set tops out at exactly 3.00 SIR values per instruction,
  so 8 leaves a 2.7× margin above everything the rest of the pipeline exists to
  consume, and sits just below the smallest recorded catastrophic ratio (8.33).
  It refuses exactly three functions at every level — `statemate::interface`
  (194.4×), `statemate::generic_FH_TUERMODUL_CTRL` (28.5×) and
  `sglib-combined::sglib_dllist_delete` (9.2×) — captures 96.7% of the recorded
  excess emitted instructions, and refuses **zero** of the 236 recorded
  lowerings whose emitted code is within 1.5× of the reference. Cost: external
  `-O1` clean 117 → 114, refused 204 → 207; base-14 byte-identical. Verified
  independently after the change: all seven rewrite-firing functions still report
  `clean` with their rewrites intact (`crc32pseudo` 1, `fir_no_red_ld` 2,
  `jpegdct` 4, `getHuffTable`/`getHuffVal` 1 each, `sglib_iq_first_element` 1,
  `matmult-int::initialise_benchmark` 2).
  Two limits, recorded rather than papered over: the bound is a *ratio*, so
  `depthconv::DepthwiseConvPerChannel` at `-O3` (13.25× emitted, ratio only 1.64
  because the reference itself is 4 instructions) is not catchable by it; and
  `lower()` — the inner function `lower_function` wraps — was public and
  unguarded, so the guard was bypassable by construction. `lower()` is now
  `pub(crate)`, making the gated path the only way to obtain SIR.
- **2026-09-23 — profile record decoupled from timing success**: `--profile-out`
  previously recorded a function only when its *timing* comparison succeeded, so
  a timing failure erased the firing record — which is why every profile file
  came back empty at `-O2`/`-O3` despite two functions firing per level. The
  record is now written whenever a rewrite fires and its differential outcome was
  `Clean`, independently of timing, with a new `unknown` retention marker. That
  marker is inert on round trip and had to be made so explicitly: without a
  dedicated `--profile-in` arm, any non-`0` third field reads as *allow*, so a
  marker the harness wrote itself would have silently granted permission on the
  next round — verified, `102 unknown` → rewrites=0, `102 1` → 1, `102 0` → 0.
  Format and field order are unchanged for entries that carry a ratio. Mismatch,
  sanitizer and refused outcomes still record nothing: a mismatching rewrite is
  not a legitimate firing, and the definitions are not threaded through those
  variants.
- **2026-09-23 — the emitted C was not well-typed; the native path only appeared
  to compile it**: the full matrix failed `external-a64/edn` because the emitted
  C for `jpegdct` initialised a `const uint16_t *` carrier from a
  `const uint8_t *` expression. clang downgrades incompatible-pointer-types to a
  warning; the AArch64 cross-GCC 14.2 makes it an error — so native validation
  had been passing on a leniency the stricter toolchain does not grant, and
  `clean` on the native path meant "clean as judged by clang", not "well-typed".
  Cause: the loop carrier is declared from the loop's *output* type while its
  initialiser comes from the *carried input*, and for `jpegdct` those are two
  independent pointee inferences of the same address (the lowerer's
  opaque-pointer inference assigns pointee width per use site). SIR's verifier
  checks only that `carried_inputs.len() == outputs.len()`, never that the two
  types agree, so the mismatch is verifier-invisible.
  **Ruled cosmetic, not a miscompile**, on three independent grounds: every use
  of the carrier is cast-based byte-offset arithmetic whose element type matches
  the SIR `ArrayAccess` node and the source LLVM at that site; the declared type
  is the one the in-loop back-edge and every post-loop consumer already use, so
  only the initialiser was mistyped; and with the diagnostic downgraded by a
  PATH shim (no repo change) the AArch64 differential for `jpegdct` runs
  `clean=1 mismatched=0`. The fix is 6 lines in `emit.rs` — cast the initialiser
  to the declared output type. **No compiler flag was relaxed**: per the standing
  policy, silencing a diagnostic that might be reporting a wrong-width access is
  the failure mode being removed, not a fix.
  Blast radius swept exhaustively: 165 emitted translation units × 3 compilers
  (clang 19.1.7, gcc 14.2, aarch64-gcc 14.2) = 495 compilations, zero non-zero
  exits after the fix; before it, exactly 4 TUs diverged and every one was
  `jpegdct`. Full gate `VERIFY PASS mode=full`, zero FAIL and zero WARN, with
  native, AArch64 and sanitizer totals identical section for section
  (base-14 249/18/0/0, external 114/207/0/0) — the cross-check that the change
  altered no behaviour.
- **2026-09-23 — OPEN: the emitted C is not byte-reproducible**: running the same
  harness binary twice on the same input produces textually different emitted C
  for `statemate` and `sglib-combined` (`crc32` is deterministic). In
  `verify_benchmark` it is line order; in
  `generic_BLOCK_ERKENNUNG_CTRL`/`generic_KINDERSICHERUNG_CTRL` the contents
  bound to `v395`/`v398` and `v0`/`v4` swap. After normalising all digit runs and
  sorting, the two outputs are identical and the differentials stay
  `mismatched=0` in both runs, so the programs are the same up to node numbering
  and print order. `ref.ll` and `driver.c` are byte-identical across runs, so the
  variation originates upstream of the emitter (NodeIds are creation-ordered and
  `emit.rs` prints them verbatim without creating nodes). Not root-caused.
  Recorded because it means **"the emitted C is byte-reproducible" is not a safe
  assumption for any gate that hashes emitted output**.
- **2026-09-23 — hypothesis killed: the base-typing precondition is not the
  blocker, and `-O2`/`-O3` hold no prize for these recognizers**: every loop in
  the 19 external corpora at all three levels was traced gate by gate (57 runs,
  a probe mirroring `sum::recognize_loop` gate for gate; the probe was deleted
  afterwards). The hypothesis that the families'
  `ArrayAccess`-over-`Type::Array` precondition is structurally unreachable is
  **false**: the lowerer does produce it — 13 `ArrayAccess` nodes over
  `Type::Array` at `-O1` across 4 functions (`createWinogradQuant`, `fir`, `mac`,
  `vec_mpy1`), all with `trip_count == extent` — and **no loop anywhere fails at
  a base-type, extent-range or trip_count-versus-extent gate**. `edn::mac` is the
  demonstration: a genuine 150-trip counted dot product whose base *is*
  `Array { I16, 150 }` with `trip_count == Some(150)`, refused two gates later —
  its accumulator is initialised from memory, and its closure holds two
  counter-indexed accesses over different bases.
  What actually binds, across all 64 loops (45 at `-O1`, 11 at `-O2`, 8 at `-O3`),
  is **the read-only effects gate and the single-stream recognition model**:
  `loop-effects+body-effects+no-element-reduction` 45,
  `loop-effects+body-effects+no-counter-sum-with-unit-invariant` 10,
  `no-element-reduction` 3, `no-counter-sum-with-unit-invariant` 3,
  `element-accumulator-init-not-zero+no-unique-array-access-on-counter` 1,
  `no-unique-array-access-on-counter` 1,
  `loop-effects+body-effects+no-unique-array-access-on-counter` 1. The families
  refuse any loop whose body writes, which is most real loops, because the
  accumulator lives in memory rather than in a carried value.
  **Prize, bounded rather than asserted**: an induction `add` compared by an
  `icmp` on a cycle occurs in 33 loops / 20 functions at `-O1` (11.3% of 177
  accepted), 10 / 7 at `-O2`, 7 / 5 at `-O3`; adding "at least one body
  `ArrayAccess` indexed by a carried input" leaves 7 loops / 5 functions at `-O1`
  (2.8%), 1 / 1 at `-O2`, 1 / 1 at `-O3`; adding the read-only gate leaves **3
  loops in 3 functions at `-O1` (1.7% — `mac`, `fir`, `fir_no_red_ld`) and zero
  at both `-O2` and `-O3`**. The cause is measured, not inferred: clang
  auto-vectorizes these reductions before lowering — SIMD `<N x T>` types are
  present at `-O2`/`-O3` and absent at `-O1` — so the levels a defensible
  baseline would use are already beyond these recognizers' reach on this corpus.
  Candidate fixes: relaxing the base precondition in `sir_opt` (~40–60 lines
  across five sites) changes the firing set by **zero** — the best candidates die
  elsewhere — and it would force the extent to come from `fact.trip_count`, which
  `loops.rs` describes in-code as "Very rough"; widening `sir_lower`'s promotion
  pass buys nothing, because the `Array` base is already produced wherever the
  morphology exists. The investment the evidence indicates is the recognition
  model — multi-stream maps and write-carrying loops — but only against a corpus
  where those shapes survive compilation, which Embench at `-O2`/`-O3` is not.
- **2026-09-23 — correction to the entry above, and the claims register**: an
  external review of the course recommendation challenged the inference drawn in
  the entry above, and the challenge is correct. "Nothing fires at `-O2`/`-O3`"
  is a statement about **pass placement**, not about the opportunity present in
  the code: a scalar-loop recognizer handed fully vectorized IR is being asked to
  reconstruct what the vectorizer already consumed. That entry concluded "the
  addressable set is the bottleneck" from comparisons that always matched
  optimization levels on both sides — which conflates the value of the rewrite
  with the cost of whole-function emission, and forces a choice between a weak
  baseline (`-O1`, where rewrites fire) and an empty firing set (`-O3`, where the
  baseline is honest). The arrangement that actually measures the product claim —
  scalar input, **strong** final backend applied to ABSAC's output, strong
  baseline — had not been run when that entry was written. It is running now,
  and its honest bound is already known: at the scalar position only three loops
  in three functions were reachable, one of which (`fir_no_red_ld`) already
  measures 0.914×.
  **Wording discipline, adopted**: the safety work is described as *the
  identified defects are contained and the specified regression matrix passes
  under the repaired harness* — never "the referee is now sound". Refusal counts
  and coverage changes stay visible beside every zero-mismatch figure, because
  zero mismatches after narrowing acceptance is a legitimate result and is not
  an increase in optimization coverage.
  **Claims register**: every retained performance claim must name its source
  snapshot, command, configuration and raw result. Historical numbers measured
  through the pre-repair harness are **superseded until reproduced**, including
  the three-arm and firing figures quoted above. Contradicted rather than merely
  stale, and corrected here explicitly: `perft/README.md` in the old tree shows
  Redis BITCOUNT at "ABSAC 1.76×" (64.90 ms against a 114.23 ms baseline); that
  table was committed one commit *before* the first real measurement and its
  numbers appear in no run. The measured result is 185 ms against clang -O3's
  79 ms — a loss of roughly 2.3×, and slower than not rewriting at all.
  **[Corrected 2026-09-23: the "185 ms against 79 ms" figure was never a real
  A/B. The witness's own `NOTES.md` read "Status: NOT YET MEASURED" for the clang
  delta. The kernel-level measurement that replaced it gives 2.16×, an emission
  effect from one line of `emit.rs` — see the `redisPopcount` entry below. The
  magnitude was right by coincidence and for a different reason.]**

- **2026-09-23 — S2 containment: a live miscompile in the unrolling families, and
  the guard that had been ported once and lost.** A probe
  (`/tmp/s5probe/probe.ll`, `probe_s2_index_scalar`) exercises `values[i] > i` —
  a reduction whose predicate scalar is *derived from the induction variable*.
  **Pre-fix**: the `any` family rewrites it, and the witness built against the
  emitted C prints `emitted=1 ref=0` against a `0/0` control: the unrolled chain
  collapses the scalar to its initial value (`bool v465 = (v463 > (0ULL));`).
  Reproduces from a clang `-O1` source path (`/tmp/s5probe/csrc/s2.ll`).
  **The referee could not see it, and that is the finding.** `emit_c_diff` fills
  buffers with random bytes, so `values[i] > i` is true on practically every
  case on both sides: `clean cases=8 rewrites=1`, `mismatched=0`. A differential
  referee whose inputs are random is blind to a predicate that is
  overwhelmingly true — *a check that cannot fail has not checked anything*, in
  the one place where the check is the product. The shape only became visible
  when `sum` was probed with the same construction: `values[i] + i` unrolls to
  `values[k] + 0`, which loses the `i` term on **every** case, not just a crafted
  one — `MISMATCH cases=8 mismatches=8 rewrites=1`.
  **Root cause.** `Loop` has no phi nodes: the lowerer reuses the carried-input
  node as the *in-body* name of the carried variable. Read inside the body it
  denotes the current iteration's value; read from outside the loop it denotes
  the initial value. A family that hoists a body expression into an unrolled
  chain and copies a carried node therefore substitutes the initial value for
  every iteration. This is the old tree's **S2** class; its
  `certify_invariant_scalar` (`sir_semantics/src/binding.rs:802`) was not among
  what the recovery ported. S1 (accumulator identity unbound) *was* ported, as
  `any.rs`'s `if !is_zero(function, reduction.variable)`.
  **Localization.** With `--allow-defs 999` (all rewrites disabled) the emitted C
  is the correct `while` loop and the witness prints `emitted=0 ref=0`; with
  rewrites enabled the scalar collapses. The defect is in the **rewrite**, not in
  lowering or emission. No change was needed in `sir_lower` or `emit.rs`.
  **Fix — the check, not the file.** One shared certificate,
  `patch::carried_names(loop_node)`, returns the ids under which a loop's carried
  variables are visible inside its body (`carried_inputs ∪ outputs`). Five
  families refuse to copy one out of the loop: `any`, `count`, `parity`, `sum`
  (in `instantiate` / `clone_with_index`) and `search` (def-200 select chain).
  `all` takes a narrower route: its `instantiate` is rooted at the `Select`
  whose arm *is* the accumulator, and that arm is already pinned to the identity
  constant, so a blanket refusal there would have cost every legitimate `all`
  firing; instead `cond_reads_carry` walks the predicate only, stopping at the
  element access (a leaf whose index operand names the carry and which
  `instantiate` replaces wholesale). `popcount`, `runtime_count` and
  `runtime_sum` need no guard — they emit a helper call over base/counter/bound
  and never re-instantiate a body expression.
  **The element-access substitution is the one sanctioned exception** in the
  guarded families, and it is above the refusal in every `instantiate`: the
  access is replaced with a fresh index, so it drops the carry rather than
  copying it. That ordering is load-bearing.
  **Firing cost: measured, and zero.** A/B over 34 corpora (base-14 + 19
  external Embench + heldout) with `SIR_OPT_TRACE=1`, pre-guard
  (`/tmp/reach/identity.log`, `/tmp/reach/id/`) against post-guard
  (`/tmp/reach/identity2.log`, `/tmp/reach/id2/`): **every per-corpus
  `EMITC_SUMMARY` line and every per-function `SIR_OPT` trace line is identical**
  — 170 firings on both sides, and the per-definition histogram matches exactly
  (def 102 ×33, 405 ×26, 404 ×25, 400 ×20, 0 ×19, 401 ×16, 101 ×9, 402 ×6, 100 ×4,
  5 ×3, 403 ×3, 4 ×3, 103 ×3). The five guarded families keep every firing they
  had: `count` (def 0) 19, `sum` (def 401) 16, `any` (def 4) 3, `all` (def 5) 3.
  Nothing was widened to keep firings, and nothing had to be traded.
  Post-fix the defect probe reports `probe_s2_index_scalar: clean rewrites=0`
  with its parameter-scalar control still firing (`probe_param_scalar:
  rewrites=1`), and the sibling probes refuse the S2 shape while firing the
  control for `count`/`parity`/`all`.
  **Regression case.** `sir_opt/tests/cases/carried_scalar.rs`, wired into the
  existing `families_rewrite_and_prove()` dispatch — **no new `#[test]`**, per the
  standing instruction. It collects all five refusals before asserting so a
  failure names every family that fired, and it asserts the parameter-scalar
  control still unrolls, so a blanket family kill cannot pass it. Falsified:
  with `carried_names` returning the empty set the report reads
  `[("any", true), ("count", true), ("parity", true), ("all", true),
  ("sum", true)]` — all five snippets do reach their family, so none of the five
  assertions is vacuous.
  **`verify.sh fast`**: `VERIFY PASS mode=fast`, workspace tests 10 passed / 0
  failed, all 14 base-14 and all 19 external corpora `mismatched=0
  harness_errors=0`, manifest 119 files.

## Claims register (2026-09-23)

Every retained performance claim names its evidence. A claim whose raw artifacts
are not held, or whose measurement predates the referee repair of 2026-09-23, is
marked accordingly — that repair changed what a `clean` result means, so anything
measured before it is **superseded until reproduced**.

| # | Claim | Source snapshot | Configuration | Raw result | Status |
|---|---|---|---|---|---|
| 1 | Differential matrix green: base-14 249/18/0/0, external 114/207/0/0, fuzz 540/220, sanitizers 114/207; native == AArch64 == sanitizers | `current`, the committed tree | clang 19.1.7, aarch64-linux-gnu-gcc 14.2, `SIR_OPT_LEVEL` default 1 | `sir/scripts/verify.sh full` log plus `tmp/verify/manifest.sha256`, 118 files | **validated** on the committed tree |
| 2 | Emission-blowup guard refuses exactly 3 functions, refuses 0 of 236 benign lowerings, captures 96.7% of excess emitted instructions | 266 lowerings over the -O1/-O2/-O3 corpora | bound `8×` plus 32 values of slack | `/tmp/vrfy_incidence/{rows.json,measure.py}` — **outside the repo** | measured; raw data unpinned |
| 3 | `edn::jpegdct` 1.0592× — the only retained rewrite-attributable win | `/tmp/external/edn.ll` at `-O1` | clang 19.1.7 | level-survey logs under `/tmp/fire2/out/` | **superseded — predates the referee repair; re-measure before quoting** |
| 4 | Old tree Gate 3 reproduced: k18 5.95×, k18-random 141×, k43 4.70×, k50 8.22× against clang -O3 | old tree `head` = `2cae1fd`, rebuilt at `/tmp/old_sir` | clang -O3 on both sides | `/tmp/gate3_rerun/out.txt` — **outside the repo** | reproduced 2026-09-23; raw data unpinned |
| 5 | ~~Redis `redisPopcount`: 185 ms against clang -O3's 79 ms — a loss of about 2.3×, and slower than not rewriting~~ | Redis `unstable`, `src/bitops.c` | clang -O3 | `/tmp/redis_bench/` — **outside the repo** | **WITHDRAWN 2026-09-23 — never a real A/B.** The witness's own `NOTES.md` read "Status: NOT YET MEASURED" for the clang delta. Superseded by the measurement below; the magnitude (2.16×) was right by coincidence and for a different reason (emission, not recognition). |
| 6 | Reachable loops: 3 in 3 functions at `-O1`, 0 at `-O2` and `-O3` | 19 Embench corpora at three levels | probe mirroring the recognizer gates; probe deleted after use | **raw data deleted** — its own task removed `/tmp/vrfy_recon` | **unpinned; must be re-derived before being quoted as evidence** |
| 7 | `perft/README.md` shows Redis BITCOUNT at "ABSAC 1.76×" | old tree `perft/README.md` | — | contradicted by #5; committed one commit before the first real run | **contradicted — superseded, not merely stale** |
| 8 | SWAR definition 8: 4.000× on the isolated kernel (20.0000 → 5.0000 Ir per 32-bit word), and **1.000×** at `-O1` without `-mpopcnt` and at `-O3 -mpopcnt` | the emitted C from `/tmp/absac_pc` probe `v3_swar` at `-O1`, against its own pre-rewrite emission | clang 19.1.7; callgrind slope `(Ir(8)−Ir(4))/4` over 1 MiB, best of 3, `taskset -c 0` | `/tmp/swarcg/{tu_raw.c,tu_rw.c}` and the built binaries — **outside the repo** | measured 2026-09-23; **raw data unpinned, and fires on no corpus** — quote it only with the concession that the backend already knows the idiom at `-O2`+ |

Three follow-ups this table makes concrete:

- **#3 and #6 must be re-measured under the repaired harness** before either is quoted again. #6 in particular is load-bearing for the current conclusion about reach, and its backing data no longer exists.
- **#2, #4 and #5 have their raw artifacts outside the repo**, so a future session re-deriving them starts from a command rather than from data. Where a claim matters, the producing script belongs in the tree.
- **Every ratio in #1 and #2 travels with its coverage and refusal counts.** The external `clean` count fell from 117 to 114 when three catastrophic functions began refusing — zero mismatches after narrowing acceptance is a legitimate result and is not an increase in coverage.

- **2026-09-23 — the pass-placement control: built, honest, and null**: the
  arrangement that measures the product claim was never built, and this was
  verified rather than assumed — `emit_c_diff.rs` at `head` has no timing mode at
  all, `bench_all.rs` is the only committed timing harness and compares the
  original source at `gcc -O3` against ABSAC's emission at `gcc -O2` with **no
  rewrites-off control**, and `gate5a_bench`/`gate6b_run` compare something else
  again. Every prior timing comparison therefore conflated the value of the
  rewrite with the cost of whole-function emission.
  The control now exists: **A** = original source at `clang -O3`, **B** = the raw
  lowering emitted with the optimizer skipped entirely, **C** = the optimized
  emission; all three in one `clang -O3` invocation with identical flags and
  target settings, one address per data symbol, three-way return-value agreement
  checked **before** any timing, interleaved balanced rounds, per-side min-of-6,
  five process repetitions.
  **Result: the rewrite adds nothing measurable at the strong-backend position,
  and the emission costs.**
  `crc32pseudo` C/B = 0.9996, C/A = 0.9985 — a no-op.
  `fir_no_red_ld` C/B = 1.0026 (inside the ±1% rep spread) while **B/A = 1.0727**:
  ABSAC's emission costs about 7.3% against the source build and the rewrite
  contributes nothing.
  `sglib_iq_first_element` C/B = 1.0492 — the *optimized* emission is 4.9% slower
  than its own unoptimized emission — but it is floor-dominated (~4.5 ns/call)
  and measured on zero input only, so it is low-confidence.
  `jpegdct` C/A = 1.5952: a large loss, and it is an emission-quality loss, not a
  rewrite loss (B does not build for it — see below), so the rewrite's own share
  there is unmeasured.
  **The null is about the arithmetic family specifically**, which is the only
  family that fires on this corpus at any level. The domain families never fire
  here, so their value at this position is **untested, not disproven** — there is
  nothing to measure. Reaching them would require the reach changes already
  retired for having no counted target.
  Blockers, each reproduced with its exact message: `jpegdct`'s B side fails to
  compile — `error: use of undeclared identifier 'o0_0'` — a **pre-existing bug
  in `emit_c`**: the unoptimized emission declares a loop-carried output inside
  the enclosing loop's block and uses it after the blocks close. It is left in
  place and would block the same measurement for any function shaped like it.
  `initialise_benchmark` refuses in every timing mode (`local calls are not
  supported yet`). `getHuffTable`/`getHuffVal` have no callable A at all —
  clang inlined both away at `-O3`.
  Whole-benchmark timing is **not reachable** in this harness and was not built:
  it needs module-wide emission with function-granular source fallback (207 of
  321 functions refuse, and `emit_c` emits no type declarations), real drivers
  and inputs per benchmark, and triple-linking — roughly 400–700 lines plus
  per-benchmark glue. The recommendation was not to build it on this evidence,
  since the rewrite-attributable signal is zero to slightly negative at function
  level and the recognized functions are a small fraction of each benchmark's
  runtime.
  Caveats: absolute figures are TSC ticks on one host (bimodal standalone), so
  only within-process interleaved minima are treated as evidence; A's own spread
  is up to ~2%, which is the resolution limit of the small ratios above; and the
  measurement is a hot, constant-input loop, not a cold working set.

- **2026-09-23 — headroom survey (interactions): one recurring opportunity, and
  it is not where a C compiler can reach it**: three candidates, each measured
  against the project's own strong baseline, with the source revision pinned
  (SQLite amalgamation 3.53.4 SHA3-256 verified; lz4 1.10.0 at `ebb370ca`).
  **lz4 block codec — null**: 97.23% of instructions sit inside the single
  inlined compressor, a flat single-pass probe loop, no materialized
  intermediate and no second consumer of the same bytes. The "a compiler could
  fuse the per-block setup" hypothesis does not survive measurement.
  **lz4 streaming window — positive but bounded**: contiguity between the carried
  window and the incoming block is established by the stream state and silently
  destroyed when `LZ4_saveDict` materializes it into a side buffer, which flips
  the matcher onto its slower extended-dictionary path (`if (matchIndex <
  startIndex)` appears only in that profile). A contiguous sliding window with
  one-time `LZ4_loadDict` seeding and periodic compaction runs 15.02 ms against
  17.53 ms — **1.16–1.17×**, callgrind −16.2% instructions, ratio unchanged.
  Stated honestly: it does **not** beat lz4's own best (`LZ4F_compressFrame`
  14.65 ms, `stableSrc=1` 14.75 ms); it beats the default path a caller without a
  stable buffer is forced onto, and the library's own documented flag already
  reaches most of that gap.
  **SQLite index intersection — recurring semantic opportunity**: a conjunction
  over two independently indexed columns is planned as one index scan plus a
  residual filter and a table fetch per candidate. Intersecting the two covering
  index scans instead measures **8.89×** (independently re-run here: 8.89× and
  6.52× against the survey's 8.93× and 6.51×), and the three-way conjunction
  6.52×. `EXPLAIN` confirms the mechanism — the natural plan is
  `SEARCH t USING INDEX ib (b=?)`, the witness is `MERGE (INTERSECT)` over two
  covering-index scans that fetches only the 103 surviving rows. For ranges
  (`a BETWEEN … AND b BETWEEN …`) the SQL-expressible alternatives reach only
  1.24× (`INTERSECT`, which degrades to two full scans and temp b-trees) and
  1.60× (`rowid IN`), while a hand bitmap reaches **10.51×** — a representation
  the engine does not have.
  **Two caveats keep this well short of a product claim.** For the equality cases
  the winning plan is already expressible in ordinary SQL: the engine can execute
  it and simply does not choose it, so that headroom is planner quality rather
  than missing capability. And the queries are constructed on a synthetic schema
  — no production query log was mined — so recurrence is argued from the shape
  and from other engines implementing it (PostgreSQL `BitmapAnd`, MySQL index
  merge, postings intersection), not from an observed workload. Selectivity
  crossover and bitmap scaling at 10^8 rows are unmeasured.
  The mechanism in this project's own terms: **both predicates recognise the same
  mathematical object — a set of rowids — in different representations, and the
  win comes from choosing how to combine the representations** (ordered merge
  when the streams share an order, bitmap when they do not). That is the thesis
  stated exactly, and it lives in a query planner rather than in compiled code.
  Together with the lz4 result, both positives are properties of how a program
  lays data out across stages, invisible to a C compiler; the one compiler-shaped
  candidate was a clean null.

- **2026-09-23 — headroom survey (Redis 7.2.5): the thesis confirmed from
  third-party code, and my own framing corrected**: three kernels, measured
  against clang 19.1.7 `-O3` at x86-64 baseline flags, tarball pinned
  (`98a8502a…`), `bitops.c` `2624690f…`, `hyperloglog.c` `f747bc0d…`, every
  ratio behind an exact-equality gate, best-of-21, pinned to one core. Wall clock
  on this box is untrustworthy — a co-tenant's jobs swung one binary 2.3×
  between rounds — so the load-bearing measurement is callgrind **instruction
  counts** via `--toggle-collect`, which are contention-immune.
  **A correction I have to take first.** I recorded earlier today that the only
  family firing on real code is redundant with clang at -O3, and that Redis
  `redisPopcount` lost to clang. The first is still true *of ABSAC's pipeline*;
  the conclusion I drew from it — that there is no headroom around these kernels
  — is false. clang emits **zero `popcnt`** for Redis's hand-written SWAR even at
  `-march=native`, on a CPU that has POPCNT; its own `__builtin_popcountll` beats
  the hand SWAR by **2.14×** at *identical* flags; `target("popcnt")` is 3.3×;
  an AVX2 nibble-LUT `vpshufb` path 6.9× (1 MiB: 90.4 µs → 27.4 µs → 13.1 µs,
  bit-exact at every size). The premise I was working from — "loops clang leaves
  scalar" — is wrong in **both** directions: clang *did* auto-vectorize the SWAR
  (to SSE2, 2.27 Ir/byte) and still lost 3.3–6.9× to the ISA path, while it
  genuinely left the BITPOS search and the HLL merge scalar even at
  `-march=native`. The profitable question is not "is the loop scalar" but **"did
  the compiler recognise the semantic operation"** — this project's research
  question, arrived at independently, from someone else's code.
  **`hllMerge` — largest finding, and the best evidence for the thesis.** Dense
  per-register merge is **92.36%** of `pfcountCommand`'s instructions on the
  canonical 4-key union (server p50 0.167 ms). Two independent misses:
  `if (val > max[i]) max[i] = val;` compiles to a *branch*, and when merging k
  independent HLLs the incoming register wins with probability ≈1/k — 25% at k=4,
  the worst possible prediction rate. Making the same loop branchless takes the
  merge from 145 µs to 36.5 µs (**3.98×**) with no other change. Separately,
  clang declines to vectorize even the counted byte-domain max — **0 `pmaxub`**
  emitted, remark `cannot identify array bounds`, which the 6-bit packing macro
  defeats. Unpacking 8 registers per 6 bytes and taking `_mm_max_epu8` 16 at a
  time gives **7.1 µs, independent of the data**, against 145 µs — **20.4×** at
  kernel scope, with no new ISA and no flag change. The object is a pointwise max
  over a packed register array (a lattice join); the packing is what hides it.
  **Stated limits, which keep this far short of a product claim.** The 20.4× is
  *kernel scope*. There is **no server-level A/B** — the agent correctly refused
  to build a witness server for it, and the 5.8–8× command-level figure is an
  explicit extrapolation from instruction share × measured p50, not a
  measurement. The regime assignment (145 µs, not 28 µs) rests on a 6% timing
  coincidence, not on dumping the four real keys' registers. And there is a
  regime where the alternative **loses**: on 4× the same bitmap, branchless is
  0.79×. The microarchitectural attribution is inferred — `perf` is unavailable
  here, so no mispredict counter confirms it.
  **`redisBitpos` — positive, bounded, and explicit about when it is zero.** The
  skip loop is 99.91% of `bitposCommand`; clang's remark is `loop not vectorized:
  could not determine number of loop iterations`, identical at `-O3` and
  `-O3 -march=native`. A baseline-SSE2 block scan (`pcmpeqb` + `pmovmskb` + `ctz`)
  is 1.54–1.55×, AVX2 2.66–2.73×, flat across three orders of magnitude of search
  distance — per-step throughput, not the tail. The win is **zero** when the
  answer is near the start (distance 0: no win; the real 9 MB file: the scan is
  never entered). And explicitly *not* headroom, recorded as a negative:
  rewriting the scalar ≤64-iteration terminal scan with `__builtin_clz` is exactly
  neutral at every distance (24550 vs 24658 ns).
  **`hllDenseRegHisto` — deprioritised on measurement.** 1.75× per kernel
  (private-4 banking breaks the same-address RMW chain), but **below 1%** of
  PFCOUNT's instruction share on the profiled path, where the histogram is
  `hllRawRegHisto`. A 1.75× win on <1% of the work is worth ~0 end-to-end.
  Recorded because it shares the merge's mechanism.
  **Two things this does not establish.** zlib was never surveyed — the
  sub-agent's permission classifier blocked it and I did not route around that
  block — so this is **single-project evidence** and recurrence is untested. And
  the prior single-kernel result that an ABSAC witness's `redisPopcount` lost
  2.3× to clang does not appear in these measurements; the survey reports it as
  unreproducible rather than reconciling it, which is the right call. That
  reconciliation is a separate experiment, now in flight: if the emitter maps a
  `Popcount` node to `__builtin_popcountll` (`emit.rs:1420-1430`), then a loss on
  this kernel means either non-recognition or a non-`Popcount` replacement — and
  that distinction decides whether the project's failure so far is in reach or in
  representation choice.

- **2026-09-23 — `redisPopcount` through the pipeline: three separate failures,
  and the ledger's own "2.3×" was never measured**: run against Redis's kernel
  verbatim (`bitops.c` `2624690f…`, `diff`-verified) at `-O1` and `-O3`, rewrites
  on and off, with a Kernighan positive control.
  **The real kernel is never lowered at all.** `lower_refused (unsupported:
  multiple loops sharing an exit CFG (nested/sequential loops) not modeled)` —
  byte-identical at both levels. `redisPopcount` has three sequential loops (the
  alignment-head LUT, the 28-byte SWAR, the tail LUT), and the lowerer models
  neither nested nor sequential multi-loop CFGs. **B and C do not exist for it**;
  the only measurable datum is A. This is not a recognizer gap and not a search
  gap — for any such function there is no SIR, so nothing downstream can run.
  **Recognition is a second, independent gap.** The `popcount` family is silent
  on every Redis shape: `rewrites=0` at both levels, with and without
  `--allow-defs 999`, emitted C byte-identical between the two runs
  (`sha f4da6f34d5e2` tail, `f3c2d7422a18` swar). Reading `sir_opt/src/popcount.rs`,
  definition 7 matches only Kernighan — `carried_inputs.len()==2`, `Ne` against
  zero, `outputs[value_pos] == And(value, Sub(value,1))` — and **structurally
  cannot match a byte-table LUT or a SWAR mask chain**. The positive control
  confirms the pipeline works when the shape does match: Kernighan lowers,
  rewrites with `def=7`, and emits exactly `__builtin_popcountll`, beating clang
  -O3 by **4.6×** on instruction count (8908 → 1932 Ir/call).
  **Emission is a third, and it is one line.** `grep -c -i popcount` over every
  emitted file: 0 for the tail and SWAR variants, 1 for Kernighan. So the emitter
  maps a recognized `Popcount` to the intrinsic correctly (`emit.rs:1418-1431`)
  — for Redis there is simply no recognized popcount to map. But the SWAR shape
  *does* lower at `-O1`, and there the loss appears: **5.0000 Ir/byte emitted
  against 2.3125 for the original C at matched `-O3` — 2.16×**, and the gap
  vanishes completely when the pointer spelling is normalised, with identical
  outputs across all 21 lengths. Cause, confirmed by me verbatim at
  `sir_benchmarks/src/emit.rs:210-228`: `ptr_offset_expr` emits
  `((T*)((uintptr_t)(base) + (offset)))[0]` rather than the natural
  `((T*)base)[i]`. **LLVM's ScalarEvolution will not build an induction variable
  through a `ptrtoint`/`inttoptr` round-trip, so the loop vectorizer never
  fires** — isolated to a two-file experiment where that spelling is the only
  difference, the natural form produces **44 SSE instructions** and the
  `uintptr_t` form **0**. The cost is specific to vectorization and not to codegen
  in general: the tail LUT loop, which cannot vectorize anyway, is exactly equal
  in both spellings (3.7500 Ir/byte).
  **The narrower reading, and its limit.** The comment on that function states a
  real motive — a GEP index is a two's-complement offset that may wrap, and C
  pointer addition with a wrapped offset is UB even when the resulting address is
  in range — so this is a deliberate correctness trade, not an oversight. That
  makes the fix a **narrowing of the safe path, not a removal of it**: emit the
  natural form only where the index is *provably* in range, keeping `uintptr_t`
  for the wrapped case. The bound is being counted before any code is written.
  If that count is large, the implication is uncomfortable and worth stating: the
  domain families recognise counted, effect-free, single-stream array reductions
  — which are exactly the loops this spelling de-vectorizes — so the pipeline may
  be finding the right loops and then emitting C that clang cannot vectorize.
  That would account for both the A/B/C null (C/B ≈ 1.000) and the 7.3–60%
  emission cost. **Stated as a hypothesis to be counted, not a finding.**
  **Claims-register correction.** The "about 2.3× loss" recorded earlier today
  was **never a measured A/B comparison**: the witness's own
  `corpus/001_redis_bitcount/NOTES.md` reads "Status: NOT YET MEASURED" for the
  clang delta. The arithmetic does reconcile — 5.0000 / 2.2679 = **2.205** —
  but the magnitude was right by coincidence, and it is an *emission* effect,
  not a recognition effect, which is why it reproduced on a hand-lowered SIR
  while the recognizer was never involved. The entry is corrected in place
  rather than deleted. `emit_c_diff` reported `mismatched=0 harness_errors=0`
  throughout, and A == B == C for all variants across all 21 lengths including
  0; no miscompile was found in anything measured here.

- **2026-09-23 — headroom survey (zlib 1.3.1): the Redis gap recurs, once, in
  exactly the same shape — and the separating test is now explicit**: zlib 1.3.1
  (`9a93b2b7…`, self-computed — the `.asc` fetch 404'd, so this pins what was
  measured, not what upstream signed). Four targets, one win, three negatives,
  and the negatives are evidenced as carefully as the win.
  **Row 1 — `crc32.c:779` braid loop: headroom, 8.94× on instruction count.** The
  object is CRC-32/IEEE (reflected poly `0xEDB88320`); zlib computes it as a
  braided table walk, `N=5` braids × `W=8`-byte words, one 256-entry lookup per
  byte (`cmpq $0x2f` in `crc32.o` confirms the braid path is what compiled).
  PCLMULQDQ folding gives **0.422 Ir/byte against 3.775 — 8.94×**, and 6.50–6.78×
  on wall clock, behind an exhaustive gate (1,311,122 checks — every length
  0..4096 × every byte alignment 0..63 × 5 initial CRCs, plus streaming chains,
  0 fails). Scoped honestly: `crc32_z` is 2.37% of gzip *compression* but
  **18.77% of decompression**, so that is ~2.1 points and ~16.7 points
  end-to-end respectively — not the whole program.
  **Why no compiler reaches it, and why the source already knew.** The refusal is
  structural, not heuristic: `loop not vectorized: value that could not be
  identified as reduction is used outside the loop`. The braid recurrence is a
  polynomial reduction over GF(2), which is not an LLVM reduction kind. And the
  one x86 instruction that would name it, SSE4.2 `crc32`, computes **Castagnoli —
  a different polynomial — so it does not apply to IEEE CRC-32**. That is why zlib
  ships an AArch64 path (`crc32b`/`crc32x` *do* match IEEE, `crc32.c:598-660`)
  and **no x86 path at all**. What is reachable on x86 is `pclmulqdq`, which is
  polynomial-agnostic and needs the folding algorithm — which no compiler will
  synthesise. "Recognise crc32 and emit the instruction" is not the win here; the
  win is a **different algorithm computing the same object**.
  **The three negatives, each with its reason.** `longest_match` is 61.00% of
  compression and looks like the obvious SIMD target, but line attribution kills
  it: the byte-compare loop is **4.35% of `longest_match` = 2.65% of
  compression**, while the cost sits in the hash-chain walk
  (`prev[cur_match & wmask]` 20.3%, `s->window + cur_match` 13.5%, quick-reject
  20.6%, `--chain_length` 13.3%). That is a **data-structure cost, not a
  mis-represented object** — no representation change recovers it.
  `inflate_fast` is 79.91% of decompression with a **flat** profile: no source
  line above 4.06%, bit-buffer refill and Huffman decode and three byte copies —
  no object to name. Plus two premise corrections: stock zlib 1.3.1 contains
  **zero** SIMD constructs in `adler32.c` (the hand-written paths belong to
  zlib-ng / Chromium forks), and `crc32` has **no runtime dispatch** — the
  representation is chosen at compile time by `#if`, with the source saying so
  itself at `crc32.c:556-563`: *"This code does not check for the presence of the
  CRC instruction at run time."* Nothing to measure in either.
  **The separating test — the part worth carrying forward.** Across four projects'
  worth of positives and nulls, one criterion divides them cleanly: **does the hot
  loop compute a named algebraic object with a hardware or algorithmic shortcut,
  or is it walking a data structure?** Every algebraic object had large headroom —
  population count (2.14–6.9×), lattice join (20.4×), first-match search (2.7×,
  bounded), CRC over GF(2) (8.94×), rowid-set intersection (8.89×). Every
  data-structure walk was null — lz4's probe loop, zlib's hash chain, zlib's
  bit-level decode. That is 5-for-5, and it restates the research question in
  measurable form: "recover the mathematical object" *is* "tell an algebraic
  object from a data-structure walk", and only the former has a representation
  worth choosing.
  **Limits.** zlib recurred **once**, not broadly — a real constraint on any claim
  of recurrence. `-march=native` is not a win here: it emits zero target
  instructions and changes nothing measurable, because on x86 there is no ISA path
  in the source for it to reach (unlike Redis, where POPCNT existed and was
  ignored). Absolute clock and IPC are unverifiable in this guest (the reported
  3072 MHz is wrong by ~1.7×), which is why instruction counts are load-bearing
  and the wall ratios are corroboration. And the survey's own pre-compaction
  codegen scan was **vacuous** — it grepped GNU objdump for literal `pclmulqdq`,
  which objdump never prints (it emits immediate-specific pseudo-mnemonics), so
  the grep could never match; the agent caught it by counting opcode bytes and
  rebuilt the scan against 8 positive controls, which now fire 1 each. Recorded
  because it is this project's own rule — *a check that cannot fail has not
  checked anything* — applied to the survey that was doing the checking.

- **2026-09-23 — bound count: my emission hypothesis is REFUTED, the emission
  figure is withdrawn, and the biggest reach item was hiding in the refusal
  tally**: measurement only, 225 functions across 26 corpora (10 base-14, 16
  external), all work outside the repo. Flags read rather than guessed:
  `SIR_OPT_LEVEL` defaults to `"1"` and that single `opt_flag` (`-O{level}`)
  compiles the emitted C and the link (`emit_c_diff.rs:1830-1831`, `1905`), while
  the A side arrives as a **prebuilt** object via `--ref-object`
  (`emit_c_diff.rs:1676`) — so A and B/C are built by different invocations and
  are not level-matched by construction.
  **The hypothesis is dead.** I proposed that `ptr_offset_expr`'s `uintptr_t`
  spelling de-vectorizes the families' own loops, and that this explains both the
  A/B/C null and the 7.3–60% emission cost. Refuted by a self-contained
  experiment: taking **the same emitted C, the same driver, and changing only
  clang's level**, Multiply's raw@-O1 / raw@-O3 = 27,275,600 / 13,402,800 =
  **2.0351**, which is *exactly* its recorded B/A of 2.035. Essentially all of
  that deficit is the level difference; the spelling moves it by +0.23%. At the
  level the referee actually uses, the spelling is free — identical instruction
  counts in 5 of 6 kernels, **0 SIMD instructions in both spellings**. The
  vectorization does require the spelling *and* `-O3` (30/319/45 SIMD uses appear
  only there), but that is not the configuration any recorded number came from.
  **So the emission-cost figure is withdrawn, not explained.** B/A as recorded
  compares emitted C against a reference built by a different invocation, and is
  indistinguishable from a pure optimization-level difference. Everything resting
  on it — including the 7.3–60% range — must be re-measured at matched levels
  before it is quoted again. This is the **second** time this session that
  mismatched optimization levels produced a conclusion I then stated as fact; the
  first was the placement claim the advisor caught. The failure mode is evidently
  mine and repeatable, and the guard has to be mechanical: no cross-level ratio
  is recorded without the two levels written beside it.
  **The reach bound for the emitter fix is 3.1%, and it rests on an assumption
  the architecture cannot discharge.** 5,413 `uintptr_t` cast sites across 365
  emitted files — 99.2% of all subscripts, since the emitter's pure path already
  writes `base[index]` (`emit.rs:679-690`, used by the byte-search fast path, 46
  real natural subscripts). Funnel: 5,413 → 472 inside a loop → 193 affine in the
  loop's own 0-init/+1-step counter → 183 with a guard bound → **169 provably
  in-range (3.1%)**. Of those, **zero** have a static extent: globals are emitted
  as `extern uint8_t x[]`, so the extent lives only in `driver.c`. All 169 need
  the harness's parameter extent **assumed**. Under this project's correctness
  policy that is not a proof, and the honest verdict is **do not write it**: the
  one kernel it measurably helps at `-O3` (vec_mpy1, −34.7%) is offset by one it
  hurts (mac, +9.8%) at the same level, and it would churn the primary emission
  route for that. If the Multiply/vec_mpy1/fir gap is the real target, the
  one-line change is the referee's optimization level, not `ptr_offset_expr`.
  **The multi-loop defect is real but small — and it is not the biggest family.**
  Refusals by reason over 225 functions: unsupported 122, malformed call 26,
  unresolved-operand 21, unsupported external call 19, unsupported instruction
  14, recursive call cycle 7, switch CFG 5, soundness gate 4. Inside
  "unsupported", **the largest single family is a loop with a separate latch
  block — 75 of 225 refusals, 33%** — against **18 (8.0%)** for the
  sequential/nested multi-loop CFG that blocked `redisPopcount`. The biggest reach
  item is ordinary CFG coverage, not the exotic case, and nobody had counted it
  before today. The multi-loop fix is worth writing **as coverage, not for
  speed**: its four corpora (nettle-sha256 7, sglib-combined 5, ud 4, nettle-aes
  2) offer 10/317/12/8 cast-form sites and 4/0/0/0 strictly provable ones, so
  unblocking the lowerer there buys ~zero measured emission.

- **2026-09-23 — referee: the reference object's optimization level is now
  enforced** (`d214700`): `--ref-object` takes a prebuilt object whose level
  cannot be inspected, and `ref_object`'s doc comment *asserted* it matched
  while nothing checked — a documented invariant with no check, which is the
  shape this project keeps finding. `--ref-level N` is now required alongside
  `--ref-object` and must equal `SIR_OPT_LEVEL`; a deliberate cross-level
  comparison requires `--allow-cross-level`. `opt_level()` is now the single
  source of truth for a value the compiler invocation and the check both read,
  so they cannot drift apart. All four outcomes verified by hand: missing level
  refused, mismatch refused naming both levels, match passes, override passes.
  Refusals surface as `harness_errors`, which the gate already treats as
  not-green, so the guard fails closed without adding a test — relevant because
  the standing instruction caps the test count.
  **The verification caught a vacuous run first, and it is worth recording.**
  My initial test passed `--function crc32` to the `crc32` corpus and got
  `clean=0 mismatched=0 lower_refused=0 harness_errors=0` — every field zero,
  which I read as "the guard did not fire". It was not firing because **no
  function existed by that name**: the corpus defines `crc32pseudo`. A run that
  did nothing looked exactly like a guard that failed. Re-run against a function
  that reaches the timing path (`verify_benchmark`), it fires. This is the
  project's own rule arriving from the other side — a check that cannot fail has
  not checked anything, and neither has a *test* that cannot reach the check.

- **2026-09-23 — SWAR-popcount obligation: dischargeable, and the prover refuted
  my own statement of it**: the feasibility gate on a new `popcount` recognizer,
  run against `sir_mech` directly (`prove_equal` / `prove_implied_equal` →
  `CheckResult::{Valid, Counterexample(model), Unknown}`), driver outside the
  repo. **No case returned `Unknown` or timed out.**
  **Correction to my premise, and the prover made it.** I stated the obligation
  as "the three-step SWAR equals `popcount`". That is **false**, and the kernel
  returned a counterexample rather than agreeing: `x=0x80000000`, lhs=16777216
  against rhs=1, in 0.001 s. The three steps compute *packed per-byte counts*,
  not the total. Redis's real shape is steps **plus the fold**, so the recognizer
  must match the fold too — a bare-steps recognizer would be refutable. The
  positive form is `(steps * 0x01010101) >> 24 == popcount(x)`, **VALID in
  0.531 s** (CNF 1439 vars / 4875 clauses).
  **Width cost, and the fix.** The 64-bit form is VALID but slow, and the figure
  is **not stable**: the same query with a **bit-identical CNF** (5303 vars /
  18209 clauses) measured 295.5 s and 313.7 s across runs, and the width sweep's
  64-bit entry 302.5 s and 537.2 s — a 1.8× spread on an identical problem.
  The honest statement is "**~300–540 s, single-digit minutes**", not a precise
  cost; an earlier draft of this entry quoted 295.5 s as though it were one, and
  is corrected here. The likely cause is contention rather than solver
  nondeterminism — this is a 2-vCPU box that had two other jobs running
  throughout — **but that is an inference, not something the run establishes**,
  and a deterministic CDCL should not vary with time at all, so it is worth
  knowing if it recurs on an idle box. What the number is *for* does not depend
  on it: the 64-bit obligation **does discharge** inside the 2M-conflict budget,
  so it is slow rather than unprovable, and the cost is CDCL *search* rather than
  encoding size, since the CNF is only ~3.7× the 32-bit one. Posed decomposed it
  is **~5.8 s in every run** (packed-counts 0.011 s + byte-sum 2.558 s + fold
  3.231 s) — cheaper than the monolithic figure whatever that figure is doing.
  **Pose the 64-bit obligation decomposed.** A width sweep 8/16/32/64 of the same
  shape gives 0.001 / 0.010 / 0.524 / ~300 s (noisy).
  **Hypotheses are supported, and the byte bound is exactly the applicability
  condition.** `prove_implied_equal` takes assumptions: the fold is VALID for
  `bytes <= 8` (0.012 s), `<= 56` (0.046 s), `<= 63` (0.068 s), and
  **`<= 64` is a counterexample** (`v=0x40404040`, lhs=0 against rhs=256). With
  no hypothesis at all, counterexample `v=0xff800000`. The boundary is exactly
  63; redis's 7×8=56 sits safely inside it. **The multiply is not required** —
  a shift-add spelling of the fold gives *identical* CNF (447/1397) and is also
  VALID, as does an `extract(31,24)` form, so the recognizer can accept either
  spelling at no extra obligation cost.
  **Negative controls, because a proof harness that cannot fail has proved
  nothing.** Wrong mask `0x55555554` → COUNTEREXAMPLE `x=0x4c3ff907` (broken→19,
  popcount→18); `0x33333332` → COUNTEREXAMPLE `x=0xf400051e`; the 64-bit wrong
  mask → COUNTEREXAMPLE `x=0xfd561f7d4cfe0002`. All three independently
  re-verified against a reference implementation. And the sharpest one: a bad
  fold constant `0x01010100` *under the byte hypothesis* → COUNTEREXAMPLE
  `v=0x08000000` — the hypothesis does not mask a wrong constant, and **sampling
  had found nothing there; the kernel did.**
  **Why the VALIDs are proofs and not agreements.** `Valid` cannot arise from a
  guess: `equiv.rs` maps `Unsatisfiable → Valid`, budget exhaustion → `Unknown`,
  and a SAT model failing the caller-side re-check → `Unknown`. Evaluator
  differential (`Bv::eval` against an independent reference) PASS; encoder
  differential forcing every literal, multiply included, PASS at both widths;
  and 2,000,000 sampled values per width against an independent Python reference,
  0 mismatches.
  **The sequencing caveat, which is the part that decides when to write it.** The
  obligation is dischargeable, so the family is *writable* — but its only known
  site, `redisPopcount`, is still behind the multi-loop CFG refusal, so a family
  written today could not fire on any real program until that reach work lands.
  It *is* testable in isolation: the SWAR loop alone lowers at `-O1` (the
  probe's `v3_swar` variant). So this is a stepping stone with a testable target,
  not a capability that pays on its own.

- **2026-09-23 — the SWAR recognizer lands as definition 8, and it fires only
  where the backend does not already know the idiom.** `sir_opt/src/popcount.rs`
  gains `SWAR_POPCOUNT = 8` beside `KERNIGHAN_POPCOUNT = 7`, built on the
  feasibility gate above. `apply_one` is now
  `apply_kernighan(function).or_else(|| apply_swar(function))`; the loop idiom is
  tried first *because* it consumes the whole loop while the SWAR recognizer is
  value-local — and, as a side effect, no existing firing can change hands.
  **What is matched, steps and fold together.** The pattern is rooted at a `Shr`
  by exactly `W - 8` (any other shift is a different chain), then the fold, then
  the three steps read backwards: `(t + (t>>4)) & 0x0F0F…`, `(u>>2 & 0x33…35) +
  (u & 0x33…)`, `x - ((x>>1) & 0x55…)`. The fold is accepted in either of the two
  spellings measured in the gate, `Mul` by `0x0101…` on either operand order, or
  the shift-add tree with sorted shifts `0, 8, …, W-8` and no multiply. Widths
  16/32/64, unsigned only. Every constant is exact and the recognizer refuses on
  anything else: the masks and the fold unit are the same named constants
  (`SWAR_M1/M2/M3`, `SWAR_FOLD_UNIT`) the obligation is phrased over, so a
  deviation cannot be matched here and proved there.
  **The obligation, and its cost.** `prove::swar_popcount_laws(width, fold)`
  returns `true` only on `CheckResult::Valid` — a `Counterexample` or an `Unknown`
  refuses, and `apply_swar` calls it before building any `Patch`. It is cached
  per `(width, fold)` in a `OnceLock<Mutex<…>>`, as `kernighan_proved` is.
  Measured: 16-bit 0.010–0.021 s; 32-bit 0.526 s (multiply spelling) and 0.872 s
  (shift-add), CNF 1439 vars / 4875 clauses in both cases — *identical CNF, so
  neither spelling carries a cost penalty*. 64-bit is posed **decomposed**
  (`byte ≤ 8` on the step term, then packed bytes sum to `popcount`, then the
  fold lemma with the byte bound as hypothesis), ≈5.8 s in every run; the
  monolithic form is rejected on the cost the gate measured (~300–540 s) rather
  than on validity. Each lemma is discharged against the actual term before the
  composite is used. The byte-bound hypothesis is the applicability condition and
  is not free: the fold is refuted at byte bound 64 (`v = 0x40404040`).
  **The isolated kernel fires.** `/tmp/absac_pc`, probe variant `v3_swar` at
  `-O1`: `SIR_OPT v3_swar def=8`, `NATIVE v3_swar: clean cases=1 rewrites=1`,
  `EMITC_SUMMARY clean=1 mismatched=0 lower_refused=0 harness_errors=0`. The
  emitted C replaces the fold (`v29`, `v31`) with
  `uint32_t v43 = __builtin_popcount(v10);` — **`popcount`, not `popcountll`**:
  the emitter picks the spelling from the *operand's* width and the word is
  `i32`. (The task premise assumed `popcountll`; that assumption was wrong, and
  the `popcountll` arm remains what the 64-bit Kernighan corpus emits.) The dead
  SWAR chain text (`v12`…`v29`) **stays in the emitted C**: `patch::apply`'s
  `sweep` seeds every non-pure node and walks all of a `Loop`'s body members, so
  a dead body node is retained. clang removes it; `mismatched=0`.
  **Instruction counts, measured, and the win is narrower than the probe
  suggested.** callgrind slope `(Ir(reps=8) − Ir(reps=4)) / 4` over a 1 MiB
  buffer, best of 3, `taskset -c 0`, **Ir per 32-bit word** (the figures are the
  loop body exactly: static counts in `v3_swar` agree at 25/25, 25/10, 32/32):

  | compile of the emitted C | pre-change (SWAR chain) | post-change (builtin) | ratio |
  |---|---|---|---|
  | `-O1` | 20.0000 | 20.0000 | **1.000×** |
  | `-O1 -mpopcnt` | 20.0000 | 5.0000 | **4.000×** |
  | `-O3 -mpopcnt` | 2.7500 | 2.7500 | **1.000×** |

  All six binaries return the same sum over 1 MiB (`4196165`). The `-O1` row is
  a genuine negative result, not a measurement failure: clang 19.1.7 at `-O1`
  **without** `-mpopcnt` expands `__builtin_popcount` back into the SWAR
  sequence — both arms compile to 25 instructions with one `imul` — and at
  `-O2`/`-O3` clang recognises the hand-written chain itself (5 `popcnt` on both
  sides). So the rewrite pays only where the backend has POPCNT *and* does not
  apply its own idiom recognition, which is the same lesson as the
  `redisPopcount` measurement arriving from the other direction: what ABSAC
  contributes here is that the object was **recovered and proved**, and the
  speedup is contingent on the backend's own behaviour rather than caused by the
  rewrite.
  **Firing cost: measured, and zero.** `apply_kernighan` is the pre-change
  `apply_one` body verbatim (diffed body-against-body) and runs first. Measured
  independently: `SIR_OPT_TRACE=1` over 34 corpora (base-14 + 19 external +
  heldout) gives 170 firings and the histogram def 102 ×33, 405 ×26, 404 ×25,
  400 ×20, 0 ×19, 401 ×16, 101 ×9, 402 ×6, 100 ×4, 5 ×3, 403 ×3, 4 ×3, 103 ×3 —
  **`def=8` occurs 0 times**, and every per-corpus trace is byte-identical to the
  pre-change baseline `/tmp/reach/id2/`. A def-8 firing is the only way this
  change can alter an outcome, so zero firings means the per-function decisions
  are unchanged. (`heldout` reports `harness_errors=1` in the baseline too —
  pre-existing and untouched.) The pre- and post-change `verify.sh fast` logs are
  identical ignoring build durations. Nothing was widened to keep firings.
  **Regression case.** One function, `swar_popcount_chain_rewrites_under_definition_8`,
  behind the existing `run()` dispatch in `sir_opt/tests/cases/runtime_loop.rs` —
  **no new `#[test]`**, and the workspace count stays 10. It asserts definition 8
  for both fold spellings and refuses two negative controls: a one-bit mask
  deviation (`0x55555554`) and a one-unit fold-shift deviation (`>> 23`). The
  `families_rewrite_and_prove` case went from ~1 s to 1.53 s: it now pays the two
  32-bit proofs, cached per `(width, fold)`.
  **`verify.sh fast`**: `VERIFY PASS mode=fast`, workspace tests 10 passed / 0
  failed / 0 ignored, all 14 base-14 and all 19 external corpora `mismatched=0
  harness_errors=0`, manifest 119 files.
  **Not established.** The 16- and 64-bit recognition paths and their obligations
  are exercised by no corpus and no test — the case covers 32-bit only, and the
  64-bit obligation was discharged out-of-tree. The only known real-world site,
  `redisPopcount`, is still behind the multi-loop CFG refusal, so definition 8
  fires on no program in any corpus: the kernel above is a probe, not a corpus
  member. This is a stepping stone exactly as the gate predicted, and the
  instruction-count table says the step it enables is worth less than the ~2.14×
  the fixed-ISA probe had suggested.

- **2026-09-24 — hllMerge dense-merge checkpoint: the 20.4× belongs to a form SIR
  cannot represent, and the honest target is a scalar ~14×**: bounded feasibility
  and correctness-contract investigation, no repo changes. Redis 7.2.5
  `hyperloglog.c` dense merge (`hllMerge`, ~1080; 92.36% of `pfcountCommand`).
  **Q1 — ingest is flag-dependent, and the blocking refusal is the biggest one we
  have.** Genuine upstream IR at `-O1` and `-O3` gives `unsupported: loop with a
  separate latch block (multi-block back-edge) not modeled` — the same family that
  accounts for **75 of 225 refusals (33%)**, so the largest reach item and the
  chosen first target are the same blocker. At `-O0` it is
  `aggregate-typed getelementptr source '%struct.redisObject'`. A verbatim
  extraction refuses identically. **The decisive counter-result:** a
  semantics-preserving respelling compiled with `-fno-vectorize -fno-slp-vectorize
  -fno-unroll-loops` at `-O2` **lowers** — 34 SIR values, `Effects(WRITE_MEMORY)`,
  no family fired. So the semantic content is ingestible and the refusals are
  narrow codegen artifacts. But redis's own `make` uses default `-O2`, which lands
  on a third refusal (`rotated loop exit (back-edge on false) that is not the
  counted eq-next form`).
  **Q2 — what is missing, and the headline number dies here.** No `Max`/`Min`
  anywhere in `NodeKind` (the pointwise max exists in SIR only as `Gt` + `Select`);
  no pointwise-map shape in any recognizer (the reduction detector produces
  accumulator-carrying forms only); packed access is *fine* (`ArrayAccess` + `Shr`
  + `And` + `Convert`); and **there is no vector type** — `Type` has no SIMD
  variant and the emitter has none. **The measured 20.4× is `_mm_max_epu8`, so it
  cannot be represented in SIR at all.** Six loop families also refuse a
  store-bearing loop before any shape analysis, via an effects check at the top.
  **Q3 — the write gate protects an "authorized region" that does not exist.**
  `patch::apply` caps a synthesized node at `READ_MEMORY`, and its own comment
  cites an authorized region — but `Patch` is `{ nodes, replacements }` with no
  region, no base, and `apply` takes no authority parameter. Another documented
  invariant with no check. The cap is currently doing the work the removed alias
  analysis used to do, so scoping it means carrying that proof instead of a
  parameter: an authorized destination on `Patch`, an authority check, a
  non-aliasing obligation, and a `Store` arm in `replace_in_kind`.
  **Q4 — the obligation is cheap, and two of the candidate gates cannot fire.**
  Measured directly against `sir_mech`: the per-field unpack identity discharges
  for all four shift classes in **35 µs**; the whole-group 48-bit query in
  **15 µs**; the join laws (idempotence, commutativity, associativity) in
  **2.04 ms**. Comfortably within budget, and far cheaper than the SWAR precedent.
  Note the group's free window must be **7 bytes, not 6** — register 8j+7's top
  bits live in the next group's first byte. Three negative controls: an off-by-one
  extraction shift is **refuted with a model** (the gate fires); a width-mismatched
  comparison is **refused by assertion** (good); and applying the *same*
  truncation to both sides comes back **Valid — a check that cannot fire**, since
  only an independent right-hand side is a check.
  **The referee was tested against this target and partly fails it.** Rebuilding
  the exact driver fill and running deliberately wrong variants: `min`, an
  off-by-one shift, a neighbour index, and a wholly dropped store all **fire** on
  the random path — so this is *not* the `values[i] > i` situation, because arg0
  and arg1 land in different buffer slots. But **`--zero` is vacuous here**: every
  wrong variant, including `min` and a dropped store, is silent, because both
  operands are zero and the branch never fires. `--zero` must not be cited as
  evidence for this rewrite. And the referee's **buffer geometry is hard-coded**
  (`bufA[8][2048 + 32]`) while the dense register array is 12288 bytes — at real
  size gcc itself reports undefined behaviour, so the shipped referee cannot
  referee this target until its slot size is derived from the IR.
  **Q5 — aliasing is unestablished, untestable by the harness, and not
  harmless.** Upstream carries no `noalias` on either pointer, and the tree has
  **no alias analysis at all** — it went with the nine-analysis layer. The
  referee structurally cannot test it: the same-address mode is selected only when
  the return type is `ptr` or the IR contains a pointer load/store, and `hllMerge`
  returns `int`, so the harness always hands distinct storage. It matters because
  if `dst == src` the fused 8-byte read/write corrupts the next group's source
  bytes — so the rewrite needs an explicit non-aliasing applicability condition
  **plus a guard**, not an assumption.
  **Q6 — a correction to the survey record, and two hard constraints on the
  witness itself.** The 20.4× was earlier attributed to `alt_merge_unpackmax`;
  re-measured, that form is **1.42×/1.73×**. The 20.4× is `alt_merge_simd`. Since
  SIMD is unrepresentable, the honest target is the **scalar** fused form:
  `fused_cmov`, ~10.5 µs and **data-independent in both regimes**, giving
  **~13.7–13.9×** on the mispredict regime — and the branchless macro-based form
  that loses on the repeated-bitmap regime (0.76×) loses because *per-register
  extraction* costs, not because branchlessness does. Two measured constraints:
  **the fast form over-reads redis's real allocation** — ASan against the real
  geometry reports `READ of size 8 … located 0 bytes after a 12313-byte region`,
  because the register array is the tail of an sds; `bench5` never saw this
  because its buffers carry 8 bytes of slack, and redis does not guarantee slack.
  And kernel-scope ratios here are **layout-sensitive**: the same upstream
  function on the same bitmap measured 27.7 µs in one harness and 40.2–41.7 µs in
  another, reproducibly.
  **Verdict: contained for a scalar fast witness; broad reconstruction for the
  20.4× headline.** Minimum path: a new pointwise-map family (~600–900 LOC), the
  unpack/join obligation wiring (~80–120), the scoped write authorization
  (~100–200 plus a soundness argument), and a latch-block admission in
  `sir_lower` — the crate the project has already flagged as its LOC-budget risk,
  and per the standing rule that reach work needs a counted target first.

- **2026-09-24 — latch-block reach: the measured bound is ZERO firings, and 11 of
  the 75 refusals misreport their cause**: the counted target the standing rule
  requires before lowerer work. Nothing in the repo was modified.
  **Tier 1 — the 75 is really 36.** The tally counts refusal *rows*; the referee
  re-reports a refusal once per call site, so 75 rows are **36 distinct
  functions**, **0 from base-14, all from external corpora**. Six distinct reasons
  all originate inside `lower_early_exit_search` (`sir_lower/src/lib.rs:5791–6609`;
  the wrapper message at `:4240`). And it is **not one shape** — the 87 natural
  loops divide into three sub-shapes, of which one is not a latch shape at all:
  **A — empty self-loops: 7 functions / 11 rows.** These are `13: br label %13`, a
  block with **no phi**; `self_loop_blocks` uses a loose substring test against a
  phi's joined operands, so with no phi it stays empty and routing falls through
  to the latch path. **The message names the wrong cause for 11 of the 75 rows** —
  a reporting/routing defect, not a lowering gap.
  **B — separate latch with no induction variable: 16 functions** (pointer chases
  like `p = p->next`, data-dependent scans, md5's round loop). Lowerable in
  principle, but no family has anything to recognise.
  **C — genuine counted `0..N` step-1 loops with a separate latch: 13 functions /
  36 loops.** This is the only sub-shape an admission would actually help.
  **Tier 2a — how many would become lowerable: at most 36 loops / 13 functions,
  and realistically far fewer.** The refusals happen *inside*
  `lower_early_exit_search`, whose first hard check is
  `if h_phis.len() != 1 { return Err("early-exit search: expected one header phi") }`
  (`:5881`), while the counted loops that carry an accumulator have ≥2 phis (md5
  has 5). So a latch admission **cannot reuse that path** — it must be a new
  loop-lowering path. Of the 87 loops, 42 have exactly one header phi, 26 have
  ≥2, **19 have none**.
  **Tier 2b — how many would then FIRE a family: ZERO.** The funnel over all 87
  loops, with the six shape families' gates read off their sources:
  ```
  87 natural loops
   → 40 effect-free loop bodies      (47 fail the effects gate, applied pre-shape)
   →  4 with an induction counter    (36 effect-free loops have no 0..N step-1 counter)
   →  2 with a LITERAL constant bound (picojpeg's two are bounded by an argument)
   →  0 with a constant-identity accumulator
   →  0 with a single fixed-array access indexed by the counter
   →  0 pass all gates
  ```
  The four survivors were hand-verified from their IR and each fails ≥2
  independent gates; **all four are search/early-exit loops** — find-mismatch,
  find-nonzero — whose result is a found flag, not a reduction, so no family in
  the current set has a member for the shape. Stated as a caveat: this is an
  IR-level necessary-condition filter rather than a run of the families, and it is
  conservative in the over-admitting direction, so the 0 can only be an
  overestimate.
  **Tier 3 — cost.** A general path is **~600–900 lines** and must be a new
  function, not a widening: the single-phi assumption is threaded through all 819
  lines of the existing `lower_early_exit_search`, and 45 of the 87 loops have ≠1
  header phi. `sir_lower` is already 11,645 lines in one file and is the flagged
  LOC-budget risk, so this is a **5–8% addition to the file already over budget**.
  **Tier 4 — the guard that would be needed.** A check that can fail: assert the
  set of functions refused with the latch message shrinks by exactly the count the
  new path claims, and that **no previously-lowering function changes its SIR**.
  Without it, a widened path silently swallowing a function into a wrong lowering
  looks identical to a win.
  **Verdict: do not write it.** The honest accounting of the 75 rows: **11 are a
  reporting defect**, 16 are pointer chases no family covers, and 13 are blocked
  upstream at the accumulator gate rather than at the latch. The deeper finding is
  that the binding constraint is not reach at all — it is the families'
  applicability gates, and those gates are **load-bearing for correctness** (the
  constant-identity accumulator requirement is the S1 fix), so they cannot simply
  be relaxed. Reach work that admits loops the families still refuse buys a lower
  refusal count and nothing else.

- **2026-09-25 — what a scalar-C emitter can actually reach: the ceiling is the
  auto-vectorizer's pattern matcher, and the referee's default level sits below
  it**: measured by emitting the scalar form ABSAC would produce for each surveyed
  win and inspecting the assembly. Scan integrity first, because a sibling survey's
  scan was vacuous: controls fire (`pmaxub` 5, mask 6, clmul 1, popcnt 1) and the
  GNU-objdump trap is confirmed live — opcode `66 0f 3a 44` renders as
  **`pclmullqlqdq`**, so a literal `grep pclmulqdq` on objdump output can never
  match. Every CLMUL result was confirmed twice, by `llvm-objdump` and by opcode
  bytes, and the agent caught two of its own vacuous scans mid-run.
  **`hllMerge` — reachable with ZERO new emitter capability.** The emitter already
  spells the winning form: `Select` is emitted as a ternary (`emit.rs:655`,
  `:1592`), which is what clang's vectorizer wants. `pmaxub` counts on the three
  spellings: packed **0**, unpacked-`if` **0**, unpacked-ternary **3** — and it
  needs no `restrict`. The ternary form measures **0.993–1.007×** against a
  hand-written `_mm_max_epu8`, i.e. it ties the intrinsic.
  **But the realistic path is only 1.02×.** Measured end to end — unpack 5 packed
  registers, byte-domain max at k=4, repack — gives **1.02×** versus the packed
  fused form, because the unpack/repack is **96%** of the time and does not
  vectorize: `(r*6)/8, (r*6)&7` extraction is a shuffle, not a strided load. The
  byte-domain max phase alone is **49.82×**. So the win is reachable **only if
  the registers are persisted one-per-byte**, which is a whole-object
  representation choice and not an emission change — precisely the "choose the
  best representation" half of the research question, and a cross-consumer change
  rather than a loop rewrite.
  **CRC / CLMUL — NOT reachable, plainly.** Seven scalar spellings plus the real
  zlib braid, all **zero** `pclmulqdq` by both detection methods, at `-O2`, `-O3`
  and `-march=native`: a loop form, a branchless masked form, a 32-term unrolled
  form, the bit-spread integer-multiply trick, a generic GF(2) polynomial
  multiply, a nibble-table form, and a `__uint128_t` form. The bit-spread trick
  emits `imulq` — a different operation. The only x86 producers in clang's set are
  `__builtin_ia32_pclmulqdq*`, and the intrinsic route needs a SIMD type that
  `sir_types::Type` does not have: **~600–1000 LOC** across `sir_types`,
  `sir_builder` and `sir_benchmarks`. **This 8.94× is out of reach for this
  architecture regardless of family work.**
  **SWAR popcount — a flag effect that evaporates at `-O2`, and it reconciles the
  original loss.** `popcnt` counts: at `-O2` baseline **0** for both spellings; at
  `-O1 -mpopcnt` the builtin gets **1** and the hand SWAR **0**; at `-O2 -mpopcnt`
  the hand SWAR gets **7**; at `-O2 -march=native` **21**. So at `-O2` and above
  LLVM idiom-recognises redis's *own* SWAR into `ctpop` and emitting
  `__builtin_popcount` wins **nothing** — which is why the ABSAC witness lost to
  clang `-O3`, and the first coherent explanation of that loss to date.
  **Definition 8's 4.000× exists only in the `-O1 + popcnt` window**, a
  configuration nothing real uses. A per-function `__attribute__((target("popcnt")))`
  would make it flag-independent — verified to produce `popcntq` at baseline
  flags — but the emitter has **no attribute mechanism at all**
  (`grep -c '__attribute__' emit.rs` → 0), so it is new surface for ~10–20 LOC,
  and it must go on the emitted function rather than a static prelude helper.
  **`redisBitpos` — reachable in pure scalar C, zero new capability.** Clang never
  reaches the block-compare form from scalar C, but it does not need to: against
  the real upstream shape, scalar-32B is **2.13×** and scalar-64B **2.15×**, while
  a hand-written SSE2 block scan is 1.22× and a hand-written AVX2 one 2.95%. At
  baseline clang auto-vectorizes the scalar loops itself, and with `-mavx2` the
  same scalar source reaches **3.66×, beating the hand-written `_mm256` version**
  — because the intrinsic form becomes `vptest` plus a branch, and the branch
  costs more than a branchless OR-reduce. **Scalar C beats intrinsics here.**
  **The bound that governs all four: `-O2`.** The referee defaults to
  `SIR_OPT_LEVEL=1`, the vectorizer does not fire at `-O1`, and every
  scalar-spelling win inverts there: the hll ternary becomes **1.47× slower than
  a plain branch** and 18.45× behind the intrinsic; the bitpos scalar scan becomes
  **0.79×, a loss**. So "let clang synthesize the SIMD" is entirely a function of
  the optimization level, and the configuration every recorded number came from is
  on the wrong side of it.
  **The synthesis.** A scalar-C-only emitter reaches exactly the objects whose
  fast form is a pointwise or reduction data-parallel map **over a contiguous
  array with a linear index** — element-wise max, compare, OR-reduce, select —
  because the branchless ternary spelling of `Select` is precisely the pattern the
  vectorizer recognises. It dies on any accessor that computes its index by
  shifting and masking (`cannot identify array bounds`), which is the packed case.
  So the reachable set is bounded below by the vectorizer's pattern matcher and
  above by the absence of a vector type — and **the intervention ABSAC's thesis
  describes is exactly the one that lifts the lower bound: change the
  representation so the object becomes a dense linear map, and the backend does
  the rest.** Two of the four wins need no emitter work at all; the binding
  constraints are a representation choice, an optimization level, and one missing
  attribute.

- **2026-09-25 — A/B/C re-measured at matched -O1/-O2/-O3: the null survives, and
  my level confound theory is REFUTED**: 338 functions across 33 corpora complete
  at all three levels, 1014 usable records, every record's three sides compiled by
  **one clang invocation at one level** with `--ref-level` enforcing the match.
  Nothing committed; work in `/tmp/abclevel/`.
  **What makes the null trustworthy this time — an attribution control.** Rows with
  `rewrites == 0` give **C/B = 1.0000 exactly in 220 of 220 rows at every level**:
  the symbol rename and the carried-output hoist are worth exactly zero
  instructions, so any C/B ≠ 1 is attributable to the rewrite and nothing else.
  Executed call counts are identical across A/B/C in **0 of 1014** records
  differing, and no record lacks `mismatched=0 harness_errors=0`.
  **C/B — the rewrite contributes nothing on third-party code, at any level.** The
  four external functions that fire (`crc32pseudo`, `fir_no_red_ld`,
  `getHuffTable`, `getHuffVal`) all measure **C/B = 1.0000 at O1, O2 and O3**. In
  the C source they *do* change — `v11 * 8ULL` becomes `v11 << 3ULL` — and change
  **no machine code at all**, because the family is multiply→shift and clang's
  instruction selection already performs that at every level ≥ 1. On the project's
  own base14 kernels the benefit is real but evaporates with level: C/B 0.7770 at
  -O1 (60/7/47 win/tie/loss) → 0.9794 → **1.0081** at -O3 (43/18/53). It collapses
  toward zero and slightly past it.
  **C/A — ABSAC beats clang on no third-party function at any level, in
  instruction counts.** External matched-source (n=71): **0 wins at -O1** (16
  losses); one "win" at -O2 and one at -O3, both `edn::mac` at 0.9109 — and `mac`
  fires no rewrite, so that is codegen luck, not an optimization. Median C/A is
  1.0000 at every level.
  **B/A, and the mechanism.** External matched-source: **1.1989 / 1.2832 / 1.2829**
  at O1/O2/O3. The gap opens because **A improves with level and B does not** —
  ABSAC's raw emission is essentially level-invariant (it is already scalar C).
  Concretely: `fir` A 17953 → 11953 → 7859 while B stays 17953 → 15555 → 15555;
  `copyY` A 458 → 83 → 83 against B 779 → 523 → 523. So clang's advantage over
  ABSAC's raw emission grows with level, and it grows because clang gets better at
  the source.
  **Firing counts are level-independent — confirmed, not assumed**: `{0: 220,
  1: 103, 2: 10, 3: 5}` identically at all three levels, 338/338 functions
  unchanged, 118 of 338 firing. The families run on SIR, as expected.
  **The noise floor, measured on byte-identical code.** `crc32::initialise_benchmark`
  is instruction-identical across A/B/C — the same binary work — yet its wall-clock
  B/A swings **1.1743 / 0.5174 / 1.0202**, a 48% swing on this 2-vCPU box. That is
  the strongest evidence yet that instruction counts must be load-bearing, and it
  is why every ratio above is quoted in Ir with wall clock only as corroboration.
  **Corrections to figures already on record.**
  `crc32pseudo` was recorded C/B = 0.9996 and C/A = 0.9985; re-measured C/B is
  **1.0000 exactly** and C/A is **1.0768** in instructions — a 7.7% *emission
  cost*, invisible to wall clock (1.0012/0.9973/1.0050) and previously recorded as
  neutral. `fir_no_red_ld` was recorded B/A = 1.0727; the instruction count is
  **1.2504** at -O3, and the old figure reproduces to within 0.4% **only in wall
  clock** (1.0775) — the recorded number was a wall-clock ratio presented beside
  instruction ratios. `sglib_iq_first_element` C/B = 1.0492 is **excluded, not
  substituted**: callgrind returns rc=-11 at all three levels, so no instruction
  count exists and the low-confidence flag on it was justified.
  **Two self-corrections by the measuring agent, worth recording as method.** A
  partial-run claim that "C/B = 1.0000 in all 58/58 rows at all three levels" was
  a **coverage artifact** — the first 100 functions in sweep order contained only
  the four arithmetic-firing external functions, precisely the ones where the
  rewrite changes no machine code; on the complete set C/B differs in 107/107/96
  rows. And an exclusion class of "no callgrind record" covering 123 of 128 rows
  collapsed to 9 after a full reparse. Separately, the sweep's inline parser used a
  regex that cannot match right-aligned percentages (`( 7.69%)`) and **silently
  dropped 291 side-values**, all recovered from the preserved `cg.out` files via
  `callgrind_annotate`.
  **Verdict: the recorded null survives, and it is sharper than what was on
  record.** The -O1 arrangement did not manufacture it; it only hid the fact that
  **clang's own optimizer does the arithmetic family's work at every level ≥ 1**.
  My hypothesis that the level confound was creating the null is refuted — the
  confound was real and worth fixing, and fixing it produced a stronger negative,
  not a win. One method deviation matters for future readings: A here is the
  **original C recompiled at each level**, not the shipped `.ll` — the two differ
  materially (`fir_no_red_ld` at -O3 is B/A 1.0519 with a shipped-`.ll` A but
  **1.2504** with a C-source A), and 35 of 341 rows fall back to the shipped `.ll`
  and are flagged weaker.

- **2026-09-27 — external consultation on the representation decision: "freeze the
  compiler", and four criticisms I accept**: the full evidence record was put to
  `gpt-6-astra` (no tools, self-contained context, adversarial brief) and the
  answer rejected all three of my candidates. Recorded here because its criticisms
  of the reasoning are the operative part.
  **The recommendation: execute (b) operationally — no new families, no lowering
  reach, no SIMD types — and test a bounds-safe, representation-preserving scalar
  `hllMerge` replacement in Redis instead.** Do not pursue the representation
  change first: the fused scalar witness claims ~13.8× *without* persistent byte
  storage, so establish whether that survives correct allocation bounds before
  changing every consumer and format boundary. Persistent byte storage adds **33%
  to dense register payloads** (16384 vs 12288 bytes) with unmeasured cache,
  bandwidth and serialization costs. SIMD is rejected on the grounds that
  expressing `pclmulqdq` establishes neither recognition of upstream CRC code nor
  correctness of the replacement — "you have measured an instruction-selection
  limitation, not established that removing it produces a useful compiler."
  **Criticism 1 — I conflate headroom, expressibility, and automatic discovery.** A
  hand-written fast implementation establishes none of the machinery connecting
  upstream code to it. "So far, your machinery contributes zero measured
  third-party benefit." Accepted, and it is the sharpest point: every win in the
  survey table is a *hand-written witness*, and the pipeline that would need to
  find it contributes nothing on third-party code at any level (the -O1/-O2/-O3
  result above).
  **Criticism 2 — the algebraic-object criterion is post-hoc, and partly
  circular.** Popcount contradicts the strong version under an optimised baseline.
  And if "has a hardware or algorithmic shortcut" is part of the *classification*,
  it partly defines the outcome it purports to predict. Data-structure walks also
  implement mathematical objects. Accepted — I fitted a criterion to five data
  points and then presented it as a discriminator. **The prospective test it
  proposes, which I had not thought of**: select 20 previously unexamined hot loops
  from unrelated projects, classify them *before* optimisation or timing, impose
  equal engineering budgets and strong deployable baselines, publish every null,
  and compare achieved benefit by class — without relabelling failures afterwards.
  **Criticism 3 — the HLL measurements do not establish the representation-change
  thesis.** A fast inner phase (49.82×), an invalid fused witness, and a
  persistent-layout proposal are three different claims and I ran them together.
  **Criticism 4 — my architectural diagnosis was wrong.** "No phi nodes is not the
  semantic defect. Functional loops can have sound explicit binders. Reusing one
  node for distinct loop-entry and iteration meanings is the defect." Accepted: the
  absence of phi nodes is a design choice that is fine if the binder is used
  soundly, and the defect is the *reuse*, not the absence. Also: "proved family
  identities do not prove lowering or emission" — which is exactly the S2 shape.
  **The prescribed smallest experiment, which is a Redis patch and not a compiler
  extension**: discard the over-reading witness; construct a bounds-safe fused
  scalar implementation preserving existing layouts and interfaces; establish
  equivalence and access bounds over the stated domain, testing actual allocation
  sizes, final packed fields, identical inputs and valid register distributions,
  with ASan and differential agreement supplementing the proof rather than
  replacing it; benchmark inside unmodified Redis. A win proves one useful patch.
  **Only afterwards** attempt automatic derivation from real upstream code with
  original/raw-lowering/rewritten attribution — that second experiment is the one
  that tests ABSAC.
  **The governing line, which is the finding**: "Your current failure mode is
  building machinery to reach opportunities whose valid, deployable value has not
  yet been established. Reverse that order."

- **2026-09-27 — `hllMerge`: a bounds-safe 19-line Redis patch wins 3.484× at
  whole-command scope. The first third-party win the project has, and it is not
  the compiler's.** Built to the director's prescription: bounds-safe,
  representation-preserving, proved and benchmarked inside unmodified Redis.
  Nothing under this repo was modified; artifacts in `/tmp/headroom_a/work/`.
  **The patch is 19 changed lines in 2 hunks** (`PATCH.diff`): an include with two
  `_Static_assert`s binding the register count and byte size, and the dense branch
  replaced by a call. Layout, interfaces, on-disk and wire format unchanged; no
  caller touched; no storage infrastructure.
  **The result.** V6 is one 8-byte `memcpy` load per 6-byte group holding eight
  registers, a u64 SWAR max, 2047 wide groups and two 3-byte tail groups.
  Instructions per command (callgrind inclusive, N=200, dense dataset verified):
  `PFCOUNT k1 k2 k3 k4` **3.484×** (1,536,775 → 441,116); `PFMERGE dst s1..s4`
  **2.374×** (2,337,537 → 984,832); identical-bitmap `PC4I` **3.453×**; 8-key
  `PC8` 3.645×. **All 15 instruction regimes improve and none regresses** — the
  worst is PM4 at 2.374×. Wall clock corroborates on an idle box with the regimes
  reported separately: `PC4` **9.594×**, `PC4I` 3.475×, `PM4` 2.523×, `PM4I`
  3.204×, worst-case drift across two independent passes 8.2%.
  **`v0/base = 1.0000×` on every single regime** — the patched tree with the
  *upstream* loop is instruction-identical to pristine Redis, so the patch
  scaffolding is free and the whole delta is the kernel.
  **Bounds are the primary evidence, on five independent lines, deliberately not
  the differential.** (1) A **placement witness** stronger than ASan: the register
  array's last byte abuts a `PROT_NONE` page — **V0, upstream, SIGSEGVs (rc=139)**;
  V1–V7 return 0 with correct results. (2) A bounds argument on the *shipped
  artifact*: the wide loop runs exactly 2047 iterations, the maximum byte touched is
  `registers[12283]` against an array ending at 12287, and the tail is byte-wise;
  **zero vector instructions in v6's `hllMerge`**. (3) Upstream's over-read
  confirmed **in the shipped binary rather than inferred from source** —
  `movzbl 0x1(%r8,%rsi,1)`, an unconditional `r[byte+1]` — and shown to be
  **unnecessary**: register 16383 occupies bits 98298–98303, entirely inside byte
  12287, so the extra byte is out of bounds *and* pointless. (4) An exhaustive
  2^16-per-lane sweep under ASan+UBSan. (5) A differential against a
  definitionally in-bounds reference, bit-identical on random, identical, all-0,
  all-63 and real distributions.
  **A self-inflicted finding worth keeping.** The measuring agent's own first
  reference implementation **reproduced upstream's bug** and read `r[12288]` for
  register 16383, making all eight variants appear to fault — a witness defect, not
  a kernel defect. Its own conclusion: "a differential alone is not a bounds
  argument; the reference has to be written carefully too." This is the second time
  this week that the *checking apparatus* was the defective component, after the
  differential driver's random-byte fill hid a live miscompile.
  **Two context figures did NOT reproduce, flagged rather than smoothed.**
  First, the **~13.7–13.9× scalar fused figure was not reproduced**: the best
  purely-scalar kernel measures **3.44×** at kernel scope (other variants 1.78×,
  2.58×, 2.79×). One variant reached 3.51× but only by GCC auto-vectorizing it into
  `pmaxub`, which cannot be bounded at source level, so it was rejected. The 20.4×
  SIMD figure was not attempted — it is the over-reading form.
  Second, **the 0.76× identical-input loss did not appear at all, and the ~5× regime
  swing is not an instruction-count effect.** Baseline instructions are flat across
  regimes (within 1%); the swing is **cycle-level**, upstream's `if (val > max[i])`
  mispredicting on disjoint inputs, measured at **2.21× and 2.67–2.82× — 2.2–2.8×,
  not ~5×**. Because V6 is branchless its cost is flat and sits *below upstream's
  own best case*, so the identical-input regime is a **3.5× win, not a 0.76× loss**;
  that loss belongs to some other formulation.
  **What this means, stated plainly.** The value is now established: a real,
  deployable, command-level win on independently authored code, bounds-proved and
  measured against Redis's own build flags (`-O3 -flto`, unchanged on both sides).
  It was reached by **a hand-written 19-line patch, with no compiler involvement
  whatsoever** — which is precisely the director's point that the project's failure
  mode has been building machinery to reach opportunities whose deployable value was
  not yet established. The value is established now; whether the compiler can be
  made to reach it is the open question, and the capability it would need is the
  latch-block admission (600–900 lines, zero corpus payoff), a pointwise-map family
  shape, a `Max` node kind, and a scoped write authorization.
  Not established: why the cited scalar kernel figure was ~4× the best reproduced
  here; a 14% `PING` inclusive-cost difference between binaries (0.005% of a real
  command, immaterial to every gate but unexplained); a bounds-safe SIMD form; and
  16-key `PFCOUNT`.

- **2026-10-12 — CORRECTION: the `hllMerge` win is upstream-obsolete, and it was
  measured against a superseded baseline. It was never mergeable.**
  Checked Redis upstream rather than the record. **Redis 8.0 already implements this,
  and better**: `hllMergeDenseAVX2` and `hllMergeDenseAarch64` with runtime CPU dispatch
  (`hyperloglog.c`), present in 8.0, 8.2, 8.4 and current 8.12. **Redis 7.2.5 — the
  audit's baseline — had only the plain scalar loop.** Redis also solved the bounds
  problem the survey did *not*: its AVX2 loop deliberately merges the first 8 and last 24
  registers with the scalar macro, per its own comment, "the AVX2 algorithm needs 4
  padding bytes at the start / at the end". The ledger listed "a bounds-safe SIMD form"
  as **not established**; Redis established it.
  **The timing is the damaging part.** The audit ran 2026-09-27 against Redis **7.2.5**
  while 8.x was already current, so the 3.484x was measured against a version line Redis
  had already improved. **It was never mergeable — not because it was wrong, but because
  the target had already moved.** A suggested PR would be closed as already implemented.
  **The residual angle is also dead.** The scalar fallback still reads `_p[_byte+1]` for
  register 16383, but the dense representation is `sdsnewlen(NULL, HLL_DENSE_SIZE)` and
  sds always allocates a trailing NUL, so that read lands inside the allocation on the
  terminator. A bug report on it would be closed as by-design. The earlier "placement
  witness" showing upstream SIGSEGV used a synthetic placement (array abutting a
  `PROT_NONE` page), not Redis's own allocation.
  **What survives is the analysis, not the artifact.** "A pointwise max over a packed
  register array (a lattice join); the packing is what hides it" remains a true and exact
  statement of the project's research question, and Redis independently reached the same
  conclusion and shipped it. **Being right was not the product.**
  **This is the first concrete instance of the advisor's warning** that the recurring
  product is "compiler-conditioned performance maintenance: find repairs, revalidate them
  after toolchain changes, retire them when upstream codegen catches up" — here upstream
  caught up *before* the patch was ever submitted. It reframes the business problem: if
  the value is "find optimizations upstream has not found", then on actively maintained
  code the project is racing the maintainers on their own code and will usually lose. What
  is defensible is either something upstream structurally cannot supply (a machine-checked
  proof, a guarantee, a certificate) or **breadth and speed** — reaching the opportunity
  across many projects faster than each project's maintainers reach it individually. A
  single hand-written patch to one function has neither property.
  **Consequence for the evidence record**: `hllMerge` may no longer be cited as the
  project's demonstrated third-party win without this correction attached. It is a
  historical lesson about decay, not a submission candidate.

- **2026-09-27 — astra RECOMMENDS: withdraw the derivation target, freeze the
  compiler, and make the product service-led — under consideration, NOT adopted**:
  second consultation, `--thinking max`, with the `hllMerge` result and the conflict
  between "attempt automatic derivation" and "no new families, no lowering reach,
  no SIMD types" put to it.
  **Status, set by the owner after reading it**: this is **advice to weigh, not a
  decision taken**. The owner's words — "it should be recorded as what astra
  recommends, not what we will necessarily take but will consider and might take."
  The project's goal is unchanged: **automated superoptimization, sold as
  performance, as a compiler or an engine**; the owner will "accept victory via any
  method", and the service route is **stage one of the path to funding, not the
  destination**. Everything below is therefore recorded as a recommendation, and the
  parts that conflict with the goal are flagged as such rather than adopted.
  **Where the advice and the goal agree**: the staged go-to-market — free service
  and public PRs to large projects first, funding once acceptance accrues — is the
  owner's own plan and astra's service-led framing is compatible with it.
  **Where they conflict**: astra's "the production compiler freeze stands" and its
  proposal that compiler development must justify itself against a hard bar are
  recommendations the owner has not accepted, because the product is meant to be
  automatic. They are recorded as a live tension, not a ruling.
  **1. Do not build the derivation path. `hllMerge` is withdrawn as the mandatory
  next compiler experiment and the production compiler freeze stands** — "a
  successful hand optimization does not justify an infrastructure expansion."
  Automatic derivation remains the test of ABSAC, but **its next end-to-end target
  must already fit the lowerer and an existing family.** The zero-firing count
  defeats the standalone reach investment; it does **not** establish that max-map
  recognition has no value. And SIMD is not a prerequisite — the winning kernel is
  scalar.
  **2. The demonstrated product is not the compiler.** "ABSAC currently consists of
  research software and a human-driven, verified-optimization practice." Redis
  validates **one valuable delivery** — not automated discovery, repeatable
  discovery economics, or willingness to pay. **Make the company service-led**: sell
  verified performance improvements, and treat the compiler as optional internal
  tooling whose development must reduce delivery cost. "This result supports neither
  the arbitrary-code automation claim nor unicorn economics. Maintainer acceptance
  of free code will not establish a business model."
  **3. Authorized next experiment — a recognition-only probe, explicitly not
  disguised integration work.** A disposable, read-only LLVM analysis **outside
  `sir_lower`**, capped at **three engineer-days**: input is untouched upstream IR
  with **no function-name matching, no hand-normalized loop, no replacement
  kernel**; recover induction, packed six-bit extraction, destination indexing and
  unsigned compare/select; output a conditional relation such as
  `dst'[i] = select(src6[i] >u dst[i], src6[i], dst[i])` with recovered widths,
  extents and applicability conditions; SMT-check the lane relation and **list the
  outstanding loop, alias and bounds obligations explicitly**; test held-out
  equivalent implementations and signedness/indexing near-misses. It requires new
  recognition code but **no `Max` node, no lowerer admission, no writable patch, no
  emission** — it tests **object recovery only**. Two standing warnings attach:
  **the upstream tail over-read must remain visible — undefined behaviour is not a
  proof shortcut** — and "a recogniser tailored solely to this known answer is a
  unit test, not research evidence."
  **4. The single next action: submit the Redis package** — the complete patch
  including kernel and header, tests and a reproducibility bundle, stating it was
  **hand-derived**, with instruction ratios reported separately from wall clock, the
  unreproduced figures retired and the bounds finding disclosed precisely. **No
  further compiler milestone gates this submission.**
  **5. A hard bar on the compiler, and an expiry.** One **20-engineer-day trial** on
  a preregistered, independently selected corpus **within existing reach and family
  coverage**, requiring **two automatically derived, proved optimizations in
  different projects, each delivering reproducible ≥5% end-to-end CPU-time savings**
  under native build flags, with original/raw-lowering/rewritten attribution
  preserved and no hand-normalized inputs, bespoke semantic adapters, or scope
  expansion to rescue the score. **Failure ends the current commercial compiler
  strategy**; the service continues only on its own economics. "Another excellent
  hand patch does not reset the compiler's clock."

- **2026-09-27 — recognition probe: the object IS recoverable from untouched
  upstream IR, and the probe says plainly where it is fitted rather than
  general**: the director-recommended three-engineer-day recognition-only probe,
  built standalone outside the tree (2633 lines of Rust, ~1 hour wall clock — well
  inside the cap), `sir_mech` as a read-only path dependency, no `sir_lower`
  change, no `Max` node, no admission, no patch, no emission.
  **A bad pin, caught and then cleared.** The probe found
  `/tmp/headroom_a/redis-7.2.5/src/hyperloglog.c` modified (sha `74afb409…`,
  carrying `HLLM_VARIANT`, an include and two `_Static_assert`s) and refused to use
  it, re-extracting the pinned tarball instead. Verified here: that modification's
  mtime is **2026-09-27 11:59**, the hllMerge patch agent's own variant
  scaffolding, and it **postdates every survey measurement** — the survey recorded
  `hyperloglog.c` as `f747bc0d…`, which *is* the pristine file, and `bitops.c` is
  untouched since extraction (mtime 2024-05-19, sha `2624690f…`). The patch agent's
  `v0/base = 1.0000×` across all 15 regimes independently confirms the scaffolding
  is instruction-free. **No recorded number is invalidated.** The practice was
  still wrong and is now recorded as a rule: a pinned reference tree is immutable;
  variants get built in a copy.
  **The result: the lane relation discharges from untouched IR.**
  `dst'[i] = select(src6[i] >u dst[i], src6[i], dst[i])` for i ∈ [0, 16384) is
  **Valid at O0, O1, O2 and O3** — a discharged equivalence, not a guess. Recovered
  structurally with no function-name matching: the induction variable and its
  extent, the packed 6-bit extraction including the byte-straddling field
  (`fshr.i8` form), the destination indexing, and the join. The **non-zero source
  base is recovered (`base_const=16` = `sizeof(hllhdr)`) rather than assumed**, and
  the relation is stated over the recovered parameters instead of a fixed shape.
  Sixteen obligations ran per level, with **non-vacuity witnesses**: mutating a
  recovered constant refutes the extraction proof, and the last-lane claims
  (`field ends inside byte(i)`, `high byte is value-dead`) are refuted at i=16382,
  showing they are about the final lane specifically.
  **The upstream over-read is computed, not asserted.** The probe recomputes the
  slack from the recovered parameters and it *changes correctly*: 16384 lanes →
  read at 12288, one byte past; a 5-bit variant → one byte past at 10240; a
  16383-lane variant → **0 bytes past**. The relation does not exploit the
  over-read, and `E′` is **refuted with model `i=16383`**, independently confirming
  the defect found in the shipped binary. But the read is still executed and the
  probe does not prove the allocation has slack — it says so rather than tidying it.
  **Four obligations listed separately and NOT discharged**: loop (trip count is
  structural, not proved to follow from the allocation), **alias** (nothing checks
  source/destination disjointness, and aliasing would make the merge a
  self-comparison), bounds (the destination having `lanes` writable elements is an
  assumption), and signedness of the model (signed arms are modelled as unsigned
  under a caller-supplied non-negativity assumption).
  **Where it is fitted rather than general — stated by the probe itself.** The join
  matcher is **operand-order sensitive**, so recognizability currently depends on
  clang's canonicalization choice rather than on the source; there is **no
  select-join path at all** (a `?:` is refused); and at **O0 the near-miss set
  recovers 0 of 15**, including loops it accepts at O2 — a controlled isolation set
  confirms the O0 path only accepts upstream's exact i64 idiom. Its own verdict: "I
  knew the answer in advance… A recognizer tuned to a known answer is a unit test;
  this one is closer to that than I would like." And a limit on the held-out
  evidence: within the authorized corpus there is **exactly one dense max-merge**,
  so held-out generalization cannot be measured on it at all — which is itself the
  finding, and the constructed near-miss set is not an independent corpus.
  **A vacuous check in the probe's own apparatus, found by its own near-miss set.**
  Obligation F chose between two encodings of the *same* `>`, so it *printed* `u<`
  while proving `u>` — and **a minimum passed it**. Replaced with a real predicate
  dispatch; the minimum now refutes it and the target still discharges. That is the
  third time this week the checking apparatus was the defective component, after
  the differential driver's random-byte fill and the patch agent's own reference
  implementation.
  **Verdict: object recovery works and is dischargeable, but is not yet a
  capability.** The two false negatives that matter are cheap and bounded — operand
  order, and a select-join path — and fixing them is what would move the recognizer
  from "fitted to one IR shape" toward "fitted to the object", which is the
  property automated optimization actually needs.

- **2026-09-27 — astra recommends: the search substrate is a proof-carrying AND/OR
  implementation graph, and the network is a replaceable scheduler that should not
  be built yet**: consultation on the representation-space design, `--thinking max`.
  Recorded as **recommendations under consideration**, per the owner's standing
  framing — the goal remains automated superoptimization. Two premises in my brief
  were corrected and are recorded as corrections.
  **CORRECTION 1 — a semantic canonical form cannot exist.** I had asked whether the
  state should be "what the code computes". Arbitrary program equivalence is
  **undecidable**: a program that simulates a machine and then returns zero is
  equivalent to `return 0` exactly when that simulation terminates, so a semantic
  canonical form with decidable equality would decide halting. "Finite correctness
  certificates are checkable; arbitrary program equivalence is not decidable." What
  *is* ruled out on the other side is syntax alone, which hides relationships, and
  semantics alone, which quotients away the very variables being optimized.
  **CORRECTION 2 — the AlphaDev diagnosis is unsupported.** AlphaDev demonstrated
  improvements in **deliberately bounded low-level tasks** (small sorting routines).
  That is neither arbitrary-program success nor evidence of a failed
  arbitrary-program attempt. "ABSAC's supplied experiments establish **ABSAC's**
  reward and expressiveness problems" — not a general lesson about AlphaZero-style
  search on arbitrary programs.
  **The design. Objective**, for source `P`, assumptions `A`, target/toolchain `H`,
  workload `D`: minimize `J_{H,D}(Q) = E_w[ C_H(compile_H(Q), w) ]` subject to
  `Q ≃_A P` — equivalence on **every defined source execution**, with no new UB.
  **State**: an **AND/OR implementation-and-proof graph**. An **OR node** holds
  alternative implementations of a contract; an **AND expansion** requires every
  component implementation, applicability condition and refinement proof to succeed;
  **shared representation variables connect every consumer of the same storage**;
  leaves are executable IR, target primitives, or unchanged original code; the graph
  carries proof status, counterexamples, compiled artifacts and context-specific
  measurements. This is the answer to "a reasoning graph?" — **yes, a reasoning
  graph of formal obligations, not natural-language reasoning.** SIR is retained as
  implementation syntax with two qualifications: increasing `NodeId`s give
  deterministic traversal but **not** canonicality, so it needs alpha-normalized
  structural keys with semantic merging behind a proof; and a select DAG represents
  conditional *pure, total* expressions, so conditional stores, trapping loads, calls
  and divergence need guarded/lazy regions and explicit effect sequencing — **an
  effects bitmask does not describe aliasing, footprints or ordering.** An e-graph is
  useful *inside* a pure-expression task but is not a representation of protocol
  closure or measured performance; an embedding is a search feature that may never
  serve as an identity or a correctness criterion.
  **Moves: "a semantic transaction, not an instruction edit."** A proposal selects
  `(scope, contract, representation, algorithm constructor, parameters)` and expands
  into implementation and proof tasks; intermediate states may hold unresolved
  alternatives; a replacement is legal only when its obligations close, and the
  certified incumbent remains available throughout. **The mechanism is data
  refinement**: with abstraction `a`, concrete `c` and invariant `R_ρ(c,a)`,
  `R_ρ(c,a) ∧ F(a,u)↓(a',o,τ) ⟹ ∃c'. G_ρ(c,u)↓(c',o,τ) ∧ R_ρ(c',a')`, with outputs
  and observable effects matching. The consequence that matters: **"a transformation
  affecting many functions can have many small, reusable certificates. Its textual
  size need not determine its proof granularity."** That is how the cross-consumer
  case becomes tractable — and the tension in my brief gets an exact local solution:
  **"global representation changes composed from local refinement proofs."** Where
  that decomposition cannot be found and checked, **return the original**.
  **How the fitted recognizer stops being fitted**: "recognition proposes a contract
  and proves it against the source. For example, propose unsigned maximum and check
  the recovered expression universally. **Operand order and ternary spelling then
  cease to be correctness gates.**" That resolves the probe's operand-order
  sensitivity and its missing select-join path as *correctness* concerns, though
  discovery can still fail — proof does not discover the contract. And a warning
  against my own post-hoc criterion: **do not encode the five-example "algebra
  versus data-structure walk" hypothesis as an exclusion rule.**
  **The HLL case, worked**: abstraction is `a_i ∈ [0,63]` logical registers, with
  both packed and byte storage related to it. A byte-layout proposal owes
  obligations for initialization, every mutation, access/merge/counting,
  copying/allocation/destruction, and **serialization, persistence, replication and
  raw-byte observation** — "a raw-byte `GET` is a consumer; an escaping pointer is an
  interface", and each must be included or mediated by a **proved adapter**. The
  layout choice commits **atomically** across consumers, and ownership/escape
  analysis must establish closure or the transformation is refused — no "private
  storage" assumption may be invented. Cost is **whole-lifetime**: expanding 6-bit to
  bytes raises payload by 4/3, and **new allocation failures are behaviour changes,
  not merely costs**.
  **Search and the network.** Use **memoized best-first AND/OR search** with bounded
  enumeration or CEGIS inside synthesis tasks — it matches the dependency structure
  and reuses subproofs — and **do not require intermediate moves to improve
  performance**: unpacking is locally expensive while enabling the winning completed
  transformation. A network is a **replaceable scheduler**: permutation-equivariant
  encoder over the typed graph, a pointer selecting a task/scope, constructor and
  parameter heads, and heads predicting proof feasibility, descendant improvement and
  evaluation cost. **The value target is the best completed, certified gain within
  the remaining budget — not source-level instruction count and not proximity to a
  guessed normal form.** An LLM may propose contracts, invariants, algorithms and
  certificates but **receives no authority over assumptions or acceptance**; MCTS
  over macro-actions solves neither representation invention nor verification nor
  reward integrity. **"With today's move set and measurements, a learned policy has
  no demonstrated productive third-party actions. Fix that before training."** And:
  **"do not simultaneously build a global e-graph, another compiler backend, MCTS and
  a training stack."**
  **Reward.** Correctness is a **hard admissibility constraint, never a finite
  penalty that speed can outweigh**: `r(Q) = log( Σ w_j C_H(compile_H(P), w_j) /
  Σ w_j C_H(compile_H(Q), w_j) )`, against **arm A — the original strong deployed
  configuration, not a degraded raw lowering**, measuring all attributable work
  including dispatch, helpers, conversions, migration and changed consumers. Four
  consequences: machine-identical artifacts get identical credit, so
  **multiply-to-shift earns exactly zero** (which is what the three-arm experiment
  measured); correctness is not "fraction of tests passed"; **proof timeout is
  censored feasibility data, not a verdict that the program is wrong**; and costs are
  **not additive across graph nodes**. And the sharpest: **"instruction reduction is
  not speed reduction"** — carry-less multiply, SIMD, cache locality and memory
  parallelism all break the equivalence, so instruction counts may be a valid
  current objective and a search feature but **must not prune every
  instruction-increasing candidate**. "Until latency selection with A/A-calibrated
  uncertainty exists, ABSAC has evidence for instruction-count wins, not general
  performance wins."
  **Soundness boundary.** The output is **a deployable program bundle plus a checked
  certificate and measurement record**, or the unchanged original; "a representation
  choice alone is not a finished optimization." The proof must cover the **actual
  emitted transformation** — "a correct family theorem does not certify an emitter
  that substitutes the initial loop index", which is the S2 defect named exactly —
  so prove between original and emitted IR or validate each translation link, and
  treat the remaining frontend/codegen assumptions as an **explicit trusted
  boundary**. Verifier results are **Proved / refuted by a validated defined
  execution / unknown**, and **unknown forbids deployment**. Edge rules stated as
  non-negotiable: bounds and UB (zero iterations preserve initial state; tails and
  extra vector lanes need object-level bounds, not merely mapped pages; alignment,
  initialization, lifetime, provenance, zero-input `ctz`); arithmetic (prove actual
  width, overflow, shift, division and floating-point semantics — integer or real
  algebra is not a substitute); aliasing and effects (prove footprints, framing and
  permitted reordering; an ATOMIC bit is insufficient for concurrency); nontermination
  (divergence-sensitive simulation and well-founded arguments); external behaviour
  (formats, error paths, observable ordering — a set abstraction that discards
  duplicates cannot justify a database transformation that exposes them). On the
  referee: differential testing is valuable as an independent attack on the trusted
  boundary and its inputs should be **branch- and relation-directed, boundary sizes,
  aliases and counterexample-derived** — **"random bytes and `mismatched=0` are not
  correctness evidence sufficient for acceptance."**
  **Falsification, stated.** The substantive hypothesis is that **"profitable
  nonlocal transformations often admit automatically discoverable, reusable, compact
  relational contracts."** It fails as substrate if useful contracts need
  human-supplied invariants per function; if consumer closure and proof construction
  consume the budget without further end-to-end wins; if lifetime measurement erases
  the apparent representation wins; or if syntax/compiler perturbations repeatedly
  defeat recovery of the same contract. Compare graph scheduling against direct
  LLM-plus-verifier proposals and plain constructor enumeration at identical budgets,
  and **delete learned scheduling if it adds no held-out gain**.

- **2026-09-27 — astra recommends: pivot the search surface (certified source
  transactions), not the correctness claim. The architecture is a *partial source
  verifier with an untrusted proposer*; the company is a verified
  performance-portability engine.** Second consultation of the day, on whether to
  stop being a compiler and become the layer above one. Recorded as recommendations
  under consideration. **Three claims I put to it are corrected here as mine.**
  **CORRECTION — "seven of the eight moves are provable with the existing kernel" is
  unsupported.** The bitvector kernel and the induction theorems prove *components*,
  not arbitrary memory framing, precomputation validity, float width changes, or newly
  disclosed assumptions: `restrict`, alignment and `assume` require facts about
  **every applicable execution, not the observed workload**, and a specialization
  guard must itself execute without UB with a fallback preserving the original domain.
  "Unknown means unchanged. A defined counterexample blocks the transaction until its
  applicability condition is proved correct." This is the same overclaim shape as the
  earlier withdrawn figures — asserted, not counted.
  **CORRECTION — the trusted boundary is not merely clang.** It includes source
  extraction, obligation construction, theorem premises and patch realization. **"A
  correct SAT answer to the wrong obligation proves nothing useful"** — which is the
  same failure as the obligation that could not fail, seen from the other side. And
  emission **shrinks, it does not disappear**: replacement boundaries still need
  correct types, aliasing, effects, storage lifetimes, ABI preservation,
  target-feature dispatch, and correspondence between the verified object and the
  exact emitted patch.
  **CORRECTION — a 295-second proof is proof cost, not measurement cost.** Budget
  both; certificates establish semantic admissibility, not speed, not workload
  representativeness, not measurement quality.
  **The reframe survives, restated**: yes to certified source transactions; no to "the
  hard part disappears". Discovering useful algorithms and proving their applicability
  remains the binding problem, and CRC folding, vectorizer enablement and database-plan
  substitution are **different research problems that happen to admit source patches**.
  **Diagnostics are sensors, not a move language.** My diagnostics-as-move-generator
  claim is downgraded: "cannot identify array bounds" does not identify a six-bit
  lattice join, suggest where conversion belongs, or establish that moving loads is
  alias-safe — the human supplied all three. The mechanisable mapping is
  `remark → hypothesis about an obstacle → controlled intervention → verification and
  experiment`, **not** `remark → patch`. **Honest planning baseline: approximately
  zero useful automatic yield on the measured corpus.** Two retrospectively selected
  human successes support using diagnostics for **triage**, not for forecasting
  automation. Test incremental value against a code-only proposer at the same action
  vocabulary and budget, and count **certified, strong-baseline improvements — not
  remarks silenced and not loops vectorized.**
  **The recommended research centre** (replacing my pairwise speed predictor, which it
  calls too small an ambition — "it ranks programs someone else invented"). The
  valuable learned object is **a transferable plan for making a mathematical operation
  executable efficiently: which semantic facts to expose, where to change
  representation, which algorithm to substitute, and which experiments resolve the
  remaining uncertainty.** Its unit is neither a program nor a diagnostic but
  **contract × surrounding use/storage context × compiler configuration × workload
  regime**. Three capabilities:
  **(A) Learn representation *boundaries*, not representations.** The decision in
  packed HLL is not "bytes beat packed bits" but **where to unpack, which consumers
  share the unpacked form, and where to repack**. Proposed mechanism: temporary
  **representation islands** — preserve the public ABI and persistent format,
  introduce a private working representation over a bounded region, discharge all
  entering and exiting relations, and learn placement and lifetime including
  conversion cost. This gives a continuum from local rewrite to project-wide layout
  migration; a learner might find conversion pays only above three sharing consumers
  or above a size threshold. **"The threshold chooses between already-correct
  implementations; it never licenses an otherwise-invalid transformation."**
  **(B) Gap capsules.** Every successful experiment yields two separate artefacts: the
  **logical** (contract, applicability conditions, implementation/proof composition,
  replayable obligations) and the **empirical** (compiler/build context, interventions
  tried, emitted-code observations, conversion-inclusive costs, uncertainty, null
  results). Minimize the witness while preserving both the semantic relation and the
  compiler-response contrast, then propose a parameterized capsule —
  **"generalization is a conjecture; each application must be checked."** Successful
  multi-step graph paths become proposed reusable macros instead of each discovery
  becoming another hand-written recognizer, so the action language can grow. And the
  limit of a proposer that only spells: **CRC folding is undiscoverable if the entire
  proposal language is alternative scalar spellings** — it needs an
  algorithm/primitive vocabulary and GF(2)/ISA semantics.
  **(C) Probe the compiler causally; model only enough to choose experiments.** Do not
  build a neural replica of LLVM. Use the real compiler as oracle and learn a
  conditional response model, running controlled variants that change loop
  decomposition alone, representation alone, or the combination, to find which
  *bundle* crosses a code-generation boundary. Cost ladder: cached proofs and reusable
  theorem instantiations → compilation, code fingerprints, optimization records →
  instruction-count screening → stable-hardware cycles and end-to-end validation.
  **Store cost distributions and ties, not noisy winner labels**; record
  `Counterexample` and `Unknown` separately; train the planner **to purchase the next
  useful proof or compiler experiment**, not to predict the fastest known candidate.
  **The flywheel and its limit**: no exclusive moat exists today. The potentially
  scarce asset is **a corpus of theorem-linked counterfactual experiments in real
  build/workload contexts** — not a database of remarks. The foundation-model analogy
  becomes credible only when additional contracts improve transfer beyond the cases
  that supplied them. And **"restoring performance after synthetic degradation is
  training, not a product win"**: every claimed win must survive comparison against
  untouched, strongly compiled upstream code.
  **First research milestone** (precisely stated): freeze the system after development
  on known examples; on untouched projects it must **recover a familiar contract
  through unfamiliar code, transfer or compose an implementation plan, certify the
  exact patch, and obtain a real gain without new human-supplied facts. Replaying
  Redis is a positive control, not that milestone.**
  **The company, per this consultation**: an opt-in CI engine returning **unchanged
  source or a small, reviewable patch** — the user supplies build and workload, the
  engine supplies the patch, a replayable verification record, adversarial regression
  inputs, a full-build benchmark comparison and a portability fallback, and
  **maintainers must not need ABSAC in their normal build**. Start with **one lane**:
  bounded unsigned first-match scans on the existing scan theorem plus explicit
  memory and tail obligations; add packed-storage islands only after transfer is
  demonstrated; **do not broaden the lowerer to enlarge the intake funnel**.
  Human-written templates are acceptable for the wedge; **human invention or bespoke
  correctness assumptions per submitted patch are not automation.** "Initially this is
  a verified autotuner, not yet a learned research breakthrough." The recurring
  product is **compiler-conditioned performance maintenance**: find repairs, revalidate
  them after toolchain changes, retire them when upstream codegen catches up.
  **Five falsification gates**, capped at **one engineer-month** with no new backend
  and no training stack, under the three generative campaigns (verification-chain
  mutation, structured differential/exhaustive-small checking, measurement negative
  controls) that must preserve the single-digit test count:
  **G1 — is the correctness asset real?** The source-to-patch path must reject
  deliberately invalid min/max, first/last-match, alias and tail mutants; **the
  min-as-max mutant must fail on a concrete witness such as `(1,2)`**; another accepted
  false obligation stops shipping through the affected path.
  **G2 — is there an automatic opportunity set?** ~24 hot sites across ≥3 projects
  **not used to invent the transformations**, ≤8 proposals per site, fixed proof and
  compile budget, **refusals in the denominator**, funnel stages reported separately;
  require two independent certified improvements, ≥1 exceeding 5% on a predeclared
  end-to-end workload. **Redis cases do not count.** Baseline-versus-baseline controls
  on stable hardware and held-out confirmation — "if only callgrind improves, you have
  instruction reductions, not yet the performance product."
  **G3 — do diagnostics contribute?** Repeat the frozen evaluation with remarks
  hidden, actions and total budget fixed; if remarks do not improve certified results
  per budget, stop building diagnostic machinery.
  **G4 — does learning contribute?** Only after productive actions exist: learner vs
  fixed enumeration and rules at equal total budget, held-out projects and later
  compiler versions; if improvement comes only from remembering known patches or from
  undoing generated damage, **abandon the learned-company claim** (a rule engine may
  remain).
  **G5 — is the missing ingredient intrinsically bespoke at our budget?** An expert
  attempts a sample of the bot's failures under the same correctness policy; if
  experts find certifiable, valuable wins while successive automation attempts need
  new per-case semantic engineering rather than transferable capsules, **the
  automatic-engine recommendation has failed economically**. If experts also find
  nothing, service is not a refuge — change the target class or stop. Noted plainly,
  as instructed: a service outcome **conflicts with the owner's automatic-product
  goal**, and no finite trial proves automation impossible.

- **2026-09-27 — astra recommends: one engine with *certified semantic lanes*, not five
  language optimizers; the opportunity is probably concentrated and our sample cannot
  estimate its prevalence; and the ambitious target is an optimizer of *observation
  frontiers*.** Third consultation of the day, on multi-language scope and the
  distribution of opportunity. Recorded as recommendations under consideration.
  **CORRECTION (mine, two entries above): "C, C++ and Rust are one product and nearly
  free incrementally" is wrong.** Shared LLVM buys some code-generation and
  measurement infrastructure; it **does not buy shared source semantics, trustworthy
  extraction, or correct patch realisation. "Rust is not nearly free."** Certificates
  and source facts do not transfer unchecked.
  **CORRECTION (mine, last turn): "compilers are structurally forbidden from changing
  your data structures" is wrong.** "They may change unobservable representations while
  preserving ABI and persistent formats. Your possible advantage is **search budget,
  domain knowledge, scope and proof — not permission incumbents lack.**" The category
  claim I made is withdrawn as stated; what survives is that the wins we found required
  a representation change *across consumers*, which is a scope-and-proof problem rather
  than a structural prohibition.
  **Language ordering, by expected return and not popularity**: **1. C** (bounded
  unsigned scans — establish useful automatic yield before anything else); **2.
  C-shaped C++** (builtin scalar and buffer operations in checked build contexts —
  templates, overloads, RAII, exceptions and containers "are not another parser");
  **3. Rust** (safe primitive integers and slices — ownership can help establish
  isolation, but bounds panics, configured overflow behaviour, drops and memory
  semantics require real bridge work; benchmark rustc separately); **4. Java**; **5.
  Python**. "Full C++ can be harder than restricted Java."
  **My Java/Python decomposition was too categorical.** Java **does not** break the
  black-box objective — the trusted JVM handles compilation and deoptimisation, so
  evaluate a fixed runtime and configuration over declared startup, warmup, history and
  workload distributions; native timing is already statistical; and Java permits
  private working representations without exposing object field layout. Python **has an
  execution engine** even when it is CPython rather than an optimising compiler, and
  compiled and JIT paths exist, so replacing interpretation with an equivalent builtin
  or native island fits the same objective — but **replacing Python integers with NumPy
  integers is generally not equivalent.**
  **The smallest semantic bridge** is a *source-faithful checker for one declared
  fragment* that (1) certifies an exact source region against a machine-precise
  contract including definedness, errors and effects, (2) establishes entry, exit,
  alias/escape and frame conditions, and (3) certifies the target-language realisation,
  the safe guard/fallback and the composition of the exact patch — **trusted or
  translation-validated, not merely an AST exporter**; building and measuring are
  separate empirical adapters. A bridge for "unsigned sequence → first matching index"
  can be small; a complete language bridge cannot.
  **Product boundary**: ordinary source in, unchanged source or a reviewable patch out
  can remain one product across all five. If Python needs a proprietary runtime,
  annotations and a new deployment workflow, **that is another product launch, not a
  logo added to the same offering.**
  **On headroom (owner's belief: "headroom all over the place"):** **"No useful
  population rate follows from your sample."** The `redisPopcount` result that vanishes
  against the stronger baseline is a **product null, not evidence of available
  headroom.** Working prior: concentration in **small observable contracts with
  multiple efficient realisations** — broader than "named algebraic objects", narrower
  than ordinary hot code. And a warning aimed at us: **"your selection process and your
  researchers' vocabulary could themselves create the apparent concentration.
  'Everything successful turns out to have an algebraic description' is unfalsifiable
  unless classes are defined before searching."** Three distributions must be kept
  distinct: **latent** headroom (a better implementation exists), **reachable** headroom
  (we can discover and certify it automatically), **commercial** headroom (it occupies
  enough application cost, often enough, to matter). "Sparse transformations can
  support a general-purpose product if they recur across unrelated workloads and
  produce material aggregate savings without bespoke onboarding. One operation exposed
  through five languages remains a specialist capability."
  **The audit design, which replaces the G2 sampling discipline** — same 24-site budget,
  changed discipline: freeze **six untouched applications and workloads, across
  predeclared strata, before reading for optimisation opportunities**; per application
  take **two hottest non-overlapping regions plus two probability-sampled remaining hot
  regions**, recording sampling weights and failed intake; **tag every site before
  proposals** (scans/reductions/sets, other numeric work, pointer/control-heavy,
  external-cost dominated); ≤8 proposals per site with refusals in the denominator and
  outcomes separated into **unsupported / unproved / certified-but-code-identical /
  certified-but-unprofitable / profitable** — *"'unsupported' is not 'no
  opportunity'"*; include conversions, guards and surrounding execution in cost;
  replicate the HLL boundary comparison under matched workload and build conditions
  **before treating its exact ratios as training labels**; and give an independent
  researcher a capped, blinded audit of a random subset of misses, where **human-only
  wins do not count as automatic yield**. Report **project-level useful-patch yield,
  cost-weighted coverage and saved cost**, not function hit rate, accounting for
  clustering by project. **The crucial result is whether savings recur outside the
  originally favoured structural class — "do not retrospectively rename every winner to
  preserve the concentration thesis."**
  **The vision: optimize observation frontiers.** Not a library of representation
  changes but **a proof-guided optimizer of when concrete state must become
  observable**: a graph of contracted computations and logical states annotated with
  ownership, aliases, effects and possible observers, jointly searching algorithms and
  representations, **partial versus complete materialisation**, conversion placement and
  lifetime and tile size, cooperating representations and their coherence, and
  interpreter/native transitions. Beyond "make an island": if a packed merge feeds a
  read-only consumer, ask whether it can **merge and consume tile by tile, never
  materialising the intermediate at all** — **"the legal boundary is the next actual
  observer, not necessarily the function boundary or loop boundary."** The untrusted
  proposer should invent an intermediate sufficient state and its relational invariant
  rather than select from known layouts, and **a counterexample should inform the next
  *topology*** (a missed observer, an island that needs shortening, a stronger isolation
  fact) — the rejected candidate stays blocked, since changing the boundary creates a
  new obligation. Allocations failures, exception ordering and escaping aliases are
  **constraints, not inconvenient costs.** The transferable object is **a
  contract-and-observation plan plus a policy for purchasing the next useful proof or
  compiler experiment** — the same plan can place a Python/native boundary or a
  packed/unpacked boundary, with language bridges instantiating its semantics and the
  counterfactual corpus teaching its economics.
  **Path**: keep the first engineer-month gate native and narrow; first lock the
  live-miscompile failure mode and the source-extraction/patch-realisation edge cases
  into regression tests — *"tests do not establish soundness, but a single-digit suite
  is no basis for expanding the trusted semantic surface"* — and get stable measurement
  hardware; then run the sampled C trial with the existing success requirement. **Only
  after that passes**, build a narrowly capped **Rust primitive-slice witness bridge**
  with the C algorithm plans and planner frozen, requiring profitable automatic
  transfer on **independently authored Rust code — "not a translation manufactured for
  the demonstration"** — against its real rustc baseline. **"A scan transferred across
  languages validates portability, not general purpose."** The first credible broader
  milestone is a small **two-axis transfer test**: establish scans in C and Rust,
  establish a materially different contract in C (packed joins, if the audit supplies
  natural cases), freeze the generic bridge rules and planning machinery, then test the
  **missing Rust/second-contract cell on held-out code without site-specific
  additions** — two narrow bridges, not five frontends, and **"do not manufacture ports
  to fill the matrix — absence of natural cases is demand evidence."**
  **Falsification, with the failures kept distinct**: no useful automatic yield in the
  native gate → **stop language expansion** (this rejects the investable automatic
  offer, not the mathematical existence of headroom); humans repeatedly find wins while
  production sites need hand-supplied invariants → the automation claim fails;
  new sites need proof and planning logic rebuilt rather than generic bridge rules
  instantiated → the scalable-plugin thesis fails (fixed per-language integration is
  expected, per-site reinvention is not); supported contracts occupy little relevant
  application cost outside one vertical, or conversion-inclusive measurement erases the
  gains → the broad-market claim fails; fixed rules match the learner at equal budget →
  the learned moat fails, though not necessarily the automatic product; a few upstream
  patches consume nearly all opportunity with little recurring benefit across releases,
  targets or workloads → the recurring-CI economics fail. If opportunity is
  concentrated in a few contracts dominating CPU across many applications, a broadly
  deployed **sparse** engine is still justified; concentration in one sector calls for
  a **vertical automatic kernel-acceleration engine sold on repeated aggregate
  savings**, not five-language coverage. If a fixed library captures essentially all the
  value, the honest outcome is a lean library or vendor-component business — **"that is
  a change from the proposed superoptimizer company, not its validation."** Closing
  line, recorded as the standard this project should hold itself to: **"Narrowing to a
  genuinely automatic engine preserves the mission. Replacing missing automation with
  customer-specific engineering does not."**

- **2026-09-27 — G1 mutant audit, first run: the obligations are vacuous with respect to
  the transformations they gate, and a live S2-class miscompile passes the entire suite
  and the referee.** This is the fourth defective check in a week, and unlike the
  previous three it is not a slip in one apparatus — it is a **missing link in the
  architecture**. Found by running astra's G1 procedure, which is therefore validated
  as an audit design: it found this on the first execution.
  **The mechanism.** `patch::apply` checks local-id disjointness, an effects cap, an
  effects floor, use-rewriting success, the live-out gate and the structural verifier.
  **It checks no semantics.** The only semantic gate a family has is its `prove::*`
  obligation, and those prove *general identities that the transformation does not
  depend on*: `any.rs` gates a 64-element array OR-reduction unrolling on
  `or_laws(width)`, which proves exactly `0|x == x` and `x|0 == x` and returns `true`
  for any width in 1..=64. The obligation never mentions the array, the extent, the
  index, the predicate or the chain. `sum.rs` does the same with `sum_laws`. Neither
  can fail in its intended role, because neither is *about* its transformation.
  **Therefore** the semantic correctness of every family rests on (a) human reasoning
  about the recognizer and the constructor, and (b) the differential referee — which is
  a test, not a proof.
  **Demonstrated, not argued.** Two mutants were injected into `any.rs` and the full
  suite was run against each.
  *Mutant 1 — `NodeKind::Or` → `NodeKind::And` in the accumulator chain.* **Caught**,
  but only by `any_slice.rs`'s structural census (`filter(matches!(Or)).count() >= 63`),
  which fired because the node *kind* changed. It is an assertion about the shape of
  the emitted graph, not about its meaning.
  *Mutant 2 — the index constant `u64(index)` → `u64(0)`, so the chain reads `a[0]`
  sixty-four times.* **This is precisely the S2 class** (wrong index substitution) and
  it is structurally invisible: same node count, same kinds, same types, same effects.
  Result: **`cargo test --workspace` fully green**, `emit_c_diff` reporting
  `mismatched=0 ... clean=1 rewrites=1`, and the referee's own compiled driver printing
  `EMITDIFF ... mismatches=0`. The emitted C differs by **254 lines** (`p0[63], p0[62],
  …` in the correct build versus `p0[0]` sixty-four times in the mutant). The only
  signal anywhere was a rustc *unused-variable warning*.
  **Why the referee is blind here, exactly.** The function ends
  `bool v15 = (v208 != (0ULL))` — it returns *whether any element is nonzero*.
  Correct: `OR of all 64 != 0`. Mutant: `a[0] != 0`. The referee's driver fills its
  buffer with xorshift-derived **random bytes**, under which both sides are true with
  probability ≈1 — and its other inputs (`lens` up to 32, `keys`, and the `--zero`
  all-zero fill) do not separate them either. This is the **same class as the shipped
  S2 escape — "both sides agreed on random data" — reproduced rather than fixed.**
  **The separating input, and the control.** With every word zero except word 5 set to
  1: mutant gives `ref=1 new=0` (**MISMATCH**); the correct build gives `ref=1 new=1`
  (**AGREE**) on the identical input, so the probe is not a test that fails everything.
  The input is *sparse and structured* — the referee generates dense uniform bytes and
  zeros, and neither separates these two programs.
  **What this does and does not mean.** The proof kernel is not implicated: it
  re-evaluates a SAT model against the original terms before reporting a counterexample,
  and its refutation path is self-checking. The claim is narrower and worse for the
  architecture: **nothing mechanically connects a proved lemma to the transformation
  that cites it.** The families are asserted and tested, not proved, and the test that
  was supposed to cover them has a demonstrated blind spot. It does **not** follow that
  any shipped transformation is wrong; it follows that we have no mechanical evidence
  that any of them is right, and one worked example of the apparatus accepting a wrong
  one.
  **The repair, per astra's G1 and the S2 lesson.** The proof must cover the actual
  emitted transformation: translate the recognized original and the constructed patch
  into `sir_mech` terms over symbolic array elements and discharge
  `prove_equal` between them, per patch, at the real extent — instead of proving a
  general law and trusting the construction. That is a bounded addition to
  `sir_opt` and it is the thing that converts "asserted by construction" into
  "checked per patch". Until it exists, the honest description of a family firing is
  **"recognized, structurally gated, differentially tested"** — never "proved".

- **2026-09-27 — astra reviews the repair for the vacuous-obligation hole and refutes it
  as specified. The proposed fix would have proved the mutant correct.** Design review,
  `--thinking max`. Verdict: **"Not sound as specified."** The repair's shape survives;
  the details I specified do not.
  **THE CRITICAL CATCH — my own fix would have blessed the bug.** The design said to
  lower the original symbolically with a memo keyed by `NodeId`. **"Memoizing a
  loop-body node by `NodeId` across different substitution environments freezes its
  first iteration. The translated original can become `a[0]`, making the repeated-`a[0]`
  patch provably equal."** That is: the unroller visits the body for k=0, memoizes the
  access as `a[0]`, then returns the memoized term for every subsequent k — so the
  *original* lowers to sixty-four copies of `a[0]`, exactly the mutant, and
  `prove_equal` reports `Valid`. Sophisticated, wrong, and worse than the hole it was
  written to close, because it would have converted a blind spot into a certificate.
  **The general form of the error**: shared lowering is standard trusted machinery but
  is **not independent corroboration** — "translating signed extension as zero
  extension would validate an incorrect signed-to-unsigned conversion rewrite." A
  common translator bug erases the distinction the check exists to detect.
  **F1 — the translator's requirements**: a closed fail-closed operator whitelist with
  exact width, signedness, Boolean, conversion and partial-operation semantics;
  scope-correct symbol identities for actual boundary inputs shared consistently
  between the two sides; and **environment-sensitive substitution and memoization,
  including load keys**. Independent variables for reads *can* be sound without array
  theory under a fixed inert memory snapshot, but sharing a variable must guarantee the
  same concrete read — base/address, access width and memory state agreeing **after
  substitution**; **keying on the unsubstituted induction-variable node is
  insufficient.** Conservative non-sharing loses proofs but never creates false
  validity. And: a SAT model over BV terms **need not describe realizable memory**, so
  kernel re-evaluation does not establish a concrete SIR counterexample.
  **A second hole I had not seen — definedness.** Value equality would accept an
  eagerly executed `a[N] & 0` folded into a result: numerically null, but an
  out-of-bounds read. "An executed unused load is even easier to miss when traversing
  only the root." All executed computations must be checked for safety — including
  address calculations and partial arithmetic — and unmodelled effects rejected.
  **F2 — my independence argument is refuted.** I had argued the two derivations are
  structurally different, so a wrong unrolling discipline would decorrelate and fail
  closed. Counterexample: the source runs 64 iterations but analysis supplies `N = 63`
  **to both construction and validation**, both terms omit `a[63]`, and `Valid` follows
  while the transformation is wrong on a defined input. **"You are not proving the
  substitution. You are assuming it while manufacturing the original's term."** The
  analysis facts feed both sides, so a shared fact error is shared proof error. Two
  defensible choices: trust every fact producer (then the proof protects nothing
  against their mistakes), or — preferred — **treat the facts as hints checked against
  the actual SIR by a small, restrictive loop-semantics checker** reading the real
  binders, initial values, condition, step and yields, establishing the exact iteration
  count including termination and counter wrap and modelling simultaneous carried-value
  updates and all live-outs. Structural checking suffices; it need not go through SAT.
  And explicitly: **"do not remove `carried_names` merely because an equality query now
  exists — only when checked binding/substitution semantics cover the cases it
  excluded."**
  **F3 — replace the authorization, not the lemmas.** **"A universally true identity is
  not itself a disease. Citing it as authorization for unrelated code is."** `or_laws`
  may stay as a kernel test or a genuinely used lemma; it **must not authorize a
  patch.** **"Every distinct candidate needs validation"** — and **caching by
  `(extent, width)` immediately restores the hole, because the correct patch and the
  mutant share that key.** Reuse requires an exact canonical obligation with checked
  context and boundary mapping, or a proved schema with mechanically checked
  instantiation. Timeout, `Unknown` and unsupported semantics mean refusal; proof cost
  never justifies an unchecked subsequent application.
  **F4 — share bounded-loop semantics, not family descriptions.** One generic extractor
  for the supported fixed-count, inert-read fragment, interpreting actual loop
  semantics rather than reconstructing a fold from each family's assertions. `search`
  is not excluded a priori — a bounded first-match scan is a fold over
  `(found, first_index)` frozen after the first match — **but preserving its value is
  insufficient if expansion introduces reads the early exit avoided**; those must be
  proved safe and unobservable or remain guarded. Data-dependent bounds need a checked
  finite upper bound and faithful guarded execution, otherwise refuse, **with no
  fallback to general algebraic obligations.**
  **F5 — put the mandatory gate at the application boundary.** Do not hand
  `patch::apply` an unbound Boolean proof result. Materialise the candidate in a
  scratch function, validate the **actual before/after change**, then commit *that exact
  candidate*, comparing every affected live-out under a common boundary-input
  interpretation. **"A proof about the intended accumulator root does not prove that
  the replacement map uses that root, preserves other live-outs, or rewrites the
  intended uses. Candidate extraction, validation and commit must be bound together; do
  not validate one object and reconstruct another."** Certified rewrite schemas with
  checked matching are a possible cheaper alternative; **"calling a handwritten
  constructor 'correct-by-construction' is not."**
  **The new trusted base, stated**: `sir_mech`'s **`Valid` path** (BV semantics, query
  construction, bit-blasting, UNSAT correctness — model re-checking protects
  counterexample reporting, **not** `Valid`); SIR typing and scope verification;
  operator lowering; environment substitution; symbol allocation, memory abstraction
  and memoization; applicability, definedness and effect checks; loop
  extraction/unrolling and every analysis fact accepted unchecked; and boundary
  extraction plus the binding between proved candidate, replacement mapping, cache
  entry and committed function. **"Recognizers and patch constructors leave the semantic
  trusted base only when none of their semantic claims are accepted unchecked."**
  Emitted-C correctness additionally still depends on faithful SIR-to-C lowering.
  **Closing condition**: "With those conditions, symbolic equality is a sound gate.
  Without them, the hole moves principally into 'what the original means', with a
  separate definedness hole hidden by treating loads as values." Also required: mutate
  the translator and unroller themselves, using independent concrete semantics as the
  oracle, with small exhaustive and sparse cases through the existing `run()` dispatch
  — **"tests harden this trusted code; they do not establish its correctness."**

- **2026-09-27 — step 1 of the repair: `loop_shape::checked_counted_loop`, and the
  measurement that the family's trip-count gate cannot tell two loops apart.**
  New module `sir_analysis::loop_shape`. It reconstructs a counted loop's semantics
  from the SIR — the counter's **initial value** (read off the carried input's own
  constant node), the step and direction, the relation, the bound — and computes the
  **exact** iteration count, refusing anything it cannot state: signed counters,
  compound guards, a successor compared by anything other than equality, a
  non-constant bound, a step that would overflow the counter's width, and any relation
  that leaves the described span. `loops.rs` by contrast reads only the constant bound;
  its own comment says the result is "at most that bound", yet families consume it as
  an exact count via `fact.trip_count != Some(extent)`.
  **Measured, and pinned as an assertion** rather than argued. For the canonical
  `p07_any_or_const_u32` (counter starts 0, 64 iterations) and for a variant identical
  except that **the counter starts at 5** (59 iterations, reading `a[5..64]`):
  `loops.rs` reports **`trip_count=Some(64)` for both**, and `loop_shape` reports
  **64 and 59**. Since the gate compares the count against the array length — also
  64 — **it passes on the 59-iteration loop as readily as on the 64-iteration one.**
  The gate believed to reject a mis-shaped counted loop does not distinguish these two
  programs.
  **What is not yet known, and is recorded as unknown.** `optimize_any` does refuse the
  start-5 fixture today, but **not on the trip count — that gate passes.** Whatever
  else refuses it has not been identified. Until it is, the refusal cannot be
  attributed to the check we believe provides it, and "it is refused in practice" is
  not the same claim as "it is refused by this gate for this reason."
  **A constraint found upstream.** A counted loop stepping by two is refused by the
  **lowerer**, not by the analysis: *"rotated loop exit (back-edge on false) that is
  not the counted eq-next form (`icmp eq next, bound` with `next = carry + 1`) …"*. So
  the **step is already pinned to 1** for this form, and the unchecked dimensions are
  the counter's initial value, the relation's direction, and the width. That shrinks
  the exposure the checked extractor has to cover, and it is the first thing in this
  chain found to be genuinely constrained rather than assumed.
  **Verified state**: build warning-free, all 10 tests green, test count unchanged (the
  new case sits behind the existing `families_rewrite_and_prove` through the `run()`
  dispatch). Next: consume `checked_counted_loop` in place of the fact, having first
  identified what actually refuses start-5.

- **2026-09-27 — traced: what refuses the start-5 loop is the lowerer's type
  inference, not the trip count. The loose count is latent, not live — and the
  safety lives in a crate that was never asked to provide it.**
  `any` requires the base of the element access to be `Type::Array { length, .. }`.
  That type is produced by `sir_lower::promote_constant_extent_buffers`, which
  promotes a `Pointer` parameter to `Array { length: K }` only when every access is
  provably within a common constant extent, and which obtains that extent from
  **`sir_lower::strict_counted_bound`**. That function opens with:
  ```rust
  if lower_constant_u64(func, counter) != Some(0) { return None; }
  ```
  **It requires the counter to start at exactly 0**, and it separately requires the
  successor to be `counter + 1`. With a start of 5 it refuses; the parameter is never
  promoted; it stays a `Pointer`; and `any`'s array requirement is what rejects the
  function. Verified: the canonical fixture's parameter is
  `Array { length: 64 }` and the start-5 fixture's is `*const u32`.
  **The conclusion that matters.** No family can presently act on a loop whose trip
  count is wrong, because acting requires an `Array`-typed base, which requires
  promotion, which requires `counter == 0` and `step == 1` — and **under exactly those
  two conditions the bound genuinely equals the count** (`0, 1, … , bound-1` is `bound`
  iterations). The loose extractor is therefore correct on the only inputs reachable
  through the current families. The exposure is real but unexercised.
  **Why this is still worth fixing, stated precisely.** The safety is not provided by
  the check we believed provided it — it is provided by two constraints in `sir_lower`
  that were written for a different purpose (deciding whether a buffer's extent is
  provable), and that nobody has been treating as a correctness gate for the families.
  It is fragile in one specific way: **any relaxation of promotion — and supporting a
  non-zero loop start is a plausible feature, not an exotic one — immediately exposes
  the loose count in `loops.rs`,** and the family would then unroll `a[0..64]` for a
  loop that reads `a[5..64]`.
  **A duplication to resolve.** There are now two extractors answering "how many times
  does this loop run": `sir_lower::strict_counted_bound` (strict, requires init 0 and
  step 1, returns the bound) and `sir_analysis::loops::estimate_trip_count` (loose,
  requires neither, returns the bound, is consumed as an exact count).
  `loop_shape::checked_counted_loop` is a third and is exact for a strictly larger
  class. The open decision is whether `loops.rs` should delegate to the checked
  extractor so the fact database reports exact counts, or whether the fact and the
  lowerer's internal bound should remain separate questions. Not yet decided; recorded
  rather than assumed.
  **State**: build warning-free, all 10 tests green, test count unchanged.

- **2026-09-27 — the five fold families now consume the checked count, at zero
  firing cost, and the guard is demonstrated load-bearing rather than insurance.**
  `any`, `all`, `count`, `parity` and `sum` replaced
  `fact.trip_count != Some(extent as u64)` with
  `loop_shape::checked_counted_loop(function, loop_id)?`, requiring both
  `shape.trip_count == extent` **and `shape.counter == counter.variable`** — the
  counter the checker identified must be the same node the family chose from the fact
  database, so a disagreement between the two recognitions refuses rather than picks.
  **Firing cost: zero.** Total rewrites across the whole `gate6a` corpus are **507 with
  the guard and 507 without** — measured by reverting the five files and re-running,
  not assumed. The new guard loses nothing that fired before.
  **The control, which is the point.** Retype the start-5 fixture's buffer as
  `Array { u32, 64 }` — exactly what `promote_constant_extent_buffers` would produce if
  it accepted a non-zero loop start — and the type obstacle is gone, the array length is
  64, and `loops.rs` still reports 64. **With the guard reverted, `optimize_any` returns
  `Some`**: it would unroll `a[0..64]` for a loop that reads `a[5..64]`. **With the
  guard, it is refused.** So the guard is not merely insurance against a hypothetical
  relaxation; it is the only thing standing between a relaxed extent promotion and a
  miscompile, and that is now demonstrated rather than argued.
  **What was deliberately *not* changed.** `runtime_sum`, `runtime_count` and `search`
  use `fact.trip_count.is_some()` with the **inverted** polarity — "the analysis found
  a constant bound", i.e. *this loop is not runtime-bounded*. Substituting the checked
  extractor there would silently change the predicate: a loop with a constant bound but
  a signed counter is `is_some() == true` under the loose extractor and
  `checked_counted_loop() == None` under the strict one, so the runtime family would
  begin claiming loops it never claimed. That is a different question with a different
  failure mode and it needs its own analysis; it is recorded here as outstanding rather
  than changed in passing. `popcount` is a scalar SWAR transform with no extent and no
  loop domain, so it has nothing to consume.
  **State**: build warning-free, all 10 tests green, test count unchanged.

- **2026-09-27 — step 2, first piece: `sir_opt::symbolic`, and its central property is
  mutated-and-caught rather than asserted.**
  The translator lowers SIR to bitvector terms under a substitution environment. The
  design decisions, each of them a direct response to a specific way this could be
  wrong:
  **The memo is per-call and never shared across environments.** `lower` mints a fresh
  memo and consults the environment before it. Astra's predicted route back to the
  mutant was "memoising a loop-body node by `NodeId` across different substitution
  environments freezes its first iteration"; this is shaped so that cannot happen, and
  the property is now tested rather than intended.
  **A closed, fail-closed operator whitelist.** Constant, Parameter, ArrayAccess,
  Convert (extend/truncate with the exact target width), Add/Sub/Mul/And/Or/Xor/Shl,
  Shr, Lt/Le/Gt/Ge/Eq/Ne, BoolAnd/BoolOr/BoolNot, Select. Everything else returns
  `None`. Two refusals are deliberate rather than incidental: an **arithmetic shift is
  not lowered at all** — a logical shift on a signed operand is a different function,
  and lowering one as the other is exactly the "nearly right operator" that would
  validate rewrites wrong on the inputs that distinguish them — and **i128 refuses**,
  being outside `sir_mech`'s 64-bit `MAX_WIDTH`.
  **Array reads without array theory.** Each `(array, concrete index)` pair is one
  symbolic variable, shared across every call on one translator, so the same read in
  the original and in the patch denotes the same value. A non-concrete index
  **refuses**: independent variables for possibly-aliasing reads would be conservative
  but could prove nothing, and sharing them without a common key would be unsound.
  **The test, and its control.** `symbolic_lowering_is_environment_sensitive` lowers
  the same element access at `k = 0` and `k = 63` and requires the two terms to differ
  — and separately requires the *same* read reached from two calls to be the *same*
  term, since without that no transformation could ever be proved. Then the translator
  was mutated: `lower` rewritten to share one memo across every call.
  **The mutation fails the test** with `left: 2, right: 2` — both environments
  returning the same term — which is the predicted defect reproduced exactly. So the
  check can fail, and fails for the right reason. Restored and re-verified: build
  warning-free, all 10 tests green, test count unchanged.
  **Not yet done, and required before any of this gates anything**: the symbolic
  unroller for the original loop; **definedness checking** — value equality alone would
  accept an eagerly executed `a[N] & 0`, which is numerically null but an out-of-bounds
  read, and an executed unused load is easier still to miss; the boundary gate that
  binds extraction, validation and commit to the same artifact; and the wiring into the
  families, which is where `prove_equal(original, patch)` finally replaces the vacuous
  obligation.

- **2026-09-27 — step 2, second piece: `symbolic::unroll`, and the query that finally
  refutes the mutant. The count is validated against the loop's own exit expression
  rather than taken from the analysis.**
  This is the piece astra's F2 refutation was aimed at — *"you are not proving the
  substitution, you are assuming it while manufacturing the original's term"* — so it
  is built the other way round:
  **The counter is derived, not imposed.** `Bv::bin` folds constant operands, so
  running the loop's *own* output expression for the counter (`counter + step`)
  produces a concrete constant at every step, and array indices resolve to concrete
  values from the loop as written. `loop_shape`'s predicted value is then required to
  **agree** with the derived one at every step; a disagreement refuses rather than
  picking one.
  **The count is validated against the actual exit expression.** The termination node
  is translated at each step and required to fold to a constant — the same constant
  throughout the iterations and a **different** one at the count. That condition is
  stated polarity-agnostically on purpose: *"constant, then different"* is the loop
  running exactly `n` times whichever way round the termination node is written, and
  the polarity was not something I was willing to assume after being wrong about this
  IR once already today.
  **Simultaneous update.** Every output is evaluated against the *same* pre-iteration
  values, which is the carried-value semantics with no phi nodes.
  **All carried outputs are returned**, so every live-out is covered rather than only
  the slot a family happens to want. Nested loops refuse. A non-concrete counter
  refuses.
  **The test, and both controls.**
  `the_unrolled_original_refutes_the_wrong_patch` unrolls `p07_any_or_const_u32` and
  asserts the count is 64, then queries the unrolled original against two candidate
  outcomes: the correct OR chain over `a[0..64]` must be **`Valid`**, and the mutant —
  the same chain reading `a[0]` sixty-four times — must be a **`Counterexample`**.
  The second is the query that finally refutes the transformation the vacuous
  obligation was certifying. The first matters as much: a check that refutes
  everything is as useless as one that refutes nothing.
  Then the count check was mutated. `loop_shape` was changed to report one iteration
  more than the loop runs, and the **unroller refuses** — `unroll` returns `None` and
  the test fails at "the canonical reduction unrolls". So the validation is
  load-bearing and can fail. (The first attempt at this control was caught earlier, by
  the families' extent check, before the unroller ran; the dispatch order was changed
  so the symbolic case runs first and the unroller's own check is what fires.)
  Restored and re-verified: build warning-free, all 10 tests green, test count
  unchanged.
  **Still outstanding before this gates anything**: **definedness** — value equality
  alone would accept an eagerly executed `a[N] & 0`, which is numerically null but an
  out-of-bounds read, and an executed unused load is easier still to miss; the
  **boundary gate** that binds extraction, validation and commit to the same artifact;
  and the **wiring** into the families, where `prove_equal(original, patch)` replaces
  the vacuous obligation.

- **2026-09-27 — step 2, third piece: definedness, and the demonstration that value
  equality cannot see it.**
  The equality query is blind to an out-of-bounds read, and this is now shown rather
  than argued. `a[N] & 0` is numerically null, so it **proves equal to zero** — the read
  rides into the result and the kernel reports `Valid`. That is exactly the hole astra
  named: *"value equality would accept an eagerly executed `a[N] & 0` … an executed
  unused load is even easier to miss when traversing only the root."*
  **The checks added.** Every `ArrayAccess` must have a concrete index after
  substitution **strictly inside the object's declared extent**; a base with no
  declared `Array` extent refuses rather than being assumed safe. Shift amounts must be
  concrete and less than the operand width — `Bv` totalises an out-of-range shift, so
  the term language cannot be relied on to have noticed, and a symbolic shift amount is
  refused rather than assumed in range. Division and remainder already refuse outright:
  they are absent from the operator whitelist.
  **The test, and its control.**
  `an_out_of_bounds_read_is_refused_and_value_equality_is_blind` asserts three things:
  `a[63]` lowers (inside a 64-element object), `a[64]` is refused, and — the point —
  `a[64] & 0` **proves equal to `0`**, which is why the bounds check has to exist
  separately. The out-of-bounds term is assembled by hand *because the lowering refuses
  to produce it*. Removing the bounds check makes the test fail at exactly
  `"a[64] is outside the object and must be refused"`. Restored and re-verified: build
  warning-free, all 10 tests green, test count unchanged.
  **A consequence worth stating for the wiring step.** Refusing a symbolic shift amount
  costs nothing today because `symbolic` is not yet consumed by any family — there is no
  firing to lose. When it is wired, that refusal becomes a real guard with a real cost,
  and the cost must be measured and recorded rather than assumed to be zero.
  **Still outstanding**: the **boundary gate** — extraction, validation and commit bound
  to the same artifact — and the **wiring** into the families, where
  `prove_equal(original, patch)` replaces the vacuous obligation.

- **2026-09-27 — step 2, fourth piece: the boundary gate. The artifact the families
  actually build is now proved equivalent to the function it replaces — and a
  wrong candidate is refuted.**
  `symbolic::equivalent(original, candidate)` lowers both functions to terms and
  queries the kernel. This is the gate astra specified: *"candidate extraction,
  validation and commit must be bound together; do not validate one object and
  reconstruct another"* — a proof about the intended accumulator root is not a proof
  that the replacement map uses that root or rewrites the intended uses.
  **Three defects were found and fixed while building it, each of which would have made
  the gate silently useless rather than loudly wrong:**
  1. **One translator per arena.** `Translator` owns a `Function`, and the candidate
     produced by `patch::apply` has its own arena with ids the original never had.
     Lowering both sides through one translator would have looked the candidate's nodes
     up in the original and read the wrong ones. Each side now gets its own translator
     and `inherit` carries the *shared meanings* across — which arena the terms live in,
     and which array read denotes which value. Without the latter the two sides would
     describe unrelated memory and nothing could ever be proved.
  2. **The loop is reached through the return, not as the root.** The first version
     handled `TupleExtract` only at the root, but the original's result is
     `zext(acc != 0)` for a predicate reduction, so the loop is reached through two
     further nodes. `TupleExtract` on a `Loop` is now handled inside the walk.
  3. **`return_node` names the `Return` node, not the value.** Lowering it directly
     refused with `WALK refuse %17: %17: () = Return`; the value is its operand.
  **The test.** `the_gate_validates_the_actual_artifact` runs the real `optimize_any` on
  the fixture, asserts the candidate genuinely differs (different arena length, no loop
  remaining), asserts each function is equivalent to **itself** — which discriminates a
  broken lowering from a broken transformation — and then asserts the candidate is
  equivalent to the original, and that a candidate whose accumulator has been rewritten
  to depend on nothing is **refused**. Both directions hold.
  **A misreading to record, because it is the same failure this project keeps having.**
  On first running, the gate printed a counterexample and I read that as "the real
  artifact does not prove equivalent" and started hunting the transformation. The test
  line said `ok`: the counterexample came from the *control*, which is supposed to be
  refuted. Reading the evidence I had asked for, rather than the evidence I expected,
  is the discipline this ledger exists to enforce, and I did not apply it for one step.
  **State**: build warning-free, all 10 tests green, test count unchanged.
  **Remaining**: the wiring — families calling `equivalent` on the candidate before
  committing it, and refusing when it does not hold — after which `or_laws` stops
  authorising anything.

- **2026-09-27 — FOUND WHILE MEASURING THE WIRING: a live miscompile in
  `runtime_sum`, in a corpus the gate does not run.** Found by measuring the firing
  cost of the wiring across all of `gate6a/`, not by looking for it. **Pre-existing and
  not caused by the wiring** — the pre-wiring baseline reproduces it identically.
  **The defect.** `gate6a/v1_corpus.ll::v08_sum_bounded(p0, p1, p2)` sums `a[start..n]`
  where `start` is `p2` and `n` is `p1`, with an early exit returning 0 when
  `start >= n`. The emitted code is:
  ```c
  uint64_t v08_sum_bounded(const uint8_t *p0, uint64_t p1, uint64_t p2) {
      return __sir_sum_u8(p0, p1);      /* p2 — `start` — never appears */
  }
  ```
  **The third parameter is dropped entirely and the guard is gone.** For any `start != 0`
  this computes a different function; where the original returns 0 on the `start >= n`
  path, the rewrite returns the sum of `a[0..n]`. Measured on the referee's own driver:
  `t=1 ref=0 new=236`, `t=2 ref=0 new=292`, `t=3 ref=0 new=303` — **three of four cases,
  on defined executions.** `t=0` agrees because `n=0`, which is why a two-case run that
  stops at `t=0` would report clean.
  **Why it was invisible.** `verify.sh`'s base-14 list is
  `v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v16` — **`v1` is not in it.** The corpus
  containing this function has never been run by the gate, and the exclusion carries no
  comment explaining it. A historical record, `docs/GATE6A_V1_RESULTS.md:54`, lists
  `v08_sum_bounded  YES  PASS  2  -`; either the corpus or the family has changed since
  it was written. Not investigated further — recorded as unknown.
  **Why the new gate does not catch it.** `runtime_sum` is one of the three families
  deliberately left unwired, because `fact.trip_count.is_some()` there carries the
  *inverted* polarity ("this loop is not runtime-bounded"), and substituting the checked
  extractor would silently change the predicate. So `equivalent` does not gate it, and
  the SIR-level transformation is a different object from the emitted C. This is exactly
  astra's point: *"emitted-C correctness additionally still depends on faithful
  SIR-to-C lowering."*
  **Against the standing correctness policy** this is a counterexample on a defined
  execution, which **blocks the affected transformation** until its applicability
  condition or its implementation is corrected. Recommended, not yet done: add `v1` to
  the base-14 list so the gate is red rather than silent — a corpus excluded without a
  reason is the same defect as a check that cannot fail — then repair or restrict
  `runtime_sum` to shapes where the lower bound is provably zero.

- **2026-09-27 — the gate's corpus list is derived from disk, not maintained by hand;
  coverage went from 14 corpora to 22, and the gate is now RED on a real miscompile.**
  `verify.sh` listed its corpora explicitly (`v2 v3 … v14 v16`) in both the native and
  AArch64 loops. **Five corpora on disk were never run in any mode**: `v1_corpus.ll`,
  `v18_corpus.ll`, `v21_popcount16.ll`, `v22_popcount64.ll`, `heldout_corpus.ll`. The
  list is now a glob over `gate6a/v*.ll`, so **a corpus added to the directory is gated
  by existing**, and this cannot recur by omission. **22 corpora are now run, up from
  14.** `heldout_corpus.ll` stays excluded and says so in its name — it is the only file
  in the directory excluded by intent rather than by accident.
  **The gate is now RED**, failing on exactly `FAIL corpus/v1_corpus exit=1` — the
  `runtime_sum` miscompile recorded above. A gate that stays green by not looking is
  worse than one that fails honestly, and this one failed honestly on the first run.
  **A check confirmed sound rather than assumed.** `run_case` **does** treat
  `harness_errors=[1-9]` as a failure alongside `mismatched=[1-9]`, so emitted C that
  does not compile fails the gate loudly. I checked it because assuming it would have
  been the same mistake as the corpus list.
  **A second defect surfaced, in an ungated corpus.** `heldout_corpus.ll::n04_stride2`
  produces emitted C that **does not compile**:
  `emitted.c:19:16: error: use of undeclared identifier 'v3'`. Recorded as a known-broken
  emitter output, not repaired. It never failed anything because the corpus is not run;
  `run_case` would catch it the moment it were, which is the argument for the glob in one
  line.
  **Also found, not defects**: `v18`, `v21_popcount16` and `v22_popcount64` are clean;
  the three `lower_refused` rows in `heldout` are the lowerer refusing correctly
  (separate latch block, unmodelled rotated exit, unresolved float phi).
  **External corpora remain WARN** — 19/19 absent, requiring
  `scripts/fetch-external.sh`. Unchanged and not the cause of the red.
  **Consequence for the historical record**: `docs/GATE6A_V1_RESULTS.md` records
  `v08_sum_bounded` as PASS. It is not. Any "PASS" in the older gate documents must be
  read as "passed the checks that happened to be run", and where a corpus was excluded
  by hand the two are not the same claim.

- **2026-09-27 — `runtime_sum` repaired: one check, one firing of cost, and that firing
  was the one producing wrong code.**
  The family requires the accumulator to start at zero but **never checked where the
  counter starts**, while the helper it emits (`__sir_sum_u8(a, n)`) sums from index 0.
  One added condition — the counter's carried input must be a constant zero — closes it,
  and it discharges the guard for free: with a zero start, `v08_sum_bounded`'s
  `start >= n` early exit is taken only when `n` is zero, where both sides return 0.
  **Measured**: `v08_sum_bounded` now reports `rewrites=0` and `mismatched=0`; the gate is
  **`VERIFY PASS mode=fast`**; and the total firing count across all of `gate6a` is **506,
  down from 507** — the guard's entire cost is the single firing that was miscompiling.
  That is the shape a correctness guard should have: it removes the wrong transformation
  and nothing else.
  **Note on the polarity question left open earlier.** `runtime_sum` still uses
  `fact.trip_count.is_some()` with its inverted meaning, and this repair deliberately
  does **not** touch it. The defect was a missing initial-value check, not the polarity,
  and fixing the thing actually wrong is preferable to changing a predicate whose
  failure mode has not been analysed. The polarity question remains open and is recorded
  as such rather than folded into this change.

- **2026-09-27 — loop-domain logic consolidated from three implementations to two, in
  one place — and the two that remain are different questions that must stay
  different.**
  The question "how many times does this loop run" was answered in three places:
  `sir_lower::strict_counted_bound` (+ its `find_counter_bound_conjunct`),
  `sir_analysis::loops::estimate_trip_count`, and the new
  `sir_analysis::loop_shape::checked_counted_loop`. They disagreed, and that
  disagreement was what created the latent trip-count hole — the loose one was safe only
  because the strict one refused first, in another crate, for another reason.
  **What was done.** `counted_bound` and its conjunct walk moved into
  `sir_analysis::loop_shape`, faithfully — including the signed-constant fallback in
  `lower_constant_u64`, which differs from `loop_shape::constant` and would have
  quietly changed which loops get promoted. `sir_lower` now calls it, and the two
  functions were deleted there. `sir_lower` gained a dependency on `sir_analysis`
  (acyclic: `sir_analysis` does not depend on `sir_lower`).
  **What this is not.** It is a **move, not a merge**. Three implementations became two,
  both now in `sir_analysis`, and `loops.rs::estimate_trip_count` remains — because
  changing *it* changes the `is_some()` predicate that `runtime_sum`, `runtime_count`
  and `search` consume, which is the unwired polarity question, which needs its own
  analysis and did not belong folded into a mechanical change.
  **And the two that remain genuinely differ.** They were not merged because they answer
  different questions, and the case that separates them is real: for
  `!found && i < K` the indices are still bounded by `K` — a valid **extent** — while
  the loop may run **fewer than K times**. A bound is not a count. Collapsing them would
  either make promotion unsound or make the families' count wrong, so
  `counted_bound` (bound, tolerant of conjuncts) and `checked_counted_loop` (exact
  count, refuses a compound termination) now sit adjacent in one file with that
  distinction written down — which is the thing that was missing, and the reason they
  were able to disagree silently for so long.
  **Verified identical behaviour**, which is the whole safety argument for touching the
  extent promotion that accidentally protects five families: **506 rewrites, unchanged**;
  `VERIFY PASS mode=fast`; all tests green; zero warnings. `sir_lower` is 53 lines
  shorter (11,645 → 11,592).

- **2026-09-27 — the polarity question resolved: the predicate was right and the field
  name was wrong. Three questions, three answers, now all named in one crate.**
  `runtime_sum`, `runtime_count` and `search` refuse when `fact.trip_count.is_some()`.
  Analysed before changing, and **swapping in the checked extractor would have been
  wrong**: `is_some()` here asks *"is the bound a compile-time constant?"* — a
  **dispatch** decision meaning "let the static family take this loop" — while
  `checked_counted_loop(...).is_some()` asks *"is this a well-formed exact counted
  loop?"* The second is stricter, so substituting it would make the runtime-bounded
  families fire on loops they never claimed. That is a dispatch change with no
  correctness content, made in the name of tidiness, and it is the kind of change this
  ledger exists to refuse.
  **What was actually wrong was the name.** The field was called `trip_count`, which
  invited exactly the reading that produced the original defect. It is now
  `LoopFact::constant_bound`, with a doc comment stating that it is a bound and not a
  count, that it equals the iteration count only when the counter starts at zero and
  steps by one, that this analysis checks neither, and which function a caller wants for
  each of the three questions. `is_finite` was dead and is removed.
  **The three questions, now distinguishable by name and sitting in `sir_analysis`:**
  | question | answer |
  |---|---|
  | how many times does this loop run? | `loop_shape::checked_counted_loop` — exact, refuses a compound termination |
  | how far can this counter's index reach? | `loop_shape::counted_bound` — an extent, tolerant of conjuncts |
  | is the bound a compile-time constant? | `LoopFact::constant_bound` — a dispatch signal, no correctness rides on it |
  The case that separates the first two is `!found && i < K`: the indices are bounded by
  `K` while the loop may run fewer than `K` times. The case that separates the third is a
  constant bound with a counter that does not start at zero — static-bounded, but not a
  well-formed count. All three are now in one crate with those differences written down,
  which is the thing whose absence let them disagree silently.
  **My own verification failed, in the exact pattern this session is about.** I checked
  the rename with `grep -c "test result: FAILED"`, which returned `0` — not because the
  tests passed but because two test files still referenced the old field name, so the
  test binary **failed to compile and no test ran at all**. Zero failures because zero
  tests ran. The gate caught it (`FAIL workspace tests`); my grep did not. The
  verification habit this needs is to check the **exit status**, not to search the output
  for a failure string.
  **Verified**: 506 rewrites unchanged, `VERIFY PASS mode=fast`, all tests green by exit
  status, zero warnings.

- **2026-09-27 — `n04_stride2` repaired: the lowerer was right, the emitter declared a
  node inside a loop and read it after the loop. The held-out corpus is now gated, and
  the guard behind the fix is recorded as unexercised.**
  The known-broken output (`emitted.c:19:16: error: use of undeclared identifier 'v3'`)
  is fixed. **The SIR was correct** — `%15 = Lt(%6, %3)` is the right *pre-tested*
  termination for a post-tested LLVM loop, checked by hand at n=0, 1 and 2 — so this was
  never a lowering defect, only an emission one.
  **The cause.** The body walk emits each statement's *whole dependency closure* where
  the statement stands, and reports those ids as emitted inside the loop, which removes
  them from the post-loop pass. `%3 = p1 << 1` sat in the body as a dependency of a
  **dead** comparison (`%14 = Lt(%13, %3)`, which nothing consumes — the termination is
  `%15`), was claimed as emitted, and the post-loop `%5 = (%3 == 0)` then read a variable
  that had been declared in a scope that had closed. **A dead statement is what made it
  fire**, which is why it looks arbitrary: remove `%14` and the bug vanishes without
  anything being fixed.
  **The fix** hoists such nodes in front of the loops. It generalises a patch already in
  the tree that does exactly this for `NodeKind::Global` alone, with the same comment —
  the general case had been met before and handled one node kind at a time.
  **An attribution I had to correct.** My first attempt hoisted too much and the referee
  reported `mismatched=7` of 8. I attributed that to the absence of the `varying` filter.
  That was wrong: the cause was that a `Loop`'s `input_nodes()` returns its own body,
  termination, outputs and carried inputs, so the loop read as an *outside consumer of
  itself* and the entire body was hoisted — `o0_1 = v11` with `v11` frozen at `0 + a[0]`,
  so every answer was `a[0]`. The mutant is the control for **skipping Loop nodes in the
  outside-consumer scan**, and it is a good one: 7 of 8 cases wrong, caught by the
  referee, on a function the gate had never run.
  **The `varying` filter, measured rather than assumed.** Hoisting is only sound for a
  node whose value is the same before the loop as inside it — the prelude is emitted
  against no carrier substitution, so hoisting a node reading a carried input or a loop
  output freezes it. The filter is the forward closure of `{carried inputs} ∪ {outputs}`.
  **It never drops a node: 0 firings across 847 functions in 23 corpora**, and disabling
  it entirely (`varying.clear()`) leaves every corpus clean with rewrites identical.
  **The gate result is itself the stronger evidence, and it needs no instrumentation.**
  Every node in `shared` has an outside reader *by construction* — that is what puts it
  there. If the filter dropped one, that node would stay declared inside the loop while
  its reader is emitted outside it, which is precisely the `use of undeclared
  identifier` failure `run_case` treats as a hard error. So **`harness_errors=0` over
  the whole corpus set, external corpora included, is proof the filter never fires** —
  a consequence of the failure mode rather than a sample. So
  it is a **soundness precondition, not a live check**: post-loop code reads a loop's
  result through `TupleExtract(loop, i)`, whose input is the Loop and not the output, so
  an output never reaches `shared`, and a body-internal node has no outside user by
  construction. It is kept because it fails *closed* if a lowerer change breaks that
  contract — the node stays in the loop, the C does not compile, and `run_case` reports a
  hard failure rather than a wrong answer. Recorded as unexercised so it is not later
  mistaken for evidence.
  **`heldout_corpus.ll` is now in the gate: 23 corpora, up from 22.** It was excluded
  "by intent" for being held out — but **a corpus that is never run is never scored**, so
  it was not held out, only unexamined, and a defect lived in it undetected for exactly
  as long as that line did. It reports `clean=13 mismatched=0 lower_refused=3
  harness_errors=0`; the three refusals are correct refusals. The name is now history:
  it is no longer held out in any meaningful sense, having supplied a defect that was
  then repaired against it, and a genuine held-out corpus would have to be authored and
  kept out of this loop.
  **The gate is now complete, not merely green.** The 19 external Embench corpora were
  fetched (`scripts/fetch-external.sh`), so the run that used to end in `WARN 19/19
  external corpora absent — the gate is NOT complete` now ends with no warning at all.
  **42 corpora, every one passing: 23 base + 19 external, zero skips, `mismatched=0` and
  `harness_errors=0` throughout.** The external set is **114 clean / 207 lower-refused
  out of 321 functions (35.5%)** — third-party code, differentially clean against
  clang. (The plan document quotes 117/204 for an earlier tree; refusals come from the
  lowerer, which this change does not touch, so that difference predates it and is
  recorded here rather than reconciled by assumption.)
  **Verified**: `VERIFY PASS mode=fast`, workspace tests `passed=10 failed=0`,
  **rewrites 506 excluding held-out — identical to before the change**, zero warnings.
  `n04_stride2` is exercised at **length 0** (its `avoid_zero` is false), which is the
  boundary its `bound == 0` guard exists for.
  **Noted, not defects**: `n04_stride2` emits two identical loads (`%8` and `%9` are the
  same `ArrayAccess`, `%8` unused) and a dead comparison (`v14`). Both are clang-trimmed
  and neither is wrong; the dead one is what exposed the scope bug.

- **2026-09-27 — the opportunity audit ran: 24/24 sites refused at intake, yield zero.
  The constraint is that we cannot read real code, not that we misrecognise it.**
  Pre-registered before any application was read (`docs/AUDIT_PREREGISTRATION.md`, six
  applications frozen by workload class, per-application predictions, tags and success
  criteria fixed in advance). Full detail in `docs/AUDIT_RESULTS.md`. The result:
  **every one of 24 sampled sites across GNU grep, zstd, jq, sqlite, libpng and lua is
  `intake-failed`.** Not one proposal was constructed. Zero sites reached `unsupported`,
  so **the families never ran on real code and their coverage of it is untested by this
  audit** — the audit measured the lowerer's intake, not the domain hypothesis.
  **What the refusals are made of.** 18 of 24 (75%) are *the code calls a function we do
  not model* — both libc (`memchr2`, `rawmemchr`, `memchr`, `strlen`, `crc32`,
  `__assert_fail`) and the application's own helpers (`png_error`, `jv_invalid`,
  `HIST_add`, `luaG_tracecall`). The rest are three control-flow reconstruction failures:
  branch-guarded memory through pointer operands (3), a separate latch block (2),
  multiple loops sharing an exit CFG (1).
  **This inverts the project's account of its own problem.** The recorded story is that
  clang vectorizes the target shapes before the lowerer sees them, implying the remedy is
  better *recognition*. The measurement puts the obstacle one stage earlier: **we refuse
  to read the code.** A single unmodelled call rejects a function that is otherwise
  exactly the shape the families exist to transform. libpng's `png_calculate_crc` — a CRC
  over a buffer, about as close to a counted single-stream reduction as real code gets —
  is refused because it calls `crc32`.
  **Both predictions of yield were falsified**, including the one the project called its
  wedge: `grep`'s Boyer-Moore executor `bmexec_trans` is 97.45% of that workload and is
  rejected for calling `memchr2`. The three "no yield" predictions were right, but for a
  reason they did not name — they hit the same intake wall as the ones predicted to
  succeed, rather than failing on their domains. **A prediction right for the wrong
  reason is not evidence for the reason.** Success criteria A and B are not met; yield,
  cost-weighted coverage and saved cost are all zero.
  **The relation to existing entries.** This is the first measurement that bears on
  astra's W1 (the apparent concentration in algebraic shapes may be our selection and
  vocabulary rather than the world's). It neither confirms nor falsifies it: the
  2x2 of site tag against outcome has an **empty outcome row**, so the sample says
  nothing about *where* wins are and a great deal about where they are not even searched
  for. Note also that the entry-level rule "a check that cannot fail has not checked
  anything" applies to this audit itself: **the domain families were never exercised, so
  this result is not evidence about them either way.**
  **A contradiction left open.** The standing advice was *not* to broaden the lowerer to
  enlarge the intake funnel. This audit says the intake funnel is the binding constraint.
  The two disagree and the measurement is the newer fact; recorded as a tension rather
  than resolved in whichever direction is more comfortable.
  **Instrument defects found and recorded** (`AUDIT_PREREGISTRATION.md` addendum 2): the
  harness sent program stdout to `/dev/null`, and GNU grep treats `/dev/null` output as
  `-q` (`grep.c:2896`), so the A1 workload silently short-circuited — 0.075 s became
  0.002 s and instructions fell 82x. The tell was arithmetic (40 MIPS), not the output.
  Only A1 was affected; it was re-profiled and no result from the bad redirect is used.

- **2026-09-27 — CORRECTION to the audit entry below: the 24/24 result is withdrawn. It
  measured the referee's linking configuration, not the lowerer.**
  `emit_c_diff`'s `direct_callees()` refuses any call to a symbol not `define`d in the
  module it is handed, and `lower_module` calls it **before** `lower_function`
  (`emit_c_diff.rs:166` vs `:170`) — so on those sites **the lowerer never ran.** Two
  configuration errors of mine caused it: I lowered **one translation unit at a time**, so
  each project's own helpers (`png_error`, `jv_invalid`, `HIST_add`) looked like unknown
  externals; and I never passed **`--ffi-support`**, which supplies implementations for
  genuine externals and which **the project's own gate passes** (`verify.sh:59`). The
  audit was stricter than the project's own evidence pipeline.
  **Corrected spot-check** on the site the thesis most predicts — `png_calculate_crc`, a
  CRC over a buffer, an S1 scan/reduction. With libpng's whole program (17 TUs,
  `llvm-link`) and a faithful CRC-32 through `--ffi-support`: **`lower_refused=0`, the
  emitter emits a working `crc32` call, and no family fires.** That is **`unsupported`,
  not `intake-failed`** — the first real data point the audit produced, and it says *we
  can read it and we have nothing for it*, which is a different and more useful claim than
  the one withdrawn.
  **What survives:** the six structural refusals (branch-guarded memory through pointer
  operands ×3, separate latch block ×2, loops sharing an exit CFG ×1) are genuine — those
  messages carry no "external call" text and come from the lowerer. **What does not:**
  "75% of refusals show the lowerer cannot ingest real code", and every consequence drawn
  from it, including the inversion of the project's story and the contradiction recorded
  against the advisor's "do not broaden the lowerer" instruction.
  **Newly visible, genuinely:** `png_write_find_filter` and `count_zero_samples` are
  refused for `call without callee: tail call void %6(...)` — **an indirect call through
  a function pointer.** That is how libpng, jq and lua all work internally, and it is a
  real wall, but it is not the one first reported.
  **The lesson, and it is the project's own rule turned on its own audit:** the instrument
  was never validated against a known-good configuration before being used to measure. The
  same defect as a gate that cannot fire, one level up. The audit must be re-run with
  whole-program IR and `--ffi-support`; until then the domain hypothesis remains untested,
  as it was before, but for an ordinary reason rather than an alarming one.

- **2026-09-27 — the opportunity audit ran: 24/24 sites refused at intake, yield zero.
  The constraint is that we cannot read real code, not that we misrecognise it.**
  **[WITHDRAWN — see the correction above. Retained unedited as the record of what was
  believed and why.]**

- **2026-09-27 — astra recommends: do NOT rehabilitate the audit as the main project; run
  ONE family against ONE real site with an "ideal recognizer" control; and the CRC result
  has a second measurement trap.** Consultation on the corrected audit, `--thinking max`,
  no tools. Recorded as recommendations to weigh, not binding rulings.
  **The correction astra made to my correction — confirmed, and it is my error.**
  `png_calculate_crc` is a **wrapper**: it loops over chunks and calls zlib's `crc32`. The
  CRC *recurrence* lives in zlib, which I supplied through `--ffi-support` as an opaque
  function, so ABSAC saw `state = crc32(state, ptr, len)` and never saw the update
  recurrence. **"No family fires" is therefore trivially true and is not evidence about
  the families at all.** The `S1 scan/reduction` tag described what the function
  *accomplishes*, not what its body *contains* — a tagging error inside the procedure
  whose entire purpose is to prevent motivated tagging. Astra's instruction: keep
  `unsupported` as the coarse status but record the subreason as **"computation behind
  opaque callee"**, keep it in the deployment denominator, and **do not count it as a
  mathematical-family trial.** Consequence: **the audit has produced zero authenticated
  information about the families.** Attached warning: *"beating a slow audit support
  implementation would be manufactured headroom."*
  **The proposed smallest experiment — a favourable existence test, not a yield
  estimate.** Pick one already-proved family with the strongest reason to beat LLVM; write
  down its algebraic preconditions, its replacement, what work it removes, and why
  `clang -O3` would leave that opportunity. Build three ways: **A** native application at
  `clang -O3`; **B** the same region through ABSAC with families disabled; **C** the same
  region with the proved transformation. Compare **C against both A and B** — *"beating a
  slow round-trip build while losing to native -O3 is not a performance win."* Attempt
  automatic recognition; if it misses, select and parameterise the rewrite **manually as
  an oracle control, but never bypass the equivalence obligation or manufacture different
  input IR.** The four outcomes separate questions the audit conflated: C cannot be proved
  / C proved but not faster / C faster but recognition misses (a concrete recognition
  problem worth funding) / automatic C beats A (**a first credible external witness**).
  Search time-boxed to two days, candidates and rejections recorded.
  **On indirect calls — devirtualization is not the general admission ticket.** Three
  cases must be distinguished: the call is **outside** the region (preserve surrounding
  code, optimise a leaf); the call is **inside** but the transformation can preserve it
  without reasoning through it (conservative effects suffice); the transformation
  **needs the callee** (only this needs devirtualization). *"A cold error callback can
  cause a whole-function lowerer to reject a hot computational loop. That is an
  optimization-boundary problem, not evidence that resolving the callback target would
  make the hot loop faster."* Explicitly: do not turn this into a partial-region
  compilation project.
  **On the selection rule — biased, but not necessarily unfair.** For estimating
  *prevalence* the two hottest functions in six mature applications are a poor sample
  (they overselect already hand-optimised code). For *immediate performance value* the
  rule is defensible: *"a performance company cannot dismiss the locations where its
  prospective customers actually spend CPU time."* Functions are additionally the wrong
  unit — inclusive time selects wrappers and dispatchers while the work sits in callees.
  The commercial quantity is **cost share x removable fraction, not acceptance rate.**
  **The ten-working-day plan**: day 1 establish a trustworthy instrument (mark the 24-site
  conclusions invalid, resolve the CRC exposure question, exercise a **supported positive
  control** through the corrected FFI path, and exercise deliberate failures recording the
  last stage actually reached — project-authored controls are fine, they validate plumbing
  not capability); days 2-3 freeze one family and one opportunity; days 4-6 the three-build
  test; days 7-9 automate only the demonstrated opportunity, preserving the initial result;
  day 10 decide from a result table.
  **Where the earlier advice survives**: conditionally. *"Do not broaden the lowerer
  merely to improve intake counts. A named, valuable opportunity blocked by one specific
  limitation can justify that specific work. Six refusal categories, by themselves, do
  not."* And the milestone restated: **proved CPU time saved in someone else's code, not a
  larger set of LLVM constructs accepted.** Also flagged: running ABSAC *before*
  vectorization is a legitimate product architecture provided the pipeline is automated on
  unmodified code and the final output beats the fully optimised baseline — what is
  cheating is weakening the comparator or protecting individual benchmark loops, and such
  a pipeline must be declared separately from the frozen optimised-input audit.

- **2026-10-12 — jq `jv_parser_next`: the audit's own S1 tag is refuted by the line-level
  profile, and the SIMD hypothesis turns out to have a workload boundary rather than a
  verdict.** The pre-registered opportunity audit tagged `jv_parser_next` (21.16% of `jq`
  on the frozen workload) as **S1 — a named algebraic object with a hardware or algorithmic
  shortcut**. That tag was attached from what the function *accomplishes* (it turns bytes
  into values), not from what its body *contains*. Profiled at line level on the same
  workload, the function's own cost is a byte-walking state machine plus value
  construction: `jv_number_with_literal` at **464 instructions per number** (600,073
  calls), `jv_string_sized` at **270 per string** (1,000,000 calls) for keys one to three
  characters long, and `jv_free` at **12.10% self**. There is no algebraic object and no
  shortcut. **The audit's separating test — "does the hot loop compute a named algebraic
  object, or is it walking a data structure?" — answers *walking a data structure***, which
  is a NO for this site, and the S1 tag was wrong. This is the first time the separating
  test and a domain tag have disagreed, and the tag lost.
  **The SIMD question, measured rather than asserted.** The quantity that decides whether a
  SIMD stage-1 scanner ("simdjson", `jq#1892`) can pay is *bytes between structural
  characters*, because that is what a wide block skips without branching. Two ordinary JSON
  shapes, both 200,000 records, measured on the raw bytes:
  short-value records give **mean run 4.19 B, max 5 B, and 0 of 611,929 16-byte blocks free
  of a structural character**; long-string logs give **mean 10.00 B, 43.71% of blocks free**.
  `jv_parser_next` self cost is **17.87%** on the first and **41.73%** on the second. On the
  first, a SIMD scanner finds a hit in every block and is pure overhead; on the second it has
  something to skip. **Both shapes are ordinary JSON and they invert the conclusion**, so a
  verdict given on one is not evidence about the other, and the audit's original S1 framing
  ("a hardware shortcut exists here") was true of neither in the form it was stated.
  **`-DNDEBUG`: measured — and my first reading of it was WRONG, corrected here rather than
  in place.** `NDEBUG` appears nowhere in `configure.ac`, `Makefile.am` or `src/Makefile.am`;
  stock configure gives `CFLAGS = -g -O2 -pthread`, so all 367 asserts in `src/` are live,
  including `assert(p->tokenpos <= p->tokenlen);` which runs **once per input character**
  (132,971,616 instructions, 2.45%). Same compiler and optimizer level, only `-DNDEBUG`
  differing: **−15.65%** instructions on the audit workload and **−21.03%** on the logs
  workload, **−6.6%** and **−8.5%** wall clock, output byte-identical.
  **The error**: I first recorded this as *"already known upstream and deliberate"*, citing
  `jaq#42` and a jq FAQ entry about `[-2] | implode`. Both citations were misread — the FAQ
  describes an assert the project **removed** in 1.7.1, not a policy of shipping asserts, and
  `jaq#42` mentions the flag in passing while benchmarking something else. Neither is a
  maintainer decision about jq's build. **What decides it is the shipped artifact**: jq's
  official `jq-1.8.2` release binary contains **100 assert expression strings**, including
  `p->tokenpos <= p->tokenlen` and `src/jv_parse.c`, where a `-DNDEBUG` build of the same
  source contains **zero**. Asserting "deliberate" from a wiki entry, when the released
  binary was one `curl` away, is the same failure this ledger keeps recording — reasoning
  about the checking apparatus instead of running it. Against the actual release artifact,
  best of 7: **291 ms → 263 ms (−9.6%)** and **412 ms → 371 ms (−10.0%)**, corroborating the
  controlled A/B; the official binary's other flags are unknown, so the controlled run is the
  evidence and this is confirmation.
  **A blanket `-DNDEBUG` would be wrong regardless, and THAT is the reportable defect.**
  `src/jq_test.c` calls a side-effecting `test_start_state()` inside `assert()`, twice, in
  `test_jq_start_resets_state` — the only thing that runs that function's validation. Under
  `NDEBUG` the call vanishes and the test stops testing, while `make check` still reports
  **9/9 PASS**. Verified rather than argued, by searching the linked binary for a string
  literal only the eliminated call would print: original/asserts-on **1**, original/NDEBUG
  **0**, fixed/NDEBUG **1**, fixed/asserts-on **1**. **Prevalence correction:** the defect
  applies to any build defining `NDEBUG`; jq's stock autotools setup does not set it. The
  claim that distro packagers do this as a matter of course is unsubstantiated and is
  withdrawn. A check that cannot fail has not checked anything, encountered in someone
  else's tree.
  **A second, smaller result**: `jv_free` opens with `pending = NULL`, so the first array or
  object torn down — which is every array and every object — mallocs a worklist and frees it
  again, and every scalar pays a `jv_mem_free(NULL)`. Starting it as a 16-element stack array
  is worth **−0.80%** / **−0.62%**. Not proposed: 0.8% does not pay for a maintainer's review.
  **Rejected after checking, recorded so they are not re-opened.** (`1`) The lead that
  `jv_string_sized` re-validates UTF-8 the parser already validated: **false** —
  `jv_parse.c:503` copies raw bytes through as `*out++ = c;`, so invalid input does reach it
  and the validation is what routes to `jvp_string_copy_replace_bad`. (`2`) Inlining
  `jv_get_kind` (2.34% audit, 1.45% logs, body is `return JVP_KIND(x);`) is an exported
  libjq symbol and therefore an ABI decision, not a drive-by.
  **Outcome: one standalone PR, and it is the test fix, not the flag.** The first draft of
  `PATCH-jq_test.diff` hoisted the call into a local variable but checked that variable only
  with `assert()`, so its failure was still ignored under `NDEBUG`. **That draft was
  incomplete.** The final patch uses explicit `if (!test_start_state(...)) exit(EXIT_FAILURE)`
  checks at both call sites. jq's `make check` passed 9/9 with assertions enabled and 9/9
  with `-DNDEBUG` (the bundled Oniguruma suites passed 7/7 and 14/14 in both configurations).
  In a separate `-DNDEBUG` fault-injection build forcing `test_start_state()` to return false,
  `tests/jqtest` exited with status 1. This verifies failure propagation, not just that the
  helper call remains in the binary.
  **`-DNDEBUG` is not part of the PR** — the measured 7-10% win is a separate maintainer
  decision, and disabling 367 internal consistency checks in a tool that handles untrusted
  input is not a drive-by change. The PR is limited to the test fix; the measurement is
  context, not a claimed benefit of the patch.
  Not proposed: `jv_get_kind` inlining (an ABI decision), SIMD (a project, not a patch), the
  0.8% worklist change. The dominant jq cost is per-value allocation and teardown (~31% as a
  malloc/free family), which SIMD does not touch. Artifacts in
  `work/pr-targets/jq/FINDINGS.md`.
  **A note on the process, since it produced this session's most expensive error twice.**
  Both times I concluded a finding was moot, I reached it by *reasoning about* evidence I
  had not collected — a wiki page read as policy, and a FAQ entry about a different assert.
  In both cases the decisive check was cheap: `curl` the release binary and search it. The
  hllMerge correction earlier the same day had the same shape (a claim about upstream state
  resolved by reading upstream rather than by argument). Recorded because the pattern, not
  the instance, is what costs.

- **2026-10-12 — The `search` family has a real use case, and the gate has been hiding it:
  its timing harness caps inputs at 32 bytes.** First end-to-end demonstration of a family
  recognising a shape clang cannot handle, with a measured win and machine-checked
  correctness.
  **The case.** The ordinary linear search
  (`for (i = 0; i < n; i++) if (a[i] == c) return i; return n;`) is compiled by clang -O3 to
  a **scalar byte-at-a-time loop with two branches per byte**. Verified, not assumed:
  `clang -O3 -S` on it emits **0 `memchr` calls and 0 vector instructions**. An early-exit
  search returning an index has a data-dependent trip count, so it does not vectorize, and
  LLVM has no `memchr` idiom recognition for it. **The library call is available and clang
  will not use it.** ABSAC recognises the loop and emits `__sir_memchr`: a 16-byte inline
  head scan (so early hits avoid the call) then vectorized `memchr` for the remainder
  (definition 403, runtime `sir_memchr`).
  **The measurement.** Original loop vs ABSAC's emitted C, same compiler, same `-O3`, both
  `noinline`, needle absent so every byte is examined, best of N:
  64 B **1.20×**, 1 KiB **8.06×**, 64 KiB **25.62×**, 1 MiB **19.40×**, 16 MiB **11.02×**,
  64 MiB **6.46×**. Peak 25.6×; still 6× at 64 MiB where the scan is bandwidth bound.
  **Correctness, on two paths because they are not the same path.** The referee reports
  `def=403`, `clean cases=500`, `mismatched=0`. But its differential feeds only
  `lens[] = {0..32}`, so it **never reaches the `n > 16` branch where `memchr` is actually
  called** — the path the win is claimed on is the path it does not test. Checked separately
  in `work/search-case/verify.c`: **90 configurations, 0 mismatches**, across sizes
  0/1/15/16/17/31/32/33/64/1000/65536/1 MiB/4 MiB x needle positions
  {absent, 0, 15, 16, 17, 31, 32, n/2, n-1}.
  **THE DEFECT: the harness is 32 bytes long.** Both the differential and the timing driver
  use `static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};`. The family's win is a
  function of scan length — it is a byte loop against a vectorized library call — and at
  n <= 32 there is no scan to vectorize. The referee therefore timed this rewrite at
  **0.82×** (`ref_ns=752064 new_ns=916862`) while the same rewrite at 64 KiB is **25.6×**.
  **The harness did not measure the transformation; it measured the size guard.** This is the
  second instance of the defect already recorded for `crc32pseudo` — "the referee's input
  distribution, not the code, decided what counted as clean" — now deciding what counts as
  *fast*, with the same consequence: a correct large win reads as a loss. **It is very likely
  why `search` does not fire on the Embench corpus and why the family has appeared useless.**
  The family pays; the ruler is 32 bytes long.
  **A real limitation, stated plainly, not smoothed.** The recognizer does not match the
  `return -1` convention. It requires the not-found sentinel to *be* the scan bound: carried
  values exactly `(false, bound, 0)`, result select `found ? index : bound`, and termination
  `Lt(index, bound)` against that same carried bound. `return n` matches; `return -1` lowers
  to carried `(false, -1, 0)` with termination against the *parameter* `n`, and is refused
  because the `Lt` never meets the carried bound. Since `memchr` returns a pointer and
  index-based searches conventionally return `-1`, a large share of real instances are
  refused on the sentinel alone. **This is the next thing to fix, and it is a recognition
  gap rather than a proof gap.**
  **Scope, honestly.** One function shape, eight needle positions, thirteen sizes. Not a
  corpus. The win requires a long input with a late or absent hit; for a hit in the first 16
  bytes the head scan makes it a wash, and for a hit at byte 0 it is strictly more work than
  the original. And nothing here is a compiler result yet — the rewrite was verified on a
  hand-written kernel of the accepted shape, not discovered by lowering a third-party project.
  Artifacts, the size sweep, `verify.c` and the referee commands: `work/search-case/`.

- **2026-10-13 — README rewritten: it had been describing a system that does not exist.**
  The public README (last touched 2026-09-19) listed six crates that are not in the
  workspace — `sir_semantics`, `sir_inference`, `sir_generation`, `sir_verification`,
  `sir_rewrite`, `sir_optimizer` — none of which appear in `Cargo.toml`. It printed an
  "Ontology Coverage" report (`Concepts implemented: 12`, `Average reasoning depth: 3.2`,
  `Compression ratio: 1.09x`) that nothing in the tree can produce, and it told the reader
  to run `cargo run -p sir_benchmarks --bin report`, a binary that no longer exists. The
  only binary in the workspace is `emit_c_diff`.
  Two consequences worth naming, because this is the same failure the ledger keeps recording
  in code: **a claim that no run can reproduce is not evidence, and a command that cannot run
  is not a reproduction** — the README was the checking apparatus, and it was the defective
  component, for a month, in the one file every visitor reads first.
  The rewrite states the evolved research question, the real nine-crate workspace, the real
  pipeline, commands that run, and keeps `docs/CAPABILITY_LEDGER.md` as the authority. It
  separates **demonstrated** (the pipeline end to end; a real proof kernel; the `search`
  family's 25.6x win on a shape clang handles badly) from **not demonstrated** (any
  end-to-end win on third-party code — the `search` result is a hand-written kernel of the
  accepted shape, not a project nobody here wrote). The `arithmetic` family firing on the
  benchmark corpus is recorded as meaning nothing, because clang performs the same
  multiply→shift peephole.

- **2026-10-17 — `search`: the not-found sentinel is decoupled from the scan bound. The
  first attempt was UNSOUND and the referee caught it.** The recognizer previously required
  the not-found value to *be* the scan bound, so `return n` matched and the conventional
  `return -1` was refused — even though they are the same loop. `SearchPlan` now carries
  `bound` (the `Lt` right-hand side, passed to `sir_memchr` as the length) and `sentinel`
  (the carried result, returned on a miss) separately, and `runtime` now means "the bound is
  not a compile-time constant" rather than "the carried result is not a constant" — the
  property that actually selects the vectorized path.
  **The unsound first version.** Decoupling alone made the `-1` shape fire, and the referee
  returned **`mismatched=1`, 483 of 500 cases**. The emitted helper reports a miss as the
  scan bound (`... : n`), so the rewrite returned `n` where the source returned `-1`. The
  recognizer was right about the shape and wrong about the contract. **Fixed by translating
  the miss**: an `Eq(result, bound)` and a `Select` back to the sentinel, since a found index
  is always below the bound and can never equal it. This is the second time this week a
  decoupling produced a plausible-looking rewrite that the differential killed; the referee
  is doing the job it exists to do.
  **Verified.** Both conventions fire definition 403 and are clean at 500 referee cases.
  Beyond the referee's 32-byte ceiling: **90 configurations each, 0 mismatches**, over 13
  sizes (0 … 4 MiB) x 9 needle positions {absent, 0, 15, 16, 17, 31, 32, n/2, n-1}. `cargo
  test --workspace` 10 tests; `verify.sh fast` green with all 19 external corpora present
  (`mismatched=0`, `harness_errors=0`).
  **Honest negative, and it matters**: the fix **does not fire on any of the 19 Embench
  corpora**. Across all of them the only definition that fires is **102** (multiply→shift,
  29 firings) — the peephole clang already performs. Embench is numeric kernels, not string
  scanners; the two corpus members that do scan bytes (`qrduino`, `slre`) already call
  `memchr`/`strlen` themselves. **So the recognizer is broader, and third-party reach is
  unchanged.** The claim "real code is full of this shape" remains untested against real
  code; the next step is a project that actually has hand-rolled scans, not another corpus
  of numeric kernels.
  **Also fixed**: `fetch-external.sh` and `verify.sh` both defaulted to `/tmp/external`. They
  now default to `work/external`, so a gate input cannot vanish with the temp directory.
  (First gate run of this change reported `SKIP` on 19/19 external corpora and still printed
  `VERIFY PASS` — a missing corpus is a WARN by design, but a pass over 4 corpora is not the
  gate the README describes.)

## Reference commands

- Extract old tree: `git archive HEAD sir | tar -x -C /tmp/old_sir`
- Old file: `git show HEAD:sir/crates/<crate>/<path>`
- Build old baseline (outside repo):
  `cd /tmp/old_sir/sir && CARGO_TARGET_DIR=/tmp/old_target cargo build --release`
