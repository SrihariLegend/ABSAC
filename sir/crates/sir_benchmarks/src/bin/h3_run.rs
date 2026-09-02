//! h3_run — H3 held-out evaluation harness for the frozen C3 (Any slice).
//!
//! This is a HARNESS for the frozen commit 49999a4 (tag `c3-freeze-any`).
//! It does not modify any frozen crate. Modes:
//!
//!   h3_run run     — one-shot run over the sealed corpus (tier A fixtures
//!                    from h3/tier_a.tsv, tier B LLVM from h3/tier_b.ll),
//!                    reporting stage boundaries and rewrite outcomes.
//!   h3_run exec    — differential SIR execution of original vs rewritten
//!                    Tier A functions over boundary + deterministic random
//!                    inputs (run BEFORE inspecting any failures).
//!   h3_run s1      — S1 reproduction: identity=true any-reductions.
//!
//! Executed inputs are interpreted with the SIR semantics below, which are
//! implemented from the SIR node types (independent of the recipe internals).

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;

use sir_builder::Builder;
use sir_lower::{list_functions, lower_function};
use sir_nodes::{CmpOperator, NodeKind};
use sir_optimizer::optimizer::Optimizer;
use sir_optimizer::config::OptimizerConfig;
use sir_printer::text::TextPrinter;
use sir_rewrite::registry::any_only_registry;
use sir_types::{ConstantData, IntegerWidth, NodeId, OverflowBehavior, Span, Type};

fn span() -> Span {
    Span::unknown()
}

// ────────────────────────────────────────────────────────────────────────
// Value model + SIR interpreter (typed, independent of recipe internals)
// ────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Bool(bool),
    /// Integer values: raw two's-complement bits, declared width + signedness.
    Int { bits: u128, signed: bool, width: u32 },
    /// Bit-vector (packed collection) — word-chunked so extents up to 512 fit.
    Bits(Vec<u64>),
    BoolArray(Vec<bool>),
    IntArray(Vec<u128>),
    Tuple(Vec<Value>),
}

fn wrap(bits: u128, width: u32) -> u128 {
    if width >= 128 { bits } else { bits & ((1u128 << width) - 1) }
}

fn int_of_ty(bits: u128, ty: &Type) -> Value {
    match ty {
        Type::Integer { width, signed, .. } => {
            let w = width.bits() as u32;
            Value::Int { bits: wrap(bits, w), signed: *signed, width: w }
        }
        _ => panic!("int_of_ty on {ty:?}"),
    }
}

fn elem_ty_of_array(ty: &Type) -> &Type {
    match ty {
        Type::Array { element, .. } | Type::Slice { element, .. } => element,
        other => panic!("expected array type, got {other:?}"),
    }
}

fn type_of(function: &sir_nodes::Function, id: NodeId) -> Type {
    function.get_node(id).map(|n| n.ty.clone()).unwrap_or_else(|| panic!("missing node {id}"))
}

fn as_bool(value: Value) -> bool {
    match value { Value::Bool(b) => b, other => panic!("expected bool, got {other:?}") }
}

fn as_bits(value: Value) -> Vec<u64> {
    match value {
        Value::Bits(words) => words,
        Value::Int { bits, width, .. } => {
            let words = (width as usize + 63) / 64;
            let mut out = vec![0u64; words.max(1)];
            out[0] = bits as u64;
            if width > 64 {
                out[1] = (bits >> 64) as u64;
            }
            out
        }
        other => panic!("expected bitvector, got {other:?}"),
    }
}

fn bits_from_bools(array: &[bool]) -> Value {
    let mut words = vec![0u64; (array.len() + 63) / 64];
    for (i, b) in array.iter().enumerate() {
        if *b {
            words[i / 64] |= 1u64 << (i % 64);
        }
    }
    Value::Bits(words)
}

/// Compare two integers with `op` using the signedness of the left operand.
fn cmp_int(l: &Value, r: &Value, op: CmpOperator) -> bool {
    let (lb, ls, lw) = match l {
        Value::Int { bits, signed, width } => (*bits, *signed, *width),
        other => panic!("cmp_int lhs {other:?}"),
    };
    let rb = match r {
        Value::Int { bits, .. } => *bits,
        other => panic!("cmp_int rhs {other:?}"),
    };
    let rb = wrap(rb, lw);
    let lb = wrap(lb, lw);
    if ls {
        let le = ((lb << (128 - lw)) as i128) >> (128 - lw);
        let re = ((rb << (128 - lw)) as i128) >> (128 - lw);
        match op {
            CmpOperator::Eq => le == re,
            CmpOperator::Ne => le != re,
            CmpOperator::Lt => le < re,
            CmpOperator::Le => le <= re,
            CmpOperator::Gt => le > re,
            CmpOperator::Ge => le >= re,
        }
    } else {
        match op {
            CmpOperator::Eq => lb == rb,
            CmpOperator::Ne => lb != rb,
            CmpOperator::Lt => lb < rb,
            CmpOperator::Le => lb <= rb,
            CmpOperator::Gt => lb > rb,
            CmpOperator::Ge => lb >= rb,
        }
    }
}

fn compare_vals(left: Value, right: Value, op: CmpOperator) -> bool {
    match (left, right) {
        (Value::Bool(l), Value::Bool(r)) => match op {
            CmpOperator::Eq => l == r,
            CmpOperator::Ne => l != r,
            _ => panic!("bool ordered compare"),
        },
        (l @ Value::Int { .. }, r @ Value::Int { .. }) => cmp_int(&l, &r, op),
        (l, r) => {
            // bit-vector involved: compare word-wise after normalising
            let mut a = as_bits(l);
            let mut b = as_bits(r);
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            match op {
                CmpOperator::Eq => a == b,
                CmpOperator::Ne => a != b,
                _ => panic!("ordered compare on bit-vectors"),
            }
        }
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
        NodeKind::Constant(ConstantData::Bool(v)) => Value::Bool(*v),
        NodeKind::Constant(ConstantData::Integer { value, .. }) => {
            let v: i128 = value.parse().expect("integer constant");
            match &node.ty {
                Type::BitVector { width } => {
                    let words = ((*width as usize) + 63) / 64;
                    let mut out = vec![0u64; words.max(1)];
                    out[0] = v as u64;
                    if words > 1 {
                        out[1] = (v as u128 >> 64) as u64;
                    }
                    Value::Bits(out)
                }
                _ => int_of_ty(v as u128, &node.ty),
            }
        }
        NodeKind::Constant(_) => panic!("unsupported constant kind on {:?}", node.ty),
        NodeKind::Add { lhs, rhs } => {
            let a = match eval_node(function, *lhs, args, overrides) {
                Value::Int { bits, width, .. } => (bits, width),
                other => panic!("add lhs {other:?}"),
            };
            let b = match eval_node(function, *rhs, args, overrides) {
                Value::Int { bits, .. } => bits,
                other => panic!("add rhs {other:?}"),
            };
            Value::Int { bits: wrap(a.0.wrapping_add(b), a.1), signed: false, width: a.1 }
        }
        NodeKind::Sub { lhs, rhs } => {
            let a = match eval_node(function, *lhs, args, overrides) {
                Value::Int { bits, width, .. } => (bits, width),
                other => panic!("sub lhs {other:?}"),
            };
            let b = match eval_node(function, *rhs, args, overrides) {
                Value::Int { bits, .. } => bits,
                other => panic!("sub rhs {other:?}"),
            };
            Value::Int { bits: wrap(a.0.wrapping_sub(b), a.1), signed: false, width: a.1 }
        }
        NodeKind::Lt { .. }
        | NodeKind::Le { .. }
        | NodeKind::Gt { .. }
        | NodeKind::Ge { .. }
        | NodeKind::Eq { .. }
        | NodeKind::Ne { .. } => {
            let op = match &node.kind {
                NodeKind::Lt { .. } => CmpOperator::Lt,
                NodeKind::Le { .. } => CmpOperator::Le,
                NodeKind::Gt { .. } => CmpOperator::Gt,
                NodeKind::Ge { .. } => CmpOperator::Ge,
                NodeKind::Eq { .. } => CmpOperator::Eq,
                NodeKind::Ne { .. } => CmpOperator::Ne,
                _ => unreachable!(),
            };
            let (lhs, rhs) = match &node.kind {
                NodeKind::Lt { lhs, rhs }
                | NodeKind::Le { lhs, rhs }
                | NodeKind::Gt { lhs, rhs }
                | NodeKind::Ge { lhs, rhs }
                | NodeKind::Eq { lhs, rhs }
                | NodeKind::Ne { lhs, rhs } => (*lhs, *rhs),
                _ => unreachable!(),
            };
            let left = eval_node(function, lhs, args, overrides);
            let right = eval_node(function, rhs, args, overrides);
            Value::Bool(compare_vals(left, right, op))
        }
        NodeKind::BoolOr { lhs, rhs } => Value::Bool(
            as_bool(eval_node(function, *lhs, args, overrides))
                || as_bool(eval_node(function, *rhs, args, overrides)),
        ),
        NodeKind::BoolAnd { lhs, rhs } => Value::Bool(
            as_bool(eval_node(function, *lhs, args, overrides))
                && as_bool(eval_node(function, *rhs, args, overrides)),
        ),
        NodeKind::Select { cond, true_val, false_val } => {
            let c = as_bool(eval_node(function, *cond, args, overrides));
            if c {
                eval_node(function, *true_val, args, overrides)
            } else {
                eval_node(function, *false_val, args, overrides)
            }
        }
        NodeKind::ArrayAccess { base, index } => {
            let index = match eval_node(function, *index, args, overrides) {
                Value::Int { bits, .. } => bits as usize,
                other => panic!("array index {other:?}"),
            };
            let elem_ty = elem_ty_of_array(&type_of(function, *base)).clone();
            match eval_node(function, *base, args, overrides) {
                Value::BoolArray(array) => Value::Bool(array[index]),
                Value::IntArray(array) => int_of_ty(array[index], &elem_ty),
                other => panic!("expected array base, got {other:?}"),
            }
        }
        NodeKind::FieldAccess { base, field } => {
            match eval_node(function, *base, args, overrides) {
                Value::Tuple(tuple) => tuple[field.parse::<usize>().unwrap()].clone(),
                other => panic!("expected tuple base, got {other:?}"),
            }
        }
        NodeKind::TupleExtract { tuple, index } => {
            match eval_node(function, *tuple, args, overrides) {
                Value::Tuple(t) => t[*index].clone(),
                other => panic!("expected tuple base, got {other:?}"),
            }
        }
        NodeKind::Pack { array } => {
            match eval_node(function, *array, args, overrides) {
                Value::BoolArray(array) => bits_from_bools(&array),
                other => panic!("expected bool array for pack, got {other:?}"),
            }
        }
        NodeKind::ArrayCmpMask { array, scalar, op } => {
            let elem_ty = elem_ty_of_array(&type_of(function, *array)).clone();
            let (ew, es) = match &elem_ty {
                Type::Integer { width, signed, .. } => (width.bits() as u32, *signed),
                other => panic!("mask element {other:?}"),
            };
            let values = match eval_node(function, *array, args, overrides) {
                Value::IntArray(values) => values,
                other => panic!("expected int array for mask, got {other:?}"),
            };
            let scalar = match eval_node(function, *scalar, args, overrides) {
                Value::Int { bits, .. } => wrap(bits, ew),
                other => panic!("mask scalar {other:?}"),
            };
            let mut bits = vec![0u64; (values.len() + 63) / 64];
            for (i, value) in values.into_iter().enumerate() {
                let elem = Value::Int { bits: wrap(value, ew), signed: es, width: ew };
                let rhs = Value::Int { bits: scalar, signed: es, width: ew };
                if cmp_int(&elem, &rhs, *op) {
                    bits[i / 64] |= 1u64 << (i % 64);
                }
            }
            Value::Bits(bits)
        }
        NodeKind::Convert { operand, .. } => {
            let target = &node.ty;
            match eval_node(function, *operand, args, overrides) {
                Value::Int { bits, .. } => int_of_ty(bits, target),
                other => panic!("convert {other:?}"),
            }
        }
        NodeKind::Loop { termination, outputs, carried_inputs, .. } => {
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
        other => panic!("unsupported node kind in H3 interpreter: {other:?}"),
    }
}

fn eval_function(function: &sir_nodes::Function, args: &[Value]) -> Value {
    let ret = function.return_node.expect("function must have a return");
    eval_node(function, ret, args, &HashMap::new())
}

/// Arguments for a fixture: array contents (bits) + optional scalar/flag.
#[derive(Clone)]
struct Inputs {
    array: Vec<u128>,   // raw element bits (bool arrays store 0/1)
    scalar: Option<u128>,
    flag: Option<bool>,
}

// ────────────────────────────────────────────────────────────────────────
// Tier A corpus (h3/tier_a.tsv) fixture builder
// ────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Row {
    id: String,
    class: String,
    kind: String,   // bool | pred
    elem: String,   // elem type name or "-"
    op: String,     // or | eq/ne/lt/le/gt/ge
    scalar: String, // - | param | const:N
    extent: usize,
    acc_slot: usize,
    identity: u8,
    consumer: String, // field | extract | select | tuple
    deviation: String,
}

fn elem_type(name: &str) -> Option<Type> {
    let mk = |bits: u32, signed: bool| Type::Integer {
        width: match bits {
            8 => IntegerWidth::I8,
            16 => IntegerWidth::I16,
            32 => IntegerWidth::I32,
            64 => IntegerWidth::I64,
            _ => unreachable!(),
        },
        signed,
        overflow: OverflowBehavior::Wrapping,
    };
    match name {
        "bool" => Some(Type::Bool),
        "u8" => Some(mk(8, false)),
        "u16" => Some(mk(16, false)),
        "u32" => Some(mk(32, false)),
        "u64" => Some(mk(64, false)),
        "i8" => Some(mk(8, true)),
        "i16" => Some(mk(16, true)),
        "i32" => Some(mk(32, true)),
        "i64" => Some(mk(64, true)),
        _ => None,
    }
}

fn parse_rows(path: &str) -> Vec<Row> {
    let text = std::fs::read_to_string(path).expect("read tier_a.tsv");
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("id\t") {
            continue; // column header
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(f.len() >= 11, "bad tier_a row: {line}");
        rows.push(Row {
            id: f[0].into(),
            class: f[1].into(),
            kind: f[2].into(),
            elem: f[3].into(),
            op: f[4].into(),
            scalar: f[5].into(),
            extent: f[6].parse().expect("extent"),
            acc_slot: f[7].parse().expect("acc_slot"),
            identity: f[8].parse().expect("identity"),
            consumer: f[9].into(),
            deviation: f[10].into(),
        });
    }
    rows
}

fn u64_const(b: &mut Builder, v: u64) -> NodeId {
    b.constant(ConstantData::u64(v), Type::u64(), span())
}

fn build_fixture(row: &Row) -> sir_nodes::Function {
    let is_bool = row.kind == "bool";
    let elem = if is_bool { Type::Bool } else { elem_type(&row.elem).expect("elem") };
    let array_ty = Type::Array { element: Box::new(elem.clone()), length: row.extent };
    let tuple_ret = row.consumer == "tuple";
    let needs_flag = row.consumer == "select";
    let mut params: Vec<(&str, Type)> = vec![("collection", array_ty)];
    if !is_bool && row.scalar == "param" && row.deviation != "unstable_scalar" {
        params.push(("scalar", elem.clone()));
    }
    if needs_flag {
        params.push(("flag", Type::Bool));
    }
    let ret_ty = if tuple_ret {
        if row.deviation == "two_reductions" {
            Type::Tuple { elements: vec![Type::Bool, Type::Bool, Type::u64()] }
        } else {
            Type::Tuple { elements: vec![Type::Bool, Type::u64()] }
        }
    } else {
        Type::Bool
    };
    let mut b = Builder::new(&row.id, &params, ret_ty);
    let collection = b.parameter_index(0).unwrap();
    let scalar_param = if !is_bool && row.scalar == "param" && row.deviation != "unstable_scalar" {
        Some(b.parameter_index(1).unwrap())
    } else {
        None
    };
    let flag_param = if needs_flag { Some(b.parameter_index(params.len() - 1).unwrap()) } else { None };

    let (start_bits, step_is_sub, extent_bound) = match row.deviation.as_str() {
        "nz_start" => (1u64, false, row.extent as u64),
        "partial_bound" => (0u64, false, (row.extent as u64).saturating_sub(1)),
        "reverse" => ((row.extent as u64).saturating_sub(1), true, 0u64),
        _ => (0u64, false, row.extent as u64),
    };
    let start = u64_const(&mut b, start_bits);
    let step = u64_const(&mut b, 1);
    let bound_const = u64_const(&mut b, extent_bound);
    let extent_const = u64_const(&mut b, row.extent as u64);
    let less_extent = u64_const(&mut b, (row.extent as u64).saturating_sub(1));
    let ident = b.constant(ConstantData::boolean(row.identity == 1), Type::Bool, span());
    let ident_t = b.constant(ConstantData::boolean(true), Type::Bool, span());

    // element access
    let element = b.array_access(collection, start, elem.clone(), span()).unwrap();
    // predicate
    let pred = if is_bool {
        element
    } else {
        let scalar = match row.scalar.as_str() {
            "param" if row.deviation != "unstable_scalar" => scalar_param.unwrap(),
            "param" => {
                // unstable scalar: scalar derived from the induction counter
                b.convert(start, elem.clone(), sir_nodes::ConvertKind::Truncate, span()).unwrap()
            }
            c => {
                let lit: i128 = c.strip_prefix("const:").expect("const:N").parse().expect("lit");
                b.constant(
                    ConstantData::Integer {
                        value: lit.to_string(),
                        width: match &elem {
                            Type::Integer { width, .. } => *width,
                            _ => IntegerWidth::I32,
                        },
                        signed: matches!(&elem, Type::Integer { signed: true, .. }),
                    },
                    elem.clone(),
                    span(),
                )
            }
        };
        match row.op.as_str() {
            "eq" => b.eq(element, scalar, span()).unwrap(),
            "ne" => b.ne(element, scalar, span()).unwrap(),
            "lt" => b.lt(element, scalar, span()).unwrap(),
            "le" => b.le(element, scalar, span()).unwrap(),
            "gt" => b.gt(element, scalar, span()).unwrap(),
            "ge" => b.ge(element, scalar, span()).unwrap(),
            other => panic!("bad pred op {other}"),
        }
    };
    let next_any = b.bool_or(ident, pred, span()).unwrap();
    let two_red = row.deviation == "two_reductions";
    let next_and = if two_red {
        Some(b.bool_and(ident_t, element, span()).unwrap())
    } else {
        None
    };
    let next_index = if step_is_sub { b.sub(start, step, span()).unwrap() } else { b.add(start, step, span()).unwrap() };
    // termination
    let condition = match row.deviation.as_str() {
        "reverse" => b.gt(start, bound_const, span()).unwrap(),
        "unstable_bound" => {
            let bound = b.select(element, extent_const, less_extent, span()).unwrap();
            b.lt(start, bound, span()).unwrap()
        }
        _ => b.lt(start, bound_const, span()).unwrap(),
    };

    // loop node
    let index_first = row.acc_slot == 1;
    let mut body = vec![element];
    if !is_bool {
        body.push(pred);
    }
    if two_red {
        body.push(next_and.unwrap());
    }
    body.push(next_any);
    body.push(next_index);
    body.push(condition);
    let outputs: Vec<NodeId> = if two_red {
        vec![next_any, next_and.unwrap(), next_index]
    } else if index_first {
        vec![next_index, next_any]
    } else {
        vec![next_any, next_index]
    };
    let carried: Vec<NodeId> = if two_red {
        vec![ident, ident_t, start]
    } else if index_first {
        vec![start, ident]
    } else {
        vec![ident, start]
    };
    let ty = if two_red {
        Type::Tuple { elements: vec![Type::Bool, Type::Bool, Type::u64()] }
    } else if index_first {
        Type::Tuple { elements: vec![Type::u64(), Type::Bool] }
    } else {
        Type::Tuple { elements: vec![Type::Bool, Type::u64()] }
    };
    let loop_node = b.r#loop(&body, condition, &outputs, &carried, ty, span()).unwrap();

    // consumer wiring
    if tuple_ret {
        b.return_value(loop_node, span()).unwrap();
        return b.build();
    }
    let slot_str = row.acc_slot.to_string();
    let acc = b.field_access(loop_node, &slot_str, Type::Bool, span()).unwrap();
    let ret_value = match row.deviation.as_str() {
        "deadidx" => {
            let idx_slot = if row.acc_slot == 0 { "1" } else { "0" };
            let idx_ty = if row.acc_slot == 0 { Type::u64() } else { Type::u64() };
            let _idx = b.field_access(loop_node, idx_slot, idx_ty, span()).unwrap();
            acc
        }
        "twofield" => {
            let acc2 = b.field_access(loop_node, &slot_str, Type::Bool, span()).unwrap();
            b.bool_and(acc, acc2, span()).unwrap()
        }
        _ => match row.consumer.as_str() {
            "field" => acc,
            "extract" => b.tuple_extract(loop_node, row.acc_slot, Type::Bool, span()).unwrap(),
            "select" => {
                let f = flag_param.expect("select needs flag param");
                let other = b.constant(ConstantData::boolean(false), Type::Bool, span());
                b.select(f, acc, other, span()).unwrap()
            }
            other => panic!("bad consumer {other}"),
        },
    };
    b.return_value(ret_value, span()).unwrap();
    b.build()
}

// ────────────────────────────────────────────────────────────────────────
// Tier A execution patterns
// ────────────────────────────────────────────────────────────────────────

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn bool_inputs(extent: usize) -> Vec<Vec<bool>> {
    let mut cases = Vec::new();
    if extent <= 12 {
        for mask in 0..(1u32 << extent) {
            cases.push((0..extent).map(|i| mask & (1 << i) != 0).collect());
        }
        return cases;
    }
    cases.push(vec![false; extent]);
    cases.push(vec![true; extent]);
    for p in [0usize, extent / 2, extent - 1] {
        if p < extent {
            let mut v = vec![false; extent];
            v[p] = true;
            cases.push(v);
        }
    }
    cases.push((0..extent).map(|i| i % 2 == 0).collect());
    let mut seed = 0xC0FFEEu64 ^ (extent as u64);
    for _ in 0..8 {
        cases.push((0..extent).map(|_| xorshift64(&mut seed) & 1 == 1).collect());
    }
    cases
}

fn int_patterns(row: &Row) -> Vec<Inputs> {
    let is_signed = row.elem.starts_with('i');
    let width: u32 = match row.elem.trim_start_matches('i').trim_start_matches('u').parse().unwrap() {
        8 => 8, 16 => 16, 32 => 32, _ => 64,
    };
    let maxv = if width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
    let half = maxv / 2;
    let scalar_values: Vec<u128> = match row.scalar.as_str() {
        c if c.starts_with("const:") => {
            let v: i128 = c.strip_prefix("const:").unwrap().parse().unwrap();
            let bits = if v < 0 { (v as u128) & maxv } else { v as u128 };
            vec![bits]
        }
        _ => {
            let mut s = vec![0, 1, half, maxv];
            if is_signed {
                s.push(maxv); // -1 in two's complement
            }
            s
        }
    };
    let _ = is_signed;
    let mut cases = Vec::new();
    let mk = |vals: &[u128]| Inputs {
        array: vals.to_vec(),
        scalar: if row.scalar == "param" { Some(0) } else { None },
        flag: None,
    };
    for sc in scalar_values {
        let base: Vec<u128> = vec![0; row.extent];
        let all_max: Vec<u128> = vec![maxv; row.extent];
        let mut first = vec![0u128; row.extent];
        if row.extent > 0 { first[0] = maxv; }
        let mut last = vec![0u128; row.extent];
        if row.extent > 0 { last[row.extent - 1] = maxv; }
        let mut alt = Vec::new();
        for i in 0..row.extent { alt.push(if i % 2 == 0 { maxv } else { 0 }); }
        let mut seed = 0xABAD1DEAu64 ^ (sc as u64) ^ (width as u64);
        let mut rnd = Vec::new();
        for _ in 0..row.extent {
            rnd.push((xorshift64(&mut seed) as u128) & maxv);
        }
        for arr in [base, all_max, first, last, alt, rnd] {
            cases.push(Inputs { array: arr, scalar: Some(sc), flag: None });
        }
    }
    let _ = mk;
    cases
}

// ────────────────────────────────────────────────────────────────────────
// Run modes
// ────────────────────────────────────────────────────────────────────────

fn args_for_row(row: &Row, inp: &Inputs) -> Vec<Value> {
    let is_bool = row.kind == "bool";
    let mut args = Vec::new();
    if is_bool {
        args.push(Value::BoolArray(inp.array.iter().map(|b| *b != 0).collect()));
    } else {
        args.push(Value::IntArray(inp.array.clone()));
    }
    if !is_bool && row.scalar == "param" && row.deviation != "unstable_scalar" {
        args.push(Value::Int { bits: inp.scalar.unwrap(), signed: row.elem.starts_with('i'), width: elem_type(&row.elem).map(|t| match t { Type::Integer { width, .. } => width.bits() as u32, _ => 64 }).unwrap() });
    }
    if row.consumer == "select" {
        args.push(Value::Bool(inp.flag.unwrap_or(false)));
    }
    args
}

fn int_arg_width(elem: &str) -> u32 {
    match elem {
        "u8" | "i8" => 8,
        "u16" | "i16" => 16,
        "u32" | "i32" => 32,
        _ => 64,
    }
}

fn mode_run() {
    let rows = parse_rows("/home/tom/dev/experiments/ABSAC/h3/tier_a.tsv");
    println!("H3 RUN — tier A (SIR fixtures), frozen C3 registry");
    println!("id\tclass\tkind\toutcome\tcands\trewrites\tnotes");
    let mut rewrote = 0usize;
    for row in &rows {
        let f = build_fixture(row);
        let opt = Optimizer::new(OptimizerConfig::default(), any_only_registry());
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let r = opt.optimize(&f);
            let cands: usize = r.iterations_detail.iter().map(|d| d.candidates.len()).sum();
            let last = r.iterations_detail.last().map(|d| format!("{:?}", d.outcome)).unwrap_or_default();
            (r.rewrites_applied, cands, last)
        }));
        match outcome {
            Ok((ra, cands, last)) => {
                if ra > 0 { rewrote += 1; }
                println!("{}\t{}\t{}\trewrites={ra}\tcands={cands}\t{last}", row.id, row.class, row.kind);
            }
            Err(_) => {
                println!("{}\t{}\t{}\tPANIC\t0\t0\tcaught panic in optimizer", row.id, row.class, row.kind);
            }
        }
    }
    println!("TIER_A_REWRITES={rewrote}/{}", rows.len());

    // Tier B: LLVM sources
    let ll_text = std::fs::read_to_string("/home/tom/dev/experiments/ABSAC/h3/tier_b.ll").expect("tier_b.ll");
    println!("\nH3 RUN — tier B (LLVM sources)");
    println!("kernel\tlowered\tverify\tf_cands\trewrites\toutcome");
    for name in list_functions(&ll_text) {
        let lowered = lower_function(&ll_text, &name);
        match lowered {
            Err(e) => {
                println!("{name}\tNO\t-\t0\t0\tlower: {e}");
            }
            Ok(func) => {
                let verified = {
                    let mut v = sir_verify::Verifier::new(&func);
                    v.verify()
                };
                let opt = Optimizer::new(OptimizerConfig::default(), any_only_registry());
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    let r = opt.optimize(&func);
                    let cands: usize = r.iterations_detail.iter().map(|d| d.candidates.len()).sum();
                    let last = r.iterations_detail.last().map(|d| format!("{:?}", d.outcome)).unwrap_or_default();
                    (r.rewrites_applied, cands, last)
                }));
                match outcome {
                    Ok((ra, cands, last)) => {
                        if ra > 0 { rewrote += 1; }
                        println!("{name}\tYES\t{}\t{cands}\t{ra}\t{last}", if verified { "PASS" } else { "FAIL" });
                    }
                    Err(_) => {
                        println!("{name}\tYES\t{}\t0\t0\tPANIC (caught)", if verified { "PASS" } else { "FAIL" });
                    }
                }
            }
        }
    }
    println!("TOTAL_REWRITES={rewrote}");
}

fn mode_exec() {
    let rows = parse_rows("/home/tom/dev/experiments/ABSAC/h3/tier_a.tsv");
    println!("H3 EXEC — differential SIR execution (original vs rewritten)");
    println!("id\tclass\tpatterns\tmismatches\tfirst_mismatch");
    for row in &rows {
        let f = build_fixture(row);
        let opt = Optimizer::new(OptimizerConfig::default(), any_only_registry());
        let res = opt.optimize(&f);
        if res.rewrites_applied == 0 {
            println!("{}\t{}\t0\t0\t(no rewrite; nothing to execute)", row.id, row.class);
            continue;
        }
        let inputs: Vec<Inputs> = if row.kind == "bool" {
            bool_inputs(row.extent).into_iter().map(|a| Inputs {
                array: a.iter().map(|b| if *b { 1 } else { 0 }).collect(),
                scalar: None,
                flag: None,
            }).collect()
        } else {
            int_patterns(row)
        };
        let mut mismatches = 0usize;
        let mut first: Option<String> = None;
        for inp in &inputs {
            let flags: Vec<Option<bool>> = if row.consumer == "select" { vec![Some(false), Some(true)] } else { vec![None] };
            for flag in flags {
                let mut a = inp.clone();
                a.flag = flag;
                let args = args_for_row(row, &a);
                let expected = match eval_function(&f, &args) {
                    Value::Bool(b) => b,
                    other => panic!("expected bool from original, got {other:?}"),
                };
                let actual = match eval_function(&res.function, &args) {
                    Value::Bool(b) => b,
                    other => panic!("expected bool from rewritten, got {other:?}"),
                };
                if expected != actual {
                    mismatches += 1;
                    if first.is_none() {
                        first = Some(format!("orig={expected} rewritten={actual} scalar={:?} flag={flag:?} first5={:?}", a.scalar, &a.array[..a.array.len().min(5)]));
                    }
                }
            }
        }
        // S2 witness search: an index-derived predicate scalar cannot be
        // expressed by the rewritten single-scalar mask form; search a small
        // value domain for a semantic witness.
        if row.deviation == "unstable_scalar" {
            let mut seed = 0x5EEDu64 ^ (row.extent as u64);
            let mut found = 0usize;
            let mut witness: Option<String> = None;
            for _ in 0..4000 {
                let arr: Vec<u128> = (0..row.extent)
                    .map(|_| (xorshift64(&mut seed) as usize % (row.extent + 2)) as u128)
                    .collect();
                let inp = Inputs { array: arr.clone(), scalar: None, flag: None };
                let args = args_for_row(row, &inp);
                let expected = match eval_function(&f, &args) {
                    Value::Bool(b) => b,
                    other => panic!("expected bool, got {other:?}"),
                };
                let actual = match eval_function(&res.function, &args) {
                    Value::Bool(b) => b,
                    other => panic!("expected bool, got {other:?}"),
                };
                if expected != actual {
                    found += 1;
                    if witness.is_none() {
                        witness = Some(format!("orig={expected} rewritten={actual} arr={arr:?}"));
                    }
                }
            }
            println!("{} S2-witness: {found}/4000 first={:?}", row.id, witness);
        }
        println!("{}\t{}\t{}\t{}\t{}", row.id, row.class, inputs.len(), mismatches, first.unwrap_or_default());
    }
}

fn mode_s1() {
    // S1 reproduction (frozen behavior; documented defect):
    // identity=true OR-reductions rewrite to pack/mask != 0 and corrupt
    // the all-false input (original: constant true; rewritten: false).
    let rows = parse_rows("/home/tom/dev/experiments/ABSAC/h3/tier_a.tsv");
    println!("H3 S1 — identity=true reduction reproduction");
    for row in rows.iter().filter(|r| r.identity == 1) {
        let f = build_fixture(row);
        let opt = Optimizer::new(OptimizerConfig::default(), any_only_registry());
        let res = opt.optimize(&f);
        println!("{} identity=true: rewrites={}", row.id, res.rewrites_applied);
        if res.rewrites_applied == 0 {
            println!("  (correct abstention)");
            continue;
        }
        let inputs: Vec<Inputs> = if row.kind == "bool" {
            bool_inputs(row.extent).into_iter().map(|a| Inputs {
                array: a.iter().map(|b| if *b { 1 } else { 0 }).collect(),
                scalar: None,
                flag: None,
            }).collect()
        } else {
            int_patterns(row)
        };
        let mut mismatches = 0usize;
        let mut first: Option<String> = None;
        for inp in &inputs {
            let args = args_for_row(row, inp);
            let expected = match eval_function(&f, &args) {
                Value::Bool(b) => b,
                other => panic!("bool expected, got {other:?}"),
            };
            let actual = match eval_function(&res.function, &args) {
                Value::Bool(b) => b,
                other => panic!("bool expected, got {other:?}"),
            };
            if expected != actual {
                mismatches += 1;
                if first.is_none() {
                    first = Some(format!("orig={expected} rewritten={actual} scalar={:?} first5={:?}", inp.scalar, &inp.array[..inp.array.len().min(5)]));
                }
            }
        }
        println!("  patterns={} mismatches={mismatches} first={:?}", inputs.len(), first);
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    match mode.as_str() {
        "run" => mode_run(),
        "exec" => mode_exec(),
        "s1" => mode_s1(),
        other => {
            eprintln!("usage: h3_run <run|exec|s1> (got '{other}')");
            std::process::exit(2);
        }
    }
    let _ = TextPrinter::new(false);
    let _ = int_arg_width("u8");
    let _ = (IntegerWidth::I8, OverflowBehavior::Wrapping);
    let _ = (CmpOperator::Eq, ConstantData::boolean(false));
}
