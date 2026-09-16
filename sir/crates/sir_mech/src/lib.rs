//! `sir_mech` — the mechanical verification kernel behind Gate 4B.
//!
//! This crate exists to replace prose proofs ("algebraic identity",
//! "induction argument") with machine-checked derivations:
//!
//! - [`bv`] — a fixed-width bitvector term language with a concrete
//!   evaluator (the object language every lemma is stated in).
//! - [`cnf`] — Tseitin bit-blasting of bitvector terms into CNF.
//! - [`sat`] — a deterministic CDCL SAT solver used to discharge
//!   bit-level identities (a counterexample model is returned when a
//!   claimed identity is false).
//! - [`equiv`] — the decision procedure: `lhs == rhs` valid, or a
//!   concrete counterexample.
//!
//! Nothing here trusts a caller. The solver either finds a model that
//! refutes the claim (and the model is re-evaluated against the
//! original terms before it is reported) or proves there is none.

pub mod arith;
pub mod bv;
pub mod cnf;
pub mod equiv;
pub mod kernel;
pub mod sat;
pub mod term;

pub use arith::{linearize, prove_entailment, ArithOutcome, LinForm};
pub use bv::{BinOp, Bv, Term, VarId, MAX_WIDTH};
pub use equiv::{
    prove_contradiction, prove_equal, prove_implied_equal, prove_implication, prove_tautology,
    CheckResult,
};
pub use kernel::{Context, Derivation, Kernel, KernelError, Schema, SchemaId, Statement, Theorem};
pub use sat::{solve_default, SatResult, DEFAULT_CONFLICT_BUDGET};
pub use term::{Arena, BvOp, FuncId, Sort, Symbol, Tid};
