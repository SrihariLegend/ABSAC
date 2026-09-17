# ABSAC Immediate Research Program — Witness Frontier

> **Status: Current implementation plan.**
> Execute these gates in order. Do not build any Arena component until these pass.

## Thesis question

The immediate question is not:

> Can we build a DeepMind-style optimizer?

It is:

> Is there a bounded semantic domain where ABSAC can automatically produce concretely verified implementations that materially outperform the strongest available compiler/library baseline on previously unseen real kernels?

That is the business and research foundation.

---

## Naming: witness frontier, not oracle

An oracle would tell us the fastest possible implementation. We do not know that — that is partly what ABSAC is intended to discover. Hand-written implementations only demonstrate that a certain amount of headroom is known to exist.

If the best hand-written implementation is 3× faster than Clang, we have strong evidence of reachable headroom.

If it ties Clang, that does **not** establish that no better implementation exists. It establishes only:

> We currently have no witness showing exploitable headroom in this kernel.

That should kill or deprioritize the **current kernel/domain hypothesis**, not the entire project. Otherwise, we would be using current human knowledge to conclude that a system intended to exceed current human knowledge cannot succeed.

---

## Execution sequence

```text
1. Establish benchmark truth.
2. Construct the strongest known implementation frontier.
3. Identify kernels with demonstrated headroom.
4. Determine which semantic/target capability reaches that headroom.
5. Test whether search contributes.
6. Build no general Arena component until these pass.
```

---

## Gate sequence

### Gate 1: Benchmark credibility

Pass when:

- both sides use equivalent build flags;
- results are stable;
- assembly explains the result;
- input-size effects are understood.

### Gate 2: Witness headroom

Pass when:

- at least one real kernel has a materially faster known candidate than the strongest baseline.

If this fails, change domains or deprioritize the kernel. Do not conclude the project is impossible.

### Gate 3: Expressive reachability

Pass when:

- ABSAC's semantic candidate language and backend can represent that candidate family.

If this fails, improve ontology/lowering — not search.

### Gate 4: Concrete correctness

```
Gate 4A: PASSED — compositional correctness argument
                    + extensive adversarial validation (16.8M tests, ASan + UBSan)

Gate 4B: PASSED — mechanically checked concrete end-to-end equivalence
                  (see docs/GATE4B_PROOF.md: per-chunk identities,
                  loop invariant for arbitrary n, tail decomposition,
                  region binding, intrinsic models, memory/overflow,
                  replayable artifacts)

Gate 4B.1: PASSED — per-rewrite application pipeline soundness closed
                   (2026-09-16) by the independent blind H4 evaluation:
                   h4c tier A 10/10 eligible positives + 7/7 required
                   abstentions, tier B 3/3 positives, all committed
                   rewrites differentially clean, zero panics. The H4b
                   run found F10 (global-array extent hardcoded to 256 in
                   sir_lower), fixed in 1da3b28. See docs/H4_RESULTS.md.
```

Scope honesty: the three development kernels at SIR level, with an
explicit trusted base (instruction/memory/front-end models). Not a
verified compiler, and not native-object equivalence. The emitter
defects F6–F8 were closed on 2026-09-17: the SIR → C loop emitter now
compiles and runs natively against the original IR on 99 lowered corpus
kernels (0 mismatches, 19 with committed rewrites); see
docs/EMITTER_F6F8_RESULTS.md. The per-rewrite application pipeline ("Gate 4B.1"
in H3/D4) was closed separately on 2026-09-16 by the independent H4
corpus (docs/H4_RESULTS.md); its assurance is SIR-level SchemaChecked,
not machine-checked. See the scope-honesty section of
docs/GATE4B_PROOF.md.

The six lemmas began as human-written mathematical arguments supported
by testing. They are no longer the evidence: Gate 4B was closed on
2026-09-15 by `sir_mech`/`sir_gate4b`, which proves the per-chunk
identities, the loop invariant (arbitrary n), the tail decomposition,
the region binding and the intrinsic semantics, and replays the result
from `gate4/proof/*.proof.txt`. The trusted base is stated explicitly
in docs/GATE4B_PROOF.md.

### Gate 5: Search value

```
Gate 5A: COMPLETE — target-plan search landscape mapped
    69 plans enumerated, Engine 0 within 0-14% of measured best.
    Deterministic selection captures most of the benefit.
    No MCTS needed at this scale.

Gate 5B-Witness: PASSED — hand-constructed semantic composition
    Multi-reduction fusion provides 28-35% improvement over
    independent vectorization. No single recipe contains this.

Gate 5B-Automation: PASSED — the fused single-pass implementation is
    generated from per-region primitive plans on a corpus sealed before
    the automation existed (5/5 fusion rows, differential-clean,
    1.70–2.07× vs the deterministic per-region baseline).

Gate 5B-Search: PASSED — the composition action space with a
    memory-traffic cost model selects fusion where Engine 0 has no fusion
    action; under early-exit-favouring assumptions it selects a mixed plan.
```

Gate 5B closed 2026-09-17 — full record in docs/GATE6B_RESULTS.md
(sealed corpus `gate6b/`, frozen harness manifest, raw runs committed
before inspection).

If this fails, MCTS is unjustified for the current action space.

### Gate 6: Generalization

Gate 6A-v0:  FAILED   50% false positives on held-out negatives (3/6).
                     Preserved as a historical record — do not overwrite.

Gate 6A-Rem: PASSED   on the regression corpus only (0/6). NOT held-out
                     proof — the corpus was inspected and is now D1.

Gate 6A-v1:  FAILED   fresh blind corpus (H1): 1/4 lowered negatives
                     falsely recognized (N15 — certificate completeness
                     gap), 1 frontend soundness bug (V07 — SIR verifier
                     caught invalid two-loop lowering), 2/8 positives
                     lost to mixed-width lowering, All-reduction recall
                     miss (V03).

Remediation D2:
  0A: mandatory SIR verification gate (lower_function fails loudly)
  0B: ReductionCertificate closed-world memory-footprint check
  1:  llvm.smax/smin, pre-header emission, operand type hints
  2:  AllReduction (select_reset form) recognized — V03/H06/k18

Gate 6A-v2:  FAILED   fresh blind corpus (H2): 1/7 lowered negatives
                     false-recognized (X02 signed-overflow — certificate
                     lacks overflow semantics), AND the first unsafe
                     candidate generation (X06 running max → PositionSearch
                     belief → BitScanForward candidate, contained only by
                     a role-completeness failure in the rewrite layer).
                     Generational trend: 3/6 → 1/4 → 1/7 observed (not
                     statistically meaningful — qualitative only).

Remediation D3 (P0A, commit c3ebb54):
  P0A: candidate generation is certificate-gated —
       sir_semantics::authorization derives TransformationAuthorizations
       from complete RegionInterfaceCertificates + domain certificates
       (Reduction with overflow semantics / PositionSearch with the
       binds-index invariant / ScalarExpression). AuthorizationDatabase
       is a required argument of CandidateGenerator::generate; the
       gate in generators::all_plans refuses candidates citing
       unauthorized operation concepts. Truths/beliefs are evidence,
       never authorization.
       Fixed pre-existing pipeline nondeterminism (HashMap-ordered
       region merging) exposed by the gate.
  D3 validation on the regression set (NOT held-out proof): 494/494
       tests, H2 corpus 11/16 lowered, 0 false positives, x02 → 0
       candidates (overflow refusal), x06 → 0 candidates and NO false
       FirstOccurrence truth (semantic precision), dev corpus unchanged
       (40/50).
  P0A hardening (commit 3691fcf, advisor audit):
       — region merging = true connected components (union-find) +
         oracle test (the old merge could under-merge transitively);
       — UntrustedProposal/AuthorizedCandidate type split (proposal →
         candidate only through a matched authorization, crate-private);
       — authorization provenance travels with candidates
         (AuthorizationRef: fingerprint + region + domains), optimizer
         rejects stale authorization before rewriting;
       — per-domain authorization issuance; the scalar grant inside
         reduction regions is now an explicit Composition certificate,
         not a capability escalation;
       — determinism invariant tests (oracle + repeated fresh-engine
         runs identical); 8/8 fresh-process runs stable.
  Remaining P0A audit queue (before Gate 6B): candidate concrete
       bindings (memory bases, predicates, bounds, accumulator) checked
       against authorization bindings — required before fusion.
  P0A concrete binding step 1 (commit e62db09):
       — ConcreteFacts on every TransformationAuthorization: memory
         bases (from the footprint certificate), reassociable
         accumulator (accumulators_are_reassociable now returns its
         identity), authorized effects; copied into every
         AuthorizationRef at the gate;
       — ConcreteBindingDigest on every candidate: FNV-1a over all
         authorization-relevant fields (fingerprint, region, context,
         definition, strategy, cited concepts, effects,
         representation, constraints, assumptions — sorted, so
         HashSet order cannot perturb it); the optimizer rejects any
         candidate whose digest no longer recomputes;
       — rewrite-engine revalidation: stale authorization or invalid
         digest → refused before any mutation; the collection a recipe
         binds must be one of the authorization's concrete memory
         bases (the "authorize A, rewrite B" confusion is denied
         structurally);
       — adversarial mutation tests: definition swap, region rebind,
         fingerprint flip, strategy-family swap — all detected.
  P0A status (advisor correction, adopted):
       structural choke point COMPLETE; authorization provenance
       COMPLETE; memory-base binding COMPLETE as step 1; exact
       proposal binding OPEN (ProposalBinding model — proposals must
       declare source nodes, live-ins/outs, memory accesses with
       index expressions, iteration domain, predicate/map, accumulator
       binding + recurrence + semantics, result, effects, definition,
       strategy family, assumptions, runtime guards; authorize() must
       retrieve the immutable authorization and compare these facts
       exactly; recipes must CONSUME the authorized role map instead
       of re-deriving bindings); scalar definedness PARTIAL (shifts
       hardened; signed div/rem + transformation legality audited and
       now gated; exact flag unmodeled); P0A overall NOT YET CLOSED.
  Digest authority model (advisor directive 1, commit 5d826fb):
       the FNV binding digest is a DIAGNOSTIC (cache key, accidental
       mutation detection) — never authorization. Authority is the
       immutable AuthorizationDatabase: candidates carry
       AuthorizationId; optimizer AND rewrite engine retrieve the
       original by id and compare carried copies exactly (region,
       fingerprint, concrete facts, region provenance, concept
       coverage); unknown id = forged/stale = rejected. Adversarial
       test: self-consistent candidate citing an unissued id is
       rejected; a fresh database rejects candidates it did not issue.
       Known limitation: definition/strategy are covered by the
       mint-time digest only (compute_binding_digest is crate-private,
       cannot be reforged outside the crate) until ProposalBinding
       records the full minted binding in the database record.
  Signed div/rem audit (advisor directive 2, commit b0e3527): found
       LIVE unsound rewrites — the symbolic ModuloToAnd rule rewrote
       signed x % 8 -> x & 7 (wrong for negatives: -1 % 8 = -1 vs
       -1 & 7 = 7) behind stub proof obligations (hardcoded constants,
       never referencing actual nodes). Fixed under the two-
       certificate model: (a) definedness — udiv/sdiv/urem/srem
       distinguished by operand-type signedness; unsigned needs a
       constant nonzero divisor; SIGNED div/rem of any kind refuses
       (INT_MIN/-1 trap unmodeled); (b) transformation legality at
       recognizer level — modulo->mask, divide->shift, shift-mask
       recognized ONLY for unsigned operands (signed truncation/
       sign-extension semantics make the identities false for
       negative x); multiply->shift stays legal for both signednesses
       (two's-complement wrapping). Rotate shift-pair: variable k
       abstains; k==0 -> shr-by-width refused by the range check;
       LLVM 'exact' flag still unmodeled (not parsed).
  GENERIC VERIFIER SOUNDNESS AUDIT (advisor P0, this cycle): FAILED
       for the stub-backed class — recorded in docs/VERIFIER_AUDIT.md.
       The verifier was accepting theorem-shaped stubs as proofs:
       trivially-equal stubs (DivideShift/MultiplyShift/ShiftMask:
       Constant(0)==Constant(0)), a hardcoded-constant stub (ModuloAnd:
       Modulo(x,16)==And(x,15) regardless of the real divisor),
       tautologies (leading/trailing zero count: f(v)==f(v)), and
       free-variable templates (bitscan/byteswap/bitreverse/
       clear-isolate-set-bit/rotate — positional VariableIds never
       bound to candidate nodes; rotate hardcodes width 64 and ignores
       k==0 poison). Response: VerificationStatus enum (Stub ->
       TestedOnly -> SchemaChecked -> ConcreteSolverChecked ->
       MachineChecked), required verification_status() trait method,
       verifier quarantine (Stub can NEVER return Proven; default
       minimum SchemaChecked), honest classification of all 20
       definitions (16 Stub, 4 SchemaChecked), quarantined families now
       expect abstention in tests with comments; lowerer fail-closed
       for ashr/sdiv/srem/signed-icmp/exact-flag (SIR Shr/Div/Rem and
       comparisons are unsigned-model; signed lowering silently
       mistranslated — D3 corpus 11/16 -> 7/16 lowered, the honest
       number); UNIT_TEST sentinel rejected by the optimizer unless
       config explicitly allows it; adversarial verifier tests
       (verifier_quarantine_tests.rs): stub + tautology never Proven,
       mutated SchemaChecked theorems rejected, strict policy
       (ConcreteSolverChecked) quarantines SchemaChecked too.
       Gate 4B (three-kernel equivalence) has since been closed — see
       docs/GATE4B_PROOF.md. This entry concerns the quarantined
       definition obligations of the generic verifier. They remain
       open (P0A queue item 3) with the first lift landed
       2026-09-17: obligations can now bind actual nodes
       (`obligation_bound(candidate, function)`) and be discharged by
       the bit-blasting ConcreteSolver; MultiplyShift (id 102) is the
       first ConcreteSolverChecked definition (15 remain Stub). See
       docs/VERIFIER_AUDIT.md.
  PS002 END-TO-END AUDIT (advisor directive 2, this cycle): the
       SchemaChecked Any candidate on PS002 was NOT a safe independent
       optimization — it was a live semantic corruption caught before
       shipping. The Any recipe did not recognize FieldAccess slot
       consumers (only TupleExtract), so it took the
       wholesale-tuple-rebuild path, filled the non-reduction slots
       with the termination bound, and `array_find_last` silently
       became "return a constant": type-valid, structurally verified,
       semantically destroyed. Advisor hypothesis 3 confirmed: "correctly
       authorized at concept level but incorrectly bound to concrete
       roles." Fix: RewriteError::UnauthorizedLiveOut +
       authorized_tuple_consumer() shared guard — reduction recipes
       (any/all/parity/popcount) may replace a loop tuple slot ONLY when
       the consumer reads the accumulator slot; a consumer reading a
       position/index live-out refuses the rewrite. Targeted regression
       added (position-mutated variant with identical Any truth must
       not rewrite). PS002/ps001 expectations flipped to honest
       abstention; recorded in docs/VERIFIER_AUDIT.md. Residual risk
       recorded: the wholesale-tuple path still assumes
       "index == termination bound at exit" (sound for ascending count
       loops, unproven in general) and now only fires when no slot
       consumer exists — a future binding pass must prove the bound
       claim per shape or refuse.
  WHOLESALE-TUPLE QUARANTINE (advisor PS002 follow-up, commit 141fb26
       follow-on): "no recognized slot consumer" is not "no observable
       consumer" — the wholesale tuple path is too dangerous to leave
       enabled. Complete use classification now gates the four
       SchemaChecked reduction recipes: EVERY use of the loop tuple
       must be a recognized slot extract (TupleExtract or numeric
       FieldAccess), exactly one consumer, reading the accumulator
       position; otherwise UnknownConsumer/UnauthorizedLiveOut →
       abstain. wrap_direct_tuple_return refuses multi-element tuples
       outright (the "index == termination bound at exit" invention is
       an unproven exit-index assumption — PS002 class). Single-value
       (non-tuple) loop results remain enabled. PS002 is the canonical
       permanent example: a TRUE theorem (any == pack != 0) applied to
       the wrong observable boundary is still an incorrect compiler
       transformation. Assurance is now two-dimensional: TheoremAssurance
       (local semantic theorem) x ApplicationAssurance (theorem
       correctly bound to the complete concrete rewrite + frame
       conditions); EndToEnd = min of both. Regression family in
       liveout_binding_tests.rs (whole-tuple, multi-consumer, opaque
       projection, position-mutation, single-authorized-slot = enabled).
  ProposalBinding CONSUMED BY THE ANY RECIPE + END-TO-END ARTIFACT
       (advisor sequence 4/5/6): sir_semantics::binding derives the
       role map; RewriteEngine derives it for every reduction region
       and passes it via RewriteRegion; AnyRecipe consumes ONLY the
       binding (collection/op/scalar/slot from the map; refuses with
       RecipeFailed when no binding). Hardening added: AmbiguousRole
       (no heuristic selection among multiple non-counter
       accumulators), forward-only +1 stride (reverse traversals
       refuse), Dead live-outs carry UseClosureEvidence (direct users
       + function fingerprint), predicate op bound as a role (fixes
       the old emit_pack hardcoded-Gt bug — masks now use the TRUE
       op). Candidate frame check (no writes/calls/alloc/loops/loads/
       div traps; every external input a certified live-in).
       CheckedApplication issued by the engine; EndToEndVerification
       Artifact::new(Proof, CheckedApplication) verifies obligation
       linkage + artifact digest, refuses on mismatch, and is the ONLY
       route to mutation for reduction rewrites (RewriteResult carries
       it). 518/518 green; corpus unchanged (40/50, 0 rewrites).
  ALL REDUCTION RECIPES CONSUME THE BINDING (2026-09-17, P0A queue
       items 1-2 for recipes): the shared helpers (require_binding,
       binding_target, binding_collection_extent,
       emit_pack_from_binding) make the authorized ProposalBinding the
       single source of collection/predicate-op/scalar/observable
       target for Any, All, Parity, Popcount and the two BitScan
       recipes. Any's inline logic moved into the shared helpers; the
       structural emit_pack (which HARDCODED CmpOperator::Gt for
       predicate collections) is deleted. All/Parity/Popcount now
       refuse with RecipeFailed when no binding is derived, except the
       scalar SetIteration path (no binding by design: the engine binds
       collection reductions only) and Popcount's table-lookup path,
       which remain role-driven and are recorded exceptions. All over
       >64-element collections refuses (SIR constants carry one u64;
       a truncated full mask is never emitted). Tests:
       sir_optimizer/tests/binding_consumption.rs (All/Parity/Popcount
       predicate collections rewrite; the mask uses the binding's TRUE
       Eq, not hardcoded Gt), popcount stale-structural-role refusal,
       native differential corpora re-verified at 99 clean / 0
       mismatched / 19 rewrites. NEXT:
       CheckedApplication assurance > SchemaChecked (solver-backed
       candidate frame); transient-use closure via value-identical
       replacement is documented but a PHI/select downstream grammar
       remains open.
  Remaining P0A queue (advisor order): (1) ProposalBinding [CONSUMED
       by all collection reduction recipes + the Any end-to-end
       artifact; application checker upgrades pending]; (2) role-map
       plumbing [COMPLETE for the recipes listed above; the
       Popcount table-lookup path and scalar SetIteration path are
       recorded role-driven exceptions]; (3) upgrade quarantined
       definitions to ConcreteSolverChecked (obligation from the actual
       pair, mutation-sensitive) [IN PROGRESS 2026-09-17: the
       ConcreteSolver backend bit-blasts bound obligations; MultiplyShift,
       the four mask-algebra definitions, ByteSwap, BitReverse and
       ShiftMask, the rotate pair, the two zero-count scans and the two
       division identities are lifted (14/16); only the two bitscan
       definitions remain (PositionSearch/X06 authorization blocker);
       obligations can bind the authorized structural roles
       (`obligation_with_roles` + structural DB in build_obligations);
       instruction-selection emission (Rol/Ror, blsr/blsi/blsmsk,
       bswap, rbit) is implemented and ctz/clz follow the tzcnt/lzcnt
       zero convention; the scan families are blocked on
       PositionSearch authorization (200/201) and the missing
       loop↔intrinsic correspondence model (202/203); ModuloAnd/DivideShift need
       a width-efficient division encoding (urem/udiv bit-blasting is
       implemented and tested, but 32-bit proofs take ~20 s each; see
       docs/VERIFIER_AUDIT.md)]; (4) map-then-sum recall, two-loop lowering,
       accumulator width, C3 freeze, fresh H3; before fusion: seal
       Gate 6B or independent post-freeze corpus creation.
  ScalarExpression definedness gate (advisor: fail closed NOW, commit
       e62db09): Div/Rem refuse unless the divisor is a constant
       nonzero literal; shifts refuse unless the amount is a constant
       in [0, width); nsw/nuw nodes refuse the scalar and Composition
       grants. Witness impact accepted: HD003/BP001 rotate-by-variable
       abstain until a DefinednessCertificate provides shift-range
       proofs (tests updated to assert abstention).
  Volatile/atomic fail-closed (lowerer): volatile stores and atomic
       loads/stores now refuse loudly with an explicit "unsupported"
       error class (x01, x05) instead of parser accidents.
  Still open in D3: map-then-sum recall (w03), two-loop lowering (w08,
       third corpus hitting the gap), u8/I64 accumulator mismatch
       (w07), early-exit gep (x08). Volatile (x01) and atomic (x05)
       are now loud UNSUPPORTED refusals — intentional abstention,
       recorded as unsupported rather than unresolved.

Gate 6A-v3:  CLOSED   sealed v3 corpus found one false-positive class
                     (conditional/masked accumulation accepted as a raw
                     sum); remediation D5 (Sum requires the element
                     itself through transparent conversions); v3 promoted
                     to regression set; the fresh v5 corpus PASSED with
                     0 false positives, 0 unsafe candidates, 0 unsafe
                     rewrites and 10/12 positives recognized (2
                     pre-registered known gaps). A v4 run failed as a
                     corpus-classification defect (semantically vacuous
                     guard) and is preserved. See docs/GATE6A_V3_RESULTS.md.

Gate 6B:     PASSED — the fusion corpus was sealed before automation
             (gate6b/manifest.sha256, commit 4aedce1) and evaluated
             one-shot with a frozen harness (7ab8edc/2594d1a): 10/10 rows
             (5 fuse + 5 refuse), differential-clean, zero panics
             (docs/GATE6B_RESULTS.md).

Generational protocol: freeze → evaluate blindly → fail → preserve the
failure → understand the missing semantics → remediate → evaluate on a
new frontier. Each failing corpus becomes the next regression set
(H0→D1, H1→D2). Failures are never overwritten by remediation results.

Pass when:

- the system produces verified wins on held-out real kernels;
- false-positive recognition rate is negligible;
- safe abstention works correctly.

Only after Gate 6 should the general Arena become a major implementation effort.

---

## Experiment 1: Benchmark truth

### Build matrix

Use identical optimization conditions:

```text
Clang -O3 -march=native
GCC   -O3 -march=native
```

Where relevant, add:

```text
LTO
PGO
explicit target architecture
```

Do not use `-ffast-math` or relaxed semantics unless the source contract permits them.

### Prevent benchmark corruption

- make benchmark functions `noinline` where appropriate;
- consume results so calls cannot be eliminated;
- inspect assembly;
- verify that inputs are not compile-time constants;
- interleave candidates to reduce temporal bias.

### Input-size matrix

```text
16 B
64 B
256 B
1 KiB
4 KiB
16 KiB
64 KiB
1 MiB
16 MiB
```

The point is to expose distinct operating regimes:

```text
Tiny:
    setup and call latency

Cache-resident:
    instruction quality and vectorization

Medium:
    sustained compute throughput

Large:
    memory bandwidth and prefetching
```

A 1 MiB-only benchmark can hide optimization quality by turning every implementation into the same memory-bandwidth test.

### Input distributions

Each kernel needs multiple data distributions.

#### `all_equal`

At minimum:

```text
All elements equal
Mismatch at first element
Mismatch near beginning
Mismatch in middle
Mismatch at last element
Random mismatch position
No mismatch
```

A scalar early-exit loop may dominate when mismatches occur early. SIMD may dominate when all values must be inspected. Neither implementation is universally faster.

The benchmark reward should reflect the customer's actual input distribution.

#### `count_masked`

Test varying selectivity and predictability:

```text
0% matches
1% matches
25% matches
50% matches
99% matches
100% matches
Alternating/adversarial patterns
Random patterns
```

This distinguishes branch-prediction gains from true arithmetic throughput.

#### `sum_ascii`

Test:

- representative ASCII;
- random bytes if valid;
- boundary values;
- lengths with every possible tail modulo vector width.

### Measurement protocol

Use calibrated iterations so each sample is long enough to rise above timer noise.

Record:

```text
Median
Minimum or best stable estimate
MAD or standard deviation
95% confidence interval
Cycles per byte/element
Throughput
Code size
```

When useful, record hardware counters:

```text
instructions
cycles
branches
branch misses
cache misses
vector instructions
```

Pin to a physical core, warm up, and randomize candidate order.

A credible win should satisfy something like:

```text
95% lower confidence bound exceeds 1.10×
```

on at least one economically relevant workload regime, without unacceptable regressions elsewhere.

Three-times faster would be excellent, but it is not required. A stable 15–30% gain on a sufficiently expensive kernel can support a business.

---

## Experiment 2: Witness frontier

### Kernels

Begin with:

- `k43_sum_ascii`
- `k50_count_masked`
- `k18_all_equal`

But classify them as development kernels afterward. They cannot also be the final held-out evidence.

### Candidate families

For each kernel:

```text
A. Original source
B. Clean conventional scalar
C. Chunked scalar
D. SWAR
E. Compiler builtins
F. LLVM semantic intrinsics where applicable
G. SSE implementation
H. AVX2 implementation
I. AVX-512 implementation where supported
J. Size-specialized/multi-versioned implementation
K. Relevant library implementation, if one exists
```

Not every family applies to every kernel.

For example:

- `sum_ascii` may use vector widening and horizontal summation.
- `count_masked` may use vector comparison, mask extraction, and popcount.
- `all_equal` may use wide comparison and reduction, but early-exit behavior makes workload distribution especially important.

### Correctness contract first

Before benchmarking, define exactly:

```text
Input type and valid lengths
Return type
Overflow behavior
Alignment guarantees
Aliasing guarantees
Endianness assumptions
Whether out-of-bounds wide reads are forbidden
Whether early termination is observable or required
Whether timing/constant-time behavior matters
```

A fast implementation that reads seven bytes past the end or violates strict aliasing is not a valid witness.

For wide scalar loads, avoid accidental undefined behavior. The experiment must distinguish:

- guaranteed alignment;
- unaligned but valid access;
- page-boundary safety;
- legal tails.

### Required output

For every kernel and size/distribution combination:

| Candidate | Correct | Median | Confidence | Cycles/element | Speedup | Notes |
|---|---:|---:|---:|---:|---:|---|
| Clang original | Yes | | | | 1.00× | |
| GCC original | Yes | | | | | |
| Chunked scalar | Yes | | | | | |
| SWAR | Yes | | | | | |
| SSE | Yes | | | | | |
| AVX2 | Yes | | | | | |
| AVX-512 | Yes | | | | | |

Also retain generated assembly for every meaningful result.

---

## Interpreting the witness frontier

### Outcome A: Large known headroom

Example:

```text
Clang baseline: 1.00×
AVX2 witness:   2.40×
```

Conclusion:

```text
The domain contains reachable headroom.
ABSAC needs a semantic path and lowering path to this family.
```

Next step: represent the semantic endpoint and emit vector/intrinsic LLVM IR.

### Outcome B: Headroom only in certain regimes

Example:

```text
16 B: scalar wins
4 KiB: AVX2 wins 1.7×
1 MiB: tie due to memory bandwidth
```

Conclusion:

```text
ABSAC needs guarded or size-specialized multi-versioning.
```

That is itself a valuable semantic optimization.

### Outcome C: Hand candidates tie Clang

Conclusion:

```text
No currently demonstrated headroom in this kernel.
```

Inspect assembly to determine whether Clang already produced the same algorithm. Then:

- deprioritize the kernel;
- search for cross-representation or cross-function opportunities;
- or retain it as a "compiler already optimal" negative example.

It does not disprove the broader project.

### Outcome D: Manual candidate is slower

Conclusion:

```text
The proposed optimization intuition was wrong for this hardware/workload.
```

That is valuable training and cost-model data.

---

## Experiment 3: Target-aware semantic lowering

Once a winning candidate family exists, ABSAC needs an action space capable of reaching it.

### Near-term backend priority

1. LLVM semantic intrinsics (`ctpop`, `ctlz`, `cttz`, rotations, byte swaps).
2. LLVM vector IR.
3. Portable builtins.
4. Explicit target intrinsics where necessary.
5. Handwritten assembly only for experiments or true backend gaps.

ABSAC should win by giving LLVM a better computation, not by prematurely reimplementing LLVM's backend.

### Candidate lowering set

For a population-count reduction, for example:

```text
Semantic operation:
    Cardinality(BitSet)

Lowering candidates:
    scalar loop
    Kernighan loop
    compiler ctpop intrinsic
    table lookup
    SWAR
    scalar POPCNT
    vector popcount
    vector lookup/emulation
    multi-versioned size-dependent implementation
```

The key is not merely adding an emitter special case. The semantic operation should survive far enough down the pipeline that target-aware lowering can make the choice.

### Important qualification about SWAR

"Add SWAR/SIMD" should not itself become dogma. Depending on the CPU:

- scalar `popcnt` may beat SWAR;
- AVX-512 may beat scalar only above certain sizes;
- SIMD setup may lose on small buffers;
- unaligned wide loads may introduce correctness or performance issues;
- large-buffer scans may be memory-bandwidth bound.

The correct candidate set is:

```text
portable scalar
compiler builtin
scalar target instruction
SWAR
SIMD
multi-versioned implementation
```

Then measure the Pareto frontier.

---

## Experiment 4: Search value

### Step 1: Establish reachability

Can ABSAC represent the semantic endpoint?

For example:

```text
CountMasked
→ PredicateMask
→ Cardinality
→ vector compare
→ mask extraction
→ ctpop
```

If not, the missing work is ontology/action/lowering — not search.

### Step 2: Enumerate before beam

If the action graph is small, enumerate every unique state to bounded depth.

Measure:

```text
Average branching factor
Unique canonical states by depth
Percentage of candidates that compile
Percentage that verify
Percentage that are performance-distinct
Frequency of temporary performance valleys
Number of useful multi-step paths
```

This tells us whether search is genuinely needed.

### Step 3: Compare engines

Under equal budgets:

```text
Current deterministic optimizer
Greedy measured selector
Exhaustive bounded search
Current beam scaffold with node-count heuristic
Beam with target-aware static heuristic
Beam with measured terminal evaluation
Random search
```

Only then decide whether MCTS is warranted.

### Search evaluator ablation

Compare candidate ordering by:

```text
SIR node count
Static target cost
Compiled instruction count
Predicted performance
Actual measured performance
```

This will show whether an advanced evaluator is necessary.

### MCTS readiness test

Do not build MCTS unless the experiment shows:

- enough unique states to make enumeration costly;
- delayed rewards;
- interacting transformations;
- meaningful uncertainty in action value;
- beam performance dependent on ordering.

If exhaustive search handles the domain, keep exhaustive search. "Grandmaster" means choosing the right method, not using the most fashionable one.

---

## Experiment 5: Generalization test

The kernels used to design the transformations are the development set.

A separate hidden set must contain:

- different source projects;
- syntactically different implementations;
- different widths and bounds;
- different input distributions;
- multiple similar operations in one function;
- irrelevant surrounding code;
- cases that must not optimize.

A convincing result is:

> ABSAC finds a verified target-aware optimization on a kernel that was not used to construct its recognizer or tune its lowering.

Without this, the system risks becoming benchmark-specific engineering.

Reserve other kernels as hidden evaluation cases.

---

## What not to do during this phase

- Do not build new Arena infrastructure during this experiment.
- Do not build MCTS before measuring the action graph.
- Do not use tests as a substitute for equivalence.
- Do not use one benchmark input as truth.
- Do not claim victory over `-O3` without strong target flags and baselines.
- Do not train and evaluate on the same kernels.
- Do not let scalar C be the only backend.
- Do not let SWAR/SIMD become dogma — measure the Pareto frontier.
- Do not conclude the project is impossible from a null witness result.

---

## Commercial success threshold

ABSAC does not need to beat every compiler on every program. That is not a realistic initial business condition.

It needs to be uniquely valuable in one expensive domain.

A commercially meaningful claim could be:

> On supported bitmap and logical-reduction kernels that remain hot after Clang `-O3`, ABSAC automatically discovers concretely verified implementations delivering a median X% improvement on target hardware.

Or:

> ABSAC finds cross-representation optimizations that conventional compilers cannot perform because they do not recover the required high-level semantics.

Clang is extraordinarily strong at:

- local simplification;
- scalar optimization;
- loop transformation;
- vectorization;
- instruction selection.

The long-term opportunity is not primarily to beat LLVM at those jobs. It is to give LLVM a better algorithm and representation:

```text
Clang:
    Optimize this loop.

ABSAC:
    This is not fundamentally a loop.
    It is finite-set cardinality under a representation contract.
    Replace the representation and all consumers.
    Now lower the resulting semantic operation.
```

That is the defensible advantage.

If a customer spends $10 million annually on a workload and ABSAC safely reduces it by 15%, that is approximately $1.5 million of potential annual value.

What matters is:

```text
large hot path
meaningful verified speedup
safe deployment
low integration cost
repeatability across customers
```

A narrow optimizer that repeatedly saves millions is a company. A universal optimizer that occasionally produces impressive demos but cannot be trusted is not.

---

## Governing decision

The project now has two documents:

### North star

```text
Two-game Arena
Universal construction
Proof-gated commitment
Learning engines
Evolving macro-actions
```

### Immediate research program

```text
Fair benchmark
→ witness frontier
→ identify demonstrated headroom
→ expose winning semantic/target actions
→ concrete verification
→ exhaustive/beam search
→ held-out generalization
→ only then Arena/MCTS
```

That is the right balance between ambition and empirical discipline. The vision remains intact, but each expensive layer must now earn its existence.

The immediate mission is sharp:

> **Find the headroom. Represent the winning implementation. Prove it. Measure it. Then establish whether search — not a hardcoded pass — was necessary to reach it.**
