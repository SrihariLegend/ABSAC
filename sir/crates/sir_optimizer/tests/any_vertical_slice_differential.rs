//! Differential execution for the narrow Any vertical slice.
//!
//! The source and rewritten SIR functions are both evaluated by this
//! test-only interpreter. This is independent of the theorem checker:
//! it exercises zero/one-element inputs, boundary positions, predicate
//! outcomes, and deterministic pseudo-random patterns.

use std::collections::HashMap;

use sir_builder::Builder;
use sir_nodes::NodeKind;
use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::any_only_registry;
use sir_types::{ConstantData, NodeId, Span, Type};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Bool(bool),
    I32(i32),
    U64(u64),
    Bits(u128),
    BoolArray(Vec<bool>),
    I32Array(Vec<i32>),
    Tuple(Vec<Value>),
}

fn bool_array(length: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length,
    }
}

fn i32_array(length: usize) -> Type {
    Type::Array {
        element: Box::new(Type::i32()),
        length,
    }
}

fn build_any(name: &str, length: usize) -> sir_nodes::Function {
    let mut builder = Builder::new(name, &[("board", bool_array(length))], Type::Bool);
    let board = builder.parameter_index(0).unwrap();
    let index = builder.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let step = builder.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let bound = builder.constant(
        ConstantData::u64(length as u64),
        Type::u64(),
        Span::unknown(),
    );
    let identity = builder.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());
    let element = builder
        .array_access(board, index, Type::Bool, Span::unknown())
        .unwrap();
    let next_any = builder.bool_or(identity, element, Span::unknown()).unwrap();
    let next_index = builder.add(index, step, Span::unknown()).unwrap();
    let condition = builder.lt(index, bound, Span::unknown()).unwrap();
    let loop_node = builder
        .r#loop(
            &[element, next_any, next_index, condition],
            condition,
            &[next_any, next_index],
            &[identity, index],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();
    let result = builder
        .field_access(loop_node, "0", Type::Bool, Span::unknown())
        .unwrap();
    builder.return_value(result, Span::unknown()).unwrap();
    builder.build()
}

fn build_predicate(name: &str, length: usize, op: sir_nodes::CmpOperator) -> sir_nodes::Function {
    let mut builder = Builder::new(
        name,
        &[("values", i32_array(length)), ("scalar", Type::i32())],
        Type::Bool,
    );
    let values = builder.parameter_index(0).unwrap();
    let scalar = builder.parameter_index(1).unwrap();
    let index = builder.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let step = builder.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let bound = builder.constant(
        ConstantData::u64(length as u64),
        Type::u64(),
        Span::unknown(),
    );
    let identity = builder.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());
    let element = builder
        .array_access(values, index, Type::i32(), Span::unknown())
        .unwrap();
    let predicate = match op {
        sir_nodes::CmpOperator::Eq => builder.eq(element, scalar, Span::unknown()),
        sir_nodes::CmpOperator::Ne => builder.ne(element, scalar, Span::unknown()),
        sir_nodes::CmpOperator::Lt => builder.lt(element, scalar, Span::unknown()),
        sir_nodes::CmpOperator::Le => builder.le(element, scalar, Span::unknown()),
        sir_nodes::CmpOperator::Gt => builder.gt(element, scalar, Span::unknown()),
        sir_nodes::CmpOperator::Ge => builder.ge(element, scalar, Span::unknown()),
    }
    .unwrap();
    let next_any = builder
        .bool_or(identity, predicate, Span::unknown())
        .unwrap();
    let next_index = builder.add(index, step, Span::unknown()).unwrap();
    let condition = builder.lt(index, bound, Span::unknown()).unwrap();
    let loop_node = builder
        .r#loop(
            &[element, predicate, next_any, next_index, condition],
            condition,
            &[next_any, next_index],
            &[identity, index],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();
    let result = builder
        .field_access(loop_node, "0", Type::Bool, Span::unknown())
        .unwrap();
    builder.return_value(result, Span::unknown()).unwrap();
    builder.build()
}

fn as_bool(value: Value) -> bool {
    match value {
        Value::Bool(value) => value,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn as_i32(value: Value) -> i32 {
    match value {
        Value::I32(value) => value,
        other => panic!("expected i32, got {other:?}"),
    }
}

fn as_u64(value: Value) -> u64 {
    match value {
        Value::U64(value) => value,
        other => panic!("expected u64, got {other:?}"),
    }
}

fn as_bits(value: Value) -> u128 {
    match value {
        Value::Bits(value) => value,
        Value::U64(value) => value as u128,
        other => panic!("expected bitvector/integer, got {other:?}"),
    }
}

fn compare_i32(lhs: i32, rhs: i32, op: sir_nodes::CmpOperator) -> bool {
    match op {
        sir_nodes::CmpOperator::Eq => lhs == rhs,
        sir_nodes::CmpOperator::Ne => lhs != rhs,
        sir_nodes::CmpOperator::Lt => lhs < rhs,
        sir_nodes::CmpOperator::Le => lhs <= rhs,
        sir_nodes::CmpOperator::Gt => lhs > rhs,
        sir_nodes::CmpOperator::Ge => lhs >= rhs,
    }
}

fn eval_node(
    function: &sir_nodes::Function,
    id: NodeId,
    args: &[Value],
    overrides: &HashMap<NodeId, Value>,
) -> Value {
    if let Some(value) = overrides.get(&id) {
        return value.clone();
    }
    let node = function
        .get_node(id)
        .unwrap_or_else(|| panic!("missing node {id}"));
    match &node.kind {
        NodeKind::Parameter { index } => args[*index].clone(),
        NodeKind::Constant(ConstantData::Bool(value)) => Value::Bool(*value),
        NodeKind::Constant(ConstantData::Integer { value, signed, .. }) if *signed => {
            Value::I32(value.parse().unwrap())
        }
        NodeKind::Constant(ConstantData::Integer { value, .. }) => {
            Value::U64(value.parse().unwrap())
        }
        NodeKind::Add { lhs, rhs } => Value::U64(
            as_u64(eval_node(function, *lhs, args, overrides))
                .wrapping_add(as_u64(eval_node(function, *rhs, args, overrides))),
        ),
        NodeKind::Sub { lhs, rhs } => Value::U64(
            as_u64(eval_node(function, *lhs, args, overrides))
                .wrapping_sub(as_u64(eval_node(function, *rhs, args, overrides))),
        ),
        NodeKind::Lt { lhs, rhs }
        | NodeKind::Le { lhs, rhs }
        | NodeKind::Gt { lhs, rhs }
        | NodeKind::Ge { lhs, rhs }
        | NodeKind::Eq { lhs, rhs }
        | NodeKind::Ne { lhs, rhs } => {
            let left = eval_node(function, *lhs, args, overrides);
            let right = eval_node(function, *rhs, args, overrides);
            let op = match &node.kind {
                NodeKind::Lt { .. } => sir_nodes::CmpOperator::Lt,
                NodeKind::Le { .. } => sir_nodes::CmpOperator::Le,
                NodeKind::Gt { .. } => sir_nodes::CmpOperator::Gt,
                NodeKind::Ge { .. } => sir_nodes::CmpOperator::Ge,
                NodeKind::Eq { .. } => sir_nodes::CmpOperator::Eq,
                NodeKind::Ne { .. } => sir_nodes::CmpOperator::Ne,
                _ => unreachable!(),
            };
            let result = match (left, right) {
                (Value::I32(left), Value::I32(right)) => compare_i32(left, right, op),
                (Value::U64(left), Value::U64(right)) => match op {
                    sir_nodes::CmpOperator::Eq => left == right,
                    sir_nodes::CmpOperator::Ne => left != right,
                    sir_nodes::CmpOperator::Lt => left < right,
                    sir_nodes::CmpOperator::Le => left <= right,
                    sir_nodes::CmpOperator::Gt => left > right,
                    sir_nodes::CmpOperator::Ge => left >= right,
                },
                (left, right) => {
                    let left = as_bits(left);
                    let right = as_bits(right);
                    match op {
                        sir_nodes::CmpOperator::Eq => left == right,
                        sir_nodes::CmpOperator::Ne => left != right,
                        sir_nodes::CmpOperator::Lt => left < right,
                        sir_nodes::CmpOperator::Le => left <= right,
                        sir_nodes::CmpOperator::Gt => left > right,
                        sir_nodes::CmpOperator::Ge => left >= right,
                    }
                }
            };
            Value::Bool(result)
        }
        NodeKind::BoolOr { lhs, rhs } => Value::Bool(
            as_bool(eval_node(function, *lhs, args, overrides))
                || as_bool(eval_node(function, *rhs, args, overrides)),
        ),
        NodeKind::ArrayAccess { base, index } => {
            let index = as_u64(eval_node(function, *index, args, overrides)) as usize;
            match eval_node(function, *base, args, overrides) {
                Value::BoolArray(array) => Value::Bool(array[index]),
                Value::I32Array(array) => Value::I32(array[index]),
                other => panic!("expected array base, got {other:?}"),
            }
        }
        NodeKind::FieldAccess { base, field } => {
            let tuple = match eval_node(function, *base, args, overrides) {
                Value::Tuple(tuple) => tuple,
                other => panic!("expected tuple base, got {other:?}"),
            };
            tuple[field.parse::<usize>().unwrap()].clone()
        }
        NodeKind::TupleExtract { tuple, index } => {
            let tuple = match eval_node(function, *tuple, args, overrides) {
                Value::Tuple(tuple) => tuple,
                other => panic!("expected tuple base, got {other:?}"),
            };
            tuple[*index].clone()
        }
        NodeKind::Pack { array } => {
            let array = match eval_node(function, *array, args, overrides) {
                Value::BoolArray(array) => array,
                other => panic!("expected bool array for pack, got {other:?}"),
            };
            let mut bits = 0u128;
            for (index, bit) in array.into_iter().enumerate() {
                if bit {
                    bits |= 1u128 << index;
                }
            }
            Value::Bits(bits)
        }
        NodeKind::ArrayCmpMask { array, scalar, op } => {
            let values = match eval_node(function, *array, args, overrides) {
                Value::I32Array(values) => values,
                other => panic!("expected i32 array for predicate mask, got {other:?}"),
            };
            let scalar = as_i32(eval_node(function, *scalar, args, overrides));
            let mut bits = 0u128;
            for (index, value) in values.into_iter().enumerate() {
                if compare_i32(value, scalar, *op) {
                    bits |= 1u128 << index;
                }
            }
            Value::Bits(bits)
        }
        NodeKind::Loop {
            termination,
            outputs,
            carried_inputs,
            ..
        } => {
            let mut carried: Vec<Value> = carried_inputs
                .iter()
                .map(|input| eval_node(function, *input, args, overrides))
                .collect();
            loop {
                let mut iteration = HashMap::new();
                for (input, value) in carried_inputs.iter().zip(carried.iter()) {
                    iteration.insert(*input, value.clone());
                }
                if !as_bool(eval_node(function, *termination, args, &iteration)) {
                    break;
                }
                carried = outputs
                    .iter()
                    .map(|output| eval_node(function, *output, args, &iteration))
                    .collect();
            }
            Value::Tuple(carried)
        }
        NodeKind::Return { value } => eval_node(function, *value, args, overrides),
        other => panic!("unsupported differential node: {other:?}"),
    }
}

fn eval_function(function: &sir_nodes::Function, args: &[Value]) -> bool {
    let return_node = function.return_node.expect("fixture must have a return");
    as_bool(eval_node(function, return_node, args, &HashMap::new()))
}

fn bool_patterns(length: usize) -> Vec<Vec<bool>> {
    let mut cases = vec![vec![false; length], vec![true; length]];
    if length > 0 {
        let mut first = vec![false; length];
        first[0] = true;
        cases.push(first);
        let mut last = vec![false; length];
        last[length - 1] = true;
        cases.push(last);
    }
    for position in [1usize, 31, 32, 63, 64, 127] {
        if position < length {
            let mut case = vec![false; length];
            case[position] = true;
            cases.push(case);
        }
    }
    cases.push((0..length).map(|index| index % 2 == 0).collect());
    for mut state in [
        0x9e3779b97f4a7c15u64,
        0x243f6a8885a308d3,
        0xb7e151628aed2a6b,
        0xdeadbeefcafebabe,
    ] {
        cases.push(
            (0..length)
                .map(|_| {
                    state ^= state << 7;
                    state ^= state >> 9;
                    state ^= state << 8;
                    state & 1 == 1
                })
                .collect(),
        );
    }
    cases
}

fn predicate_patterns(length: usize, scalar: i32) -> Vec<Vec<i32>> {
    let mut cases = vec![
        vec![scalar; length],
        vec![scalar - 1; length],
        vec![scalar + 1; length],
    ];
    if length > 0 {
        let mut first = vec![scalar - 1; length];
        first[0] = scalar + 1;
        cases.push(first);
        let mut last = vec![scalar - 1; length];
        last[length - 1] = scalar + 1;
        cases.push(last);
    }
    for position in [1usize, 31, 32, 63, 64, 127] {
        if position < length {
            let mut case = vec![scalar - 1; length];
            case[position] = scalar + 1;
            cases.push(case);
        }
    }
    for mut state in [
        0x1234_5678_9abc_def0u64,
        0x0f0f_f0f0_55aa_aa55,
        0x3141_5926_5358_9793,
        0xc001_d00d_feed_face,
    ] {
        cases.push(
            (0..length)
                .map(|_| {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (state >> 32) as i32 % 17 - 8
                })
                .collect(),
        );
    }
    cases
}

#[test]
fn rewritten_boolean_any_matches_source_at_all_sir_boundaries() {
    for length in [0usize, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128] {
        let source = build_any(&format!("any_diff_{length}"), length);
        let optimizer = Optimizer::new(OptimizerConfig::default(), any_only_registry());
        let result = optimizer.optimize(&source);
        assert_eq!(
            result.rewrites_applied, 1,
            "Any must rewrite at length {length}; records: {:?}",
            result.iterations_detail
        );
        for input in bool_patterns(length) {
            let args = [Value::BoolArray(input.clone())];
            let expected = eval_function(&source, &args);
            let actual = eval_function(&result.function, &args);
            assert_eq!(
                actual, expected,
                "SIR differential mismatch at length {length}, input {input:?}"
            );
        }
    }
}

#[test]
fn rewritten_predicate_any_matches_all_comparison_outcomes() {
    let scalar = 3;
    for op in [
        sir_nodes::CmpOperator::Eq,
        sir_nodes::CmpOperator::Ne,
        sir_nodes::CmpOperator::Lt,
        sir_nodes::CmpOperator::Le,
        sir_nodes::CmpOperator::Gt,
        sir_nodes::CmpOperator::Ge,
    ] {
        for length in [0usize, 1, 31, 32, 64, 65, 128] {
            let source = build_predicate(&format!("predicate_any_{op:?}_{length}"), length, op);
            let optimizer = Optimizer::new(OptimizerConfig::default(), any_only_registry());
            let result = optimizer.optimize(&source);
            assert_eq!(
                result.rewrites_applied, 1,
                "predicate Any must rewrite for {op:?} at length {length}; records: {:?}",
                result.iterations_detail
            );
            for values in predicate_patterns(length, scalar) {
                let args = [Value::I32Array(values.clone()), Value::I32(scalar)];
                let expected = eval_function(&source, &args);
                let actual = eval_function(&result.function, &args);
                assert_eq!(
                    actual, expected,
                    "predicate differential mismatch for {op:?}, length {length}, values {values:?}"
                );
            }
        }
    }
}
