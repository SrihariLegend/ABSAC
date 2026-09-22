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
- **2026-09-19 — narrow-counter Kernighan proof**: a 7-bit counter
  encoding cut the width-32 bounded proof from 35.6s to 16.5s, but that
  is still pipeline-infeasible; the experiment was reverted. Production
  widths still need the kernel induction proof.

## Reference commands

- Extract old tree: `git archive HEAD sir | tar -x -C /tmp/old_sir`
- Old file: `git show HEAD:sir/crates/<crate>/<path>`
- Build old baseline (outside repo):
  `cd /tmp/old_sir/sir && CARGO_TARGET_DIR=/tmp/old_target cargo build --release`
