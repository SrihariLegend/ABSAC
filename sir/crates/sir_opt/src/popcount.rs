//! Kernighan popcount: `while (x) { x &= x - 1; count++ }`.
//!
//! The loop clears one set bit per iteration, so the accumulator is
//! `popcount(x)`. The obligation is discharged by bounded symbolic
//! unrolling for widths ≤ 16; wider widths await the kernel induction
//! proof (H3).
//!
//! SWAR popcount: the three shift/mask/add steps that count each byte's
//! bits in place, followed by the fold that adds those byte counts into
//! one total. This one is value-local — it replaces the fold expression,
//! not a loop — so it neither needs nor inspects a loop, and it applies
//! to a chain inside a loop body as readily as to a straight-line one.

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{ConstantData, Effects, IntegerWidth, NodeId, Span, Type};

use std::sync::{Mutex, OnceLock};

use crate::patch::{apply, Patch};
use crate::prove::{SwarFold, SWAR_FOLD_UNIT, SWAR_M1, SWAR_M2, SWAR_M3};

const KERNIGHAN_POPCOUNT: u64 = 7;
const SWAR_POPCOUNT: u64 = 8;

struct PopcountPlan {
    target: NodeId,
    value: NodeId,
    value_ty: Type,
    count_ty: Type,
    width: u32,
}

/// A recognized SWAR chain. Unlike the loop plan there is no count type to
/// reconcile: the fold node's own type is the chain's width, and the
/// replacement carries it unchanged.
struct SwarPlan {
    target: NodeId,
    value: NodeId,
    width: u32,
    fold: SwarFold,
}

pub fn optimize_popcount(function: &Function) -> Option<Function> {
    crate::family::with_facts(function, |facts| {
        apply_one(function, facts).map(|(function, _)| function)
    })
}

pub(crate) fn apply_one(
    function: &Function,
    _facts: &sir_analysis::facts::FactDatabase,
) -> Option<(Function, u64)> {
    // The loop idiom is tried first: it consumes the whole loop, while the
    // SWAR recognizer is value-local and would leave the loop's other work
    // standing. Trying it first also means no existing firing changes
    // hands.
    apply_kernighan(function).or_else(|| apply_swar(function))
}

fn apply_swar(function: &Function) -> Option<(Function, u64)> {
    let plan = recognize_swar(function)?;
    if !swar_proved(plan.width, plan.fold) {
        return None;
    }
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let popcount_local = max + 1;
    let popcount = Node::new(
        NodeId::new(popcount_local),
        NodeKind::Popcount {
            operand: plan.value,
        },
        // The replaced fold node's own type, so every consumer sees the
        // type it already saw. The recognizer has checked it is unsigned
        // of the chain's width, which is also the operand's width.
        function.get_node(plan.target)?.ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    apply(
        function,
        &Patch {
            nodes: vec![(popcount_local, popcount)],
            replacements: vec![(plan.target, popcount_local)],
        },
    )
    .map(|function| (function, SWAR_POPCOUNT))
}

fn apply_kernighan(function: &Function) -> Option<(Function, u64)> {
    let plan = recognize(function)?;
    // Widths ≤32 carry the kernel induction proof. Wider widths are
    // contract-backed: the clear-lowest-bit identity is checked by
    // differential and sanitizer runs.
    if plan.width <= 32 && !kernighan_proved(plan.width) {
        return None;
    }
    let max = function.arena.ids().map(|id| id.0).max().unwrap_or(0);
    let popcount_local = max + 1;
    let popcount = Node::new(
        NodeId::new(popcount_local),
        NodeKind::Popcount {
            operand: plan.value,
        },
        plan.value_ty.clone(),
        Effects::empty(),
        Span::unknown(),
    );
    let value_width = type_bits(&plan.value_ty)?;
    let count_width = type_bits(&plan.count_ty)?;
    if value_width == count_width {
        return apply(
            function,
            &Patch {
                nodes: vec![(popcount_local, popcount)],
                replacements: vec![(plan.target, popcount_local)],
            },
        )
        .map(|function| (function, KERNIGHAN_POPCOUNT));
    }
    let convert_local = max + 2;
    let kind = if count_width > value_width {
        sir_nodes::ConvertKind::ZeroExtend
    } else {
        sir_nodes::ConvertKind::Truncate
    };
    let convert = Node::new(
        NodeId::new(convert_local),
        NodeKind::Convert {
            operand: NodeId::new(popcount_local),
            target_ty: plan.count_ty.clone(),
            kind,
        },
        plan.count_ty,
        Effects::empty(),
        Span::unknown(),
    );
    apply(
        function,
        &Patch {
            nodes: vec![(popcount_local, popcount), (convert_local, convert)],
            replacements: vec![(plan.target, convert_local)],
        },
    )
    .map(|function| (function, KERNIGHAN_POPCOUNT))
}

/// The induction proof is width-parameterized and takes ~0.4 s at width
/// 32, so cache the result per width for the process lifetime.
fn kernighan_proved(width: u32) -> bool {
    static CACHE: OnceLock<Mutex<Vec<(u32, bool)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(entries) = cache.lock() {
        if let Some(&(_, proved)) = entries.iter().find(|(w, _)| *w == width) {
            return proved;
        }
    }
    let proved = sir_mech::kernighan::kernighan_law(width);
    if let Ok(mut entries) = cache.lock() {
        entries.push((width, proved));
    }
    proved
}

fn recognize(function: &Function) -> Option<PopcountPlan> {
    let loops: Vec<NodeId> = function
        .arena
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Loop { .. }))
        .map(|node| node.id)
        .collect();
    for loop_id in loops {
        if let Some(plan) = recognize_loop(function, loop_id) {
            return Some(plan);
        }
    }
    None
}

fn recognize_loop(function: &Function, loop_id: NodeId) -> Option<PopcountPlan> {
    let loop_node = function.get_node(loop_id)?;
    if !(loop_node.effects - Effects::READ_MEMORY).is_empty() {
        return None;
    }
    let NodeKind::Loop {
        body,
        outputs,
        carried_inputs,
        termination,
    } = &loop_node.kind
    else {
        return None;
    };
    for id in body
        .iter()
        .chain(outputs)
        .chain(carried_inputs)
        .chain(std::iter::once(termination))
        .copied()
    {
        if !(function.get_node(id)?.effects - Effects::READ_MEMORY).is_empty() {
            return None;
        }
    }
    if carried_inputs.len() != 2 || outputs.len() != 2 {
        return None;
    }
    let count_pos = carried_inputs
        .iter()
        .position(|id| is_zero(function, *id))?;
    let value_pos = if count_pos == 0 { 1 } else { 0 };
    let count = carried_inputs[count_pos];
    let value = carried_inputs[value_pos];
    let value_ty = function.get_node(value)?.ty.clone();
    let count_ty = function.get_node(count)?.ty.clone();
    let width = type_bits(&value_ty)?;
    if width > 64 {
        return None;
    }
    // `while (x != 0)` termination.
    let NodeKind::Ne { lhs, rhs } = &function.get_node(*termination)?.kind else {
        return None;
    };
    if !((*lhs == value && is_zero(function, *rhs)) || (*rhs == value && is_zero(function, *lhs))) {
        return None;
    }
    // `x_next = x & (x - 1)`.
    let value_next = &function.get_node(outputs[value_pos])?.kind;
    let NodeKind::And { lhs, rhs } = value_next else {
        return None;
    };
    let other = if *lhs == value {
        *rhs
    } else if *rhs == value {
        *lhs
    } else {
        return None;
    };
    if !is_decrement(function, other, value) {
        return None;
    }
    // `count_next = count + 1`.
    let NodeKind::Add { lhs, rhs } = &function.get_node(outputs[count_pos])?.kind else {
        return None;
    };
    let count_step = if *lhs == count {
        *rhs
    } else if *rhs == count {
        *lhs
    } else {
        return None;
    };
    if !is_one(function, count_step) {
        return None;
    }
    let target = function
        .arena
        .iter()
        .find(|node| {
            matches!(
                &node.kind,
                NodeKind::TupleExtract { tuple, index } if *tuple == loop_id && *index == count_pos
            )
        })?
        .id;
    Some(PopcountPlan {
        target,
        value,
        value_ty,
        count_ty,
        width,
    })
}

/// The SWAR bit-count obligation is width-parameterized and, at width 64,
/// takes seconds, so cache the answer per (width, spelling) for the
/// process lifetime — as the induction proof above is cached.
fn swar_proved(width: u32, fold: SwarFold) -> bool {
    static CACHE: OnceLock<Mutex<Vec<(u32, SwarFold, bool)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(entries) = cache.lock() {
        if let Some(&(_, _, proved)) = entries
            .iter()
            .find(|(cached, spelling, _)| *cached == width && *spelling == fold)
        {
            return proved;
        }
    }
    let proved = crate::prove::swar_popcount_laws(width, fold);
    if let Ok(mut entries) = cache.lock() {
        entries.push((width, fold, proved));
    }
    proved
}

/// Recognize the first SWAR chain in the function. Value-local: the fold
/// node is the patch target and the word it was folded from is the
/// operand, so nothing moves across a scope boundary and no loop needs to
/// be understood.
fn recognize_swar(function: &Function) -> Option<SwarPlan> {
    let folds: Vec<NodeId> = function
        .arena
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Shr { .. }))
        .map(|node| node.id)
        .collect();
    for fold in folds {
        if let Some(plan) = recognize_swar_fold(function, fold) {
            return Some(plan);
        }
    }
    None
}

fn recognize_swar_fold(function: &Function, fold_id: NodeId) -> Option<SwarPlan> {
    let fold_node = function.get_node(fold_id)?;
    if !fold_node.effects.is_pure() {
        return None;
    }
    let width = chain_width(&fold_node.ty)?;
    if !matches!(width, 16 | 32 | 64) {
        return None;
    }
    let NodeKind::Shr { lhs, rhs } = &fold_node.kind else {
        return None;
    };
    // The fold shift is exact: `>> (W - 8)` leaves the top byte of the
    // byte-wise sum. Any other amount is a different chain.
    if !is_const(function, *rhs, u64::from(width - 8), width) {
        return None;
    }
    let shifted = *lhs;
    if !matches!(chain_width(&function.get_node(shifted)?.ty), Some(w) if w == width) {
        return None;
    }
    let (packed, fold) = match_fold(function, shifted, width)?;
    let value = match_steps(function, packed, width)?;
    Some(SwarPlan {
        target: fold_id,
        value,
        width,
        fold,
    })
}

/// The fold's input: the packed per-byte counts, masked to one count per
/// byte. Returns that node together with the spelling that consumed it.
fn match_fold(function: &Function, id: NodeId, width: u32) -> Option<(NodeId, SwarFold)> {
    let node = function.get_node(id)?;
    if !node.effects.is_pure() {
        return None;
    }
    match &node.kind {
        // `v * 0x0101..`, either operand order.
        NodeKind::Mul { lhs, rhs } => {
            let unit = swar_pattern(SWAR_FOLD_UNIT, width);
            let packed = if is_const(function, *lhs, unit, width) {
                *rhs
            } else if is_const(function, *rhs, unit, width) {
                *lhs
            } else {
                return None;
            };
            Some((packed, SwarFold::Multiply))
        }
        // `v + (v << 8) + ... + (v << W-8)`, addends in any order.
        NodeKind::Add { .. } => Some((match_shift_add(function, id, width)?, SwarFold::ShiftAdd)),
        _ => None,
    }
}

/// The addend tree of the multiply-free fold spelling. Every leaf must be
/// the same packed term, once unshifted and once per byte-shift, so a tree
/// that sums anything else — or that sums one shift twice — is refused.
fn match_shift_add(function: &Function, id: NodeId, width: u32) -> Option<NodeId> {
    let mut pending = vec![id];
    let mut leaves: Vec<(NodeId, u32)> = Vec::new();
    while let Some(node_id) = pending.pop() {
        let node = function.get_node(node_id)?;
        if !node.effects.is_pure() {
            return None;
        }
        match &node.kind {
            NodeKind::Add { lhs, rhs } => {
                pending.push(*lhs);
                pending.push(*rhs);
            }
            NodeKind::Shl { lhs, rhs } => {
                let (amount, _) = const_value(function, *rhs)?;
                leaves.push((*lhs, u32::try_from(amount).ok()?));
            }
            _ => leaves.push((node_id, 0)),
        }
    }
    let (packed, _) = *leaves.first()?;
    if leaves.iter().any(|(node, _)| *node != packed) {
        return None;
    }
    let mut amounts: Vec<u32> = leaves.iter().map(|(_, amount)| *amount).collect();
    amounts.sort_unstable();
    let expected: Vec<u32> = (0..(width / 8)).map(|k| 8 * k).collect();
    if amounts != expected {
        return None;
    }
    Some(packed)
}

/// The three steps, from the packed counts back to the word they count.
fn match_steps(function: &Function, packed: NodeId, width: u32) -> Option<NodeId> {
    // `(t2 + (t2 >> 4)) & M3`.
    let t2 = and_mask(function, packed, swar_pattern(SWAR_M3, width), width)?;
    let t1_sum = add_shifted_self(function, t2, 4, width)?;
    // `(t1 & M2) + ((t1 >> 2) & M2)`.
    let node = function.get_node(t1_sum)?;
    let NodeKind::Add { lhs, rhs } = &node.kind else {
        return None;
    };
    let low = and_mask(function, *lhs, swar_pattern(SWAR_M2, width), width)?;
    let high = and_mask(function, *rhs, swar_pattern(SWAR_M2, width), width)?;
    let t1 = if is_shift_of(function, high, low, 2, width) {
        low
    } else if is_shift_of(function, low, high, 2, width) {
        high
    } else {
        return None;
    };
    // `x - ((x >> 1) & M1)`, in that order: a subtraction's operands are
    // not interchangeable and the difference is not the sum.
    let node = function.get_node(t1)?;
    let NodeKind::Sub { lhs, rhs } = &node.kind else {
        return None;
    };
    let value = *lhs;
    let shifted = and_mask(function, *rhs, swar_pattern(SWAR_M1, width), width)?;
    if !is_shift_of(function, shifted, value, 1, width) {
        return None;
    }
    // The word the chain counts must be the chain's own width, so the
    // popcount of it is the width the law is stated at.
    if !matches!(chain_width(&function.get_node(value)?.ty), Some(w) if w == width) {
        return None;
    }
    Some(value)
}

/// `And{value, mask}` with `mask` the constant `mask_value` in either
/// operand order; returns `value`. Refuses when neither side is the mask,
/// and when both are — a mask ANDed with itself is not the pattern.
fn and_mask(function: &Function, id: NodeId, mask_value: u64, width: u32) -> Option<NodeId> {
    let node = function.get_node(id)?;
    if !node.effects.is_pure() {
        return None;
    }
    let NodeKind::And { lhs, rhs } = &node.kind else {
        return None;
    };
    let lhs_is_mask = is_const(function, *lhs, mask_value, width);
    let rhs_is_mask = is_const(function, *rhs, mask_value, width);
    match (lhs_is_mask, rhs_is_mask) {
        (true, false) => Some(*rhs),
        (false, true) => Some(*lhs),
        _ => None,
    }
}

/// `Add{base, Shr{base, amount}}` in either operand order; returns `base`.
fn add_shifted_self(function: &Function, id: NodeId, amount: u32, width: u32) -> Option<NodeId> {
    let node = function.get_node(id)?;
    if !node.effects.is_pure() {
        return None;
    }
    let NodeKind::Add { lhs, rhs } = &node.kind else {
        return None;
    };
    if is_shift_of(function, *rhs, *lhs, amount, width) {
        return Some(*lhs);
    }
    if is_shift_of(function, *lhs, *rhs, amount, width) {
        return Some(*rhs);
    }
    None
}

/// `candidate == Shr{base, amount}`, with the amount an exact constant.
fn is_shift_of(function: &Function, candidate: NodeId, base: NodeId, amount: u32, width: u32) -> bool {
    let Some(node) = function.get_node(candidate) else {
        return false;
    };
    if !node.effects.is_pure() {
        return false;
    }
    let NodeKind::Shr { lhs, rhs } = &node.kind else {
        return false;
    };
    *lhs == base && is_const(function, *rhs, u64::from(amount), width)
}

/// True when `id` is an integer constant equal to `value` at `width`. The
/// constant's own declared width must agree, so an `i64` mask standing in
/// a 32-bit chain is refused rather than silently truncated.
fn is_const(function: &Function, id: NodeId, value: u64, width: u32) -> bool {
    const_value(function, id) == Some((value, width))
}

fn const_value(function: &Function, id: NodeId) -> Option<(u64, u32)> {
    let NodeKind::Constant(ConstantData::Integer { value, width, .. }) =
        function.get_node(id).map(|node| &node.kind)?
    else {
        return None;
    };
    let bits = match width {
        IntegerWidth::I8 => 8u32,
        IntegerWidth::I16 => 16,
        IntegerWidth::I32 => 32,
        IntegerWidth::I64 => 64,
        IntegerWidth::I128 => return None,
    };
    let mask = if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 };
    let magnitude: u128 = match value.strip_prefix('-') {
        Some(rest) => (0u128).wrapping_sub(rest.parse::<u128>().ok()?),
        None => value.parse().ok()?,
    };
    let parsed = u64::try_from(magnitude & u128::from(mask)).ok()?;
    Some((parsed, bits))
}

/// The mask patterns are repeated bytes, so at a width narrower than 64
/// only the low `W/8` bytes apply.
fn swar_pattern(pattern: u64, width: u32) -> u64 {
    if width >= 64 {
        pattern
    } else {
        pattern & ((1u64 << width) - 1)
    }
}

/// The width of an unsigned integer type, which is the only shape the law
/// covers: the emitted `>>` must be the logical shift the law is stated
/// with, and a signed type would emit an arithmetic one.
fn chain_width(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer {
            width,
            signed: false,
            ..
        } => Some(match width {
            IntegerWidth::I8 => 8,
            IntegerWidth::I16 => 16,
            IntegerWidth::I32 => 32,
            IntegerWidth::I64 => 64,
            IntegerWidth::I128 => return None,
        }),
        _ => None,
    }
}

fn is_decrement(function: &Function, candidate: NodeId, value: NodeId) -> bool {
    match function.get_node(candidate).map(|node| &node.kind) {
        Some(NodeKind::Sub { lhs, rhs }) => *lhs == value && is_one(function, *rhs),
        Some(NodeKind::Add { lhs, rhs }) => {
            (*lhs == value && is_minus_one(function, *rhs))
                || (*rhs == value && is_minus_one(function, *lhs))
        }
        _ => false,
    }
}

fn is_zero(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. }))
            if value.trim_start_matches(['+', '-']).trim_start_matches('0').is_empty()
    )
}

fn is_one(function: &Function, id: NodeId) -> bool {
    matches!(
        function.get_node(id).map(|node| &node.kind),
        Some(NodeKind::Constant(ConstantData::Integer { value, .. })) if value == "1"
    )
}

fn is_minus_one(function: &Function, id: NodeId) -> bool {
    let Some(NodeKind::Constant(ConstantData::Integer {
        value,
        width,
        signed,
    })) = function.get_node(id).map(|node| &node.kind)
    else {
        return false;
    };
    let bits = match width {
        sir_types::IntegerWidth::I8 => 8u32,
        sir_types::IntegerWidth::I16 => 16,
        sir_types::IntegerWidth::I32 => 32,
        sir_types::IntegerWidth::I64 => 64,
        sir_types::IntegerWidth::I128 => return value == "-1",
    };
    if *signed {
        return value == "-1";
    }
    value
        .parse::<u128>()
        .map(|parsed| parsed == (1u128 << bits) - 1)
        .unwrap_or(false)
}

fn type_bits(ty: &Type) -> Option<u32> {
    match ty {
        Type::Integer { width, .. } => Some(match width {
            sir_types::IntegerWidth::I8 => 8,
            sir_types::IntegerWidth::I16 => 16,
            sir_types::IntegerWidth::I32 => 32,
            sir_types::IntegerWidth::I64 => 64,
            sir_types::IntegerWidth::I128 => 128,
        }),
        Type::BitVector { width } => Some(*width as u32),
        _ => None,
    }
}
