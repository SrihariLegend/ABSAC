use sir_builder::Builder;
use sir_types::{ConstantData, Span, Type};

fn u64_type() -> Type {
    Type::u64()
}
fn i32_type() -> Type {
    Type::i32()
}
fn bool_type() -> Type {
    Type::Bool
}
fn bool_array(len: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length: len,
    }
}
fn unknown() -> Span {
    Span::unknown()
}

pub fn build_ps001_first_set_bit() -> sir_nodes::Function {
    let mut b = Builder::new("first_set_bit", &[("board", bool_array(64))], u64_type());
    let board = b.parameter_index(0).unwrap();

    let i_init = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let i_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let limit = b.constant(ConstantData::u64(64), u64_type(), unknown());
    let found_init = b.constant(ConstantData::boolean(false), bool_type(), unknown());
    let index_init = b.constant(ConstantData::u64(64), u64_type(), unknown()); // Sentinel = 64

    // loop body
    let elem = b
        .array_access(board, i_init, bool_type(), unknown())
        .unwrap();
    let new_found = b.bool_or(found_init, elem, unknown()).unwrap();

    let not_found_yet = b.bool_not(found_init, unknown()).unwrap();
    let is_first = b.bool_and(elem, not_found_yet, unknown()).unwrap();
    let new_index = b.select(is_first, i_init, index_init, unknown()).unwrap();

    let i_next = b.add(i_init, i_step, unknown()).unwrap();

    // cond: !found && i < limit
    let not_found = b.bool_not(found_init, unknown()).unwrap();
    let in_bounds = b.lt(i_init, limit, unknown()).unwrap();
    let cond = b.bool_and(not_found, in_bounds, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[
                elem,
                new_found,
                not_found_yet,
                is_first,
                new_index,
                i_next,
                not_found,
                in_bounds,
                cond,
            ],
            cond,
            &[new_found, new_index, i_next],
            &[found_init, index_init, i_init],
            Type::Tuple {
                elements: vec![bool_type(), u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();

    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

pub fn build_ps002_last_set_bit() -> sir_nodes::Function {
    let mut b = Builder::new("last_set_bit", &[("board", bool_array(64))], u64_type());
    let board = b.parameter_index(0).unwrap();

    let i_init = b.constant(ConstantData::u64(63), u64_type(), unknown()); // start at 63
    let i_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let limit = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let found_init = b.constant(ConstantData::boolean(false), bool_type(), unknown());
    let index_init = b.constant(ConstantData::u64(64), u64_type(), unknown()); // Sentinel = 64

    let elem = b
        .array_access(board, i_init, bool_type(), unknown())
        .unwrap();
    let new_found = b.bool_or(found_init, elem, unknown()).unwrap();

    let not_found_yet = b.bool_not(found_init, unknown()).unwrap();
    let is_first = b.bool_and(elem, not_found_yet, unknown()).unwrap();
    let new_index = b.select(is_first, i_init, index_init, unknown()).unwrap();

    // Reverse iteration: i_next = i_init - 1
    let i_next = b.sub(i_init, i_step, unknown()).unwrap();

    // cond: !found && i >= limit
    let not_found = b.bool_not(found_init, unknown()).unwrap();
    let in_bounds = b.ge(i_init, limit, unknown()).unwrap();
    let cond = b.bool_and(not_found, in_bounds, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[
                elem,
                new_found,
                not_found_yet,
                is_first,
                new_index,
                i_next,
                not_found,
                in_bounds,
                cond,
            ],
            cond,
            &[new_found, new_index, i_next],
            &[found_init, index_init, i_init],
            Type::Tuple {
                elements: vec![bool_type(), u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();

    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

pub fn build_ps003_trailing_zero_count() -> sir_nodes::Function {
    let mut b = Builder::new("trailing_zero_count", &[("value", u64_type())], u64_type());
    let value = b.parameter_index(0).unwrap();

    let n_init = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let x_init = value;

    let n_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let x_step = b.constant(ConstantData::u64(1), u64_type(), unknown());

    // while (x & 1) == 0 { x >>= 1; n += 1; }

    let one = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let zero = b.constant(ConstantData::u64(0), u64_type(), unknown());

    let bit = b.bit_and(x_init, one, unknown()).unwrap();
    let cond = b.eq(bit, zero, unknown()).unwrap();

    let x_next = b.shr(x_init, x_step, unknown()).unwrap();
    let n_next = b.add(n_init, n_step, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[bit, cond, x_next, n_next],
            cond,
            &[x_next, n_next],
            &[x_init, n_init],
            Type::Tuple {
                elements: vec![u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();

    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

pub fn build_ps004_leading_zero_count() -> sir_nodes::Function {
    let mut b = Builder::new("leading_zero_count", &[("value", u64_type())], u64_type());
    let value = b.parameter_index(0).unwrap();

    let n_init = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let mask_init = b.constant(ConstantData::u64(1 << 63), u64_type(), unknown());

    let n_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let mask_step = b.constant(ConstantData::u64(1), u64_type(), unknown());

    let zero = b.constant(ConstantData::u64(0), u64_type(), unknown());

    // while (value & mask) == 0 { mask >>= 1; n += 1; }

    let bit = b.bit_and(value, mask_init, unknown()).unwrap();
    let cond = b.eq(bit, zero, unknown()).unwrap();

    let mask_next = b.shr(mask_init, mask_step, unknown()).unwrap();
    let n_next = b.add(n_init, n_step, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[bit, cond, mask_next, n_next],
            cond,
            &[mask_next, n_next],
            &[mask_init, n_init],
            Type::Tuple {
                elements: vec![u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();

    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::default_registry;

#[test]
fn ps001_first_set_bit_optimizer() {
    // ADVISOR P0 VERIFIER QUARANTINE: the BitscanForwardDefinition
    // obligation is a variable-placeholder template that never binds the
    // actual source/candidate operands (Stub) — it may not authorize a
    // rewrite. The loop->tz rewrite returns when the definition is
    // upgraded to ConcreteSolverChecked with a node-bound obligation.
    let func = build_ps001_first_set_bit();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    // The Any-reduction candidate over the same loop is SchemaChecked
    // and still proves/rewrites; the hard quarantine invariant is that
    // the bitscan path must not produce TrailingZeros.
    let has_tz = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, sir_nodes::NodeKind::TrailingZeros { .. }));
    assert!(!has_tz, "BitscanForward is Stub-quarantined: must not rewrite to TrailingZeros");
}

#[test]
fn ps002_last_set_bit_optimizer() {
    let func = build_ps002_last_set_bit();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(result.rewrites_applied, 1);
}

#[test]
fn ps003_trailing_zero_count_optimizer() {
    // TrailingZeroCountDefinition obligation is a tautology —
    // Stub-quarantined (advisor P0 verifier audit).
    let func = build_ps003_trailing_zero_count();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(result.rewrites_applied, 0, "TrailingZeroCount is Stub-quarantined");
}

#[test]
fn ps004_leading_zero_count_optimizer() {
    // LeadingZeroCountDefinition obligation is a tautology —
    // Stub-quarantined (advisor P0 verifier audit).
    let func = build_ps004_leading_zero_count();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(result.rewrites_applied, 0, "LeadingZeroCount is Stub-quarantined");
}

/// The PS001 `first_set_bit` termination-condition form (as built by the
/// `sir_benchmarks` PS001 spec):
///
/// ```text
/// is_true = arr[i]
/// not_found = !is_true
/// next_i = i + 1
/// cond = !is_true && (next_i < 64)
/// loop output = next_i
/// ```
pub fn build_ps001_termination_form() -> sir_nodes::Function {
    let mut b = Builder::new(
        "first_set_bit",
        &[("board", bool_array(64))],
        Type::Tuple {
            elements: vec![u64_type()],
        },
    );
    let board = b.parameter_index(0).unwrap();
    let one = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let sixty_four = b.constant(ConstantData::u64(64), u64_type(), unknown());
    let i_init = b.constant(ConstantData::u64(0), u64_type(), unknown());

    let is_true = b.array_access(board, i_init, bool_type(), unknown()).unwrap();
    let not_found = b.bool_not(is_true, unknown()).unwrap();
    let next_i = b.add(i_init, one, unknown()).unwrap();
    let bounds_check = b.lt(next_i, sixty_four, unknown()).unwrap();
    let cond = b.bool_and(not_found, bounds_check, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[is_true, not_found, next_i, bounds_check, cond],
            cond,
            &[next_i],
            &[i_init],
            Type::Tuple {
                elements: vec![u64_type()],
            },
            unknown(),
        )
        .unwrap();

    b.return_value(loop_node, unknown()).unwrap();
    b.build()
}

#[test]
fn ps001_termination_form_optimizer() {
    // The PS001 termination-condition form used to be recognized as
    // Stub-quarantined (advisor P0 verifier audit) — the rewrite must
    // NOT fire until the obligation binds actual nodes and is discharged
    // concretely. No TrailingZeros may appear in the result.
    let func = build_ps001_termination_form();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(result.rewrites_applied, 0, "BitscanForward is Stub-quarantined");

    let has_tz = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, sir_nodes::NodeKind::TrailingZeros { .. }));
    let has_loop = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }));
    assert!(!has_tz, "TrailingZeros must NOT appear (bitscan Stub-quarantined)");
    assert!(has_loop, "the search loop must survive (no authorized rewrite)");
}
