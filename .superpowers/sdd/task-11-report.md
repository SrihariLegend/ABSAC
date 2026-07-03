# Task 11-13 Report: RewriteEngine, PopcountRecipe, Integration Tests

## Status: Complete

## Summary

Implemented the final three tasks of Phase 0013 (Verified Rewriting):

1. **Task 11 (RewriteEngine)** -- Created `sir/crates/sir_rewrite/src/engine.rs`. The `RewriteEngine` orchestrates the full verified rewrite pipeline: ID verification, region assembly from `StructuralDatabase`, recipe lookup and invocation via `RecipeRegistry`, graph surgery via `RewriteBuilder`, structural verification via `sir_verify::Verifier`, and result production (`RewriteResult` with `GraphDiff`).

2. **Task 12 (PopcountRecipe)** -- Created `sir/crates/sir_rewrite/src/recipes/mod.rs` and `sir/crates/sir_rewrite/src/recipes/popcount.rs`. The `PopcountRecipe` implements `RewriteRecipe` for BS001: constructs `pack(board) -> popcount(packed)` as the replacement subgraph. Includes 4 unit tests covering definition ID, name, patch structure, and missing-role error handling.

3. **Task 13 (Integration Tests)** -- Created `sir/crates/sir_rewrite/tests/integration_test.rs`. Five integration tests covering:
   - BS001 end-to-end pipeline execution (Tier 5)
   - Definition mismatch rejection (Tier 6)
   - Structural verification pass-through (Tier 4)
   - Provenance tracking (Tier 9)
   - Missing structural description error (Tier 7)

## Files Created

- `sir/crates/sir_rewrite/src/engine.rs` -- RewriteEngine orchestration (196 lines)
- `sir/crates/sir_rewrite/src/recipes/mod.rs` -- recipes module root
- `sir/crates/sir_rewrite/src/recipes/popcount.rs` -- PopcountRecipe (116 lines)
- `sir/crates/sir_rewrite/tests/integration_test.rs` -- integration tests (179 lines)

## Files Modified

- `sir/crates/sir_rewrite/src/lib.rs` -- added `pub mod engine;` and `pub mod recipes;`

## Test Results

- `cargo test -p sir_rewrite`: 18 tests pass (13 unit + 5 integration)
- `cargo test` (workspace): 365 tests pass, 0 failures, 0 regressions

## Commit

`564965c` -- `feat: add RewriteEngine, PopcountRecipe, and integration tests`

## Post-Merge Fixes (2026-07-03)

After the Phase 0013 final branch review, four correctness bugs were fixed:

### C1: External reference corruption in `rewrite_kind_refs`

**File:** `sir/crates/sir_rewrite/src/builder.rs`

The `resolve` closure in `rewrite_kind_refs` unconditionally mapped every `NodeId` through the `id_map`, including external references (original function NodeIds like the board parameter) that were encoded as `LocalNodeId` placeholders. When a local node happened to share the same numeric ID, the external reference was remapped to the wrong global NodeId, creating self-referencing cycles.

**Fix:** Added `original_ids: &BTreeSet<NodeId>` parameter. The `resolve` closure now checks: if the NodeId exists in the original function's arena, return it unchanged (external reference). Only NodeIds not in the original function are remapped through the `id_map`.

### C2: `collect_role_nodes` removed function parameters

**File:** `sir/crates/sir_rewrite/src/builder.rs`

The obsolete-node collection included the `collection` node (the board parameter, NodeId 0), causing `sir_verify` parameter index mismatch when the parameter node was removed from the arena.

**Fix:** Before removing obsolete nodes, the code now checks if the node is a `NodeKind::Parameter { .. }` and skips it if so, preserving function parameters in the arena.

### I2: `replace_all_uses` incorrectly overwrote `return_node`

**File:** `sir/crates/sir_rewrite/src/builder.rs`

`replace_all_uses` had a block that replaced `function.return_node` from the old result node ID to the new replacement node ID, making `return_node` point to a non-Return node (e.g., a `Popcount` node).

**Fix:** Removed the `return_node` update block entirely. `replace_in_kind` already handles replacing operand references in the `Return { value }` node through the arena-wide iteration.

### I4: Unused `sir_builder` dependency

**File:** `sir/crates/sir_rewrite/Cargo.toml`

`sir_builder` was listed as a `[dependencies]` entry but only used by integration tests.

**Fix:** Moved `sir_builder` from `[dependencies]` to `[dev-dependencies]`.

### M2: `Pack` variant moved to own "Data conversion" section in `NodeKind`

**File:** `sir/crates/sir_nodes/src/node_kind.rs`

The `Pack` variant was incorrectly placed in the "Bitwise (unary)" section alongside `Not`, `Popcount`, `LeadingZeros`, `TrailingZeros`. It is an array-to-bitvector conversion, not a bitwise operation. Moved to a new "Data conversion" section before "Select", with its own arm in `input_nodes()`.

### M5: Integration tests accept both Ok and Err, masking pipeline failures

**File:** `sir/crates/sir_rewrite/tests/integration_test.rs`

Three tests used `if let Ok(result)` patterns that silently accepted `Err` outcomes.

**Fix:**
1. `bs001_end_to_end_rewrite_produces_valid_sir` -- changed from `if let Ok` to an explicit `match` that asserts the error is one of `RewriteError::RecipeFailed`, `RewriteError::MissingRole`, or `RewriteError::StructuralVerificationFailed`.
2. `rewritten_function_passes_sir_verify` -- same explicit match with error-type assertions.
3. `provenance_tracks_recipe_id` -- added an `else if let Err(ref e)` branch that asserts the error is not `RewriteError::InternalInvariantViolation` (which would indicate a compiler bug).

### M6: Add Display impls for RewriteResult types

**File:** `sir/crates/sir_rewrite/src/result.rs`

Added `Display` implementations for `NodeProvenance`, `GraphDiff`, and `EdgeChange` to improve debugging and diff reporting.

### M7: Verify Pack's output type in sir_verify

**File:** `sir/crates/sir_verify/src/verifier.rs`

Added verification that a Pack node's declared type is `BitVector` with a width matching the array length. For `Array(Bool, length)`, the output must be `BitVector { width: length }`. `Slice(Bool)` is accepted without width checking (dynamic length).

### Verification

- `cargo build -p sir_rewrite`: builds with no errors
- `cargo test -p sir_rewrite`: 18 tests pass (13 unit + 5 integration)
- `cargo test -p sir_verify`: 8 tests pass (0 failures)
- `cargo test` (workspace): all tests pass, zero regressions
