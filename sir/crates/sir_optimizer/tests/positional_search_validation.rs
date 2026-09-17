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
    // BitscanForward is ConcreteSolverChecked since 2026-09-17: the
    // found-flag forward search is recognized as FirstOccurrence,
    // authorized under PositionSearch (X06), proven against
    // FirstTrue(seq) == ctz(Pack(seq)), and rewritten.
    let func = build_ps001_first_set_bit();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    let has_tz = result
        .function
        .arena
        .iter()
        .any(|n| matches!(n.kind, sir_nodes::NodeKind::TrailingZeros { .. }));
    assert!(has_tz, "the bitscan rewrite must select TrailingZeros");
    assert!(result.rewrites_applied > 0);
}

// PS002 END-TO-END AUDIT (advisor): the loop's live-out is the POSITION
// (field 1 of the loop tuple), not the boolean `found` accumulator the
// Any theorem covers. The pre-audit Any rewrite rebuilt the tuple and
// silently changed `array_find_last` into "return a constant" —
// type-valid, structurally verified, semantically destroyed. The
// authorized-consumer guard now refuses (UnauthorizedLiveOut), and the
// bitscan path is Stub-quarantined. The honest result is abstention.
#[test]
fn ps002_last_set_bit_optimizer() {
    let func = build_ps002_last_set_bit();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "the position live-out is not the accumulator slot; rewrite must refuse"
    );
}


/// Advisor's targeted regression: a function whose RETURNED POSITION
/// semantics differ from PS002 while the Any truth (the `found`
/// accumulator) is identical — the sentinel is 0 here and the scan
/// starts at 32. A rewrite that cannot see the position binding would
/// treat both programs identically; it must therefore reject both. If
/// the optimizer rewrites this function, the Any binding is not
/// authorized for the position live-out and the guard has regressed.
#[test]
fn ps002_position_mutation_must_not_rewrite() {
    let mut b = Builder::new("last_set_bit_variant", &[("board", bool_array(64))], u64_type());
    let board = b.parameter_index(0).unwrap();

    let i_init = b.constant(ConstantData::u64(32), u64_type(), unknown()); // start at 32
    let i_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let limit = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let found_init = b.constant(ConstantData::boolean(false), bool_type(), unknown());
    let index_init = b.constant(ConstantData::u64(0), u64_type(), unknown()); // Sentinel = 0

    let elem = b
        .array_access(board, i_init, bool_type(), unknown())
        .unwrap();
    let new_found = b.bool_or(found_init, elem, unknown()).unwrap();

    let not_found_yet = b.bool_not(found_init, unknown()).unwrap();
    let is_first = b.bool_and(elem, not_found_yet, unknown()).unwrap();
    let new_index = b.select(is_first, i_init, index_init, unknown()).unwrap();

    let i_next = b.sub(i_init, i_step, unknown()).unwrap();

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

    // Returns the POSITION (slot 1), not the boolean accumulator —
    // identical live-out structure to PS002.
    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    let func = b.build();

    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "position-mutated variant must not rewrite: the Any theorem covers only \
         the found slot; a rewrite blind to the position live-out would corrupt \
         both this and PS002 identically"
    );
}
