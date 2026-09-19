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
use std::fmt::Write as _;
use sir_types::{ConstantData, Effects, NodeId, Type};
use std::collections::{HashMap, HashSet};

/// Post-order DFS over dataflow inputs restricted to `in_set`.
///
/// Emission order must follow the dataflow graph, not arena order: the
/// old emitter wrote post-loop (and loop-body) statements in arena order,
/// so a node could be referenced before its definition (H3 finding F6 —
/// post-loop emission order).
fn topo_order(seeds: &[NodeId], in_set: &HashSet<NodeId>, func: &Function) -> Vec<NodeId> {
    fn visit(
        id: NodeId,
        in_set: &HashSet<NodeId>,
        func: &Function,
        visited: &mut HashSet<NodeId>,
        visiting: &mut HashSet<NodeId>,
        order: &mut Vec<NodeId>,
    ) {
        if visited.contains(&id) || !in_set.contains(&id) {
            return;
        }
        // A cycle cannot be emitted as straight-line statements; leave it
        // to the caller (the inliner has its own cycle guard).
        if !visiting.insert(id) {
            return;
        }
        if let Some(node) = func.get_node(id) {
            for dep in sir_analysis::graph::dataflow_inputs(&node.kind) {
                visit(dep, in_set, func, visited, visiting, order);
            }
        }
        visiting.remove(&id);
        visited.insert(id);
        order.push(id);
    }

    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();
    for &seed in seeds {
        visit(seed, in_set, func, &mut visited, &mut visiting, &mut order);
    }
    order
}

/// All non-atom dataflow nodes reachable from `seeds` (excluding the
/// seeds' own atoms). Atoms (parameters, constants, carried inputs) are
/// inlined at use sites; everything else must be emitted as a statement.
fn dependency_closure(
    seeds: &[NodeId],
    func: &Function,
    stops: &HashSet<NodeId>,
) -> HashSet<NodeId> {
    fn visit(
        id: NodeId,
        func: &Function,
        stops: &HashSet<NodeId>,
        out: &mut HashSet<NodeId>,
        seen: &mut HashSet<NodeId>,
    ) {
        if !seen.insert(id) || stops.contains(&id) {
            return;
        }
        let Some(node) = func.get_node(id) else {
            return;
        };
        match &node.kind {
            NodeKind::Parameter { .. } | NodeKind::Constant(_) => {}
            NodeKind::Loop { .. } | NodeKind::Return { .. } => {}
            _ => {
                out.insert(id);
                for dep in sir_analysis::graph::dataflow_inputs(&node.kind) {
                    visit(dep, func, stops, out, seen);
                }
            }
        }
    }

    let mut out = HashSet::new();
    let mut seen = HashSet::new();
    for &seed in seeds {
        visit(seed, func, stops, &mut out, &mut seen);
    }
    out
}

/// Map a SIR type to a C type string.
pub fn c_type(ty: &Type) -> String {
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
        // Bitvector masks (rewrite outputs) share one fixed 8-limb
        // representation; the helpers below keep unused limbs zero.
        Type::BitVector { .. } => "sir_bv".to_string(),
        _ => "void".to_string(),
    }
}

/// Element size in bytes (bitvectors/aggregates have none).
fn int_byte_width(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer { width, .. } => Some(match width {
            sir_types::IntegerWidth::I8 => 1,
            sir_types::IntegerWidth::I16 => 2,
            sir_types::IntegerWidth::I32 => 4,
            sir_types::IntegerWidth::I64 => 8,
            sir_types::IntegerWidth::I128 => 16,
        }),
        _ => None,
    }
}

fn is_bitvector(ty: &Type) -> bool {
    matches!(ty, Type::BitVector { .. })
}

/// Runtime support for bitvector masks: `Pack` / `ArrayCmpMask` outputs
/// and their comparisons, popcounts and bit scans. Emitted once, only in
/// functions that use a bitvector (H3 noted Pack/ArrayCmpMask as the
/// unexercised half of the emitter; rewrite outputs on array-backed
/// collections use them, e.g. `any → mask != 0`).
fn bitvector_prelude() -> &'static str {
    r#"
#include <string.h>

typedef struct { uint64_t w[8]; } sir_bv;

static sir_bv __sir_bv_zero(void) {
    sir_bv r;
    for (int i = 0; i < 8; i++) r.w[i] = 0;
    return r;
}
static int __sir_bv_eq(sir_bv a, sir_bv b) {
    for (int i = 0; i < 8; i++) if (a.w[i] != b.w[i]) return 0;
    return 1;
}
static int __sir_bv_ne(sir_bv a, sir_bv b) { return !__sir_bv_eq(a, b); }
static uint64_t __sir_bv_popcount(sir_bv a) {
    uint64_t c = 0;
    for (int i = 0; i < 8; i++) c += (uint64_t)__builtin_popcountll(a.w[i]);
    return c;
}
static uint64_t __sir_bv_ctz(sir_bv a, unsigned width) {
    for (unsigned i = 0; i < width && i < 512; i++)
        if ((a.w[i / 64] >> (i % 64)) & 1u) return i;
    return width;
}
static uint64_t __sir_bv_clz(sir_bv a, unsigned width) {
    for (unsigned i = 0; i < width && i < 512; i++) {
        unsigned bit = width - 1 - i;
        if ((a.w[bit / 64] >> (bit % 64)) & 1u) return i;
    }
    return width;
}
static uint64_t __sir_bv_bsr(sir_bv a, unsigned width) {
    for (unsigned i = 0; i < width && i < 512; i++) {
        unsigned bit = width - 1 - i;
        if ((a.w[bit / 64] >> (bit % 64)) & 1u) return bit;
    }
    return width;
}
static sir_bv __sir_mask_cmp(const uint8_t *base, unsigned n, unsigned elem_bytes,
                             uint64_t scalar, int op) {
    sir_bv r = __sir_bv_zero();
    for (unsigned i = 0; i < n && i < 512; i++) {
        uint64_t v = 0;
        memcpy(&v, base + (size_t)i * elem_bytes, elem_bytes);
        int take = 0;
        switch (op) {
            case 0: take = (v == scalar); break;
            case 1: take = (v != scalar); break;
            case 2: take = (v < scalar); break;
            case 3: take = (v <= scalar); break;
            case 4: take = (v > scalar); break;
            case 5: take = (v >= scalar); break;
        }
        if (take) r.w[i / 64] |= (uint64_t)1 << (i % 64);
    }
    return r;
}
static sir_bv __sir_pack_bools(const bool *b, unsigned n) {
    sir_bv r = __sir_bv_zero();
    for (unsigned i = 0; i < n && i < 512; i++)
        if (b[i]) r.w[i / 64] |= (uint64_t)1 << (i % 64);
    return r;
}
"#
}

/// Runtime helpers for instruction-selection operations (rotates, bit
/// reversal). `Rol`/`Ror` semantics are width-relative, so the helper
/// reduces the amount modulo the true bit width and never performs a
/// shift by the width (which is UB in C).
fn alu_prelude() -> &'static str {
    r#"
static uint64_t __sir_rotl(uint64_t x, uint64_t k, unsigned w) {
    if (w == 0) return 0;
    k %= w;
    if (k == 0) return x;
    return (x << k) | (x >> (w - k));
}
static uint64_t __sir_rotr(uint64_t x, uint64_t k, unsigned w) {
    if (w == 0) return 0;
    k %= w;
    if (k == 0) return x;
    return (x >> k) | (x << (w - k));
}
static uint64_t __sir_rbit(uint64_t x, unsigned w) {
    if (w == 0) return 0;
    x = ((x >> 1) & 0x5555555555555555ull) | ((x & 0x5555555555555555ull) << 1);
    x = ((x >> 2) & 0x3333333333333333ull) | ((x & 0x3333333333333333ull) << 2);
    x = ((x >> 4) & 0x0F0F0F0F0F0F0F0Full) | ((x & 0x0F0F0F0F0F0F0F0Full) << 4);
    x = ((x >> 8) & 0x00FF00FF00FF00FFull) | ((x & 0x00FF00FF00FF00FFull) << 8);
    x = ((x >> 16) & 0x0000FFFF0000FFFFull) | ((x & 0x0000FFFF0000FFFFull) << 16);
    x = (x >> 32) | (x << 32);
    return (w < 64) ? (x >> (64 - w)) : x;
}
/* tzcnt/lzcnt conventions: zero counts as the full width. */
static uint64_t __sir_ctz(uint64_t x, unsigned w) {
    if (x == 0) return w;
    unsigned r = (w <= 32) ? (unsigned)__builtin_ctz((uint32_t)x)
                           : (unsigned)__builtin_ctzll(x);
    return (r < w) ? r : w;
}
static uint64_t __sir_clz(uint64_t x, unsigned w) {
    if (w == 0) return 0;
    if (x == 0) return w;
    if (w <= 32) {
        unsigned r = (unsigned)__builtin_clz((uint32_t)x);
        return (r > 32 - w) ? (r - (32 - w)) : 0;
    }
    unsigned r = (unsigned)__builtin_clzll(x);
    return (r > 64 - w) ? (r - (64 - w)) : 0;
}
/* bit-scan-reverse: highest set index, width sentinel for zero. */
static uint64_t __sir_bsr(uint64_t x, unsigned w) {
    if (w == 0) return 0;
    if (x == 0) return w;
    unsigned lead;
    if (w <= 32) {
        unsigned r = (unsigned)__builtin_clz((uint32_t)x);
        lead = (r > 32 - w) ? (r - (32 - w)) : 0;
    } else {
        unsigned r = (unsigned)__builtin_clzll(x);
        lead = (r > 64 - w) ? (r - (64 - w)) : 0;
    }
    return w - 1 - lead;
}
"#
}

fn sanitize_ident(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
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

/// The actual bit width of an integer type (unlike `int_width`, which
/// promotes narrow types for C builtins).
fn int_bits(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer { width, .. } => Some(match width {
            sir_types::IntegerWidth::I8 => 8,
            sir_types::IntegerWidth::I16 => 16,
            sir_types::IntegerWidth::I32 => 32,
            sir_types::IntegerWidth::I64 => 64,
            sir_types::IntegerWidth::I128 => 128,
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
        Type::BitVector { .. } => {
            let v = data.as_u64().unwrap_or(0);
            format!("((sir_bv){{ .w = {{ (uint64_t){}ULL }} }})", v)
        }
        Type::Unit => "((void)0)".to_string(),
        _ => "0".to_string(),
    }
}

/// Emit an operand — either an inlined expression (for constants/params) or
/// a variable reference (for nodes already emitted as statements).
/// Execution ordinal of a Loop node: loops are built in execution order
/// and their arena ids ascend, so this ordinal namespaces the emitted
/// carrier/output variables (`c{ord}_{i}` / `o{ord}_{i}`) for
/// sequential-loop functions.
fn loop_ordinal(func: &Function, loop_id: NodeId) -> usize {
    func.arena
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Loop { .. }))
        .position(|n| n.id == loop_id)
        .unwrap_or(0)
}

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
                .map(|p| {
                    // Sanitize LLVM auto-generated names like %0, %1
                    if p.name.starts_with('%') {
                        format!("p{}", idx)
                    } else {
                        p.name.clone()
                    }
                })
                .unwrap_or_else(|| format!("p{}", idx))
        }
        NodeKind::Constant(_) => emit_expr(node, func, carrier_map),
        _ => format!("v{}", id.as_u64()),
    }
}

/// Render a pure integer/bool node graph as a single C expression.
///
/// Returns `None` when any node is unsupported or not side-effect-free
/// (stores, calls, bitvector operations, nested loops, ...). Loop guards use
/// this to emit canonical `while (cond)` loops; anything else keeps the
/// explicit statement-based pre-test form.
fn emit_pure_expr(
    id: NodeId,
    func: &Function,
    carrier_map: &std::collections::HashMap<NodeId, String>,
    depth: usize,
) -> Option<String> {
    let mut out = String::new();
    if emit_pure_expr_into(&mut out, id, func, carrier_map, depth) {
        Some(out)
    } else {
        None
    }
}

/// Buffer-writing form of [`emit_pure_expr`]: recursive operands append
/// directly to `out` instead of allocating one `String` per node.
fn emit_pure_expr_into(
    out: &mut String,
    id: NodeId,
    func: &Function,
    carrier_map: &std::collections::HashMap<NodeId, String>,
    depth: usize,
) -> bool {
    if depth > 64 {
        return false;
    }
    if let Some(name) = carrier_map.get(&id) {
        out.push_str(name);
        return true;
    }
    let Some(node) = func.get_node(id) else {
        return false;
    };
    let rec = |out: &mut String, operand: NodeId| {
        emit_pure_expr_into(out, operand, func, carrier_map, depth + 1)
    };
    let binary = |out: &mut String, lhs: NodeId, op: &str, rhs: NodeId| {
        out.push('(');
        if !emit_pure_expr_into(out, lhs, func, carrier_map, depth + 1) {
            return false;
        }
        out.push_str(op);
        if !emit_pure_expr_into(out, rhs, func, carrier_map, depth + 1) {
            return false;
        }
        out.push(')');
        true
    };
    match &node.kind {
        NodeKind::Constant(data) => {
            out.push_str(&constant_literal(data, &node.ty));
            true
        }
        NodeKind::Parameter { .. } => {
            out.push_str(&emit_operand(id, func, carrier_map));
            true
        }
        NodeKind::Add { lhs, rhs } => binary(out, *lhs, " + ", *rhs),
        NodeKind::Sub { lhs, rhs } => binary(out, *lhs, " - ", *rhs),
        NodeKind::Mul { lhs, rhs } => binary(out, *lhs, " * ", *rhs),
        NodeKind::Div { lhs, rhs } => binary(out, *lhs, " / ", *rhs),
        NodeKind::Rem { lhs, rhs } => binary(out, *lhs, " % ", *rhs),
        NodeKind::Neg { operand } => {
            out.push_str("(-");
            if !rec(out, *operand) {
                return false;
            }
            out.push(')');
            true
        }
        NodeKind::And { lhs, rhs } => binary(out, *lhs, " & ", *rhs),
        NodeKind::Or { lhs, rhs } => binary(out, *lhs, " | ", *rhs),
        NodeKind::Xor { lhs, rhs } => binary(out, *lhs, " ^ ", *rhs),
        NodeKind::Shl { lhs, rhs } => binary(out, *lhs, " << ", *rhs),
        NodeKind::Shr { lhs, rhs } => binary(out, *lhs, " >> ", *rhs),
        NodeKind::Not { operand } => {
            out.push_str("(~");
            if !rec(out, *operand) {
                return false;
            }
            out.push(')');
            true
        }
        NodeKind::Convert { operand, kind, .. } => match kind {
            sir_nodes::ConvertKind::ZeroExtend
            | sir_nodes::ConvertKind::SignExtend
            | sir_nodes::ConvertKind::Truncate => {
                out.push_str("((");
                out.push_str(&c_type(&node.ty));
                out.push_str(") ");
                if !rec(out, *operand) {
                    return false;
                }
                out.push(')');
                true
            }
        },
        NodeKind::Eq { lhs, rhs } => binary(out, *lhs, " == ", *rhs),
        NodeKind::Ne { lhs, rhs } => binary(out, *lhs, " != ", *rhs),
        NodeKind::Lt { lhs, rhs } => binary(out, *lhs, " < ", *rhs),
        NodeKind::Le { lhs, rhs } => binary(out, *lhs, " <= ", *rhs),
        NodeKind::Gt { lhs, rhs } => binary(out, *lhs, " > ", *rhs),
        NodeKind::Ge { lhs, rhs } => binary(out, *lhs, " >= ", *rhs),
        NodeKind::BoolAnd { lhs, rhs } => binary(out, *lhs, " && ", *rhs),
        NodeKind::BoolOr { lhs, rhs } => binary(out, *lhs, " || ", *rhs),
        NodeKind::BoolNot { operand } => {
            out.push_str("(!");
            if !rec(out, *operand) {
                return false;
            }
            out.push(')');
            true
        }
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => {
            out.push('(');
            if !rec(out, *cond) || !{
                out.push_str(" ? ");
                true
            } || !rec(out, *true_val)
                || !{
                    out.push_str(" : ");
                    true
                }
                || !rec(out, *false_val)
            {
                return false;
            }
            out.push(')');
            true
        }
        NodeKind::ArrayAccess { base, index } => {
            if !rec(out, *base) {
                return false;
            }
            out.push('[');
            if !rec(out, *index) {
                return false;
            }
            out.push(']');
            true
        }
        _ => false,
    }
}

/// True when `cond` is exactly `match` or `match && !found` (the two shapes
/// the lowerer produces for the result select of a first-occurrence loop).
fn cond_matches(func: &Function, cond: NodeId, match_id: NodeId, found: NodeId) -> bool {
    if cond == match_id {
        return true;
    }
    let Some(NodeKind::BoolAnd { lhs, rhs }) = func.get_node(cond).map(|n| &n.kind) else {
        return false;
    };
    let (other, has_match) = if *lhs == match_id {
        (*rhs, true)
    } else if *rhs == match_id {
        (*lhs, true)
    } else {
        (NodeId::new(0), false)
    };
    if !has_match {
        return false;
    }
    matches!(
        func.get_node(other).map(|n| &n.kind),
        Some(NodeKind::BoolNot { operand }) if *operand == found
    )
}

/// Nodes reachable from a return, a memory effect, or an external call.
/// Used to ignore dead tuple projections (the lowerer can leave them behind)
/// when deciding which loop outputs a consumer actually reads.
fn live_node_ids(func: &Function) -> HashSet<NodeId> {
    let mut live = HashSet::new();
    let mut stack: Vec<NodeId> = Vec::new();
    for node in func.arena.iter() {
        if matches!(
            node.kind,
            NodeKind::Return { .. }
                | NodeKind::Store { .. }
                | NodeKind::Call { .. }
                | NodeKind::ExternalCall { .. }
                | NodeKind::Deallocate { .. }
                | NodeKind::Intrinsic { .. }
        ) {
            stack.push(node.id);
        }
    }
    while let Some(id) = stack.pop() {
        if !live.insert(id) {
            continue;
        }
        if let Some(node) = func.get_node(id) {
            stack.extend(node.kind.input_nodes());
        }
    }
    live
}

/// True when the node graph reads memory (loads or array accesses). Used to
/// reject the cached-length fast path when the loop bound could change between
/// iterations.
fn node_reads_memory(func: &Function, id: NodeId, depth: usize) -> bool {
    if depth > 64 {
        return true;
    }
    let Some(node) = func.get_node(id) else {
        return true;
    };
    match &node.kind {
        NodeKind::ArrayAccess { .. } | NodeKind::Load { .. } => true,
        NodeKind::Constant(_) | NodeKind::Parameter { .. } => false,
        _ => node
            .kind
            .input_nodes()
            .iter()
            .any(|dep| node_reads_memory(func, *dep, depth + 1)),
    }
}

/// True when the carried index starts at zero.
fn index_starts_at_zero(func: &Function, idx_carry: NodeId) -> bool {
    matches!(
        func.get_node(idx_carry).map(|n| &n.kind),
        Some(NodeKind::Constant(data)) if data.as_u64() == Some(0)
    )
}

/// True when the index output advances the carry by exactly one.
fn index_step_is_one(func: &Function, output: NodeId, carry: NodeId) -> bool {
    let Some(NodeKind::Add { lhs, rhs }) = func.get_node(output).map(|n| &n.kind) else {
        return false;
    };
    let other = if *lhs == carry {
        *rhs
    } else if *rhs == carry {
        *lhs
    } else {
        return false;
    };
    matches!(
        func.get_node(other).map(|n| &n.kind),
        Some(NodeKind::Constant(data)) if data.as_u64() == Some(1)
    )
}

/// Emit the canonical early-exit form for a recognized first-occurrence loop:
/// carried `[found=false, result, index]`, `found' = found || match`,
/// `result' = match ? index : result`, `index' = index + step`, and
/// termination `!found && bound(index)`.
///
/// The generic carried-flag/select lowering keeps two cmovs, a `setcc` and an
/// `or` per iteration; clang's code for the original IR is a tight
/// compare/branch loop. Emitting `if (match) { result = index; found = true;
/// break; }` reproduces the source idiom. The transformation only applies
/// when the match and bound graphs are pure and only the found/result outputs
/// are read by post-loop consumers; the index output may differ by one
/// between the two forms, so any external read of it disables the pattern.
#[allow(clippy::too_many_arguments)]
fn try_emit_first_occurrence(
    loop_node: &Node,
    func: &Function,
    out: &mut String,
    carried_inputs: &[NodeId],
    outputs: &[NodeId],
    termination: NodeId,
    carrier_map: &std::collections::HashMap<NodeId, String>,
) -> bool {
    let Some(NodeKind::BoolAnd { lhs, rhs }) = func.get_node(termination).map(|n| &n.kind) else {
        return false;
    };
    let (not_found, bound_id) = (*lhs, *rhs);
    let Some(NodeKind::BoolNot {
        operand: found_carry,
    }) = func.get_node(not_found).map(|n| &n.kind)
    else {
        return false;
    };
    let found_carry = *found_carry;
    let Some(found_slot) = carried_inputs.iter().position(|c| *c == found_carry) else {
        return false;
    };
    // The flag must start false, otherwise the loop is not a search.
    let empty = std::collections::HashMap::new();
    if func
        .get_node(found_carry)
        .map(|n| emit_expr(n, func, &empty))
        .as_deref()
        != Some("false")
    {
        return false;
    }
    let Some(found_out) = outputs.get(found_slot).copied() else {
        return false;
    };
    let (lhs, rhs) = match func.get_node(found_out).map(|n| &n.kind) {
        Some(NodeKind::Or { lhs, rhs }) | Some(NodeKind::BoolOr { lhs, rhs }) => (lhs, rhs),
        _ => return false,
    };
    let match_id = if *lhs == found_carry {
        *rhs
    } else if *rhs == found_carry {
        *lhs
    } else {
        return false;
    };
    if match_id == found_carry || match_id == bound_id {
        return false;
    }
    // Index slot: its output adds a step to its own carried input.
    let mut idx_slot = None;
    for (i, &oi) in outputs.iter().enumerate() {
        if let Some(NodeKind::Add { lhs, rhs }) = func.get_node(oi).map(|n| &n.kind) {
            let ci = carried_inputs[i];
            if *lhs == ci || *rhs == ci {
                idx_slot = Some(i);
                break;
            }
        }
    }
    let Some(idx_slot) = idx_slot else {
        return false;
    };
    let idx_carry = carried_inputs[idx_slot];
    // Result slot: select the current index when the match condition holds.
    let mut res_slot = None;
    for (i, &oi) in outputs.iter().enumerate() {
        if i == found_slot || i == idx_slot {
            continue;
        }
        if let Some(NodeKind::Select {
            cond,
            true_val,
            false_val,
        }) = func.get_node(oi).map(|n| &n.kind)
        {
            let result_carry = carried_inputs[i];
            if *false_val == result_carry
                && *true_val == idx_carry
                && cond_matches(func, *cond, match_id, found_carry)
            {
                res_slot = Some(i);
                break;
            }
        }
    }
    let Some(res_slot) = res_slot else {
        return false;
    };
    // Only the found and result outputs may be read after the loop. The index
    // output legitimately differs by one iteration between the generic and
    // early-exit forms.
    let live = live_node_ids(func);
    for node in func.arena.iter() {
        if !live.contains(&node.id) {
            continue;
        }
        let index = match &node.kind {
            NodeKind::TupleExtract { tuple, index } if *tuple == loop_node.id => Some(*index),
            NodeKind::FieldAccess { base, field } if *base == loop_node.id => {
                field.parse::<usize>().ok()
            }
            _ => None,
        };
        if let Some(index) = index {
            if index != found_slot && index != res_slot {
                return false;
            }
        }
    }
    // Every expression must inline into a pure C expression.
    let Some(bound_expr) = emit_pure_expr(bound_id, func, carrier_map, 0) else {
        return false;
    };
    let Some(match_expr) = emit_pure_expr(match_id, func, carrier_map, 0) else {
        return false;
    };
    let Some(next_expr) = emit_pure_expr(outputs[idx_slot], func, carrier_map, 0) else {
        return false;
    };
    let (Some(found_var), Some(idx_var), Some(res_var)) = (
        carrier_map.get(&found_carry),
        carrier_map.get(&idx_carry),
        carrier_map.get(&carried_inputs[res_slot]),
    ) else {
        return false;
    };

    // Side-effecting body nodes would be dropped by the early-exit form.
    if let NodeKind::Loop { body, .. } = &loop_node.kind {
        for id in body {
            if let Some(node) = func.get_node(*id) {
                if !(node.effects - Effects::READ_MEMORY).is_empty() {
                    return false;
                }
            }
        }
    }

    // Byte-equality searches can use the library/search idiom: for n >= 32
    // hand the scan to `memchr` (vectorized in libc and recognized by the
    // downstream compiler), otherwise keep the scalar early-exit loop. The
    // library baseline measured 8-26x on no-hit scans and ~parity on early
    // hits; the threshold keeps small scans on the no-call path.
    let mut byte_search: Option<(NodeId, NodeId)> = None;
    if let Some(NodeKind::Eq { lhs, rhs }) = func.get_node(match_id).map(|n| &n.kind) {
        for (array_id, scalar_id) in [(*lhs, *rhs), (*rhs, *lhs)] {
            let Some(array_node) = func.get_node(array_id) else {
                continue;
            };
            if array_node.ty != Type::u8() {
                continue;
            }
            let NodeKind::ArrayAccess { base, index } = &array_node.kind else {
                continue;
            };
            if *index != idx_carry {
                continue;
            }
            let scalar_ok = match func.get_node(scalar_id) {
                Some(node) if node.ty == Type::u8() => true,
                Some(node) => matches!(
                    &node.kind,
                    NodeKind::Constant(data) if data.as_u64().is_some_and(|v| v <= 255)
                ),
                None => false,
            };
            if scalar_ok {
                byte_search = Some((*base, scalar_id));
                break;
            }
        }
    }
    if let Some((base, scalar)) = byte_search {
        let strict_lt = matches!(
            func.get_node(bound_id).map(|n| &n.kind),
            Some(NodeKind::Lt { lhs, .. }) if *lhs == idx_carry
        );
        if strict_lt
            && !node_reads_memory(func, bound_id, 0)
            && index_starts_at_zero(func, idx_carry)
            && index_step_is_one(func, outputs[idx_slot], idx_carry)
        {
            let bound_rhs = match func.get_node(bound_id).map(|n| &n.kind) {
                Some(NodeKind::Lt { rhs, .. }) => Some(*rhs),
                _ => None,
            };
            if let (Some(bound_len_expr), Some(base_expr), Some(scalar_expr), Some(next_scalar)) = (
                bound_rhs.and_then(|id| emit_pure_expr(id, func, carrier_map, 0)),
                emit_pure_expr(base, func, carrier_map, 0),
                emit_pure_expr(scalar, func, carrier_map, 0),
                emit_pure_expr(outputs[idx_slot], func, carrier_map, 0),
            ) {
                let _ = next_scalar; // step==1 already verified
                let res_type = func
                    .get_node(carried_inputs[res_slot])
                    .map(|n| c_type(&n.ty))
                    .unwrap_or_else(|| "uint64_t".to_string());
                let lid = loop_node.id.as_u64();
                let _ = write!(out, 
                    "    uint64_t __sir_n{lid} = (uint64_t)({bound_len_expr});\n"
                );
                let _ = write!(out, "    if (__sir_n{lid} >= 32u) {{\n");
                let _ = write!(out, 
                    "        const void *__sir_hit{lid} = memchr((const void *)({base_expr}), \
                     (int)({scalar_expr}), (size_t)__sir_n{lid});\n"
                );
                let _ = write!(out, 
                    "        if (__sir_hit{lid} != (const void *)0) {{\n"
                );
                let _ = write!(out, 
                    "            {res_var} = ({res_type})((const uint8_t *)__sir_hit{lid} - \
                     (const uint8_t *)({base_expr}));\n"
                );
                let _ = write!(out, "            {found_var} = true;\n");
                out.push_str("        }\n");
                out.push_str("    } else {\n");
                let _ = write!(out, "        while (({idx_var}) < __sir_n{lid}) {{\n");
                let _ = write!(out, 
                    "            if ({match_expr}) {{ {res_var} = {idx_var}; \
                     {found_var} = true; break; }}\n"
                );
                let _ = write!(out, "            {idx_var} = {next_expr};\n");
                out.push_str("        }\n");
                out.push_str("    }\n");
                return true;
            }
        }
    }

    let _ = write!(out, "    while ({bound_expr}) {{\n");
    let _ = write!(out, 
        "        if ({match_expr}) {{ {res_var} = {idx_var}; {found_var} = true; break; }}\n"
    );
    let _ = write!(out, "        {idx_var} = {next_expr};\n");
    out.push_str("    }\n");
    true
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
            format!(
                "({} + {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Sub { lhs, rhs } => {
            format!(
                "({} - {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Mul { lhs, rhs } => {
            format!(
                "({} * {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Div { lhs, rhs } => {
            format!(
                "({} / {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Rem { lhs, rhs } => {
            format!(
                "({} % {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::And { lhs, rhs } => {
            format!(
                "({} & {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Or { lhs, rhs } => {
            format!(
                "({} | {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::BoolAnd { lhs, rhs } => {
            format!(
                "({} && {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::BoolOr { lhs, rhs } => {
            format!(
                "({} || {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::BoolNot { operand } => {
            format!("(!{})", emit_operand(*operand, func, carrier_map))
        }
        NodeKind::Xor { lhs, rhs } => {
            format!(
                "({} ^ {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Shl { lhs, rhs } => {
            format!(
                "({} << {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Shr { lhs, rhs } => {
            format!(
                "({} >> {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Not { operand } => {
            format!("(~{})", emit_operand(*operand, func, carrier_map))
        }
        // Rotates: the helper takes the true bit width so the shift by
        // `w - k` never reaches the C undefined-behaviour cases; the
        // result is cast back to the node's declared type.
        NodeKind::Rol { lhs, rhs } | NodeKind::Ror { lhs, rhs } => {
            let x = emit_operand(*lhs, func, carrier_map);
            let k = emit_operand(*rhs, func, carrier_map);
            let width = func
                .get_node(*lhs)
                .and_then(|n| int_bits(&n.ty))
                .unwrap_or(32);
            let helper = if matches!(node.kind, NodeKind::Rol { .. }) {
                "__sir_rotl"
            } else {
                "__sir_rotr"
            };
            format!(
                "(({}){}((uint64_t)({}), (uint64_t)({}), {}u))",
                c_type(&node.ty),
                helper,
                x,
                k,
                width
            )
        }
        // Instruction-selection intrinsics: each expands to the
        // semantic SIR operation it selects. Unknown names fail loudly
        // at compile time instead of silently emitting 0.
        NodeKind::Intrinsic { name, args } => match name.as_str() {
            "blsr" if args.len() == 1 => {
                let x = emit_operand(args[0], func, carrier_map);
                format!("(({})(({}) & (({}) - 1)))", c_type(&node.ty), x, x)
            }
            "blsi" if args.len() == 1 => {
                let x = emit_operand(args[0], func, carrier_map);
                format!("(({})(({}) & (0 - ({}))))", c_type(&node.ty), x, x)
            }
            "blsmsk" if args.len() == 1 => {
                let x = emit_operand(args[0], func, carrier_map);
                format!("(({})(({}) ^ (({}) - 1)))", c_type(&node.ty), x, x)
            }
            "bswap" if args.len() == 1 => {
                let x = emit_operand(args[0], func, carrier_map);
                let width = func
                    .get_node(args[0])
                    .and_then(|n| int_bits(&n.ty))
                    .unwrap_or(32);
                let builtin = match width {
                    16 => "__builtin_bswap16",
                    32 => "__builtin_bswap32",
                    _ => "__builtin_bswap64",
                };
                format!("(({}){}((uint64_t)({})))", c_type(&node.ty), builtin, x)
            }
            "rbit" if args.len() == 1 => {
                let x = emit_operand(args[0], func, carrier_map);
                let width = func
                    .get_node(args[0])
                    .and_then(|n| int_bits(&n.ty))
                    .unwrap_or(64);
                format!(
                    "(({})__sir_rbit((uint64_t)({}), {}u))",
                    c_type(&node.ty),
                    x,
                    width
                )
            }
            "sir_memchr" if args.len() == 3 => {
                let base = emit_operand(args[0], func, carrier_map);
                let key = emit_operand(args[1], func, carrier_map);
                let len = emit_operand(args[2], func, carrier_map);
                format!(
                    "(({})__sir_memchr((const uint8_t *)({}), (int)({}), (uint64_t)({})))",
                    c_type(&node.ty),
                    base,
                    key,
                    len
                )
            }
            "sir_sum_u8" | "sir_sum_u16" | "sir_sum_u32" | "sir_sum_u64"
                if args.len() == 2 =>
            {
                let base = emit_operand(args[0], func, carrier_map);
                let len = emit_operand(args[1], func, carrier_map);
                let (helper, element) = match name.as_str() {
                    "sir_sum_u8" => ("__sir_sum_u8", "uint8_t"),
                    "sir_sum_u16" => ("__sir_sum_u16", "uint16_t"),
                    "sir_sum_u32" => ("__sir_sum_u32", "uint32_t"),
                    _ => ("__sir_sum_u64", "uint64_t"),
                };
                format!(
                    "(({}){helper}((const {element} *)({}), (uint64_t)({})))",
                    c_type(&node.ty),
                    base,
                    len
                )
            }
            other => format!("__sir_unsupported_intrinsic_{}()", sanitize_ident(other)),
        },
        NodeKind::Popcount { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            if op_node.map(|n| is_bitvector(&n.ty)).unwrap_or(false) {
                return format!("__sir_bv_popcount({})", o);
            }
            let width = op_node.and_then(|n| int_width(&n.ty)).unwrap_or(32);
            let builtin = if width <= 32 {
                "__builtin_popcount"
            } else {
                "__builtin_popcountll"
            };
            format!("{}({})", builtin, o)
        }
        NodeKind::LeadingZeros { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            if let Some(n) = op_node {
                if let Type::BitVector { width } = n.ty {
                    return format!("__sir_bv_clz({}, {}u)", o, width);
                }
            }
            // lzcnt convention: clz(0) = width (the raw builtin is UB for 0).
            let width = op_node.and_then(|n| int_bits(&n.ty)).unwrap_or(32);
            format!(
                "(({})__sir_clz((uint64_t)({}), {}u))",
                c_type(&node.ty),
                o,
                width
            )
        }
        NodeKind::TrailingZeros { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            if let Some(n) = op_node {
                if let Type::BitVector { width } = n.ty {
                    return format!("__sir_bv_ctz({}, {}u)", o, width);
                }
            }
            // tzcnt convention: ctz(0) = width (the raw builtin is UB for 0).
            let width = op_node.and_then(|n| int_bits(&n.ty)).unwrap_or(32);
            format!(
                "(({})__sir_ctz((uint64_t)({}), {}u))",
                c_type(&node.ty),
                o,
                width
            )
        }
        NodeKind::BitScanReverse { operand } => {
            let o = emit_operand(*operand, func, carrier_map);
            let op_node = func.get_node(*operand);
            if let Some(n) = op_node {
                if let Type::BitVector { width } = n.ty {
                    return format!("__sir_bv_bsr({}, {}u)", o, width);
                }
            }
            // bsr convention: highest set index, width sentinel for zero.
            let width = op_node.and_then(|n| int_bits(&n.ty)).unwrap_or(32);
            format!(
                "(({})__sir_bsr((uint64_t)({}), {}u))",
                c_type(&node.ty),
                o,
                width
            )
        }
        NodeKind::Eq { lhs, rhs } => {
            let bv = func
                .get_node(*lhs)
                .map(|n| is_bitvector(&n.ty))
                .unwrap_or(false);
            if bv {
                return format!(
                    "__sir_bv_eq({}, {})",
                    emit_operand(*lhs, func, carrier_map),
                    emit_operand(*rhs, func, carrier_map)
                );
            }
            format!(
                "({} == {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Ne { lhs, rhs } => {
            let bv = func
                .get_node(*lhs)
                .map(|n| is_bitvector(&n.ty))
                .unwrap_or(false);
            if bv {
                return format!(
                    "__sir_bv_ne({}, {})",
                    emit_operand(*lhs, func, carrier_map),
                    emit_operand(*rhs, func, carrier_map)
                );
            }
            format!(
                "({} != {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::ArrayCmpMask { array, scalar, op } => {
            let arr_node = func.get_node(*array);
            let (elem_bytes, length) = match arr_node.map(|n| &n.ty) {
                Some(Type::Array { element, length }) => {
                    (int_byte_width(element).unwrap_or(1), *length)
                }
                _ => (1, 0),
            };
            let op_code = match op {
                sir_nodes::CmpOperator::Eq => 0,
                sir_nodes::CmpOperator::Ne => 1,
                sir_nodes::CmpOperator::Lt => 2,
                sir_nodes::CmpOperator::Le => 3,
                sir_nodes::CmpOperator::Gt => 4,
                sir_nodes::CmpOperator::Ge => 5,
            };
            format!(
                "__sir_mask_cmp((const uint8_t *){}, {}u, {}u, (uint64_t)({}), {})",
                emit_operand(*array, func, carrier_map),
                length,
                elem_bytes,
                emit_operand(*scalar, func, carrier_map),
                op_code
            )
        }
        NodeKind::Pack { array } => {
            let arr_node = func.get_node(*array);
            let length = match arr_node.map(|n| &n.ty) {
                Some(Type::Array { length, .. }) => *length,
                _ => 0,
            };
            format!(
                "__sir_pack_bools((const bool *){}, {}u)",
                emit_operand(*array, func, carrier_map),
                length
            )
        }
        NodeKind::Lt { lhs, rhs } => {
            format!(
                "({} < {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Le { lhs, rhs } => {
            format!(
                "({} <= {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Gt { lhs, rhs } => {
            format!(
                "({} > {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Ge { lhs, rhs } => {
            format!(
                "({} >= {})",
                emit_operand(*lhs, func, carrier_map),
                emit_operand(*rhs, func, carrier_map)
            )
        }
        NodeKind::Select {
            cond,
            true_val,
            false_val,
        } => {
            format!(
                "({} ? {} : {})",
                emit_operand(*cond, func, carrier_map),
                emit_operand(*true_val, func, carrier_map),
                emit_operand(*false_val, func, carrier_map)
            )
        }
        NodeKind::ArrayAccess { base, index } => {
            format!(
                "{}[{}]",
                emit_operand(*base, func, carrier_map),
                emit_operand(*index, func, carrier_map)
            )
        }
        NodeKind::Convert { operand, kind, .. } => {
            // Emit the appropriate C cast. For zero-extend/truncate, use (target_type)value.
            let o = emit_operand(*operand, func, carrier_map);
            let target_c = c_type(&node.ty);
            match kind {
                sir_nodes::ConvertKind::ZeroExtend
                | sir_nodes::ConvertKind::SignExtend
                | sir_nodes::ConvertKind::Truncate => {
                    format!("(({}) {})", target_c, o)
                }
            }
        }
        NodeKind::TupleExtract { tuple, index } => {
            // Loop output variable, namespaced by the loop's execution
            // ordinal so sequential loops do not collide: o{ord}_{index}.
            if matches!(
                func.get_node(*tuple).map(|n| &n.kind),
                Some(NodeKind::Loop { .. })
            ) {
                format!("o{}_{}", loop_ordinal(func, *tuple), index)
            } else {
                format!("o_{}", index)
            }
        }
        NodeKind::FieldAccess { base, field } => {
            // Numeric field projection of a Loop tuple is an output read
            // (the builder represents `field_access(loop, "1")` as a
            // FieldAccess, while the lowerer uses TupleExtract). Any
            // other base/field has no C lowering: fail loudly at compile
            // time instead of emitting a silent 0.
            if matches!(
                func.get_node(*base).map(|n| &n.kind),
                Some(NodeKind::Loop { .. })
            ) {
                if let Ok(index) = field.parse::<usize>() {
                    return format!("o{}_{}", loop_ordinal(func, *base), index);
                }
            }
            format!(
                "__sir_unsupported_field_access_{}_{}",
                base.as_u64(),
                sanitize_ident(field)
            )
        }
        _ => "0".to_string(),
    }
}

/// Emit a SIR function as C source code.
pub fn emit_c(func: &Function) -> String {
    let mut out = String::new();

    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n");
    out.push_str("#include <string.h>\n\n");

    // Bitvector masks need the runtime helpers (Pack/ArrayCmpMask).
    let uses_bitvectors = func.arena.iter().any(|node| is_bitvector(&node.ty))
        || func.params.iter().any(|p| is_bitvector(&p.ty));
    if uses_bitvectors {
        out.push_str(bitvector_prelude());
        out.push('\n');
    }

    // Instruction-selection operations need the rotate/reverse helpers.
    let uses_alu_helpers = func.arena.iter().any(|node| match &node.kind {
        NodeKind::Rol { .. }
        | NodeKind::Ror { .. }
        | NodeKind::TrailingZeros { .. }
        | NodeKind::LeadingZeros { .. }
        | NodeKind::BitScanReverse { .. } => true,
        NodeKind::Intrinsic { name, .. } => name == "rbit",
        _ => false,
    });
    if uses_alu_helpers {
        out.push_str(alu_prelude());
        out.push('\n');
    }

    // Library byte-search primitive: first index of `key` below `n`, or `n`.
    let uses_memchr = func
        .arena
        .iter()
        .any(|node| matches!(&node.kind, NodeKind::Intrinsic { name, .. } if name == "sir_memchr"));
    if uses_memchr {
        out.push_str(
            "/* memchr-style bounded search: first index of key below n, or n. */\n\
             static uint64_t __sir_memchr(const uint8_t *base, int key, uint64_t n) {\n\
             \x20   if (n == 0) return 0;\n\
             \x20   const void *hit = memchr((const void *)base, key, (size_t)n);\n\
             \x20   return hit ? (uint64_t)((const uint8_t *)hit - base) : n;\n\
             }\n",
        );
        out.push('\n');
    }
    for (intrinsic, element, helper) in [
        ("sir_sum_u8", "uint8_t", "__sir_sum_u8"),
        ("sir_sum_u16", "uint16_t", "__sir_sum_u16"),
        ("sir_sum_u32", "uint32_t", "__sir_sum_u32"),
        ("sir_sum_u64", "uint64_t", "__sir_sum_u64"),
    ] {
        let used = func
            .arena
            .iter()
            .any(|node| matches!(&node.kind, NodeKind::Intrinsic { name, .. } if name == intrinsic));
        if !used {
            continue;
        }
        out.push_str(&format!(
            "/* Runtime-bound wrapping sum: four accumulators for ILP. */\n\
             static uint64_t {helper}(const {element} *a, uint64_t n) {{\n\
             \x20   uint64_t s0 = 0, s1 = 0, s2 = 0, s3 = 0, i = 0;\n\
             \x20   for (; i + 4 <= n; i += 4) {{\n\
             \x20       s0 += a[i]; s1 += a[i + 1]; s2 += a[i + 2]; s3 += a[i + 3];\n\
             \x20   }}\n\
             \x20   for (; i < n; i++) s0 += a[i];\n\
             \x20   return s0 + s1 + s2 + s3;\n\
             }}\n"
        ));
        out.push('\n');
    }

    // Function signature
    let ret_c = c_type(&func.return_ty);
    let params: Vec<String> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            // Sanitize LLVM auto-generated names like %0, %1
            let pname = if p.name.starts_with('%') {
                format!("p{}", i)
            } else {
                p.name.clone()
            };
            if let Type::Array { element, .. } = &p.ty {
                // Arrays decay to pointers in C function params
                format!("const {} *{}", c_type(element), pname)
            } else {
                format!("{} {}", c_type(&p.ty), pname)
            }
        })
        .collect();

    let name = func.name.split("::").last().unwrap_or(&func.name);
    let _ = write!(out, "{} {}({}) {{\n", ret_c, name, params.join(", "));

    // Find Loop nodes (execution order = ascending id) and the Return.
    let loops: Vec<&Node> = func
        .arena
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Loop { .. }))
        .collect();
    let mut return_node: Option<&Node> = None;
    for node in func.arena.iter() {
        if matches!(node.kind, NodeKind::Return { .. }) {
            return_node = Some(node);
        }
    }

    if !loops.is_empty() {
        // SEQUENTIAL MULTI-LOOP COMPOSITION (2026-09-17): emit each loop
        // in execution order. Carrier/output variables are namespaced by
        // the loop ordinal, and TupleExtract resolves to the producing
        // loop's output variable, so a later loop may thread an earlier
        // loop's result. Nodes that belong to any loop are excluded from
        // the post-loop straight-line emission.
        let mut loop_ids: HashSet<NodeId> = HashSet::new();
        for (ordinal, loop_n) in loops.iter().enumerate() {
            let emitted_ids = emit_loop(loop_n, func, &mut out, ordinal);
            loop_ids.extend(emitted_ids);
            if let NodeKind::Loop {
                body,
                termination,
                outputs,
                ..
            } = &loop_n.kind
            {
                loop_ids.extend(body.iter().copied());
                loop_ids.insert(*termination);
                loop_ids.extend(outputs.iter().copied());
            }
        }
        // Emit remaining non-loop, non-parameter, non-return nodes AFTER the
        // loops (e.g., TupleExtract that reads loop outputs), in dataflow
        // order rather than arena order (F6).
        let post: Vec<NodeId> = func
            .arena
            .iter()
            .filter(|node| {
                !matches!(
                    node.kind,
                    NodeKind::Parameter { .. }
                        | NodeKind::Return { .. }
                        | NodeKind::Loop { .. }
                        | NodeKind::Constant(_)
                ) && !loop_ids.contains(&node.id)
            })
            .map(|node| node.id)
            .collect();
        let post_set: HashSet<NodeId> = post.iter().copied().collect();
        let empty_map = HashMap::new();
        for id in topo_order(&post, &post_set, func) {
            let node = func.get_node(id).expect("arena node");
            if node.ty == Type::Unit && node.effects.is_pure() {
                continue;
            }
            let ty = c_type(&node.ty);
            let expr = emit_expr(node, func, &empty_map);
            if node.ty == Type::Unit {
                let _ = write!(out, "    (void)({expr});\n");
            } else {
                let _ = write!(out, "    {} v{} = {};\n", ty, id.as_u64(), expr);
            }
        }
    } else {
        // No loop: emit all non-parameter, non-return nodes as statements
        let all: Vec<NodeId> = func
            .arena
            .iter()
            .filter(|node| {
                !matches!(
                    node.kind,
                    NodeKind::Parameter { .. } | NodeKind::Return { .. } | NodeKind::Loop { .. }
                )
            })
            .map(|node| node.id)
            .collect();
        let all_set: HashSet<NodeId> = all.iter().copied().collect();
        let empty_map = HashMap::new();
        for id in topo_order(&all, &all_set, func) {
            let node = func.get_node(id).expect("arena node");
            if node.ty == Type::Unit && node.effects.is_pure() {
                continue;
            }
            let ty = c_type(&node.ty);
            let expr = emit_expr(node, func, &empty_map);
            if node.ty == Type::Unit {
                let _ = write!(out, "    (void)({expr});\n");
            } else {
                let _ = write!(out, "    {} v{} = {};\n", ty, id.as_u64(), expr);
            }
        }
    }

    // Return statement
    if let Some(ret) = return_node {
        if let NodeKind::Return { value } = &ret.kind {
            let empty_map = std::collections::HashMap::new();
            if func.return_ty == Type::Unit {
                let _ = write!(out, "    return;\n");
            } else {
                let ret_expr = emit_operand(*value, func, &empty_map);
                let _ = write!(out, "    return {};\n", ret_expr);
            }
        }
    }

    out.push_str("}\n");
    out
}

/// Emit a Loop node as a pre-tested C while loop.
///
/// SIR semantics (see the interpreter in `h3_run`): the termination is
/// evaluated with the *current* carried values BEFORE the body runs, and
/// the loop continues while it is true. The old emitter wrote a
/// post-tested `while (1) { body; if (!term) break; }`, which executed
/// the body once even when the entry guard was false (out-of-bounds
/// reads for `n == 0`) and evaluated the termination on the updated
/// carries — H3 finding F8 (loop-control polarity/entry-guard). It also
/// referenced the reconstructed termination node without emitting it —
/// H3 finding F6, with F7 (buffer element-width typing) fixed in the
/// lowerer's opaque-pointer inference.
///
/// Returns the set of node ids emitted inside the loop.
fn emit_loop(
    loop_node: &Node,
    func: &Function,
    out: &mut String,
    ordinal: usize,
) -> HashSet<NodeId> {
    let (body, termination, outputs, carried_inputs) = match &loop_node.kind {
        NodeKind::Loop {
            body,
            termination,
            outputs,
            carried_inputs,
        } => (body, *termination, outputs, carried_inputs),
        _ => return HashSet::new(),
    };

    // Build carrier map: each carried input maps to the loop's output
    // variable o{ordinal}_{index} (namespaced for sequential loops). The
    // outputs ARE the carriers: updating them in place avoids the
    // `o_i = value; c_i = o_i;` copy chain that made the emitted loop
    // harder for downstream compilers to canonicalize.
    let mut carrier_map: HashMap<NodeId, String> = HashMap::new();
    for (i, &ci) in carried_inputs.iter().enumerate() {
        carrier_map.insert(ci, format!("o{}_{}", ordinal, i));
    }

    // Declare carrier/output variables with initial values (the carried-input
    // nodes are constants or params — emit their initial expression). SIR
    // loop semantics return the carried values when the termination is false
    // on entry (a zero-trip loop), so an uninitialized `o_i` would return
    // garbage for `n == 0`.
    let empty_map = std::collections::HashMap::new();
    for (i, &ci) in carried_inputs.iter().enumerate() {
        let ci_node = func.get_node(ci).unwrap();
        let ty = outputs
            .get(i)
            .and_then(|oi| func.get_node(*oi))
            .map(|n| c_type(&n.ty))
            .unwrap_or_else(|| c_type(&ci_node.ty));
        let init = emit_expr(ci_node, func, &empty_map);
        let _ = write!(out, "    {} o{}_{} = {};\n", ty, ordinal, i, init);
    }

    let carried_set: HashSet<NodeId> = carried_inputs.iter().copied().collect();
    // The termination and its non-atom dependencies are evaluated at the
    // top of every iteration, before any body statement: a load in the
    // condition must not execute when the entry guard is false.
    let term_set = dependency_closure(&[termination], func, &carried_set);
    let mut body_seeds: Vec<NodeId> = body.clone();
    body_seeds.extend(outputs.iter().copied());
    let body_set = dependency_closure(&body_seeds, func, &carried_set);

    if try_emit_first_occurrence(
        loop_node,
        func,
        out,
        carried_inputs,
        outputs,
        termination,
        &carrier_map,
    ) {
        let mut emitted: HashSet<NodeId> = carried_set.clone();
        emitted.extend(outputs.iter().copied());
        emitted.insert(termination);
        emitted.extend(body.iter().copied());
        emitted.extend(term_set.iter().copied());
        emitted.extend(body_set.iter().copied());
        return emitted;
    }
    let mut emitted: HashSet<NodeId> = HashSet::new();
    if let Some(term_expr) = emit_pure_expr(termination, func, &carrier_map, 0) {
        // Fully inlinable pure condition: emit it directly in the while
        // condition. This is the canonical loop shape downstream compilers
        // expect (no `while (1) { if (!cond) break; }` indirection).
        let _ = write!(out, "    while ({term_expr}) {{\n");
    } else {
        // Complex termination (loads or other dependencies): they must run at
        // the top of each iteration before the condition is tested. Keep the
        // explicit pre-test form.
        out.push_str("    while (1) {\n");
        for id in topo_order(&[termination], &term_set, func) {
            let node = func.get_node(id).expect("arena node");
            let ty = c_type(&node.ty);
            let expr = emit_expr(node, func, &carrier_map);
            let _ = write!(out, "        {} v{} = {};\n", ty, id.as_u64(), expr);
            emitted.insert(id);
        }
        let term_expr = format!("v{}", termination.as_u64());
        let _ = write!(out, "        if (!({term_expr})) break;\n");
    }

    // Remaining body nodes, in dataflow order.
    for id in topo_order(&body_seeds, &body_set, func) {
        if !emitted.insert(id) {
            continue;
        }
        let node = func.get_node(id).expect("arena node");
        let ty = c_type(&node.ty);
        let expr = emit_expr(node, func, &carrier_map);
        let _ = write!(out, "        {} v{} = {};\n", ty, id.as_u64(), expr);
    }

    // Assign outputs: the outputs are the carriers, so this both records the
    // iteration result and updates the values the next iteration (and any
    // post-loop consumer) reads.
    for (i, &oi) in outputs.iter().enumerate() {
        let val = emit_operand(oi, func, &carrier_map);
        let _ = write!(out, "        o{}_{} = {};\n", ordinal, i, val);
    }

    out.push_str("    }\n");
    emitted
}
