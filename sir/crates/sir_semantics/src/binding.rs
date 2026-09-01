//! ProposalBinding (advisor sequence item 4): the exact binding between
//! one concrete source region, its authorization, its complete
//! observable interface, and the transformation.
//!
//! PS002 canonical lesson: a true theorem applied to the wrong
//! observable boundary is still an incorrect compiler transformation.
//! ProposalBinding makes the observable interface EXPLICIT: every
//! stage (authorization, candidate generation, theorem construction,
//! application checking, rewrite) consumes the SAME role map; no
//! stage scans the graph for roles independently.
//!
//! Invariants (advisor acceptance criteria):
//! 1. Every theorem variable comes from a declared role.
//! 2. Every candidate operand comes from that same role map.
//! 3. Every source live-out has a `LiveOutBinding`.
//! 4. Unknown uses make binding incomplete (fail-closed).
//! 5. Every memory/effect operation appears in the frame.
//! 6. Rewrite recipes consume roles; they do not scan globally.
//! 7. Artifacts and candidates carry the same role-map digest.
//! 8. A rewrite invalidates the map and all derived artifacts.
//!
//! The derivation below is the ONE legitimate place roles are
//! scanned; it is fail-closed — any role that cannot be bound
//! concretely is a `BindingError`, never a guessed value.

use std::collections::HashSet;

use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::{Function, NodeKind};
use sir_transform::roles::RegionRoles;
use sir_types::{ConstantData, Effects, NodeId, RegionId};

use crate::authorization::{ConcreteFacts, IntegerSemantics};
use crate::structure::StructuralDescription;

// ─────────────────────────────────────────────────────────────────
// Digest (identifier, not proof)
// ─────────────────────────────────────────────────────────────────

/// FNV-1a 64-bit digest over a canonical byte string. A digest is an
/// identifier binding artifacts to each other — the exact obligation
/// is always reconstructed from the binding, never matched by digest
/// alone.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ─────────────────────────────────────────────────────────────────
// Errors — fail-closed
// ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingError {
    /// The structural description carries no reduction role set.
    NoRoles,
    /// The region loop has no certified reduction recurrence.
    NoReduction,
    /// A shape the binder does not support (fail-closed, with reason).
    UnsupportedShape(&'static str),
    /// A use of a region value has a form the binder cannot classify.
    /// Unknown means NOT dead: the binding is incomplete.
    UnclassifiedUse { use_site: NodeId, of_value: NodeId },
    /// The frame condition violates the conservative contract.
    FrameUnsupported(&'static str),
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingError::NoRoles => write!(f, "region has no reduction role set"),
            BindingError::NoReduction => {
                write!(f, "region loop has no certified reduction recurrence")
            }
            BindingError::UnsupportedShape(reason) => write!(f, "unsupported shape: {reason}"),
            BindingError::UnclassifiedUse { use_site, of_value } => write!(
                f,
                "unclassified use of %{} at %{}: unknown means not dead, binding incomplete",
                of_value.0, use_site.0
            ),
            BindingError::FrameUnsupported(reason) => {
                write!(f, "frame condition unsupported: {reason}")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Live-out binding
// ─────────────────────────────────────────────────────────────────

/// Which part of the loop's observable result this live-out is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveOutKind {
    /// The whole loop result (single-output loop).
    WholeValue,
    /// One slot of the loop's result tuple.
    Slot(usize),
}

/// Binding of one source live-out (advisor ProposalBinding semantics).
///
/// - Preserved: the exact source value remains available after the
///   rewrite, or the candidate value is structurally identical. NOT
///   derivable by the current reduction recipes (never produced yet).
/// - Reconstructed: a candidate value replaces the source live-out,
///   backed by a concrete theorem. The covering theorem's obligation
///   digest is bound at EndToEnd-artifact time, not here.
/// - Dead: complete use-closure proves no observable use outside the
///   rewritten region. Projections, copies, selects, stores, returns,
///   calls, aggregate operations — any use that cannot be followed
///   means NOT dead.
/// - Guarded: a runtime predicate establishes preconditions with a
///   semantically valid fallback (future machinery; never produced).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveOutBinding {
    Preserved,
    Reconstructed { slot: usize },
    Dead,
    Guarded { guard: NodeId },
}

/// One classified observable of the loop result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveOutSlot {
    pub kind: LiveOutKind,
    pub binding: LiveOutBinding,
}

// ─────────────────────────────────────────────────────────────────
// Frame condition
// ─────────────────────────────────────────────────────────────────

/// The application frame: everything a rewrite must preserve beyond
/// the theorem's subject. A theorem proves value equality; the frame
/// proves the rewrite changes nothing else observable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCondition {
    /// Memory bases the source region reads.
    pub source_reads: Vec<NodeId>,
    /// The region writes memory (conservative contract forbids).
    pub source_writes: bool,
    /// Volatile or atomic operations present (contract forbids).
    pub has_volatile_or_atomic: bool,
    /// Calls present (contract forbids).
    pub has_calls: bool,
    /// Possible unmodeled traps (division/remainder) present.
    pub possible_traps: bool,
    /// The loop is known to terminate (finite trip count).
    pub terminates: bool,
    /// SIR loops have exactly one normal exit by construction.
    pub single_normal_exit: bool,
    /// Number of observable outputs of the loop.
    pub output_count: usize,
}

impl FrameCondition {
    /// The conservative contract for the first vertical slice
    /// (advisor directive): pure loop, read-only nonvolatile memory,
    /// no atomics, no stores, no calls, no unmodeled traps, single
    /// normal exit, known finite iteration domain.
    pub fn supported_conservative(&self) -> bool {
        !self.source_writes
            && !self.has_volatile_or_atomic
            && !self.has_calls
            && !self.possible_traps
            && self.terminates
            && self.single_normal_exit
    }
}

// ─────────────────────────────────────────────────────────────────
// ReductionRoleMap
// ─────────────────────────────────────────────────────────────────

/// The complete role map of a reduction region — the ONLY source of
/// operands for authorization, candidate generation, theorem
/// construction, application checking, and rewrite (advisor invariant:
/// no stage rediscovers roles independently).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReductionRoleMap {
    pub region: RegionId,
    /// The loop node producing the reduction result.
    pub loop_node: NodeId,
    /// Collection base (parameter/array the loop reads).
    pub collection: NodeId,
    /// The element-access node reading one element per iteration.
    pub element_access: NodeId,
    /// The induction counter (a loop-carried variable, unit stride).
    pub induction: Option<NodeId>,
    /// The induction counter's start value.
    pub start: Option<NodeId>,
    /// The loop bound (the termination comparison's non-induction side).
    pub bound: Option<NodeId>,
    /// Iteration stride (only ±1 supported; anything else refuses).
    pub stride: i64,
    /// The element predicate / map input combined into the accumulator.
    pub predicate: Option<NodeId>,
    /// The accumulator's carried variable.
    pub accumulator: NodeId,
    /// Reduction kind ("bitwise_or", "bitwise_and", "sum", ...).
    pub recurrence: String,
    /// The accumulator's identity (the initial carried value).
    pub identity: Option<sir_types::ConstantData>,
    /// The accumulator's position in the loop's output tuple.
    pub reduction_position: usize,
    /// Values defined outside the loop that the loop depends on.
    pub live_ins: Vec<NodeId>,
    /// Union of effects over the loop and its body.
    pub effects: Effects,
    pub integer_semantics: IntegerSemantics,
}

impl ReductionRoleMap {
    /// Stable identity of this exact binding. A digest is an
    /// identifier, not a proof: artifacts and candidates must carry
    /// the SAME digest, and the obligation is always reconstructed
    /// from the map before replay.
    pub fn digest(&self) -> u64 {
        let mut parts: Vec<String> = vec![
            format!("loop={}", self.loop_node.0),
            format!("collection={}", self.collection.0),
            format!("element_access={}", self.element_access.0),
            format!("reduction_position={}", self.reduction_position),
            format!("recurrence={}", self.recurrence),
            format!("effects={:?}", self.effects),
        ];
        parts.sort();
        fnv1a64(parts.join("|").as_bytes())
    }
}

// ─────────────────────────────────────────────────────────────────
// Derivation — the ONE legitimate role scan
// ─────────────────────────────────────────────────────────────────

/// The reduction role set recognized in a structural description.
struct RegionReductionRoles {
    region: RegionId,
    collection: NodeId,
    accumulator: Option<NodeId>,
    loop_node: NodeId,
}

fn extract_reduction_roles(structural: &StructuralDescription) -> Option<RegionReductionRoles> {
    for role in &structural.roles {
        match role {
            RegionRoles::BooleanCollectionReduction {
                collection,
                accumulator,
                result,
            } => {
                return Some(RegionReductionRoles {
                    region: structural.region,
                    collection: *collection,
                    accumulator: *accumulator,
                    loop_node: *result,
                });
            }
            RegionRoles::PredicateCollectionReduction {
                collection,
                scalar: _,
                operator: _,
                accumulator,
                result,
            } => {
                return Some(RegionReductionRoles {
                    region: structural.region,
                    collection: *collection,
                    accumulator: *accumulator,
                    loop_node: *result,
                });
            }
            _ => continue,
        }
    }
    None
}

/// The integer value of an integer constant node, if any.
fn constant_i64(function: &Function, id: NodeId) -> Option<i64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(ConstantData::Integer { value, .. }) => value.parse::<i64>().ok(),
        _ => None,
    }
}

/// Locate the loop's unit-stride induction counter.
///
/// A carried variable whose paired output is `carry ± 1`. Returns
/// (carried index, start value, stride). Fail-closed: any other
/// stride refuses.
struct Induction {
    carry: NodeId,
    stride: i64,
}

fn find_induction(
    function: &Function,
    outputs: &[NodeId],
    carried_inputs: &[NodeId],
) -> Option<Induction> {
    for (&carry, &output) in carried_inputs.iter().zip(outputs.iter()) {
        let node = function.get_node(output)?;
        let step: Option<i64> = match &node.kind {
            NodeKind::Add { lhs, rhs } if *lhs == carry => constant_i64(function, *rhs),
            NodeKind::Sub { lhs, rhs } if *lhs == carry => constant_i64(function, *rhs).map(|v| -v),
            _ => None,
        };
        if let Some(step) = step {
            if step.abs() == 1 {
                return Some(Induction {
                    carry,
                    stride: step,
                });
            }
        }
    }
    None
}

/// Build the frame condition by scanning the loop subgraph for every
/// effect-bearing or trapping operation (advisor invariant 5: every
/// memory/effect operation appears in the frame).
fn build_frame(
    function: &Function,
    facts: &FactDatabase,
    loop_node: NodeId,
    body: &[NodeId],
    effects: Effects,
    output_count: usize,
    concrete: &ConcreteFacts,
) -> FrameCondition {
    let mut has_calls = false;
    let mut possible_traps = false;
    for node_id in body.iter().copied() {
        let Some(node) = function.get_node(node_id) else {
            continue;
        };
        match node.kind {
            NodeKind::Call { .. } | NodeKind::ExternalCall { .. } | NodeKind::Intrinsic { .. } => {
                has_calls = true;
            }
            NodeKind::Div { .. } | NodeKind::Rem { .. } => possible_traps = true,
            _ => {}
        }
    }

    let has_volatile_or_atomic =
        effects.contains(sir_types::Effects::ATOMIC) || effects.contains(sir_types::Effects::IO);

    FrameCondition {
        source_reads: concrete.memory_bases.clone(),
        source_writes: concrete.has_memory_writes,
        has_volatile_or_atomic,
        has_calls,
        possible_traps,
        terminates: facts
            .loops
            .get(&loop_node)
            .map(|f| f.is_finite)
            .unwrap_or(false),
        single_normal_exit: true,
        output_count,
    }
}

// ─────────────────────────────────────────────────────────────────
// Complete use-closure live-out classification
// ─────────────────────────────────────────────────────────────────

/// All slots of the result tuple: the reduction slot is reconstructed
/// by the candidate; every other slot is unobserved (single-use
/// closure) and therefore Dead.
fn classified_slots(output_count: usize, reduction_slot: usize) -> Vec<LiveOutSlot> {
    let mut slots: Vec<LiveOutSlot> = (0..output_count)
        .filter(|pos| *pos != reduction_slot)
        .map(|pos| LiveOutSlot {
            kind: LiveOutKind::Slot(pos),
            binding: LiveOutBinding::Dead,
        })
        .collect();
    slots.push(LiveOutSlot {
        kind: LiveOutKind::Slot(reduction_slot),
        binding: LiveOutBinding::Reconstructed {
            slot: reduction_slot,
        },
    });
    slots.sort_by_key(|s| match s.kind {
        LiveOutKind::WholeValue => usize::MAX,
        LiveOutKind::Slot(p) => p,
    });
    slots
}

/// Classify EVERY observable use of the loop result.
///
/// The loop node is the region's observable interface: its users are
/// the only way values computed inside the loop escape. Classification
/// (advisor semantics):
///
/// - zero uses → every output is Dead (complete closure: nothing can
///   observe the loop's value);
/// - exactly one use, and it is either the whole result flowing to
///   `Return` (single-output loop) or a recognized extraction of the
///   reduction slot → Reconstructed (the candidate value replaces it
///   under the theorem);
/// - a projection of a NON-reduction slot → UnclassifiedUse (PS002:
///   the theorem does not cover that slot; silently returning a bound
///   constant for it is the canonical corruption);
/// - more than one use, or any unrecognized use form → UnclassifiedUse.
///
/// Unknown means NOT dead. Fail-closed.
pub fn classify_live_outs(
    function: &Function,
    map: &ReductionRoleMap,
) -> Result<Vec<LiveOutSlot>, BindingError> {
    let loop_node = function
        .get_node(map.loop_node)
        .ok_or(BindingError::UnsupportedShape("loop node missing"))?;
    let NodeKind::Loop { outputs, .. } = &loop_node.kind else {
        return Err(BindingError::UnsupportedShape("node is not a loop"));
    };

    let users = graph::users(map.loop_node, &function.arena);

    if users.is_empty() {
        // Zero observable uses: every output is dead. Complete
        // use-closure evidence — no use site exists anywhere.
        if outputs.len() == 1 {
            return Ok(vec![LiveOutSlot {
                kind: LiveOutKind::WholeValue,
                binding: LiveOutBinding::Dead,
            }]);
        }
        return Ok((0..outputs.len())
            .map(|pos| LiveOutSlot {
                kind: LiveOutKind::Slot(pos),
                binding: LiveOutBinding::Dead,
            })
            .collect());
    }

    if users.len() > 1 {
        // Recipes replace exactly one consumer; multiple observable
        // uses are not classified (unknown means not dead).
        return Err(BindingError::UnclassifiedUse {
            use_site: users[0],
            of_value: map.loop_node,
        });
    }

    let use_site = users[0];
    let use_node = function
        .get_node(use_site)
        .ok_or(BindingError::UnclassifiedUse {
            use_site,
            of_value: map.loop_node,
        })?;

    match &use_node.kind {
        // Whole-value consumption.
        NodeKind::Return { .. } => {
            if outputs.len() == 1 {
                Ok(vec![LiveOutSlot {
                    kind: LiveOutKind::WholeValue,
                    binding: LiveOutBinding::Reconstructed {
                        slot: map.reduction_position,
                    },
                }])
            } else {
                // PS002: a multi-element tuple returned wholesale has
                // unbound slots (index, position) the theorem does not
                // cover. Binding incomplete.
                Err(BindingError::UnclassifiedUse {
                    use_site,
                    of_value: map.loop_node,
                })
            }
        }
        // Slot consumption: FieldAccess "N" or TupleExtract N.
        NodeKind::FieldAccess { base, field } if *base == map.loop_node => {
            let slot: usize = field.parse().map_err(|_| BindingError::UnclassifiedUse {
                use_site,
                of_value: map.loop_node,
            })?;
            if slot == map.reduction_position {
                Ok(classified_slots(outputs.len(), slot))
            } else {
                // A slot the theorem does not cover (e.g. the index
                // live-out of a find-style loop) — never "preserved by
                // assumption".
                Err(BindingError::UnclassifiedUse {
                    use_site,
                    of_value: map.loop_node,
                })
            }
        }
        NodeKind::TupleExtract { tuple, index } if *tuple == map.loop_node => {
            if *index == map.reduction_position {
                Ok(classified_slots(outputs.len(), *index))
            } else {
                Err(BindingError::UnclassifiedUse {
                    use_site,
                    of_value: map.loop_node,
                })
            }
        }
        _ => Err(BindingError::UnclassifiedUse {
            use_site,
            of_value: map.loop_node,
        }),
    }
}

// ─────────────────────────────────────────────────────────────────
// ProposalBinding — the artifact consumed by all downstream stages
// ─────────────────────────────────────────────────────────────────

/// The complete binding of one concrete source region to a
/// transformation: the role map, the complete observable interface
/// (every live-out classified), and the application frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalBinding {
    pub map: ReductionRoleMap,
    pub live_outs: Vec<LiveOutSlot>,
    pub frame: FrameCondition,
}

impl ProposalBinding {
    /// A binding is complete only when the frame satisfies the
    /// conservative contract AND every observable live-out carries an
    /// explicit binding. Classification happens at derivation (an
    /// unclassified use is an error, not a field), so completeness
    /// here reduces to the frame contract plus at least one classified
    /// observable.
    pub fn is_complete(&self) -> bool {
        self.frame.supported_conservative() && !self.live_outs.is_empty()
    }

    /// Digest of the underlying role map (identifier, not proof).
    pub fn digest(&self) -> u64 {
        self.map.digest()
    }
}

/// Derive the full proposal binding for a reduction region:
/// role map + complete live-out classification + frame condition.
///
/// Fail-closed at every step. This is the single entry point the
/// application checker consumes; recipes, generation, and theorem
/// construction must consume the SAME binding (advisor invariant 6).
pub fn derive_proposal_binding(
    function: &Function,
    facts: &FactDatabase,
    structural: &StructuralDescription,
    concrete: &ConcreteFacts,
) -> Result<ProposalBinding, BindingError> {
    // 1. Roles.
    let roles = extract_reduction_roles(structural).ok_or(BindingError::NoRoles)?;

    // 2. Loop shape.
    let loop_node_kind = function
        .get_node(roles.loop_node)
        .ok_or(BindingError::UnsupportedShape("loop node missing"))?
        .kind
        .clone();
    let NodeKind::Loop {
        body,
        termination,
        outputs,
        carried_inputs,
    } = loop_node_kind
    else {
        return Err(BindingError::UnsupportedShape(
            "role result is not a loop node",
        ));
    };

    // 3. Certified recurrence.
    let loop_fact = facts
        .loops
        .get(&roles.loop_node)
        .ok_or(BindingError::UnsupportedShape(
            "region loop has no loop facts",
        ))?;
    let reduction = loop_fact
        .reductions
        .iter()
        .find(|r| match roles.accumulator {
            Some(acc) => r.variable == acc,
            None => true,
        })
        .ok_or(BindingError::NoReduction)?;
    // The authorization's certified accumulator (accumulators_are_
    // reassociable — the induction counter is excluded there) is the
    // authority. A role set disagreeing with it is a binding failure:
    // the role map must bind the SAME accumulator the checker certified.
    if let Some(certified) = concrete.accumulator {
        if certified != reduction.variable {
            return Err(BindingError::UnsupportedShape(
                "certified accumulator differs from the bound recurrence variable",
            ));
        }
    }

    // 4. Reduction position (the recurrence's output in the tuple).
    let reduction_position = carried_inputs
        .iter()
        .position(|c| *c == reduction.variable)
        .ok_or(BindingError::UnsupportedShape(
            "reduction carry not among loop carried inputs",
        ))?;
    // 5. Induction counter (unit stride contract).
    let induction = find_induction(function, &outputs, &carried_inputs).ok_or(
        BindingError::UnsupportedShape("no unit-stride induction counter (stride contract)"),
    )?;

    // 6. Bound: the termination comparison must test the induction
    //    counter (its carried value is the current index) against
    //    exactly one other value.
    let term = function
        .get_node(termination)
        .ok_or(BindingError::UnsupportedShape("termination node missing"))?;
    let term_inputs = term.kind.input_nodes();
    if term_inputs.len() != 2 {
        return Err(BindingError::UnsupportedShape(
            "termination is not a two-operand comparison",
        ));
    }
    let bound = if term_inputs[0] == induction.carry {
        Some(term_inputs[1])
    } else if term_inputs[1] == induction.carry {
        Some(term_inputs[0])
    } else {
        return Err(BindingError::UnsupportedShape(
            "termination does not compare the induction counter",
        ));
    };

    // 7. Element access bound to the collection.
    let mut element_access: Option<NodeId> = None;
    for node_id in body.iter().copied() {
        let Some(node) = function.get_node(node_id) else {
            continue;
        };
        match &node.kind {
            NodeKind::ArrayAccess { base, .. } if *base == roles.collection => {
                element_access = Some(node.id);
            }
            NodeKind::Load { ptr } => {
                if let Some(ptr_node) = function.get_node(*ptr) {
                    if let NodeKind::ArrayAccess { base, .. } = ptr_node.kind {
                        if base == roles.collection {
                            element_access = Some(node.id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let element_access = element_access.ok_or(BindingError::UnsupportedShape(
        "no element access bound to the collection",
    ))?;

    // 8. Identity: the accumulator's carried input must be a constant.
    let identity_carry = carried_inputs
        .get(reduction_position)
        .copied()
        .ok_or(BindingError::UnsupportedShape("accumulator carry missing"))?;
    let identity = match function.get_node(identity_carry) {
        Some(node) => match &node.kind {
            NodeKind::Constant(data) => Some(data.clone()),
            _ => None,
        },
        None => None,
    }
    .ok_or(BindingError::UnsupportedShape(
        "accumulator identity is not a constant",
    ))?;

    // 9. Live-ins: transitive dataflow inputs of the loop node
    //    (termination + carried inputs) UNION the transitive inputs of
    //    every body node. Nodes defined inside the loop subgraph (the
    //    body/outputs) are excluded — everything remaining is defined
    //    outside the loop and observed by it.
    let live_ins: Vec<NodeId> = {
        let mut set: HashSet<NodeId> = graph::transitive_inputs(roles.loop_node, &function.arena);
        for &n in body.iter() {
            set.extend(graph::transitive_inputs(n, &function.arena));
        }
        for &n in body.iter() {
            set.remove(&n);
        }
        for &n in outputs.iter() {
            set.remove(&n);
        }
        set.remove(&roles.loop_node);
        let mut v: Vec<NodeId> = set.into_iter().collect();
        v.sort();
        v
    };

    // 10. Effects: union over the loop and its body.
    let mut effects = Effects::empty();
    for node_id in std::iter::once(roles.loop_node).chain(body.iter().copied()) {
        if let Some(node) = function.get_node(node_id) {
            effects |= node.effects;
        }
    }

    let loop_node = roles.loop_node;
    let map = ReductionRoleMap {
        region: roles.region,
        loop_node,
        collection: roles.collection,
        element_access,
        induction: Some(induction.carry),
        start: Some(induction.carry),
        bound,
        stride: induction.stride,
        predicate: Some(reduction.invariant_value),
        accumulator: reduction.variable,
        recurrence: reduction.reduction_kind.clone(),
        identity: Some(identity),
        reduction_position,
        live_ins,
        effects,
        integer_semantics: IntegerSemantics::Modular,
    };

    // 11. Frame — fail-closed at binding time (advisor: incomplete
    //     bindings must not become legal candidates).
    let frame = build_frame(
        function,
        facts,
        roles.loop_node,
        &body,
        map.effects,
        outputs.len(),
        concrete,
    );
    if !frame.supported_conservative() {
        return Err(BindingError::FrameUnsupported("conservative contract"));
    }

    // 12. Complete use-closure classification of every observable.
    let live_outs = classify_live_outs(function, &map)?;

    Ok(ProposalBinding {
        map,
        live_outs,
        frame,
    })
}
