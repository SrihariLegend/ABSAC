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

- **Gate 4B status: OPEN** — "Generic verifier soundness audit: FAILED
  for the ModuloAnd/DivideShift class" (and 13 further definitions).
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
