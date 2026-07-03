# 0004 — Graph Invariants

## Overview

The SIR verifier checks seven invariants on every function graph. Verification
runs after every transformation pass and before code generation.

## Invariant 1: SSA Uniqueness

Every `NodeId` in the arena is unique. Enforced by `BTreeMap` key constraint.
Defensively re-checked by the verifier.

## Invariant 2: Reference Validity

Every `NodeId` appearing as an input in any `NodeKind` field must exist in the
arena. No dangling references.

## Invariant 3: Cycle Freedom

The dependency graph must be a DAG. **Exception**: Nodes within a `Loop` body
may reference the loop's termination condition, forming a controlled cycle.

Cycle detection uses three-color DFS:
- White = unvisited
- Gray = in progress (back-edge to gray = cycle)
- Black = finished

## Invariant 4: Type Correctness

Every operation's inputs must match expected types:

| Operation | LHS Type | RHS Type | Result Type |
|-----------|----------|----------|-------------|
| Add/Sub/Mul/Div/Rem | Integer or Float | Same as LHS | Same as operands |
| Neg | Integer or Float | — | Same as operand |
| And/Or/Xor | Integer | Same as LHS | Same as operands |
| Not/Popcount/LeadingZeros/TrailingZeros | Integer | — | Same as operand |
| Shl/Shr/Rol/Ror | Integer | Integer (any) | Same as LHS |
| Eq/Ne/Lt/Le/Gt/Ge | Any | Same as LHS | Bool |
| BoolAnd/BoolOr/BoolNot | Bool | Bool | Bool |
| Select | cond: Bool | true_val, false_val: same type | Same as true_val |
| Load | Pointer or Reference | — | (from pointer) |
| Store | Pointer or Reference | (from pointer) | Unit |
| Allocate | — | count: Integer | Pointer |
| FieldAccess | Struct | — | Field's type |
| ArrayAccess | Array or Slice | index: Integer | Element type |

## Invariant 5: Return Validity

Exactly one `Return` node must exist in the function. Its value type must match
the function's declared `return_ty`.

## Invariant 6: Parameter Validity

`Parameter` node indices must be `0..params.len()`. Each parameter slot must
have exactly one corresponding `Parameter` node.

## Invariant 7: Loop Well-Formedness

- `Loop.termination` must be `Bool`
- All nodes in `body`, `outputs`, and `carried_inputs` must exist
- `carried_inputs.len() == outputs.len()` (one-to-one correspondence)
