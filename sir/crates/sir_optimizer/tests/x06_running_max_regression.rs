//! X06 regression: the running maximum must never be treated as a
//! position search.
//!
//! Safety (advisor item 9a): the pipeline generates ZERO candidates.
//! Semantic precision (advisor item 9): recognition must not emit a
//! FirstOccurrence truth — the `buf[i] > m ? buf[i] : m` select binds
//! memory-derived VALUES, not a position.

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_types::{ConstantData, Span, Type};

fn u64_type() -> Type {
    Type::u64()
}
fn u8_type() -> Type {
    Type::u8()
}
fn unknown() -> Span {
    Span::unknown()
}

/// SIR shape of x06_running_max:
///
/// ```text
/// m = 0; i = 0
/// loop (carried [m, i]):
///     elem = buf[i]
///     gt   = gt(m, elem)
///     m    = select(gt, elem, m)      // memory-derived select
///     i    = i + 1
/// until i >= n
/// return m
/// ```
pub fn build_running_max() -> sir_nodes::Function {
    let mut b = Builder::new(
        "running_max",
        &[("buf", Type::Slice { element: Box::new(u8_type()) }), ("n", u64_type())],
        u8_type(),
    );
    let buf = b.parameter_index(0).unwrap();
    let n = b.parameter_index(1).unwrap();

    let i_init = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let one = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let m_init = b.constant(ConstantData::u64(0), u8_type(), unknown());

    let elem = b.array_access(buf, i_init, u8_type(), unknown()).unwrap();
    let gt = b.gt(m_init, elem, unknown()).unwrap();
    let m_next = b.select(gt, m_init, elem, unknown()).unwrap();
    let i_next = b.add(i_init, one, unknown()).unwrap();
    let cond = b.lt(i_next, n, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, gt, m_next, i_next, cond],
            cond,
            &[m_next, i_next],
            &[m_init, i_init],
            Type::Tuple { elements: vec![u8_type(), u64_type()] },
            unknown(),
        )
        .unwrap();

    let m_out = b.tuple_extract(loop_node, 0, u8_type(), unknown()).unwrap();
    b.return_value(m_out, unknown()).unwrap();
    b.build()
}

#[test]
fn x06_running_max_is_safe_and_precise() {
    use sir_optimizer::config::OptimizerConfig;
    use sir_optimizer::optimizer::Optimizer;
    use sir_rewrite::registry::default_registry;
    use sir_semantics::concepts::SemanticConcept;

    let func = build_running_max();

    // ── Semantic precision: no FirstOccurrence truth may be emitted ──
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = sir_semantics::semantics::SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    for truth in semantics.database().truths() {
        assert_ne!(
            truth.concept,
            SemanticConcept::FirstOccurrence,
            "running max must not be recognized as FirstOccurrence"
        );
        assert_ne!(
            truth.concept,
            SemanticConcept::LastOccurrence,
            "running max must not be recognized as LastOccurrence"
        );
    }

    // ── Safety: zero candidates (X06 containment) ──
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    let total_candidates: usize = result
        .iterations_detail
        .iter()
        .map(|r| r.candidates_generated)
        .sum();
    assert_eq!(
        total_candidates, 0,
        "running max must generate zero candidates"
    );
    assert_eq!(result.rewrites_applied, 0);
}
