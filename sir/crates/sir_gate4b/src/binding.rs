//! Requirement 1 — mechanical binding of the proof to the SIR region.
//!
//! The proof is about a C artifact; this module establishes that the
//! artifact is exactly what the *actual ABSAC pipeline* emits for a
//! named function in the committed LLVM IR, and that the theorem's
//! operands are the pipeline's own role bindings:
//!
//! ```text
//!   corpus/kernels.ll  --lower-->  SIR function
//!        --analysis+semantics-->  semantic truths
//!        --plan derivation-->     VectorPlan { buffer, length, predicate }
//!        --emitter-->             C text
//! ```
//!
//! Checks performed (all fail-closed):
//! - the emitted text equals the committed `gate4/*.c` byte for byte
//!   (after removing the includes the CLI binary prints);
//! - the plan's operation matches the recognized kernel kind;
//! - the plan's buffer/length/predicate identifiers equal the roles the
//!   C template matcher captured;
//! - those identifiers are the emitter's names for SIR parameters 0..n.

use std::path::Path;

use sir_analysis::manager::AnalysisManager;
use sir_benchmarks::vector_derive::{derive_vector_plan, param_name};
use sir_benchmarks::vector_emit::emit_vectorized;
use sir_benchmarks::vector_plan::{VectorOp, VectorPredicate};
use sir_lower::lower_function;
use sir_semantics::semantics::SemanticEngine;
use sir_types::NodeId;

use crate::emitted::{EmittedKernel, KernelKind};

/// The result of binding a recognized C artifact to its SIR region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SirBinding {
    pub ll_path: String,
    pub ll_digest: u64,
    pub function: String,
    pub plan_operation: String,
    pub plan_predicate: String,
    /// (role, C identifier) as derived by the pipeline.
    pub plan_roles: Vec<(String, String)>,
    /// SIR parameters in order: (index, emitter name, node name).
    pub sir_params: Vec<(usize, String, String)>,
    /// Checks, in order, with their outcome.
    pub checks: Vec<(String, bool)>,
}

impl SirBinding {
    pub fn all_ok(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|(_, ok)| *ok)
    }
}

fn strip_includes(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("#include"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Run the real pipeline over `ll_text` for `kernel.function` and bind
/// the result to the committed artifact at `generated_text`.
pub fn bind(
    kernel: &EmittedKernel,
    generated_text: &str,
    ll_path: &Path,
    ll_text: &str,
) -> Result<SirBinding, String> {
    let ll_digest = crate::emitted::fnv1a64(ll_text.as_bytes());
    let func = lower_function(ll_text, &kernel.function)
        .map_err(|e| format!("lowering {} failed: {e:?}", kernel.function))?;
    let mut mgr = AnalysisManager::new();
    mgr.run_all(&func);
    let mut engine = SemanticEngine::new();
    engine.derive(&func, mgr.database());
    let truths: Vec<_> = engine.database().truths().cloned().collect();
    let plan = derive_vector_plan(&func, &truths)
        .ok_or_else(|| {
            let concepts: Vec<String> = truths
                .iter()
                .map(|t| format!("{:?}", t.concept))
                .collect();
            format!(
                "pipeline derived no vector plan for {} (truths: {concepts:?})",
                kernel.function
            )
        })?;

    let emitted = emit_vectorized(&func, &plan);
    let mut checks: Vec<(String, bool)> = Vec::new();
    checks.push((
        "emitted C equals the committed artifact".to_string(),
        strip_includes(&emitted) == strip_includes(generated_text),
    ));

    // Operation ↔ kernel kind.
    let op_matches = match (&plan.operation, &kernel.kind) {
        (VectorOp::Cardinality, KernelKind::CardinalityMasked { .. }) => true,
        (VectorOp::All, KernelKind::AllEquality { .. }) => true,
        (VectorOp::Sum, KernelKind::SumAscii { .. }) => true,
        _ => false,
    };
    checks.push(("plan operation matches kernel kind".to_string(), op_matches));

    // Roles.
    let mut plan_roles: Vec<(String, String)> = Vec::new();
    plan_roles.push(("buffer".to_string(), plan.buffer_name.clone()));
    plan_roles.push(("length".to_string(), plan.length_name.clone()));
    let mut role_matches = true;
    match &kernel.kind {
        KernelKind::CardinalityMasked {
            buf,
            n,
            mask,
            target,
        } => {
            role_matches &= plan.buffer_name == *buf && plan.length_name == *n;
            match &plan.predicate {
                Some(VectorPredicate::MaskedEqual { mask: m, target: t }) => {
                    plan_roles.push(("mask".to_string(), m.clone()));
                    plan_roles.push(("target".to_string(), t.clone()));
                    role_matches &= m == mask && t == target;
                }
                _ => role_matches = false,
            }
        }
        KernelKind::AllEquality { buf, n, val } => {
            role_matches &= plan.buffer_name == *buf && plan.length_name == *n;
            match &plan.predicate {
                Some(VectorPredicate::Equal(v)) => {
                    plan_roles.push(("value".to_string(), v.clone()));
                    role_matches &= v == val;
                }
                _ => role_matches = false,
            }
        }
        KernelKind::SumAscii { buf, n } => {
            role_matches &= plan.buffer_name == *buf && plan.length_name == *n;
        }
    }
    checks.push((
        "plan role identifiers equal the C template roles".to_string(),
        role_matches,
    ));

    // SIR parameters: the emitter's name for parameter k must be the
    // plan's identifier for that role.
    let mut sir_params = Vec::new();
    for (idx, p) in func.params.iter().enumerate() {
        sir_params.push((idx, param_name(&func, NodeId::new(idx as u64)), p.name.clone()));
    }
    let params_ok = plan_roles.iter().all(|(role, ident)| match role.as_str() {
        "buffer" => sir_params
            .first()
            .map(|(_, n, _)| n == ident)
            .unwrap_or(false),
        "length" => sir_params
            .get(1)
            .map(|(_, n, _)| n == ident)
            .unwrap_or(false),
        "mask" | "value" => sir_params
            .get(2)
            .map(|(_, n, _)| n == ident)
            .unwrap_or(false),
        "target" => sir_params
            .get(3)
            .map(|(_, n, _)| n == ident)
            .unwrap_or(false),
        _ => false,
    });
    checks.push((
        "role identifiers are the SIR parameter bindings".to_string(),
        params_ok,
    ));

    Ok(SirBinding {
        ll_path: ll_path.display().to_string(),
        ll_digest,
        function: kernel.function.clone(),
        plan_operation: format!("{:?}", plan.operation),
        plan_predicate: format!("{:?}", plan.predicate),
        plan_roles,
        sir_params,
        checks,
    })
}
