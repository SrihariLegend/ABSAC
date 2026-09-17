//! P0A binding consumption: All / Parity / Popcount must rewrite through
//! the authorized ProposalBinding — not by re-deriving roles — and the
//! predicate mask must use the binding's TRUE comparison operator.
//!
//! Each kernel is a predicate-collection loop over `Array<u8, 64>` with a
//! single accumulator-slot consumer (the shape the observable classifier
//! marks `Reconstructed`), so the rewrite is expected to fire.

use sir_builder::Builder;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

enum Concept {
    All,
    Parity,
    Popcount,
}

/// `acc`-recurrence over `key_pred = vals[i] == key`:
///   All      → acc = acc && pred   (Bool)
///   Parity   → acc = acc != pred   (Bool)
///   Popcount → acc = acc + (pred ? 1 : 0)  (i32)
fn build_predicate_reduction(concept: Concept) -> sir_nodes::Function {
    let (name, acc_init, acc_ty, fields) = match concept {
        Concept::All => ("all_eq", ConstantData::Bool(true), Type::Bool, 0usize),
        Concept::Parity => ("parity_eq", ConstantData::Bool(false), Type::Bool, 0usize),
        Concept::Popcount => ("popcount_eq", ConstantData::i32(0), Type::i32(), 0usize),
    };
    let _ = fields;
    let return_ty = acc_ty.clone();
    let mut b = Builder::new(
        name,
        &[
            (
                "vals",
                Type::Array {
                    element: Box::new(Type::u8()),
                    length: 64,
                },
            ),
            ("key", Type::u8()),
        ],
        return_ty.clone(),
    );
    let vals = b.parameter_index(0).unwrap();
    let key = b.parameter_index(1).unwrap();
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let init = b.constant(acc_init, acc_ty.clone(), Span::unknown());

    let elem = b
        .array_access(vals, i_init, Type::u8(), Span::unknown())
        .unwrap();
    let pred = b.eq(elem, key, Span::unknown()).unwrap();
    let next_acc = match concept {
        Concept::All => b.bool_and(init, pred, Span::unknown()).unwrap(),
        Concept::Parity => b.ne(init, pred, Span::unknown()).unwrap(),
        Concept::Popcount => {
            let zero = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
            let one = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());
            let inc = b.select(pred, one, zero, Span::unknown()).unwrap();
            b.add(init, inc, Span::unknown()).unwrap()
        }
    };
    let i_next = b.add(i_init, i_step, Span::unknown()).unwrap();
    let cond = b.lt(i_init, limit, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, pred, next_acc, i_next, cond],
            cond,
            &[next_acc, i_next],
            &[init, i_init],
            Type::Tuple {
                elements: vec![acc_ty.clone(), Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    // Single accumulator-slot consumer: the observable classifier marks
    // slot 0 Reconstructed; the index slot is unobserved.
    let acc_out = b
        .field_access(loop_node, "0", acc_ty, Span::unknown())
        .unwrap();
    b.return_value(acc_out, Span::unknown()).unwrap();
    b.build()
}

fn rewrite(concept: Concept) -> sir_optimizer::OptimizationResult {
    let func = build_predicate_reduction(concept);
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    optimizer.optimize(&func)
}

#[test]
fn all_predicate_collection_rewrites_through_the_binding() {
    let result = rewrite(Concept::All);
    assert_eq!(
        result.rewrites_applied, 1,
        "All over a predicate collection with a single accumulator slot must rewrite"
    );
}

#[test]
fn parity_predicate_collection_rewrites_through_the_binding() {
    let result = rewrite(Concept::Parity);
    assert_eq!(
        result.rewrites_applied, 1,
        "Parity over a predicate collection with a single accumulator slot must rewrite"
    );
}

#[test]
fn popcount_predicate_collection_rewrites_through_the_binding() {
    let result = rewrite(Concept::Popcount);
    assert_eq!(
        result.rewrites_applied, 1,
        "Popcount over a predicate collection with a single accumulator slot must rewrite"
    );
}

#[test]
fn predicate_mask_uses_the_bindings_true_operator() {
    // The old structural `emit_pack` hardcoded `Gt`; an `==` predicate
    // would have been rewritten as `>`. The binding carries the operator,
    // and the emitted ArrayCmpMask must match it.
    let result = rewrite(Concept::All);
    assert_eq!(result.rewrites_applied, 1);
    let mut mask_ops: Vec<sir_nodes::CmpOperator> = Vec::new();
    for node in result.function.arena.iter() {
        if let sir_nodes::NodeKind::ArrayCmpMask { op, .. } = &node.kind {
            mask_ops.push(*op);
        }
    }
    assert_eq!(
        mask_ops,
        vec![sir_nodes::CmpOperator::Eq],
        "the mask operator must come from the binding (Eq), not a hardcoded Gt"
    );
}
