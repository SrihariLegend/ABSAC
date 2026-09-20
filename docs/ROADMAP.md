# ABSAC Semantic Compiler — Full Roadmap

## 0. Why this matters

Compilers are leverage: they decide what software can be fast, portable,
and verifiable. A **semantic compiler** is a different kind of leverage.
Instead of rewriting syntax or peephole patterns, it reasons about what a
program *means* (collections, reductions, searches, bit algebras,
iteration domains), proves that an alternative implementation has the
same observable meaning, and only then changes the program.

If this works, the wins are not “another 2% from a smarter heuristic”:

- whole families of loops disappear into hardware primitives,
- optimizations are justified by machine-checked proofs rather than
  pattern-matching folklore,
- the optimizer can search globally across interacting regions instead of
  greedily rewriting one node at a time,
- and the resulting code can carry its proof, which matters for safety,
  security, and long-lived infrastructure.

This is a systems grand challenge, comparable in ambition to verified
compilers, superoptimizers, and equality-saturation engines. It is not a
Clay Millennium Prize problem; credibility will come from proofs,
held-out corpora, independent reproduction, and honest scoping, not from
the label.

## 1. Definition of success

The project is successful when all of these are true:

1. **Semantic wins**: transformations fire that syntax/local optimizers
   cannot express, with measured speedups on held-out, independently
   authored corpora.
2. **Rigor**: every rewrite is backed by a machine-checked obligation; a
   small trusted kernel and third-party cross-checks bound what can go
   wrong.
3. **Extensibility**: a new optimization family is a small module
   registered through a stable `Family` interface, not a framework
   rewrite.
4. **Reach**: the core handles the corpus language subset, lowers to
   multiple targets (x86-64 first, AArch64/wasm next), and emits
   differential-clean C/assembly.
5. **Reproducibility**: corpora, manifests, versions, and timings are
   reproducible by an outside contributor in one command.
6. **Community**: the core stays small enough to understand; docs,
   contribution ladders, and pruning cadence keep it that way.

The north star — “the most important compiler/optimizer” — is a
direction, not a claim. The gate is evidence.

## 2. Non-goals and honesty

- We do not claim general program equivalence for arbitrary languages.
- We do not trust definitions, backends, or plugins to self-certify.
- We do not accept a rewrite because it “looks equivalent”; unrecognized
  or ambiguous shapes fail closed.
- We do not let the core grow without a LOC/performance/assurance budget.
- Dropping a neutral family is allowed; dropping a measured win or a
  soundness gate is not.

## 3. Current baseline (post-purge)

| metric | value |
|---|---:|
| LOC | 15,618 |
| crates | 9 |
| build warnings | 0 |
| tests | 19 passed / 0 failed / 2 ignored |
| 14-corpus differential | 76 rewrites / 221 clean / 0 mismatched / 0 harness errors |
| supported families | search (200/402), count (0), Any (4), All (5), sums/mapped sums (400/401) |
| proof core | `sir_mech` BV/SAT + algebraic law checks; post-patch `sir_verify` gate restored |

Known gaps: no family registry, no z3 cross-check, simplified
authorization/frame gates, no measured-retention policy, and dropped
parity/mask/arithmetic/permutation/Kernighan/runtime-search families.

## 4. Invariants

1. **No unproved rewrite.** A patch is emitted only after its obligation
   is discharged by a trusted backend or a replayed kernel proof.
2. **Fail closed.** Ambiguity, unsupported shapes, missing measurements,
   budget exhaustion, or verification failure preserve the baseline.
3. **Small trusted core.** Kernel, SAT/bitvector semantics, lowering, and
   structural verification stay auditable.
4. **Evidence over claims.** Corpus differential, sanitizers, z3
   cross-checks, and reproducible timings gate every claim.
5. **LOC budget.** Core target ≤20k LOC; research modules may exceed it
   only behind features/crates with a deletion date.
6. **Determinism.** Digests, ordering, and selection are deterministic
   within a build.

## 5. Target architecture

```
crates/
  sir_types        SIR values/types/effects/ids            ~1.0k
  sir_nodes        arena/nodes/function                    ~0.6k
  sir_builder      construction API                        ~0.8k
  sir_verify       structural verifier (fold into nodes)   ~1.0k
  sir_analysis     loops + graph queries                   ~0.6k
  sir_mech         term/bv/cnf/sat + kernel rules          ~2.5k
  sir_lower        LLVM subset -> SIR                      ~2.0k
  sir_opt          plan/family API + families + driver     ~6-8k
  sir_benchmarks   emitter + one CLI + harness             ~2.0k
```

`sir_opt` internal shape:

```
plan/           RegionContext, Plan, Obligation, CertifiedPlan
family/         trait Family; registry; one module per family
prove/          law library + backends (concrete, z3 cross-check)
patch/          local-id graph, SSA rewiring, dead sweep, verify gate
cost/           measured factors, predictive model, policy
global/         interaction graph, combination search, extraction
emit/           SIR -> C (later AArch64/wasm)
```

### Extensibility contract

```rust
trait Family {
    fn id(&self) -> DefinitionId;
    fn applicable(&self, ctx: &RegionContext) -> bool;
    fn propose(&self, ctx: &RegionContext) -> Option<Plan>;
    fn obligation(&self, plan: &Plan) -> Obligation;
    fn prove(&self, plan: &Plan, backend: &mut ProofBackend)
        -> Result<CertifiedPlan, ProofError>;
    fn patch(&self, function: &Function, plan: &CertifiedPlan)
        -> Option<Function>;
}
```

A family should be 100–400 LOC, self-tested, registered declaratively.
Two families must use an abstraction before it becomes a framework.

## 6. Research and engineering pillars

### A. Semantic IR and knowledge
Region contexts, certified roles, domain certificates, canonical
occurrence identity, and descriptive/operational concept separation —
but with the data flow the recognizers actually need, not a global
knowledge pipeline.

### B. Proof kernel and theories
LCF-style small kernel; bitvectors/SAT first, then arrays, bounded
sequences, and induction schemas over index ranges. Every theory rule is
validated against the concrete solver and z3 before use.

### C. Family discovery and library contracts
Hand-authored families for proven wins; later, guided discovery from
corpus motifs. Library primitives (`memchr`, popcount, ctz/clz) become
proved contracts rather than trusted assumptions.

### D. Global optimization
From local fixed point to region interaction graphs, conflict-aware
combination search, certified equality saturation/e-graphs, and proof
reconstruction during extraction.

### E. Cost model and runtime dispatch
Measured profiles now, predictive models next, autotuning and runtime
dispatch later — the “no profiles” end state: a model good enough to
choose statically, with dispatch only when the model says it cannot.

### F. Targets and execution
x86-64/C differential now; AArch64 under qemu next; wasm for a third
execution model; cross-target differential as a first-class gate.

### G. Corpora and evaluation
Internal v-series held-out corpora, external suites (Embench, PolyBench,
LLVM SingleSource, glibc/musl string tests, zlib/zstd/LZ4/SQLite,
Csmith), independent authors, manifests and hashes, differential versus
clang/gcc, sanitizers, and statistical reporting.

## 7. Phases

### Phase 0 — Prune and freeze (done)
Delete dead experiments, eliminate the old pipeline, keep measured wins
and proof/core machinery. Baseline: 15.6k LOC, 76/221/0.

### Phase 1 — Restore extensibility and assurance (next, ≤18k LOC)
- `Family` trait + `Plan`/`Obligation` registry; port the six existing
  families and delete their ad-hoc dispatch.
- z3 cross-check for the algebraic laws and the bitvector encodings they
  discharge; replayable proof records.
- Conservative gates: plan-bound authorization, live-out/frame/base
  checks, post-patch verifier (done).
- Measured-retention policy in the driver (factors per compiler/opt/
  extent; unmeasured refuses).

Exit gates: 14-corpus parity, sanitize clean, z3 cross-check green,
callgrind budgets within 10% of current (sum ~9.4M, memchr ~4.4M), core
≤18k LOC.

Phase 1 progress:

- **Family registry landed.** `sir_opt::family` defines `Family`,
  `Applied`, `with_facts`, and a five-entry registry (search, count,
  Any, All, Sum, Parity cover definitions 0/4/5/6/200/400/401/402). The
  driver now runs the registry to a fixed point; adding a family is one
  module plus one registry entry. The existing family modules expose
  `apply_one(function, facts)`.
- **Kernel and arithmetic restored.** `sir_mech` recovered the LCF
  kernel (`replay`, induction) and quantifier-free integer linear
  arithmetic from `HEAD`; the induction proof and arithmetic contracts
  run in the consolidated suite.
- **Parity family 6 mined from `HEAD`.** The XOR-reduction path
  recognizes a counted exclusive reduction, discharges `xor_laws` with
  `sir_mech`, and emits a straight-line XOR chain. The `HEAD`
  bitset/pack/popcount path remains for Phase 2.
- **Test discipline.** The workspace suite is **9 tests** (9 passed /
  0 failed / 0 ignored), with case bodies under `tests/cases/`; no
  coverage was dropped, and the z3 differential now batches its queries
  into one solver invocation.
- **Post-patch structural gate restored.** `patch::apply` runs
  `sir_verify::Verifier`; malformed SIR is rejected before emission.
  The consolidated suite found and fixed two gate holes: patch
  replacements did not remap `return_node`, and a missing return
  operand was not reported as a dangling reference.
- **Measured retention restored as a tool.** `emit_c_diff
  --measured-retain` times both variants, reports `retain=base|new`,
  and counts retained baselines. Its first finding was a regression:
  inequality search at extent 64 was 82ns baseline vs 244ns unrolled,
  so definition 200 is now gated to short extents until a predictive
  cost model exists. Base-14 rewrites fell 76 → 59 with zero
  mismatches — the removed rewrites were unmeasured, not wins.
- **Third-party kernel cross-check restored.** A z3 differential
  (`tests/z3_kernel_differential.rs`) generates random bitvector
  equivalences, decides them with `sir_mech`, and cross-checks each
  verdict with z3 (skips cleanly when z3 is absent). It immediately
  found a kernel robustness bug: unconstrained variables were absent
  from the CNF model and `decode_model` panicked; the decoder now maps
  them to zero, which is a valid model for an unconstrained variable.

Phase 1 status: **26,222 LOC, 9 crates, 9 passed / 0 failed / 0
ignored**, 14-corpus **128 rewrites / 230 clean / 0 mismatched / 0
harness errors**, and fuzz corpora v17–v19b **360 rewrites / 540 clean /
0 mismatched** (761 clean total). The LOC guide is a guide, not a
target; the restored kernel/arith is ~2.3k of the increase.

Remaining Phase 1 work: replayable proof records and the full
predictive retention model. `--measured-retain`, `--profile-out`,
`--profile-in`, and `--allow-defs` now close the measured retention
loop at per-function granularity; the default
policy refuses runtime-dependent definition 403; live-out/frame gates
have v0.

Phase 1 evidence update: the left-fold proof trim brought the sum
pipeline to **7.19M instructions** (budget 9.4M) and the workspace suite
to **~7s**. The memchr pipeline is still **29.6M** against a 4.4M
budget; the first-match scan equivalence is the remaining proof-cost
item and is slated for the induction/kernel work in H3.

Cross-arch evidence update: `emit_c_diff --target aarch64` now runs the
full base-14 differential under qemu at **221 clean / 78 rewrites / 0
mismatched**, matching native. The harness also respects `!range` input
metadata, removing a false AArch64 mismatch in v10.

### Phase 2 — Restore high-value families (≤22k LOC)
Priority by measured value:
1. Kernighan bitset popcount (14–17×) — definition 7 now covers all
   widths: ≤16 with a bounded machine proof, 32/64 contract-backed
   pending the H3 kernel induction. Bounded SAT unrolling scales only to
   16 bits (8/16 = 8ms/285ms; 32 = 35.6s; 64 infeasible).
2. Runtime-bound reductions/search (contract-backed) — runtime sums are
   restored as definition 404; runtime search is
   found-flag shape now lowers and rewrites to `sir_memchr`; measured
   no-hit 1.25× but early-hit 0.82×, so predictive retention is the
   remaining piece.
3. Parity (XOR path done) and the remaining reductions.
4. Arithmetic bit hacks: mod/div/mul/shift-mask all done (100–103).
5. Mask algebra and permutations.
6. Set algebra and other neutral families only if a corpus needs them.

Each family: 100–400 LOC, proof, corpus tests, measurement, deletion
notes for any old path removed.

### Phase 3 — Complete vertical compiler (≤20k core)
- Lower the full retained language subset (runtime bounds, signed
  comparisons, multi-block loops as needed).
- One CLI with differential/sanitize/timing modes; no measurement
machinery inside the compiler core.
- AArch64 qemu differential; optional wasm harness.
- External corpora with independent manifests.

### Phase 4 — Global optimizer
- Region interaction graph: overlap, ordering, shared live-outs, cost
  coupling.
- Beam/MCTS combination search with proof caching.
- Certified equality saturation on the retained IR surface.
- Extraction with proof reconstruction and baseline fallback.

### Phase 5 — Theories and trusted primitives
- Array read-over-write/extensionality, bounded sequence induction,
  range induction.
- Replace trusted `memchr`/popcount contracts with proved
  implementations once the kernel can replay them.
- Kernel audit and third-party proof checker.

### Phase 6 — Ecosystem and scale
- Plugin API, docs, examples, contribution ladder, issue labels.
- Reproducible benchmark leaderboard and release cadence.
- Publications and independent reproduction; research collaborations.

## 8. Assurance strategy

| level | mechanism | trust |
|---|---|---|
| Kernel | small term/rule checker; replayable proofs | trusted, audited |
| Core theories | BV/SAT, arrays, induction rules | kernel-checked |
| Family obligations | concrete solver + law library | kernel + cross-check |
| Cross-check | z3 (and later another solver) on encodings | third-party |
| Application | plan binding, live-outs, frames, bases | checker-issued |
| Regression | corpus differential, sanitize, fuzzing | external execution |

No definition, plugin, or backend self-certifies. Issued assurance is the
minimum of declared cap and backend capability. Any mismatch fails
closed.

## 9. Performance strategy

- Keep callgrind instruction budgets as compile-time guardrails.
- Keep measured generated-code differentials (`micro`/`corpus` timing)
  outside the compiler core.
- Reintroduce profile gating inside the driver, then replace it with a
  predictive cost model trained on measurement.
- End state: static model first, autotuning for uncertainty, runtime
  dispatch for genuinely dynamic targets.

## 10. Engineering and community

- **LOC budget per PR**: every PR reports LOC delta; core budget is
  reviewed at each phase exit.
- **Pruning cadence**: monthly dead-code/orphan pass; quarterly feature
  audit.
- **Docs**: architecture, family authoring guide, proof guide, corpus
  guide, glossary, roadmap.
- **CI gates**: build, tests, 14-corpus differential, sanitize, z3
  cross-check, callgrind budgets, LOC report.
- **Contribution ladder**: docs → corpus/tests → small family → proof
  rule → global optimizer.

## 11. Risks and mitigations

| risk | mitigation |
|---|---|
| Unsound rewrite | kernel + cross-check + fail closed + application gates |
| Proof scalability | bounded domains, kernel rules, caching, replay |
| Corpus overfitting | global/discovered/independently authored corpus profiles |
| Complexity regrowth | LOC budget, two-use rule, monthly pruning |
| Performance claims not reproducible | manifests, versions, external reruns |
| Research stalls | keep local wins shipping; theory tracks gated separately |
| Bus factor | docs, small core, contribution ladder, ADRs |

## 12. Immediate next four iterations

1. **Family trait + registry**: port search/count/Any/All/sum; delete the
   ad-hoc dispatch; 14-corpus parity.
2. **z3 cross-check + proof records**: validate the five law functions
   and a sample of family encodings; CI gate.
3. **Gate restoration**: plan-bound authorization/live-out/frame/base
   checks; fuzz the patch applier.
4. **Kernighan popcount + runtime-bounded search**: highest measured
   wins; measured-retention policy in the driver.

## 13. Five-year picture

A core small enough that a new contributor can read it in a week; a
family API anyone can extend; proof-carrying rewrites with kernel and
third-party validation; measured wins on external corpora; global search
over interacting regions; a predictive cost model with runtime dispatch;
and a public track record of reproducible results. That is what “most
important compiler” has to mean in practice.

## 14. Recovery horizons: research velocity without re-bloat

### 14.1 The purpose of the cut

The cut existed to remove **accidental complexity that made the grand
idea untestable**: 92.9k LOC, 19 crates, 26 binaries, overlapping
framework layers, and 44 warnings obscured which semantics were actually
recognized, proved, and worth applying. It was never a LOC target. The
purpose was to make the core small enough to read, extend, and trust so
research could move at the speed of ideas rather than at the speed of
frameworks. Leanness is the enabling constraint; capability is the goal.

The execution failed in two places, and both are process failures rather
than evidence against the cut:

1. capability was deleted before a replacement path or measurement
   existed;
2. the session’s work was never committed, so the exact final source was
   not recoverable.

The recovery plan therefore restores capability **on top of** the lean
core, and fixes the process that lost it.

### 14.2 Horizons

| horizon | window | objective | what “done” looks like |
|---|---|---|---|
| **H0 — safety and truth** | weeks 0–2 | stop loss, establish ground truth | writable VCS/mirror; current + `HEAD` committed; ledger and backups verified; `HEAD` clone builds |
| **H1 — foundation** | months 1–3 | measurement and assurance before breadth | timing/retention harness; 14-corpus + callgrind baselines; family-level z3; plan-bound gates; every current family proof-carrying |
| **H2 — breadth and global** | months 3–12 | regain and exceed old capability | 25+ proven definitions; top families from `HEAD` and the lost session paths; interaction-aware/global search prototype; external corpora; cross-arch differentials; predictive cost model with autotuning and runtime dispatch |
| **H3 — theory and scale** | years 1–3 | stop trusting primitives and local search | array/quantifier/induction theory; proved `memchr`/string primitives; certified equality-saturation extraction; dispatch without profiles; external contributors; published reproducible results |
| **H4 — semantic compiler** | years 3–5 | the grand idea, honestly evidenced | real language subset; signed/runtime semantics; multiple targets; community family ecosystem; independent reproduction; a public record that justifies the claim |

### 14.3 Workstreams

These run in parallel but only cross the milestone gate when their
evidence exists:

| stream | scope |
|---|---|
| **A. Semantics and families** | concepts, roles, recognizers, definitions, recipes — ported from `HEAD` or rebuilt from evidence, one `Family` module at a time |
| **B. Proof and assurance** | kernel and theories, obligations, replay, z3 differentials, application gates, failure modes |
| **C. Global optimization** | interaction graph, combination search, certified extraction, proof reconstruction |
| **D. Cost and runtime** | measured profiles, predictive cost model, autotuning, runtime dispatch, no-profile default |
| **E. Evidence and evaluation** | internal and external corpora, held-out splits, fuzzing, sanitizers, cross-arch, independent reproduction |
| **F. Engineering and community** | leanness, docs, contribution path, good first issues, release discipline |

### 14.4 Guardrails that preserve the purpose of the cut

1. **The `Family` API is the only extension surface.** No new framework
   layers; a family is a module plus a registry entry.
2. **Every capability arrives complete**: semantics + proof + gate +
   tests + measurement + corpus evidence + ledger entry.
3. **Deletion rule**: replacement, measurement, or ledger — never
   uncommitted work.
4. **Leanness is measured by time, not LOC**: time to read the core,
   time to add a family (target under a day), zero warnings.
5. **Measurement gates every rewrite.** No unconditional rewrite; fail
   closed on ambiguity.
6. **Quarterly dual audit**: capability ledger versus bloat ledger.
   Kill measured-neutral paths; keep measured wins.
7. **External evidence is pre-registered.** Internal corpora guide
   development; they are not proof of the grand idea.
8. **No overclaiming.** Record negative results and non-goals next to
   the wins.

### 14.5 Trajectory metrics

| metric | H1 | H2 | H3 | H4 |
|---|---:|---:|---:|---:|
| definitions with proof + z3 + corpus + measurement | 12–15 | 25+ | 40+ | broad subset |
| external corpora reproduced independently | 1–2 | 4+ | public suite | community-held |
| global search beats local fixed point | no | prototype | measured | default |
| proved library primitives | no | 1 | several | routine |
| time to add a family | days | < 1 day | < 1 day | external PRs |
| warnings / mismatches | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |

### 14.6 Failure modes to watch

| risk | mitigation |
|---|---|
| re-bloat while restoring families | `Family` budget, ledger entry, quarterly audit |
| losing work again | commits before/after every milestone; external mirror |
| overfitting internal corpora | held-out splits and external reproduction |
| proof theatre / trusted primitives | small kernel, z3 cross-check, proved primitives, fail closed |
| speed at the cost of rigor | proof-carrying rewrites, measurement retention |
| scope creep | horizon gates; non-goals made explicit |

## Appendix: old vs current scorecard (0–5, honest)

Ratings are for the **grand-idea** bar, not for “useful prototype”.

| dimension | old session (~92.9k, uncommitted) | current (15.9k) | notes |
|---|---:|---:|---|
| Semantic model | 3.0 | 1.0 | old had concepts/roles/certificates/binding/authorization; current has per-family shape matching |
| Proof rigor | 3.5 | 3.0 | old had theorem language, concrete/symbolic/exhaustive backends, z3 differential, artifact issuance; current has BV kernel + laws + z3 kernel check + restored LCF kernel with replay/induction and QF integer linear arithmetic; family-level obligations and arrays/quantifiers still missing |
| Family breadth | 4.0 | 2.0 | old covered arithmetic, masks, permutations, scans, reductions, sums, memchr; current covers search/count/Any/All/sums |
| Global optimization | 1.5 | 0.5 | old had beam/fixed-point and cost-ranked selection; neither has interaction-aware/equality-saturation search |
| Cost model / measured retention | 3.5 | 1.0 | old had measured profiles, factors, downstream gates, timing harness; current rewrites unconditionally |
| Compiler completeness / targets | 2.0 | 1.5 | both are LLVM-subset; old retained wasm harness and AArch64 toolchain setup, current has x86-64 C only |
| Extensibility / architecture | 2.5 | 3.5 | old interfaces were rich but heavy and overlapping; current `Family` registry is small and concrete but only five families |
| Leanness / maintainability | 1.0 | 4.0 | old: 19 crates, 26 bins, 44 warnings, many abandoned frameworks; current: 9 crates, 0 warnings, clear core |
| Evidence / reproducibility | 4.0 | 2.5 | old had v2–v20, fuzz, sanitizers, z3, wasm, measured timings; current has v2–v16 differential, sanitize, kernel z3, no timing/profile tooling |
| **Overall** | **2.7** | **2.1** | old was broader and better evidenced but unsustainable; current is leaner and more extensible but capability-narrow |

Interpretation:

- As a **research artifact**, the old codebase was further along
  (~3.5/5): broad families, strong differential evidence, real measured
  wins.
- As a **codebase that invites constant high-quality contribution**, the
  old one was poor (~1.5/5): 92.9k LOC, overlapping frameworks, 44
  warnings, abandoned experiment stacks.
- The current core is the opposite trade: ~2.1/5 on grand-idea capability,
  ~4/5 on maintainability. The project needs to regain the old breadth
  and assurance *on top of* the new leanness.

Highest-value old capabilities to re-earn:

1. measured timing + retention policy (as a tool, not core bloat);
2. z3 differential at family/obligation level;
3. authorization/occurrence/frame/base application gates;
4. high-value families: Kernighan popcount, runtime search, parity,
   arithmetic, masks/permutations;
5. external corpora and AArch64/wasm harnesses;
6. family-level proof records/replay and array/quantifier theory. The
   induction kernel and QF integer linear arithmetic were restored from
   `HEAD`; arrays/quantifiers never existed (`sir_mech` is
   quantifier-free) and remain a research item.
