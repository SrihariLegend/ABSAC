//! SIR → C emitter.
//!
//! Emits compilable C from a SIR Function. Handles parameters, constants,
//! arithmetic/bitwise/comparison ops, Select, ArrayAccess, Popcount, Loop
//! (with carried inputs/outputs), TupleExtract, and Return.
//!
//! Limitations (honest):
//! - No Load/Store/pointer support yet (pure-kernel subset)
//! - No cast/convert (SIR doesn't have one — gap #2)
//! - Tuple results from loops are handled by emitting separate output vars

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, NodeId, Type};

/// Map a SIR type to a C type string.
fn c_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "bool".to_string(),
        Type::Integer { width, signed, .. } => {
            let prefix = if *signed { "int" } else { "uint" };
            match width {
                sir_types::IntegerWidth::I8 => format!("{}8_t", prefix),
                sir_types::IntegerWidth::I16 => format!("{}16_t", prefix),
                sir_types::IntegerWidth::I32 => format!("{}32_t", prefix),
                sir_types::IntegerWidth::I64 => format!("{}64_t", prefix),
                sir_types::IntegerWidth::I128 => format!("{}64_t", prefix), // no 128-bit C type, fall back
            }
        }
        Type::Unit => "void".to_string(),
        Type::Pointer { pointee, mutable } => {
            let inner = c_type(pointee);
            if *mutable {
                format!("{} *", inner)
            } else {
                format!("const {} *", inner)
            }
        }
        Type::Array { element, length } => {
            format!("{}[{}]", c_type(element), length)
        }
        _ => "void".to_string(),
    }
}

/// Get the width of an integer type.
fn int_width(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer { width, .. } => Some(match width {
            sir_types::IntegerWidth::I8 => 32, // popcount promotes to int in C
            sir_types::IntegerWidth::I16 => 32,
            sir_types::IntegerWidth::I32 => 32,
            sir_types::IntegerWidth::I64 => 64,
            sir_types::IntegerWidth::I128 => 64,
        }),
        _ => None,
    }
}

/// Emit a constant as a C literal.
fn constant_literal(data: &ConstantData, ty: &Type) -> String {
    match ty {
        Type::Integer { signed, .. } => {
            if *signed {
                let v = data.as_i64().unwrap_or(0);
                format!("({})", v)
            } else {
                let v = data.as_u64().unwrap_or(0);
                format!("({}ULL)", v)
            }
        }
        Type::Bool => {
            if data.as_u64().unwrap_or(0) != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => "0".to_string(),
    }
}

/// Emit an operand — either an inlined expression (for constants/params) or
/// a variable reference (for nodes already emitted as statements).
fn emit_operand(
    id: NodeId,
    func: &Function,
    carrier_map: &std::collections::HashMap<NodeId, String>,
) -> String {
    if let Some(name) = carrier_map.get(&id) {
        return name.clone();
    }
    let node = match func.get_node(id) {
        Some(n) => n,
        None => return format!("/* missing {} */ 0", id),
    };
    match &node.kind {
        NodeKind::Parameter { .. } => {
            // Top-level parameter: use the param name from the function signature.
            // Parameter nodes have NodeId = their index (0, 1, 2, ...).
            let idx = node.id.as_u64() as usize;
            func.params
                .get(idx)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("p{}", idx))
        }
        NodeKind::Constant(_) => emit_expr(node, func, carrier_map),
        _ => format!("v{}", id.as_u64()),
    }
}

/// Emit a node as a C expression.
fn emit_expr(
    node: &Node,
    func: &Function,
    carrier_map: &std::collections::HashMap<NodeId, String>,
) -> String {
    match &node.kind {
        NodeKind::Parameter { .. } => emit_operand(node.id, func, carrier_map),
        NodeKind::Constant(data) => constant_literal(data, &node.ty),
        NodeKind::Add { lhs, rhs } => {
            format!("({} + {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Sub { lhs, rhs } => {
            format!("({} - {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Mul { lhs, rhs } => {
            format!("({} * {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Div { lhs, rhs } => {
            format!("({} / {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Rem { lhs, rhs } => {
            format!("({} % {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::And { lhs, rhs } => {
            format!("({} & {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Or { lhs, rhs } => {
            format!("({} | {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Xor { lhs, rhs } => {
            format!("({} ^ {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Shl { lhs, rhs } => {
            format!("({} << {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Shr { lhs, rhs } => {
            format!("({} >> {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Not { operand } => {
            format!("(~{})", emit_operand(*operand, func, carrier_map))
        }
        NodeKind::Popcount { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            let width = op_node.and_then(|n| int_width(&n.ty)).unwrap_or(32);
            let builtin = if width <= 32 { "__builtin_popcount" } else { "__builtin_popcountll" };
            format!("{}({})", builtin, o)
        }
        NodeKind::LeadingZeros { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            let width = op_node.and_then(|n| int_width(&n.ty)).unwrap_or(32);
            let builtin = if width <= 32 { "__builtin_clz" } else { "__builtin_clzll" };
            format!("{}({})", builtin, o)
        }
        NodeKind::TrailingZeros { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            let width = op_node.and_then(|n| int_width(&n.ty)).unwrap_or(32);
            let builtin = if width <= 32 { "__builtin_ctz" } else { "__builtin_ctzll" };
            format!("{}({})", builtin, o)
        }
        NodeKind::Eq { lhs, rhs } => {
            format!("({} == {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Ne { lhs, rhs } => {
            format!("({} != {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Lt { lhs, rhs } => {
            format!("({} < {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Le { lhs, rhs } => {
            format!("({} <= {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Gt { lhs, rhs } => {
            format!("({} > {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Ge { lhs, rhs } => {
            format!("({} >= {})", emit_operand(*lhs, func, carrier_map), emit_operand(*rhs, func, carrier_map))
        }
        NodeKind::Select { cond, true_val, false_val } => {
            format!("({} ? {} : {})",
                emit_operand(*cond, func, carrier_map),
                emit_operand(*true_val, func, carrier_map),
                emit_operand(*false_val, func, carrier_map))
        }
        NodeKind::ArrayAccess { base, index } => {
            format!("{}[{}]", emit_operand(*base, func, carrier_map), emit_operand(*index, func, carrier_map))
        }
        NodeKind::Convert { operand, kind, .. } => {
            // Emit the appropriate C cast. For zero-extend/truncate, use (target_type)value.
            let o = emit_operand(*operand, func, carrier_map);
            let target_c = c_type(&node.ty);
            match kind {
                sir_nodes::ConvertKind::ZeroExtend | sir_nodes::ConvertKind::SignExtend | sir_nodes::ConvertKind::Truncate => {
                    format!("(({}) {})", target_c, o)
                }
            }
        }
        NodeKind::TupleExtract { index, .. } => {
            // Loop output variable: o_{index}
            format!("o_{}", index)
        }
        _ => "0".to_string(),
    }
}

/// Emit a SIR function as C source code.
pub fn emit_c(func: &Function) -> String {
    let mut out = String::new();

    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");

    // Function signature
    let ret_c = c_type(&func.return_ty);
    let params: Vec<String> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if let Type::Array { element, .. } = &p.ty {
                // Arrays decay to pointers in C function params
                format!("const {} *{}", c_type(element), p.name)
            } else {
                format!("{} {}", c_type(&p.ty), p.name)
            }
        })
        .collect();

    let name = func.name.split("::").last().unwrap_or(&func.name);
    out.push_str(&format!("{} {}({}) {{\n", ret_c, name, params.join(", ")));

    // Find Loop and Return nodes
    let mut loop_node: Option<&Node> = None;
    let mut return_node: Option<&Node> = None;
    for node in func.arena.iter() {
        match &node.kind {
            NodeKind::Loop { .. } => loop_node = Some(node),
            NodeKind::Return { .. } => return_node = Some(node),
            _ => {}
        }
    }

    if let Some(loop_n) = loop_node {
        emit_loop(loop_n, func, &mut out);
        // Collect all node IDs that belong to the loop (body, outputs, carried,
        // termination) so we can skip them when emitting post-loop statements.
        let mut loop_ids: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        if let NodeKind::Loop { body, termination, outputs, carried_inputs } = &loop_n.kind {
            loop_ids.extend(body.iter().copied());
            loop_ids.insert(*termination);
            loop_ids.extend(outputs.iter().copied());
            loop_ids.extend(carried_inputs.iter().copied());
        }
        // Emit remaining non-loop, non-parameter, non-return nodes AFTER the loop
        // (e.g., TupleExtract that reads loop outputs). Skip any node that was
        // part of the loop.
        let empty_map = std::collections::HashMap::new();
        for node in func.arena.iter() {
            if matches!(node.kind, NodeKind::Parameter { .. } | NodeKind::Return { .. } | NodeKind::Loop { .. } | NodeKind::Constant(_)) {
                continue;
            }
            if loop_ids.contains(&node.id) {
                continue;
            }
            let ty = c_type(&node.ty);
            let expr = emit_expr(node, func, &empty_map);
            out.push_str(&format!("    {} v{} = {};\n", ty, node.id.as_u64(), expr));
        }
    } else {
        // No loop: emit all non-parameter, non-return nodes as statements
        let empty_map = std::collections::HashMap::new();
        for node in func.arena.iter() {
            if matches!(node.kind, NodeKind::Parameter { .. } | NodeKind::Return { .. } | NodeKind::Loop { .. }) {
                continue;
            }
            let ty = c_type(&node.ty);
            let expr = emit_expr(node, func, &empty_map);
            out.push_str(&format!("    {} v{} = {};\n", ty, node.id.as_u64(), expr));
        }
    }

    // Return statement
    if let Some(ret) = return_node {
        if let NodeKind::Return { value } = &ret.kind {
            let empty_map = std::collections::HashMap::new();
            let ret_expr = emit_operand(*value, func, &empty_map);
            out.push_str(&format!("    return {};\n", ret_expr));
        }
    }

    out.push_str("}\n");
    out
}

/// Emit a Loop node as a C while loop.
fn emit_loop(loop_node: &Node, func: &Function, out: &mut String) {
    let (body, termination, outputs, carried_inputs) = match &loop_node.kind {
        NodeKind::Loop { body, termination, outputs, carried_inputs } => {
            (body, *termination, outputs, carried_inputs)
        }
        _ => return,
    };

    // Build carrier map: each carried input maps to C variable c_{index}
    let mut carrier_map: std::collections::HashMap<NodeId, String> = std::collections::HashMap::new();
    for (i, &ci) in carried_inputs.iter().enumerate() {
        carrier_map.insert(ci, format!("c_{}", i));
    }

    // Declare carrier variables with initial values (the carried-input nodes
    // are constants or params — emit their initial expression).
    let empty_map = std::collections::HashMap::new();
    for (i, &ci) in carried_inputs.iter().enumerate() {
        let ci_node = func.get_node(ci).unwrap();
        let ty = c_type(&ci_node.ty);
        let init = emit_expr(ci_node, func, &empty_map);
        out.push_str(&format!("    {} c_{} = {};\n", ty, i, init));
    }

    // Declare output variables
    for (i, &oi) in outputs.iter().enumerate() {
        let oi_node = func.get_node(oi).unwrap();
        let ty = c_type(&oi_node.ty);
        out.push_str(&format!("    {} o_{};\n", ty, i));
    }

    // The while loop
    out.push_str("    while (1) {\n");

    // Emit ALL body nodes (except carried inputs) as variable assignments.
    // This includes output and termination nodes — they need to be computed
    // before we can assign outputs and check the termination condition.
    let skip_set: std::collections::HashSet<NodeId> = carried_inputs.iter().copied().collect();
    for &node_id in body {
        if skip_set.contains(&node_id) {
            continue;
        }
        let node = func.get_node(node_id).unwrap();
        let ty = c_type(&node.ty);
        let expr = emit_expr(node, func, &carrier_map);
        out.push_str(&format!("        {} v{} = {};\n", ty, node_id.as_u64(), expr));
    }

    // Assign outputs
    for (i, &oi) in outputs.iter().enumerate() {
        let val = emit_operand(oi, func, &carrier_map);
        out.push_str(&format!("        o_{} = {};\n", i, val));
    }

    // Check termination (loop continues while termination is true)
    let term_expr = emit_operand(termination, func, &carrier_map);
    out.push_str(&format!("        if (!({})) break;\n", term_expr));

    // Update carriers
    for (i, _) in carried_inputs.iter().enumerate() {
        out.push_str(&format!("        c_{} = o_{};\n", i, i));
    }

    out.push_str("    }\n");
}
