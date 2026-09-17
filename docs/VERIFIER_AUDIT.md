# Verification-Definition Registry Audit (Advisor P0)

**Date:** Gate 6A-v3 remediation cycle.
**Verdict: FAILED for the stub-backed class.** The generic verifier was
accepting "proof obligations" that are theorem-shaped stubs — hardcoded
examples, tautologies, or free-variable templates that never bind the
actual source/candidate operands. A recognizer's assumption was being
laundered through a "proof" that could not fail.

## Response implemented (this commit)

1. **`VerificationStatus` enum** (Stub → TestedOnly → SchemaChecked →
   ConcreteSolverChecked → MachineChecked) with a required
   `verification_status()` trait method — no default, fail-closed.
2. **Verifier enforcement**: `Verifier::verify()` looks the definition
   up in the registry (not the obligation) and returns
   `Unknown(InsufficientAssurance)` for any definition below the
   engine's minimum level. **Stub can never return Proven.** Default
   minimum: `SchemaChecked` (research mode with honest labeling);
   production policy should require `ConcreteSolverChecked`.
3. **Honest classification** of every definition (table below).
4. **Lowerer fail-closed** (`sir_lower`): signed opcodes `ashr`, `sdiv`,
   `srem` and signed icmp predicates (`sgt/sge/slt/sle`) are now loud
   UNSUPPORTED refusals — SIR's single Shr/Div/Rem/comparison nodes are
   unsigned-model (the lowerer types every LLVM `iN` as unsigned), so
   lowering signed opcodes silently produced differently-defined
   programs. The `exact` division/shift flag (poison on inexact) is
   refused loudly.
5. **UNIT_TEST sentinel hardening**: the optimizer now rejects candidates
   carrying the `AuthorizationId::UNIT_TEST` sentinel unless the config
   explicitly permits it (`allow_unit_test_authorizations: false` by
   default). Production candidates are always minted with real ids.

## The 10 binding questions, asked of every definition

For each definition: does the obligation reference the actual source and
candidate nodes? Would changing a constant, swapping an operand,
changing signedness, or changing width cause rejection?

**Honest answer for the stub class: no.** The obligations are built from
`VariableId::new(0)` placeholders and hardcoded constants. They never
reference the actual source/candidate nodes, so no input mutation can
change the verdict — the verifier was asserting the recognizer's
assumption by fiat.

## Registry classification

| Definition | Obligation shape | Status |
|---|---|---|
| ModuloAnd (100) | `Constant(0) == Constant(0)` trivially-equal stub; comment admits the real theorem is never built | **Stub** |
| DivideShift (101) | `Constant(0) == Constant(0)` — trivially equal | **Stub** |
| MultiplyShift (102) | `Constant(0) == Constant(0)` | **Stub** |
| ShiftMask (103) | `Constant(0) == Constant(0)` | **Stub** |
| LeadingZeroCount (203) | `LeadingZeros(v) == LeadingZeros(v)` — tautology; free variable | **Stub** |
| TrailingZeroCount (202) | `TrailingZeros(v) == TrailingZeros(v)` — tautology | **Stub** |
| BitScanForward (200) | variable placeholder, `domain: None`, no node binding | **Stub** |
| BitScanReverse (201) | variable placeholder, `domain: None` | **Stub** |
| ByteSwap (312) | fixed width-64 shape with hardcoded 0xFF/8; free `VariableId` never bound to candidate nodes | **Stub** |
| BitReverse (313) | variable-shape template, no candidate binding | **Stub** |
| ClearLowestSetBit (300) | variable-template | **Stub** |
| IsolateLowestSetBit (301) | variable-template | **Stub** |
| IsolateLowestClearBit (302) | variable-template | **Stub** |
| SetLowestClearBit (303) | variable-template | **Stub** |
| RotateLeft (310) | hardcoded width 64, free k; k==0 is poison in LLVM (shr by width) — never modeled | **Stub** |
| RotateRight (311) | same | **Stub** |
| Popcount (0) | binds candidate's FixedLength constraint; exhaustive/symbolic over semantic form | **SchemaChecked** |
| Any (4) | binds FixedLength; trusted evaluator | **SchemaChecked** |
| All (5) | binds FixedLength; trusted evaluator | **SchemaChecked** |
| Parity (6) | binds FixedLength; trusted evaluator | **SchemaChecked** |

Answers to the binding questions per class:

- **Trivially-equal stubs** (divide_shift, multiply_shift, shift_mask):
  theorem is `Constant(0) == Constant(0)`. Changing divisor 8→7, UDiv→SDiv,
  or any width change: **no effect** — accepted unconditionally. That is
  the definition of not a proof.
- **Hardcoded-constant stub** (modulo_and): theorem is
  `Modulo(x, 16) == And(x, 15)` regardless of the actual divisor;
  divisor 8→7 causes no rejection. Live unsoundness found in Gate 6A-v2:
  AR001 (i32) "verified" `-1 % 8 → -1 & 7`, which is false.
- **Tautologies** (leading/trailing zero count): `f(v) == f(v)` — true by
  construction; proves nothing about the candidate.
- **Free-variable templates** (bitscan, byte swap, bit reverse,
  clear/isolate/set-bit, rotate): right *shape*, wrong *binding* — the
  `VariableId`s are positional placeholders never tied to candidate
  nodes; rotate hardcodes width 64 and ignores the `0 < k < width`
  definedness requirement (k==0 → shr by width = poison).
- **SchemaChecked four** (popcount/all/any/parity): theorem built from
  the candidate's constraints (FixedLength), discharged by exhaustive
  enumeration over a trusted handwritten evaluator. Honest limitation:
  variable binding is positional, not node-identifying; the exhaustive
  backend ignores context constraints not represented in the domain.

## Failure records preserved

- **Definition-level machine checking: OPEN** — "Generic verifier
  soundness audit: FAILED for the ModuloAnd/DivideShift class" (and 13
  further definitions). This is the quarantined-definition obligation
  (P0A queue item 3, `ConcreteSolverChecked`), distinct from the
  three-kernel Gate 4B, which was closed on 2026-09-15 by
  docs/GATE4B_PROOF.md. Quarantine stays until each definition's
  obligation is built from the actual source/candidate pair.
- All zoo arithmetic entries, AR001–AR003, HD001/HD004/HD005/HD007/HD008,
  HD004/HD005 emission tests, PS001/PS003/PS004, and COMP001 now expect
  **abstention** with quarantine comments. The identities themselves are
  true for unsigned operands; they return when each definition's
  obligation is built from the actual source/candidate pair and
  discharged concretely (`ConcreteSolverChecked`).
- Lowerer now refuses loudly: `ashr`, `sdiv`, `srem`, signed icmp
  predicates, and the `exact` flag. D3 corpus: 11/16 → 7/16 lowered —
  four kernels previously "lowered" with potentially mistranslated
  signed comparisons (e.g. `w05_count_positive`: `icmp sgt i32 %10, 0`
  lowered to an unsigned `> 0` that counts every nonzero value as
  positive). Honest abstention replaces silent mistranslation.

## Path back for quarantined families

1. Obligation construction moves from template to **the actual pair**:
   source expression and candidate expression bound to concrete
   `NodeId`s, widths, and constants from the candidate's binding.
2. The verifier checks that changing any bound field (divisor, width,
   signedness, operand order) changes the obligation — mutation tests
   like `verifier_quarantine_tests.rs` generalize this.
3. Discharge with the symbolic/exhaustive backends and record the
   obligation + result in the proof artifact (ConcreteSolverChecked).
4. Only then may the definition's `verification_status` be raised and
   the corresponding recipe re-enabled.

## Quarantine lift — ConcreteSolver backend + MultiplyShift (2026-09-17)

The first lift implements the path above end to end:

- **New `ConcreteSolver` backend** (`backends/concrete_solver.rs`,
  `VerificationBackend::ConcreteSolver`, cap
  `ConcreteSolverChecked`). An obligation with a finite domain is
  lowered into `sir_mech` bitvector terms at the variables' declared
  widths and discharged by the bit-blasting CDCL solver
  (`prove_equal`): UNSAT of the negated difference is a proof, a SAT
  model is re-evaluated and returned as a counterexample, and
  unsupported shapes / missing domains / solver-budget exhaustion stay
  Unknown (fail closed). `Verifier::verify` runs this backend first for
  definitions whose cap requires it and whose domain is finite;
  everything else keeps the historical symbolic-first order.
- **Obligations bind the actual function version**:
  `TransformationDefinition::obligation_bound(candidate, function)`
  (default = the legacy template, so stubs are unchanged) and
  `Verifier::build_obligations(..., function)`. A definition reads the
  candidate's authorized `region_nodes`.
- **MultiplyShift (id 102) is ConcreteSolverChecked**: the theorem is
  `x * C == x << log2(C)` with `C` and the width taken from the actual
  `Mul` node. A pair that cannot be bound (no power-of-two constant, no
  authorized region) gets a non-identity theorem with no domain and can
  never be Proven. Tests (`tests/concrete_solver_upgrade.rs`): the
  correct pair is Proven with ISSUED `ConcreteSolverChecked`; a mutated
  claim (`x * 8 == x << 4`) is Rejected with a counterexample; a
  non-power-of-two constant and an unauthorized region are not Proven.
- **Enabling the definition exposed two latent recipe bugs** (the
  quarantine had hidden them): the recipe assumed the constant is the
  RHS, so `32 * x` would have emitted `32 << tzcnt(x)`; and it only
  parsed unsigned constants, refusing `i32 2`. Both fixed with
  regression tests (`tests/arithmetic_quarantine_lift.rs`: both operand
  orders, signed and unsigned, non-power-of-two abstains, DivideShift
  still quarantined).
- **Expectations updated**: AR003 is `Optimizes`; the semantic-zoo
  multiply-by-power-of-two rows expect 1 rewrite; `validate_ba003`
  asserts the shift-left.

Remaining quarantined: 1 definition (BitScanReverse). Everything else is
lifted: MultiplyShift, the four mask-algebra
definitions, ByteSwap, BitReverse, ShiftMask, the two rotate
definitions, the two zero-count definitions, ModuloAnd and DivideShift.
BitScanForward joined them on 2026-09-17 (lift 9 below).
precisely:

- **ModuloAnd / DivideShift → LIFTED 2026-09-17** (lift 8 below). The
  `urem`/`udiv` bit-blasting is in `sir_mech` (restoring division,
  SMT-LIB zero-divisor semantics, exhaustive 4-bit eval cross-check,
  SAT-proved identities at 4/8/16/32 bits), and the proof-cost blocker
  was removed by CNF constant folding.
- **ShiftMask → LIFTED 2026-09-17** (see below).
- **Mask-algebra (clear/isolate/set lowest bit) → LIFTED 2026-09-17**
  (see the next section).
- **Zero-scan and bit-permutation (rotate/byteswap/bitreverse)
  families** need their operations lowered into `sir_mech` terms;
  constant-amount rotates and lowest-bit expressions are already
  modeled. The emitter blocker is gone: `Rol`/`Ror`, `blsr`/`blsi`/
  `blsmsk`, `bswap`, `rbit` now emit correct C (rotates via a
  width-relative helper; bswap via the builtin; rbit/blsr/blsi/blsmsk
  via helpers), and unknown intrinsics fail loudly at compile time
  instead of emitting 0. What remains for those families is the
  definition binding + solver lowering (`ctz`/`clz`/bit-reverse terms),
  not emission.

### Scan-family blockers (recorded 2026-09-17)

The four scan definitions have distinct, now-measured blockers:

- **BitScanForward (200) → LIFTED 2026-09-17** (lift 9 below).
- **BitScanReverse (201) remains Stub** with three recorded blockers:
  (a) the historical obligation `LastTrue(seq) == LeadingZeros(Pack(seq))`
  is **FALSE** — the solver returns the counterexample for an MSB-set
  mask (lhs 63 vs rhs 0); (b) the recipe equally emits `LeadingZeros`, a
  latent miscompile the quarantine hid; (c) the reduction binding's
  trip-count contract is forward-only (`i < bound`, zero-based), while
  the reverse search iterates `i = 63 .. 0`. A correct lift needs a
  bit-scan-reverse intrinsic (highest set index, width sentinel for
  zero) plus reverse counted-loop trip-count support.
- **TrailingZeroCount / LeadingZeroCount (202/203) → LIFTED 2026-09-17**
  (lift 7 below).
- **Emitter prerequisite (done)**: `TrailingZeros`/`LeadingZeros` now
  emit `__sir_ctz`/`__sir_clz` with the tzcnt/lzcnt convention
  (`ctz(0) = clz(0) = width`); the previous raw builtins were
  undefined behaviour for zero. Native tests cover 0/1/0x10 and the
  MSB case. This is required for any future scan rewrite to execute
  natively.

## Quarantine lift 7 — zero-count scans with a loop↔intrinsic model (2026-09-17)

- **New `SemanticExpression::Reverse`** (boolean-sequence reversal) plus
  interpreter, normalizer and solver support; the solver also gained
  `LogicalSequence`/`Pack` lowering, `FirstTrue` (forward found-flag
  scan), `LastTrue` (overwrite fold), `TrailingZeros` (reverse overwrite
  fold) and `LeadingZeros` (reverse found-flag scan). The two sides of
  each obligation are deliberately different constructions so the proof
  is not reflexive.
- **Definitions 202/203 are ConcreteSolverChecked**: the obligation
  binds the loop's scalar and unsigned width from the recognized
  PositionSearch (or SetIteration) role and proves
  `ctz(x) == FirstTrue(bits(x))` / `clz(x) == FirstTrue(Reverse(bits(x)))`
  (tzcnt/lzcnt convention for zero). Signed scalars and regions without
  the role carry no domain and are never Proven.
- **Recipes fixed**: both zero-count recipes replaced `region.result()`
  (the tuple-typed loop) directly, leaving the function's field-access
  consumer reading a stale/default value — the emitted C returned 0 for
  every input. They now target the tuple-extract consumer with the
  consumer's type (`leading_zeros_typed` added). This was a live
  miscompile hidden by the quarantine.
- **Interpreter fix**: `TrailingZeros(BitVector)` returned 128 for zero;
  it now follows the width convention, matching the solver and emitter.
- **Evidence**: semantic-zoo ps003/ps004 and positional-search
  PS003/PS004 flip to `Optimizes` (they rewrite); verifier tests cover
  both proofs plus signed/no-role refusal; a native test runs the
  rewritten loops and checks tz(0/0x10/1) = 64/4/0 and
  lz(0/1/MSB) = 64/63/0.
- **Harness fix recorded**: `emit_c_native`'s stdout redirection is now
  serialized by a mutex — parallel tests racing on `dup2(1)` could leave
  fd 1 on `/dev/null` and swallow later test output (the workspace run
  appeared to lose nine passing tests).

## Quarantine lift 8 — division identities + CNF constant folding (2026-09-17)

- **CNF constant folding**: `Cnf::{and2, or2, xor2, xnor2, mux}` now
  simplify when an input is the constant-true/false literal (and on
  equal/opposite pairs). This preserves semantics exactly and collapses
  the gate chains for constant operands — the divisor in these
  obligations is always a literal. Measured effect on the solver's
  identity suite: the 4/8/16/32-bit `x/C == x>>log2 C` and
  `x%C == x&(C-1)` proofs went from ~44 s (32-bit alone ~20 s per
  identity) to **0.75 s for the whole suite** including width 32.
- **ModuloAnd (100) / DivideShift (101) are ConcreteSolverChecked**:
  the obligations bind the actual `Rem`/`Div` role (divisor constant,
  operand, width; role/node agreement checked) and prove
  `x % C == x & (C-1)` / `x / C == x >> log2(C)` for UNSIGNED operands.
  Signed operands and non-power-of-two divisors carry no domain and are
  never Proven. Recipes validate the same conditions (the old recipes
  assumed these without checking) and emit the real mask/shift.
- **Evidence**: verifier tests for both proofs plus signed/non-power-of-
  two refusals; semantic-zoo `arith_mod_unsigned_*` /
  `arith_div_unsigned_*` and AR001/AR002 flip to rewriting; ba001/ba002
  assert the AND/shift; a native test runs `x % 16` and `x / 8`
  (0xFFFFFFFF → 15 / 536870911, 0x12345678 → 8 / 38177487).
- **Count**: 14/16 lifted; only the BitScan pair remains, blocked on
  PositionSearch (X06) authorization.

## Quarantine lift 9 — BitScanForward + found-flag search recognition (2026-09-17)

- **Found-flag search recognition** (`recognizers/position_search.rs`):
  a new detector covers the PS001/PS002 kernel shape — a carried `found`
  flag guarded by `!found` in the termination, a carried induction value
  whose successor is `i ± 1` and an output, a position select whose TRUE
  arm is that induction value, and a select condition derived from
  memory and guarded by `!found`. It emits FirstOccurrence (forward) or
  LastOccurrence (reverse); the X06 check passes because the select
  binds the index and its condition is a real element test.
- **Binding/trip-count accept guarded terminations**: `find_counter_bound`
  and `comparison_is_counter_lt` look through `BoolAnd` conjuncts, and
  `estimate_trip_count` does the same, so `!found && i < limit` is a
  counted loop with the array's declared extent.
- **Engine**: a candidate authorized under the PositionSearch domain no
  longer requires a reduction binding — that binding correctly refuses
  the position live-out (outside the reduction theorem), so the engine
  assembles the region without it. Position recipes consume the
  authorized PositionSearch role (`emit_pack_for_position_search`) for
  the collection and extent; this is a recorded exception to the
  reduction-binding rule, alongside the Popcount table-lookup path.
- **BitScanForward (200) is ConcreteSolverChecked**: binds the role's
  boolean-array extent and proves `FirstTrue(seq) == ctz(Pack(seq))`.
  The recipe now targets the FieldAccess/TupleExtract consumer (the old
  code only recognized TupleExtract) and emits `TrailingZeros(pack(arr))`.
- **Evidence**: semantic-zoo ps001 and positional-search PS001 rewrite
  (trailing-zero selection asserted); a native test runs the rewritten
  function — first set bit at index 5 returns 5, an empty array returns
  the sentinel 64. ps002 remains blocked and a regression test pins it;
  the stub-quarantine tests now use BitScanReverse (201) as the
  canonical remaining stub.

## Quarantine lift 3 — ByteSwap + role-based obligation context (2026-09-17)

- **Role-aware obligations**: `TransformationDefinition::obligation_with_roles`
  (default delegates to the function-only path) and
  `Verifier::build_obligations(..., function, structural_db)` give a
  definition the authorized `StructuralDescription` of the candidate's
  region. Families whose semantics live in a recognized role can now
  bind from it instead of re-scanning the graph.
- **ByteSwap (312) is ConcreteSolverChecked**: the obligation reads the
  recognized `BitPermutation { operand, kind: ByteSwap { perm_width,
  type_width } }` role and proves, with the bit-blasting solver,
  `bswap(x) >> (type_width - perm_width) == <byte-swapped low
  perm_width bits>` at the actual word width. The recipe (already
  correct for partial swaps) selects `bswap`; the emitter expands it
  via `__builtin_bswap32`. HD004 now rewrites, and a native test runs
  the optimized/emitted function and checks the values
  (`emit_c_native.rs`).
- **Recorded scope caveat**: the solver proves the semantic operation
  equals the canonical permutation built from the role's operand and
  widths. The *source pattern* match is the recognizer's structural
  role — versioned by the authorization fingerprint — not a
  node-by-node translation of the source subgraph into the theorem
  (that stronger form remains future work). This is still materially
  stronger than the old free-variable template: wrong operand, wrong
  widths or a mutated role shape change the bound obligation.
- **Next**: BitReverse (313) needs the same role binding plus a
  bit-reversal lowering; the scan families need `ctz`/`clz` lowering;
  rotate families are definedness-blocked on variable amounts
  (constant-amount obligations can bind once candidates exist).

## Quarantine lift 4 — BitReverse (2026-09-17)

- **BitReverse (313) is ConcreteSolverChecked**: binds the recognized
  `BitPermutation { kind: BitReverse { perm_width, type_width } }` role
  and the concrete solver proves `rbit(x) >> (type_width - perm_width)`
  equals the reversal of the low `perm_width` bits at the actual width
  (new full-width bit-reversal lowering). HD005 flips to `Optimizes`;
  a native test runs the optimized/emitted `rbit` form and checks
  values (0x01→0x80, 0x0F→0xF0, 0xA5→0xA5, 0→0); the permutation test
  asserts the `Intrinsic(rbit)` selection. Verifier tests cover the
  bound role and the no-role refusal.
- **Rotate families → LIFTED 2026-09-17** (lift 6 below).

## Quarantine lift 6 — rotate pair + constant-amount definedness (2026-09-17)

- **Root cause of the rotate candidate gap**: the scroll-range
  definedness gate accepted only a *literal* constant shift amount, so
  the canonical rotate `(x << k) | (x >> (W - k))` — whose amount is the
  constant expression `W - k` (a `Sub` of literals) — never authorized,
  the authorization database stayed empty, and certificate-gated
  generation produced zero candidates. `scalar_expression_is_defined`
  now evaluates constant add/sub chains (`constant_amount`); variable
  amounts still refuse, so HD003/BP001 remain definedness-blocked.
- **RotateLeft (310) / RotateRight (311) are ConcreteSolverChecked**:
  `rotate_bind.rs` verifies the actual `Or(Shl(x,k), Shr(x,W-k))`
  source pattern (role operand/direction/amount, both shift constants,
  `0 < k < W`), and the concrete solver proves the constant-amount
  identity. Left selects `Rol`, right selects `Ror`; variable-amount
  obligations carry no domain and are never Proven.
- **Evidence**: verifier tests cover both directions (Proven with
  issued ConcreteSolverChecked) and the variable-amount refusal; the
  optimizer tests assert constant left/right rewrite and Rol/Ror
  selection plus variable abstention; a native test runs the rewritten
  `Rol` and checks `0x80000001 → 0xC`, `0x12345678 → 0x91A2B3C0`.

## Quarantine lift 5 — ShiftMask + real mask-extraction recipe (2026-09-17)

- **ShiftMask (103) is ConcreteSolverChecked**: binds the recognized
  `ArithmeticOperation` role (inner shift-left, constant amount,
  unsigned operand) and the concrete solver proves
  `(x << k) >> k == x & ((1 << (W-k)) - 1)` for `0 ≤ k < W`. `k = 0`
  uses the all-ones mask; signed operands and `k ≥ W` are refused (no
  domain). 32-bit instances solve instantly (shifts/masks only).
- **The recipe was a stub** (`mask = shl(rhs, rhs)` — unrelated to the
  identity). It now validates the region role, the constant amount and
  the unsigned width, and emits the real mask constant + AND. HD/ba004
  and the semantic-zoo `arith_mask_unsigned_{2,4,8,16}` rows now
  rewrite; signed rows and the full-width `k = W` row stay at zero.
  A native test runs the optimized/emitted form and checks
  `0xFFFFFFFF → 0x0FFFFFFF`, `0x12345678 → 0x02345678`, `0xF → 0xF`.

## Quarantine lift 2 — mask algebra + instruction-selection emission (2026-09-17)

- **Emitter** (`sir_benchmarks::emit`): `Rol`/`Ror` lower to a
  width-relative rotate helper (`k %= w`, no shift-by-width UB);
  `blsr`/`blsi`/`blsmsk` expand to `x&(x-1)`, `x&(0-x)`,
  `x^(x-1)`; `bswap` uses `__builtin_bswap16/32/64`; `rbit` uses a
  bit-reversal helper. An unknown intrinsic now emits a call to an
  undeclared function (loud compile/link error) rather than the old
  silent `0`. Native tests execute every one of these
  (`emit_c_native.rs`).
- **Definitions 300-303** (ClearLowestSetBit, IsolateLowestSetBit,
  IsolateLowestClearBit, SetLowestClearBit) are ConcreteSolverChecked:
  the obligation binds the candidate's actual pattern (`mask_bind.rs`
  matches `x&(x-1)`, `x&-x` / `x&(0-x)`, `~x&(x+1)`, `x|(x+1)` with
  either operand order) and the concrete solver discharges the semantic
  identity at the operand's width. Unbound regions (wrong pattern, no
  authorized nodes) carry no domain and are never Proven.
- **End-to-end**: HD001/HD007/HD008 flip to `Optimizes`; a new native
  test runs the optimizer on each pattern, asserts the rewrite fired,
  emits the rewritten `blsr`/`blsi`/`blsmsk` form, compiles and
  executes it, and checks the values. Verifier binding tests cover all
  four patterns plus an unrelated-region refusal.

C3 remains an Any-only freeze.

## PS002 end-to-end audit (advisor directive, this commit)

The advisor flagged that PS002 (`last_set_bit`) was "legitimate" only by
proxy of arriving through the SchemaChecked Any path rather than the
Stub bitscan path. End-to-end inspection (new diagnostic binary
`ps002_inspect`) found a **live semantic corruption**, not a safe
independent optimization:

- The PS002 loop's live-out is the **position** (field 1 of the loop
  tuple: index of the last true element), not the boolean `found`
  accumulator (field 0) the Any theorem covers.
- The Any recipe only recognized `TupleExtract` consumers; the builder
  emits `FieldAccess` — so the recipe misclassified the function as
  "tuple returned wholesale", rebuilt the loop tuple as
  `(any_bit, termination_bound, termination_bound)`, and the returned
  position silently became a **constant**. Type-valid, structurally
  verified (sir_verify passed), authorized by a true SchemaChecked
  theorem — and semantically destroyed.
- Classification: advisor hypothesis **3** — "the candidate is correctly
  authorized at concept level but incorrectly bound to concrete roles."
  The theorem `exists(seq) == (pack(seq) != 0)` is true and says
  nothing about the position output. ProposalBinding was exactly the
  missing link.

**Fix (fail-closed, no ProposalBinding required):**

- `RewriteError::UnauthorizedLiveOut { consumer, field, reduction_position }`
- Shared helper `authorized_tuple_consumer()` (sir_rewrite helpers):
  finds both TupleExtract and FieldAccess consumers of the loop result
  and refuses unless the consumer reads the accumulator slot.
- Wired into the any/all/parity/popcount recipes. The popcount recipe
  additionally no longer trusts a consumer of a non-accumulator slot.
- PS002/ps001 expectations flipped to abstention (bitscan path was
  already Stub-quarantined; the Any path now refuses).
- Advisor's targeted regression added:
  `ps002_position_mutation_must_not_rewrite` — a variant with identical
  Any truth but different position semantics (sentinel 0, scan starts
  at 32) must not rewrite either; a rewrite blind to the position
  binding would corrupt both identically.

Residual risk (recorded, not fixed): the wholesale-tuple path
(`wrap_direct_tuple_return`) still fills non-reduction slots with the
loop termination bound under the assumption "index == bound at exit"
— sound for ascending count loops, unproven in general. It now only
fires when the loop tuple has **no** slot consumer. A future binding
pass must either prove the bound claim per shape or refuse.

Test state: 502/502 passing (was 504; PS002/ps001 expectations flipped
to honest abstention). Corpora unchanged: dev 40/50 lowered, D3 7/16,
0 rewrites either side.

## PS002 as canonical example (advisor follow-up, this commit)

**A true theorem applied to the wrong observable boundary is still an
incorrect compiler transformation.** PS002 is the canonical permanent
example:

```text
The theorem was true              exists(seq) == (pack(seq) != 0)
The recognized concept was true   DisjunctiveReduction on `found`
The authorization referred to a real region
The candidate was structurally valid
The rewrite was still wrong       array_find_last returned a constant
```

The theorem described ONE PROJECTION of the loop result (field 0,
`any`), while the rewrite replaced the ENTIRE result tuple and invented
values for the other observable projections (field 1, the returned
position, became the termination-bound constant). Proving equality of
`source_tuple.any` does not license the claim
`source_tuple == candidate_tuple`.

### Two-dimensional assurance (recorded, to implement)

```text
TheoremAssurance:      is the local semantic theorem valid?
ApplicationAssurance:  is that theorem correctly bound to the complete
                       concrete rewrite?

EndToEndAssurance = min(TheoremAssurance, ApplicationAssurance)
```

PS002: TheoremAssurance plausibly SchemaChecked; ApplicationAssurance
FAILED (observable live-outs not covered); EndToEndAssurance FAILED.
Even a MachineChecked local theorem must not automatically produce a
MachineChecked rewrite — the checker must issue assurance for the
COMPLETE artifact (theorem + source role binding + candidate role
binding + complete live-out map + frame conditions + identities).

### Wholesale tuple reconstruction: QUARANTINED

"No recognized slot consumer" is not equivalent to "no observable
consumer exists" — the tuple may escape whole, be copied, stored, or
pass through an unrecognized projection. Implemented this commit:

- `authorized_tuple_consumer()` is now a COMPLETE use classification:
  every use of the loop result must be a recognized slot extract
  reading the accumulator position; any unknown consumer form, any
  uncovered slot, any additional consumer, or no consumer at all →
  `UnknownConsumer`/`UnauthorizedLiveOut` → abstain.
- `wrap_direct_tuple_return()` refuses multi-element tuples outright —
  filling non-reduction slots with the termination bound was an
  unproven exit-index assumption (sound only for specific ascending
  zero-trip-checked shapes, never proven).
- Single-value (non-tuple) loop results remain enabled: every use
  observes the whole value, which IS the theorem's subject.
- Enabled tuple case: exactly ONE consumer, reading the accumulator
  slot. Everything else abstains until ProposalBinding provides the
  complete live-out map.

Enabled-case audit of the four SchemaChecked recipe integrations
(popcount/all/any/parity × their recipes): all four share
`authorized_tuple_consumer` and inherit the complete-use rule. Frame
conditions (unchanged effects/memory — all four regions are
READ_MEMORY-only pure loops) hold by recognizer gating. The exit-index
assumption is no longer reachable (helper refuses). Residual: the
per-recipe audit questions (PHI flows, copies, stores — no such
NodeKinds are consumed by the recipes today) are recorded here;
ProposalBinding must make the enumeration complete rather than
pattern-based.

### PS002 regression family (liveout_binding_tests.rs)

```text
whole tuple return ............... abstain
multiple consumers (slots 0+1) ... abstain
unrecognized projection form ..... abstain
single accumulator-slot extract .. MAY rewrite
```

Plus `ps002_position_mutation_must_not_rewrite` (position semantics
mutated, Any truth unchanged → abstain). Test state: 506/506.
Corpora unchanged (dev 40/50, D3 7/16, 0 rewrites).

## Checker-issued assurance (advisor item 3 — implemented)

The authority model is now enforced in code:

```text
Definition            declares a CAP on assurance (was: "declares status")
Theorem checker       ISSUES assurance = min(declared cap, backend capability)
Backend               declares capability (what its method can establish)
Policy                gates on the ISSUED level, never the declaration
```

- `TransformationDefinition::verification_status()` is documented as a
  **cap**: raising it cannot raise the issued level.
- `VerificationBackend::max_assurance()`: Symbolic → SchemaChecked
  (handwritten algebraic rules); Exhaustive → ConcreteSolverChecked
  (complete enumeration of the declared finite domain — the backend
  refuses oversized domains, so Proven implies complete coverage of
  the declared domain; replay = re-enumerate with recorded limits).
  MachineChecked is reserved for a trusted-prover replay artifact;
  nothing issues it yet.
- `Verifier::verify` stamps `proof.assurance` (issued) and
  `proof.obligation_digest` (binds artifact to exact concrete
  obligation) post-discharge, then gates policy on the ISSUED level.
- `Verifier::with_registry` allows testing with adversarial
  definitions — used to prove a self-certifying definition (declares
  MachineChecked, backed by a tautology) is issued at SchemaChecked
  and fails a strict (ConcreteSolverChecked) policy.

Tests: `definition_cannot_self_certify_machine_checked`,
`self_certified_machine_checked_fails_strict_policy`,
`issued_assurance_from_symbolic_backend_is_schema_checked`.

Residual (honest): the `assurance` field is publicly writable by
construction sites within the workspace; authority is enforced by the
single issuance point in `Verifier::verify` (any proof returned to the
pipeline has been stamped by the checker). True unforgeability
(artifact constructors not exported) arrives with the two-artifact
model below.

## Next: two-artifact end-to-end model (advisor sequence, pending)

```text
CheckedTheorem      theorem artifact (proof.assurance + obligation_digest
                    are its first implementation)
CheckedApplication  AuthorizationId, function fingerprint, source region,
                    candidate identity, role-map identity, complete
                    live-out map, frame condition, assumptions/guards,
                    checker-issued assurance, result
EndToEndVerificationArtifact
                    issued ONLY when both artifacts match on source
                    function/region, candidate, definition, role map,
                    assumptions; only this artifact authorizes mutation
```

## Recipe integration audit matrix (advisor — honest state)

| Family | Local theorem | Exact accumulator binding | Complete live-outs | Frame condition | End-to-end |
|---|---|---|---|---|---|
| Any      | Schema | Binding (role map) | Use-closure + Dead evidence | Source + candidate frames | Chain closed (Schema/Schema) |
| All      | Schema | Binding (role map) | Use-closure + Dead evidence | Source + candidate frames | Chain OPEN (recipe not wired) |
| Parity   | Schema | Binding (role map) | Use-closure + Dead evidence | Source + candidate frames | Chain OPEN (recipe not wired) |
| Popcount | Schema | Binding (role map) | Use-closure + Dead evidence | Source + candidate frames | Chain OPEN (recipe not wired) |

The Any vertical slice now closes the full chain: the engine derives
the ProposalBinding (canonical binder), AnyRecipe consumes ONLY the
role map, the candidate frame is checked against the conservative
contract (no writes/calls/allocations/loops/loads/div-traps, every
external input a certified live-in), the engine issues
`CheckedApplication`, and `EndToEndVerificationArtifact::new(Proof,
CheckedApplication)` verifies obligation linkage + artifact digest
before mutation. End-to-end assurance for Any is currently
min(Schema, Schema) = Schema: the application checker is structural
(role map + live-outs + frames), not solver-backed. All/Parity/
Popcount still scan via their own recipe paths — the chain is open
until they consume the binding the same way. H3 must not freeze until
at least one transformation completes the full chain (C3 gate
requirement) — Any satisfies that.

## ProposalBinding (first derivation, commit of 2026-07 session)

`sir_semantics::binding` now contains the ProposalBinding model — the
exact binding between one concrete source region, its authorization,
its complete observable interface, and the application frame:

- `ReductionRoleMap` — collection, element access, induction (+ start,
  bound, stride ±1 contract), predicate, accumulator, recurrence,
  identity, reduction position, live-ins, effects, integer semantics.
  This is the ONLY legitimate role scan; downstream stages must
  consume the map, not rescan the graph.
- `LiveOutBinding` — Preserved / Reconstructed { slot } / Dead /
  Guarded. Complete use-closure over the loop node's users: zero uses
  → all outputs Dead; exactly one use that is the whole result flowing
  to Return (single output) or a recognized reduction-slot extraction
  → Reconstructed (+ Dead for every unobserved slot); a non-reduction
  slot projection, multiple uses, or any unrecognized use form →
  `BindingError::UnclassifiedUse` (unknown means NOT dead).
- `FrameCondition` + conservative contract: read-only nonvolatile
  memory, no atomics, no stores, no calls, no unmodeled traps, single
  normal exit, known finite trip count. Derivation REFUSES (fail-
  closed) when the contract fails — incomplete bindings cannot become
  legal candidates.
- Authority cross-check: the accumulator bound into the map must equal
  the authorization's certified accumulator
  (`accumulators_are_reassociable`, which excludes the induction
  counter). A role set disagreeing with the certified accumulator is a
  binding error.

Pipeline fix required by the derivation: `derive_roles` previously
selected the role accumulator with a naive last-match over the loop's
detected reductions — the unit-stride induction counter (itself a
"sum" recurrence) could be bound as the accumulator for loops that
have both. It now skips unit counters, matching
`accumulators_are_reassociable`. 512/512 tests pass; dev corpus
unchanged (40/50 lowered, 0 rewrites).

Tests (`crates/sir_semantics/tests/proposal_binding.rs`):
- `any_accumulator_slot_extract_binds_completely` — the enabled case
  (single accumulator-slot extract) binds completely: role map,
  conservative frame, slot 0 Reconstructed + slot 1 Dead, stable
  digest.
- `any_whole_tuple_return_refuses_binding` — PS002 shape refused.
- `any_index_slot_consumer_refuses_binding` — a slot the theorem does
  not cover is neither preserved nor dead; refused.

Still open end-to-end: the binding is derived but NOT yet consumed by
the recipes (they still scan via `RewriteRegion` accessors) and no
`CheckedApplication`/`EndToEndVerificationArtifact` is issued yet. The
audit matrix stays honest: the four families remain Open end-to-end
until a recipe consumes the binding and an application checker issues
the second artifact.
