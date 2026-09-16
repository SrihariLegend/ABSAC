//! Live-out binding regression tests (advisor PS002 follow-up).
//!
//! PS002 canonical lesson: a TRUE theorem applied to the wrong observable
//! boundary is still an incorrect compiler transformation. These tests
//! pin the abstention behavior of the reduction recipes whenever the
//! complete live-out interface is not covered by the theorem:
//!
//!   - whole tuple return .............. abstain (tuple escapes)
//!   - multiple tuple consumers ........ abstain
//!   - unrecognized projection form .... abstain
//!   - single accumulator-slot extract . the ONE enabled tuple case
//!
//! A non-tuple (single-value) loop result is also enabled: every use
//! observes the whole value, which IS the theorem's subject.

use sir_builder::Builder;
use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::{any_only_registry, default_registry};
use sir_types::{ConstantData, Span, Type};

fn bool_array(n: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length: n,
    }
}

fn u64_type() -> Type {
    Type::u64()
}

fn bool_type() -> Type {
    Type::Bool
}

/// Any-reduction loop over a bool[64] array. The loop returns the tuple
/// (any: bool, i: u64). `consumer` builds the downstream use of the loop
/// node and returns the value the function returns.
fn build_any_loop(
    name: &str,
    return_ty: Type,
    consumer: impl FnOnce(&mut Builder, sir_types::NodeId) -> Result<sir_types::NodeId, sir_builder::BuildError>,
) -> Result<sir_nodes::Function, sir_builder::BuildError> {
    let mut b = Builder::new(name, &[("board", bool_array(64))], return_ty);
    let board = b.parameter_index(0).unwrap();
    let i_init = b.constant(ConstantData::u64(0), u64_type(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), u64_type(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), u64_type(), Span::unknown());
    let any_init = b.constant(ConstantData::Bool(false), bool_type(), Span::unknown());

    let elem = b.array_access(board, i_init, bool_type(), Span::unknown()).unwrap();
    let next_any = b.bool_or(any_init, elem, Span::unknown()).unwrap();
    let i_next = b.add(i_init, i_step, Span::unknown()).unwrap();
    let cond = b.lt(i_init, limit, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, next_any, i_next, cond],
            cond,
            &[next_any, i_next],
            &[any_init, i_init],
            Type::Tuple {
                elements: vec![bool_type(), u64_type()],
            },
            Span::unknown(),
        )
        .unwrap();

    let ret = consumer(&mut b, loop_node)?;
    b.return_value(ret, Span::unknown())?;
    Ok(b.build())
}

/// Whole tuple return: the loop's (any, i) tuple IS the function result.
/// The Any theorem covers slot 0 only; the rewrite must abstain —
/// "no recognized slot consumer" is not proof that no observable
/// consumer exists.
#[test]
fn any_whole_tuple_return_must_not_rewrite() {
    let func = build_any_loop(
        "any_whole_tuple",
        Type::Tuple {
            elements: vec![bool_type(), u64_type()],
        },
        |_b, loop_node| Ok(loop_node),
    )
    .unwrap();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "whole-tuple escape must abstain: no recognized slot consumer is not \
         proof that no observable consumer exists"
    );
}

/// Two consumers observe the loop tuple: one reads the accumulator slot
/// (0, covered by the Any theorem), one reads the index slot (1, NOT
/// covered). Replacing one consumer would dangle or corrupt the other.
/// Must abstain.
#[test]
fn multiple_tuple_consumers_must_not_rewrite() {
    let func = build_any_loop("any_two_consumers", u64_type(), |b, loop_node| {
        let acc = b.field_access(loop_node, "0", bool_type(), Span::unknown())?;
        let index = b
            .field_access(loop_node, "1", u64_type(), Span::unknown())
            .unwrap();
        // Both live-outs stay observable: return the index when any
        // element is true, else the sentinel 0.
        let zero = b.constant(ConstantData::u64(0), u64_type(), Span::unknown());
        let picked = b.select(acc, index, zero, Span::unknown()).unwrap();
        Ok(picked)
        },
    )
    .unwrap();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "two tuple consumers (slot 0 covered, slot 1 not) must abstain: the \
         theorem covers slot 0 only and the second consumer observes it not"
    );
}

/// Unrecognized projection form: a FieldAccess whose field name is not a
/// tuple slot number ("last") observing the loop tuple. The recipe
/// cannot classify what this consumer observes, so it must abstain —
/// the PS002 corruption began with an incomplete consumer
/// classification.
#[test]
fn unrecognized_projection_form_must_not_rewrite() {
    let func = build_any_loop("any_opaque_projection", u64_type(), |b, loop_node| {
        // Non-numeric field name: not a recognizable tuple slot.
        b.field_access(loop_node, "last", u64_type(), Span::unknown())
    })
    .unwrap();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "unrecognized projection form must abstain: the consumer's observed \
         slot cannot be determined, so the live-out binding is incomplete"
    );
}

/// The ONE enabled tuple case: exactly one consumer, reading the
/// accumulator slot (0). The Any theorem covers exactly this value.
#[test]
fn single_accumulator_slot_extract_may_rewrite() {
    let func = build_any_loop("any_acc_slot_only", bool_type(), |b, loop_node| {
        b.field_access(loop_node, "0", bool_type(), Span::unknown())
    })
    .unwrap();
    // This test exercises the Any theorem itself, so it pins the
    // explicit Any-only registry. (Any is also back in
    // `default_registry` since the H4 closure, docs/H4_RESULTS.md.)
    let optimizer = Optimizer::new(OptimizerConfig::default(), any_only_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 1,
        "single accumulator-slot consumer is fully covered by the Any theorem \
         and may rewrite"
    );
}
