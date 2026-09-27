//! Checked loop semantics: an exact, refuse-on-doubt description of a
//! counted loop, reconstructed from the SIR rather than pattern-matched.
//!
//! `loops.rs` reports a trip count for a counted loop by reading the
//! *constant bound* the counter is compared against. It never inspects the
//! counter's initial value, the direction of the relation, or the step
//! magnitude, and its own comment says the result is "at most that bound".
//! That value is then consumed as an *exact* count — `families` compare
//! `fact.trip_count != Some(extent)` — so a loop that begins anywhere other
//! than the value the bound implies, or that steps by anything other than
//! one, is described wrongly instead of being refused.
//!
//! This module states the count exactly and returns `None` whenever it
//! cannot. It is deliberately restrictive: it accepts only an unsigned
//! counter that starts at a constant, advances by a nonzero constant, and
//! is compared once against a constant, with no wrap. Everything else —
//! signed counters, compound guards, a counter compared against a
//! non-constant, a step that would overflow the counter's width — refuses.

use sir_nodes::{Function, NodeKind};
use sir_types::{NodeId, Type};

/// Which way a relation points, after normalising the counter onto the
/// left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rel {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

/// A counted loop whose iteration count this module can state exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountedLoop {
    pub loop_id: NodeId,
    /// The carried input that counts.
    pub counter: NodeId,
    /// The counter's initial value — the carried input's own constant.
    pub counter_init: u64,
    /// Step magnitude, and the direction it moves in.
    pub step: u64,
    pub ascending: bool,
    /// The constant the counter (or its successor) is compared against.
    pub bound: u64,
    /// Exact number of iterations.
    pub trip_count: u64,
    pub carried_inputs: Vec<NodeId>,
    pub outputs: Vec<NodeId>,
}

/// Reconstruct a counted loop's exact semantics, or refuse.
pub fn checked_counted_loop(function: &Function, loop_id: NodeId) -> Option<CountedLoop> {
    let node = function.get_node(loop_id)?;
    let NodeKind::Loop {
        termination,
        outputs,
        carried_inputs,
        ..
    } = &node.kind
    else {
        return None;
    };
    if carried_inputs.len() != outputs.len() {
        return None;
    }
    let (lhs, rhs, rel) = relation(function, *termination)?;

    for (index, &counter) in carried_inputs.iter().enumerate() {
        let next = *outputs.get(index)?;
        // The counter must be a constant start, unsigned, and stepped by a
        // nonzero constant.
        let init = constant(function, counter)?;
        let width = unsigned_width(function, counter)?;
        let Some((step, ascending)) = step_of(function, counter, next) else {
            continue;
        };
        let Some((count, bound)) =
            trip_count(function, lhs, rhs, rel, counter, next, init, step, ascending)
        else {
            continue;
        };
        // No wrap: the counter must not overflow its width mid-loop.
        let span = step.checked_mul(count)?;
        let end = if ascending {
            init.checked_add(span)?
        } else {
            init.checked_sub(span)?
        };
        if end > max_of(width) {
            continue;
        }
        return Some(CountedLoop {
            loop_id,
            counter,
            counter_init: init,
            step,
            ascending,
            bound,
            trip_count: count,
            carried_inputs: carried_inputs.clone(),
            outputs: outputs.clone(),
        });
    }
    None
}

/// Decompose the termination node into a single comparison. A compound
/// guard is not described here and refuses.
fn relation(function: &Function, termination: NodeId) -> Option<(NodeId, NodeId, Rel)> {
    let node = function.get_node(termination)?;
    let (lhs, rhs, rel) = match &node.kind {
        NodeKind::Lt { lhs, rhs } => (*lhs, *rhs, Rel::Lt),
        NodeKind::Le { lhs, rhs } => (*lhs, *rhs, Rel::Le),
        NodeKind::Gt { lhs, rhs } => (*lhs, *rhs, Rel::Gt),
        NodeKind::Ge { lhs, rhs } => (*lhs, *rhs, Rel::Ge),
        NodeKind::Eq { lhs, rhs } => (*lhs, *rhs, Rel::Eq),
        _ => return None,
    };
    Some((lhs, rhs, rel))
}

/// The step and direction of a carried input, if its output is a constant
/// offset of it.
fn step_of(function: &Function, counter: NodeId, next: NodeId) -> Option<(u64, bool)> {
    let node = function.get_node(next)?;
    let (delta, ascending) = match &node.kind {
        NodeKind::Add { lhs, rhs } if *lhs == counter => (constant(function, *rhs)?, true),
        NodeKind::Add { lhs, rhs } if *rhs == counter => (constant(function, *lhs)?, true),
        NodeKind::Sub { lhs, rhs } if *lhs == counter => (constant(function, *rhs)?, false),
        _ => return None,
    };
    if delta == 0 {
        return None;
    }
    Some((delta, ascending))
}

/// The exact iteration count and the bound it was derived from, or `None`
/// when the shape is outside the fragment this module describes.
#[allow(clippy::too_many_arguments)]
fn trip_count(
    function: &Function,
    lhs: NodeId,
    rhs: NodeId,
    rel: Rel,
    counter: NodeId,
    next: NodeId,
    init: u64,
    step: u64,
    ascending: bool,
) -> Option<(u64, u64)> {
    // Orient the relation so the counter is on the left, and the bound is a
    // constant. `Lt(c, B)`, `Gt(B, c)` and `Lt`-with-operands-swapped are
    // the same statement.
    let (rel, bound_side) = if lhs == counter || lhs == next {
        (rel, rhs)
    } else if rhs == counter || rhs == next {
        (flip(rel)?, lhs)
    } else {
        return None;
    };
    let bound = constant(function, bound_side)?;
    // A bound that is itself the counter's successor is not a count.
    if bound_side == next {
        return None;
    }

    // Comparing the *successor* against the bound shifts where the exit
    // lands by one step.
    let on_successor = lhs == next || rhs == next;
    let (rel, bound) = if on_successor {
        match rel {
            Rel::Eq => (Rel::Eq, bound.checked_sub(step)?),
            _ => return None,
        }
    } else {
        (rel, bound)
    };

    let count = match (rel, ascending) {
        (Rel::Lt, true) => ceil_div(bound.checked_sub(init)?, step),
        (Rel::Le, true) => bound.checked_sub(init)?.checked_div(step)?.checked_add(1)?,
        (Rel::Gt, false) => ceil_div(init.checked_sub(bound)?, step),
        (Rel::Ge, false) => init.checked_sub(bound)?.checked_div(step)?.checked_add(1)?,
        (Rel::Eq, true) => exact_steps(bound.checked_sub(init)?, step)?,
        (Rel::Eq, false) => exact_steps(init.checked_sub(bound)?, step)?,
        // An unsigned counter never ascends into a `>` bound, and the
        // mirrored cases are not spans this module states.
        _ => return None,
    };
    Some((count, bound))
}

/// `n / d` rounded up, for `n`, `d` nonzero.
fn ceil_div(n: u64, d: u64) -> u64 {
    n / d + u64::from(n % d != 0)
}

/// The number of `d`-sized steps that land exactly on `n`, or `None`.
fn exact_steps(n: u64, d: u64) -> Option<u64> {
    if n % d != 0 {
        return None;
    }
    Some(n / d)
}

fn flip(rel: Rel) -> Option<Rel> {
    Some(match rel {
        Rel::Lt => Rel::Gt,
        Rel::Le => Rel::Ge,
        Rel::Gt => Rel::Lt,
        Rel::Ge => Rel::Le,
        Rel::Eq => Rel::Eq,
    })
}

fn constant(function: &Function, id: NodeId) -> Option<u64> {
    match function.get_node(id).map(|node| &node.kind) {
        Some(NodeKind::Constant(data)) => data.as_u64(),
        _ => None,
    }
}

fn max_of(width: u32) -> u64 {
    if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn unsigned_width(function: &Function, id: NodeId) -> Option<u32> {
    match &function.get_node(id)?.ty {
        Type::Integer {
            width,
            signed: false,
            ..
        } => Some(bits(*width)),
        Type::BitVector { width } => Some(*width as u32),
        _ => None,
    }
}

fn bits(width: sir_types::IntegerWidth) -> u32 {
    match width {
        sir_types::IntegerWidth::I8 => 8,
        sir_types::IntegerWidth::I16 => 16,
        sir_types::IntegerWidth::I32 => 32,
        sir_types::IntegerWidth::I64 => 64,
        sir_types::IntegerWidth::I128 => 128,
    }
}
