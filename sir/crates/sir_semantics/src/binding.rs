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

use std::collections::{BTreeSet, HashSet};

use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::{Function, NodeKind};
use sir_transform::roles::RegionRoles;
use sir_types::{ConstantData, Effects, IntegerWidth, NodeId, OverflowBehavior, RegionId, Type};

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
    /// A role cannot be uniquely identified: the region contains
    /// several legitimate candidates (e.g. two non-counter
    /// accumulators in one loop) and the concept does not trace to
    /// exactly one recurrence. No heuristic selection permitted.
    AmbiguousRole(&'static str),
    /// A supplied binding differs from a fresh canonical derivation.
    /// Validation must replay the binder instead of trusting a copied
    /// role map or digest.
    BindingMismatch { expected: u64, supplied: u64 },
    /// The concrete authorization requested by the application is not
    /// the issuer for this structural region.
    AuthorizationMismatch(&'static str),
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
            BindingError::AmbiguousRole(role) => {
                write!(
                    f,
                    "role '{role}' is ambiguous: no unique concept-to-recurrence identity"
                )
            }
            BindingError::BindingMismatch { expected, supplied } => write!(
                f,
                "proposal binding differs from canonical derivation: expected {expected:#x}, supplied {supplied:#x}"
            ),
            BindingError::AuthorizationMismatch(reason) => {
                write!(f, "authorization does not bind this region: {reason}")
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
/// Evidence for a `Dead` live-out classification: the complete
/// observable closure was checked against THIS function version. The
/// binding must be reconstructed from the current function before any
/// application — a Dead label from an earlier pass proves nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseClosureEvidence {
    /// Direct dataflow users of the loop node found in this function
    /// version (the complete observable boundary).
    pub direct_users: usize,
    /// Every transparent downstream node reached from the classified
    /// projection, including that projection. This records the closure
    /// grammar actually inspected rather than only its first edge.
    pub closure_nodes: Vec<NodeId>,
    /// Tuple slots observed by the closure. A dead slot is dead relative
    /// to this explicit observed-slot set, not merely because no first
    /// consumer was recognized.
    pub observed_slots: Vec<usize>,
    /// Fingerprint of the exact function version checked.
    pub function_fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveOutBinding {
    Preserved,
    Reconstructed {
        slot: usize,
    },
    /// No observable use exists in THIS exact function version.
    Dead {
        evidence: UseClosureEvidence,
    },
    Guarded {
        guard: NodeId,
    },
}

/// One classified observable of the loop result. `use_site` is the
/// node whose value the candidate replaces (the recognized projection
/// or the `Return`) — the ONLY node a recipe may target for this
/// live-out. Downstream users are recorded in `UseClosureEvidence`
/// for dead sibling slots; the replacement is value-identical
/// (`replace_all_uses`), so every transparent downstream consumer
/// observes an equal value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveOutSlot {
    pub kind: LiveOutKind,
    pub binding: LiveOutBinding,
    /// Complete transparent downstream closure checked from the
    /// recognized observable boundary. This is present for both live
    /// and dead slots; `Dead` additionally embeds the same evidence in
    /// its binding state for version-specific replay.
    pub closure: Option<UseClosureEvidence>,
    /// The recognized use site the rewrite replaces. `None` for Dead
    /// slots (nothing to replace).
    pub use_site: Option<NodeId>,
}

// ─────────────────────────────────────────────────────────────────
// Frame condition
// ─────────────────────────────────────────────────────────────────

/// Provenance for the exact counted-loop contract consumed by the
/// first forward reduction candidate. This is stronger than a generic
/// `is_finite` loop fact: it binds the start, comparison direction,
/// bound, stride, integer semantics, and collection extent together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TripCountEvidence {
    pub start: NodeId,
    pub start_value: u64,
    pub bound: NodeId,
    pub bound_value: usize,
    pub comparison: sir_nodes::CmpOperator,
    pub stride: i64,
    pub extent: usize,
    pub integer_width: IntegerWidth,
    pub signed: bool,
    pub overflow: OverflowBehavior,
    pub zero_trip_possible: bool,
    pub normal_termination: bool,
}

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
    /// Exact counted-loop evidence used to establish `terminates` and
    /// prevent partial scans, reverse scans, wraparound, or over-read.
    pub trip_count: Option<TripCountEvidence>,
}

impl FrameCondition {
    /// The conservative contract for the first vertical slice
    /// (advisor directive): pure loop, read-only nonvolatile memory,
    /// no atomics, no stores, no calls, no unmodeled traps, single
    /// normal exit, known finite iteration domain.
    /// Digest of every source-frame field, including counted-loop
    /// provenance. This is an identity key for application artifacts,
    /// never a substitute for deriving the frame from the current SIR.
    pub fn digest(&self) -> u64 {
        crate::binding::fnv1a64(format!("{:?}", self).as_bytes())
    }

    pub fn supported_conservative(&self) -> bool {
        !self.source_writes
            && !self.has_volatile_or_atomic
            && !self.has_calls
            && !self.possible_traps
            && self.terminates
            && self.single_normal_exit
            && self
                .trip_count
                .as_ref()
                .map(|evidence| evidence.normal_termination)
                .unwrap_or(false)
    }
}

// ─────────────────────────────────────────────────────────────────
// ReductionRoleMap
// ─────────────────────────────────────────────────────────────────

/// D4/S2: classification of the predicate scalar's dependency, bound
/// into the role map and its digest. A broadcast scalar is only
/// rewritable when it is loop-invariant; an index-dependent predicate
/// (e.g. `values[i] > i`) is a SEPARATE future candidate family and
/// must never pass under the invariant binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PredicateScalarClass {
    /// No predicate scalar (identity-only boolean accumulation).
    None,
    /// A constant literal (e.g. `> 0`).
    Constant,
    /// A function parameter or constant used directly.
    LiveInParameter,
    /// A pure derivation over invariants (e.g. a widening convert of a
    /// parameter) whose transitive inputs are all certified
    /// loop-invariant and non-memory-derived.
    DerivedInvariant,
}

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
    /// Iteration stride (only forward +1 is supported by the first
    /// candidate; reverse traversal refuses).
    pub stride: i64,
    /// The element predicate / map input combined into the accumulator.
    pub predicate: Option<NodeId>,
    /// Predicate collection: the concrete comparison operator the
    /// elements are tested with (bound from the structural role's
    /// operator node — never hardcoded in a recipe).
    pub predicate_op: Option<sir_nodes::CmpOperator>,
    /// Predicate collection: the scalar operand elements are compared
    /// against.
    pub predicate_scalar: Option<NodeId>,
    /// D4/S2: certified dependency class of the predicate scalar
    /// (transitively loop-invariant, non-memory-derived). Part of the
    /// artifact digest — mutating the scalar changes the class and
    /// the digest together.
    pub predicate_scalar_class: PredicateScalarClass,
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
            format!("region={}", self.region.0),
            format!("loop={}", self.loop_node.0),
            format!("collection={}", self.collection.0),
            format!("element_access={}", self.element_access.0),
            format!("induction={:?}", self.induction),
            format!("start={:?}", self.start),
            format!("bound={:?}", self.bound),
            format!("stride={}", self.stride),
            format!("predicate={:?}", self.predicate),
            format!("predicate_op={:?}", self.predicate_op),
            format!("predicate_scalar={:?}", self.predicate_scalar),
            format!("predicate_scalar_class={:?}", self.predicate_scalar_class),
            format!("accumulator={}", self.accumulator.0),
            format!("identity={:?}", self.identity),
            format!("reduction_position={}", self.reduction_position),
            format!("live_ins={:?}", self.live_ins),
            format!("recurrence={}", self.recurrence),
            format!("effects={:?}", self.effects),
            format!("integer_semantics={:?}", self.integer_semantics),
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
    /// Predicate role: the comparison node id (None for plain boolean
    /// collections).
    predicate_operator: Option<NodeId>,
    /// Predicate role: the compared-against scalar (None for plain
    /// boolean collections).
    predicate_scalar: Option<NodeId>,
}

fn extract_reduction_roles(
    structural: &StructuralDescription,
) -> Result<Option<RegionReductionRoles>, BindingError> {
    let mut found = None;
    for role in &structural.roles {
        let candidate = match role {
            RegionRoles::BooleanCollectionReduction {
                collection,
                accumulator,
                result,
            } => Some(RegionReductionRoles {
                region: structural.region,
                collection: *collection,
                accumulator: *accumulator,
                loop_node: *result,
                predicate_operator: None,
                predicate_scalar: None,
            }),
            RegionRoles::PredicateCollectionReduction {
                collection,
                scalar,
                operator,
                accumulator,
                result,
            } => Some(RegionReductionRoles {
                region: structural.region,
                collection: *collection,
                accumulator: *accumulator,
                loop_node: *result,
                predicate_operator: Some(*operator),
                predicate_scalar: Some(*scalar),
            }),
            _ => None,
        };
        if let Some(candidate) = candidate {
            if found.is_some() {
                return Err(BindingError::AmbiguousRole("reduction role"));
            }
            found = Some(candidate);
        }
    }
    Ok(found)
}

/// The integer value of an integer constant node, if any.
fn constant_i64(function: &Function, id: NodeId) -> Option<i64> {
    match &function.get_node(id)?.kind {
        NodeKind::Constant(ConstantData::Integer { value, .. }) => value.parse::<i64>().ok(),
        _ => None,
    }
}

/// Locate the loop's induction counter. FORWARD-ONLY for the first
/// vertical slice: a carried variable whose paired output is
/// `carry + 1`. Every current candidate implements forward reduction;
/// a reverse traversal must refuse here rather than trust the candidate
/// to match.
struct Induction {
    carry: NodeId,
    stride: i64,
}

fn find_induction(
    function: &Function,
    outputs: &[NodeId],
    carried_inputs: &[NodeId],
) -> Result<Induction, BindingError> {
    let mut candidates = Vec::new();
    for (&carry, &output) in carried_inputs.iter().zip(outputs.iter()) {
        let Some(node) = function.get_node(output) else {
            continue;
        };
        let step: Option<i64> = match &node.kind {
            NodeKind::Add { lhs, rhs } if *lhs == carry => constant_i64(function, *rhs),
            _ => None,
        };
        if let Some(step) = step {
            if step == 1 {
                candidates.push(Induction {
                    carry,
                    stride: step,
                });
            }
        }
    }
    match candidates.as_slice() {
        [] => Err(BindingError::UnsupportedShape(
            "no unit-stride induction counter (stride contract)",
        )),
        [induction] => Ok(Induction {
            carry: induction.carry,
            stride: induction.stride,
        }),
        _ => Err(BindingError::AmbiguousRole("induction")),
    }
}

/// Prove the exact forward counted-loop contract required by the
/// first reduction candidate. The generic loop analysis is deliberately
/// not authoritative here: it does not prove comparison direction,
/// zero-based start, full collection extent, or wraparound safety.
fn derive_trip_count(
    function: &Function,
    facts: &FactDatabase,
    loop_node: NodeId,
    termination: NodeId,
    induction: &Induction,
    bound: NodeId,
    collection: NodeId,
) -> Result<TripCountEvidence, BindingError> {
    let loop_fact = facts
        .loops
        .get(&loop_node)
        .ok_or(BindingError::UnsupportedShape(
            "region loop has no loop facts",
        ))?;
    if !loop_fact.is_finite {
        return Err(BindingError::UnsupportedShape(
            "counted loop is not proven finite",
        ));
    }

    let term = function
        .get_node(termination)
        .ok_or(BindingError::UnsupportedShape("termination node missing"))?;
    match &term.kind {
        NodeKind::Lt { lhs, rhs } if *lhs == induction.carry && *rhs == bound => {}
        _ => {
            return Err(BindingError::UnsupportedShape(
                "forward reduction requires induction < bound termination",
            ));
        }
    }

    let collection_length = match function.get_node(collection).map(|node| &node.ty) {
        Some(Type::Array { length, .. }) => *length,
        _ => {
            return Err(BindingError::UnsupportedShape(
                "collection has no fixed extent for counted-loop proof",
            ));
        }
    };
    let start_value = constant_i64(function, induction.carry)
        .filter(|value| *value >= 0)
        .map(|value| value as u64)
        .ok_or(BindingError::UnsupportedShape(
            "induction start is not a non-negative constant",
        ))?;
    if start_value != 0 {
        return Err(BindingError::UnsupportedShape(
            "forward reduction requires zero-based induction start",
        ));
    }
    let bound_value = constant_i64(function, bound)
        .filter(|value| *value >= 0)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(BindingError::UnsupportedShape(
            "induction bound is not a non-negative constant",
        ))?;
    if bound_value != collection_length {
        return Err(BindingError::UnsupportedShape(
            "counted loop does not cover the complete collection extent",
        ));
    }

    let induction_node =
        function
            .get_node(induction.carry)
            .ok_or(BindingError::UnsupportedShape(
                "induction start node missing",
            ))?;
    let bound_node = function
        .get_node(bound)
        .ok_or(BindingError::UnsupportedShape(
            "induction bound node missing",
        ))?;
    let (integer_width, signed, overflow) = match (&induction_node.ty, &bound_node.ty) {
        (
            Type::Integer {
                width,
                signed,
                overflow,
            },
            Type::Integer {
                width: bound_width,
                signed: bound_signed,
                overflow: bound_overflow,
            },
        ) if width == bound_width && signed == bound_signed && overflow == bound_overflow => {
            (*width, *signed, *overflow)
        }
        _ => {
            return Err(BindingError::UnsupportedShape(
                "induction and bound integer semantics differ",
            ));
        }
    };
    if signed {
        return Err(BindingError::UnsupportedShape(
            "forward reduction requires unsigned induction semantics",
        ));
    }
    Ok(TripCountEvidence {
        start: induction.carry,
        start_value,
        bound,
        bound_value,
        comparison: sir_nodes::CmpOperator::Lt,
        stride: induction.stride,
        extent: collection_length,
        integer_width,
        signed,
        overflow,
        zero_trip_possible: collection_length == 0,
        normal_termination: true,
    })
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
    trip_count: Option<TripCountEvidence>,
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

    let has_volatile_or_atomic = effects.contains(sir_types::Effects::ATOMIC)
        || effects.contains(sir_types::Effects::VOLATILE)
        || effects.contains(sir_types::Effects::IO);

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
            .unwrap_or(false)
            && trip_count.is_some(),
        single_normal_exit: true,
        output_count,
        trip_count,
    }
}

// ─────────────────────────────────────────────────────────────────
// Complete use-closure live-out classification
// ─────────────────────────────────────────────────────────────────

/// Walk every transparent downstream consumer of a recognized projection.
/// Equality replacement is valid for these consumers because the recipe
/// replaces the projection value itself; an unknown direct loop use still
/// refuses before this walk is entered.
fn transitive_use_closure(function: &Function, root: NodeId) -> Vec<NodeId> {
    let mut visited = BTreeSet::new();
    let mut worklist = vec![root];
    while let Some(node) = worklist.pop() {
        if !visited.insert(node) {
            continue;
        }
        worklist.extend(graph::users(node, &function.arena));
    }
    visited.into_iter().collect()
}

/// D4/S1: is this constant the monoid identity of the recurrence that
/// the reduction theorem actually proves?
///
/// The Any theorem proves `acc = ∨ predicate`, whose identity is
/// `false` (Boolean) or `0` (integer/bitwise). An OR seeded with
/// `true` computes an always-true value — it is NOT Any, and rewriting
/// it as `Pack(...) != 0` corrupts the program (H3 h3a23/h3a24).
/// Bitwise-And seeding (future All family) requires `true` / all-ones
/// (for the declared width). Sum/Xor require `0`; Product requires `1`.
pub fn monoid_identity_ok(recurrence: &str, identity: &sir_types::ConstantData) -> bool {
    use sir_types::{ConstantData, IntegerWidth};
    let is_zero = match identity {
        ConstantData::Bool(b) => !*b,
        ConstantData::Integer { value, .. } => value == "0",
        _ => false,
    };
    let is_one = match identity {
        ConstantData::Bool(b) => *b,
        ConstantData::Integer { value, .. } => value == "1",
        _ => false,
    };
    let is_all_ones = match identity {
        ConstantData::Bool(b) => *b,
        ConstantData::Integer {
            value,
            width,
            signed,
        } => {
            // All-ones for the declared width: 2^w - 1, or -1 as a
            // signed constant of that width.
            let width_bits: u32 = match width {
                IntegerWidth::I8 => 8,
                IntegerWidth::I16 => 16,
                IntegerWidth::I32 => 32,
                IntegerWidth::I64 => 64,
                IntegerWidth::I128 => 128,
                _ => return false,
            };
            let all_ones = (1u128 << width_bits) - 1;
            if *signed {
                value.parse::<i128>() == Ok(-1)
            } else {
                value.parse::<u128>().map(|v| v == all_ones).unwrap_or(false)
            }
        }
        _ => false,
    };
    match recurrence {
        "bitwise_or" => is_zero,
        "bitwise_xor" => is_zero,
        "sum" => is_zero,
        "bitwise_and" => is_all_ones,
        "product" => is_one,
        _ => false, // unknown recurrence: never assume an identity
    }
}

/// D4/S2: certify that `scalar` is loop-invariant by walking its full
/// transitive dataflow closure. Refuses (returns `UnsupportedShape`)
/// when the closure reaches any loop-carried input, any loop output,
/// any memory/shape/call-derived operation, or any unknown kind.
/// The only permitted closure members are pure invariants: constants,
/// parameters, and widening/narrowing converts over invariants.
///
/// Index-dependent predicates (`values[i] > i` where `i` is the
/// induction counter) are a separate future candidate family; they are
/// refused here, never misclassified.
pub fn certify_invariant_scalar(
    function: &Function,
    body: &[NodeId],
    outputs: &[NodeId],
    carried_inputs: &[NodeId],
    scalar: NodeId,
    collection: NodeId,
    loop_node: NodeId,
) -> Result<PredicateScalarClass, BindingError> {
    if scalar == collection || scalar == loop_node {
        return Err(BindingError::UnsupportedShape(
            "predicate scalar is the collection or loop node",
        ));
    }
    if body.contains(&scalar) || outputs.contains(&scalar) {
        return Err(BindingError::UnsupportedShape(
            "predicate scalar is defined inside the loop (not invariant)",
        ));
    }

    // Transitive dataflow closure of the scalar.
    let mut closure: Vec<NodeId> = Vec::new();
    let mut seen = HashSet::new();
    let mut worklist = vec![scalar];
    while let Some(node_id) = worklist.pop() {
        if !seen.insert(node_id) {
            continue;
        }
        closure.push(node_id);
        if let Some(node) = function.get_node(node_id) {
            worklist.extend(node.kind.input_nodes());
        }
    }

    // Any loop-carried value or loop output in the closure means the
    // scalar varies across iterations (H3 h3a21 poison: the carried
    // index-start constant is per-iteration in loop semantics).
    let forbidden: HashSet<NodeId> =
        carried_inputs.iter().chain(outputs.iter()).copied().collect();
    for node_id in &closure {
        if forbidden.contains(node_id) {
            return Err(BindingError::UnsupportedShape(
                "predicate scalar depends on a loop-carried value (not invariant)",
            ));
        }
    }

    // Permitted pure invariant kinds only; anything memory-derived,
    // shape-derived, call-derived, or a nested loop is unknown state.
    let mut kind: Option<PredicateScalarClass> = None;
    for node_id in &closure {
        let Some(node) = function.get_node(*node_id) else {
            return Err(BindingError::UnsupportedShape(
                "predicate scalar closure reaches a missing node",
            ));
        };
        match &node.kind {
            NodeKind::Constant(_) | NodeKind::Parameter { .. } => {
                if kind.is_none() {
                    kind = Some(if *node_id == scalar {
                        match &node.kind {
                            NodeKind::Constant(_) => PredicateScalarClass::Constant,
                            _ => PredicateScalarClass::LiveInParameter,
                        }
                    } else {
                        PredicateScalarClass::LiveInParameter
                    });
                }
            }
            NodeKind::Convert { .. } => {
                kind.get_or_insert(PredicateScalarClass::DerivedInvariant);
            }
            _ => {
                return Err(BindingError::UnsupportedShape(
                    "predicate scalar depends on memory/shape/call-derived or unknown state",
                ));
            }
        }
    }
    Ok(kind.unwrap_or(PredicateScalarClass::DerivedInvariant))
}

fn classified_slots(
    output_count: usize,
    reduction_slot: usize,
    evidence: UseClosureEvidence,
    projection: NodeId,
) -> Vec<LiveOutSlot> {
    let mut slots: Vec<LiveOutSlot> = (0..output_count)
        .filter(|pos| *pos != reduction_slot)
        .map(|pos| LiveOutSlot {
            kind: LiveOutKind::Slot(pos),
            binding: LiveOutBinding::Dead {
                evidence: evidence.clone(),
            },
            closure: Some(evidence.clone()),
            use_site: None,
        })
        .collect();
    slots.push(LiveOutSlot {
        kind: LiveOutKind::Slot(reduction_slot),
        binding: LiveOutBinding::Reconstructed {
            slot: reduction_slot,
        },
        closure: Some(evidence),
        use_site: Some(projection),
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

    // D4 refinement: a direct user that is BOTH pure and has zero
    // downstream users cannot be observed by any execution of this
    // exact function version. Counting such provably-dead projections
    // as "consumers" manufactured a second consumer that blocked
    // otherwise sound rewrites (lowered loops materialize a dead index
    // TupleExtract; debug/tracing projections are equivalent).
    // Soundness: the rewrite replaces the loop node; the dead
    // projection then references nothing reachable from Return and is
    // removed by RewriteBuilder's mark-and-sweep DCE. `Return` always
    // counts as observable (it IS the function's result).
    let observable: Vec<NodeId> = users
        .iter()
        .copied()
        .filter(|user_id| {
            let Some(node) = function.get_node(*user_id) else {
                return true; // cannot classify a missing node — keep it observable
            };
            if matches!(node.kind, NodeKind::Return { .. }) {
                return true;
            }
            !(node.effects.is_pure() && graph::users(*user_id, &function.arena).is_empty())
        })
        .collect();
    let raw_user_count = users.len();

    if observable.is_empty() {
        // Zero observable uses: every output is dead. Complete
        // use-closure evidence — no use site exists anywhere in this
        // exact function version. (Any provably-dead projections are
        // included in the count for evidence fidelity.)
        let evidence = UseClosureEvidence {
            direct_users: raw_user_count,
            closure_nodes: Vec::new(),
            observed_slots: Vec::new(),
            function_fingerprint: crate::authorization::function_fingerprint(function),
        };
        if outputs.len() == 1 {
            return Ok(vec![LiveOutSlot {
                kind: LiveOutKind::WholeValue,
                binding: LiveOutBinding::Dead {
                    evidence: evidence.clone(),
                },
                closure: Some(evidence),
                use_site: None,
            }]);
        }
        return Ok((0..outputs.len())
            .map(|pos| LiveOutSlot {
                kind: LiveOutKind::Slot(pos),
                binding: LiveOutBinding::Dead {
                    evidence: evidence.clone(),
                },
                closure: Some(evidence.clone()),
                use_site: None,
            })
            .collect());
    }

    if observable.len() > 1 {
        // Recipes replace exactly one consumer; multiple observable
        // uses are not classified (unknown means not dead).
        return Err(BindingError::UnclassifiedUse {
            use_site: observable[0],
            of_value: map.loop_node,
        });
    }

    let use_site = observable[0];
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
                    closure: Some(UseClosureEvidence {
                        direct_users: users.len(),
                        closure_nodes: transitive_use_closure(function, map.loop_node),
                        observed_slots: vec![map.reduction_position],
                        function_fingerprint: crate::authorization::function_fingerprint(function),
                    }),
                    use_site: Some(use_site),
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
                let evidence = UseClosureEvidence {
                    direct_users: users.len(),
                    closure_nodes: transitive_use_closure(function, use_site),
                    observed_slots: vec![slot],
                    function_fingerprint: crate::authorization::function_fingerprint(function),
                };
                Ok(classified_slots(outputs.len(), slot, evidence, use_site))
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
                let evidence = UseClosureEvidence {
                    direct_users: users.len(),
                    closure_nodes: transitive_use_closure(function, use_site),
                    observed_slots: vec![*index],
                    function_fingerprint: crate::authorization::function_fingerprint(function),
                };
                Ok(classified_slots(outputs.len(), *index, evidence, use_site))
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

/// Validate a supplied binding by re-running the canonical binder on
/// the current function and comparing the complete artifact. A digest
/// is diagnostic only; equality is checked after reconstruction.
pub fn validate_proposal_binding(
    function: &Function,
    facts: &FactDatabase,
    structural: &StructuralDescription,
    concrete: &ConcreteFacts,
    supplied: &ProposalBinding,
) -> Result<(), BindingError> {
    let expected = derive_proposal_binding(function, facts, structural, concrete)?;
    if expected == *supplied {
        Ok(())
    } else {
        Err(BindingError::BindingMismatch {
            expected: expected.digest(),
            supplied: supplied.digest(),
        })
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
    let roles = extract_reduction_roles(structural)?.ok_or(BindingError::NoRoles)?;

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
    // Role identity chain (advisor): recognized reduction concept →
    // certificate accumulator (role) → proposal accumulator. NO
    // heuristic selection among multiple non-counter recurrences — a
    // loop carrying `sum += value; count += predicate(value)` has two
    // legitimate accumulators and the concept must trace to exactly
    // ONE. First/last/node-order selection is forbidden.
    let non_counter: Vec<_> = loop_fact
        .reductions
        .iter()
        .filter(|r| !crate::authorization::is_unit_counter(function, r))
        .collect();
    let reduction = match roles.accumulator {
        Some(acc) => {
            let matches: Vec<_> = non_counter
                .iter()
                .copied()
                .filter(|r| r.variable == acc)
                .collect();
            match matches.as_slice() {
                [reduction] => *reduction,
                [] => return Err(BindingError::NoReduction),
                _ => return Err(BindingError::AmbiguousRole("accumulator")),
            }
        }
        None => {
            if non_counter.len() != 1 {
                return Err(BindingError::AmbiguousRole("accumulator"));
            }
            non_counter[0]
        }
    };
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
    let reduction_positions: Vec<usize> = carried_inputs
        .iter()
        .enumerate()
        .filter_map(|(position, carry)| (*carry == reduction.variable).then_some(position))
        .collect();
    let reduction_position = match reduction_positions.as_slice() {
        [position] => *position,
        [] => {
            return Err(BindingError::UnsupportedShape(
                "reduction carry not among loop carried inputs",
            ));
        }
        _ => return Err(BindingError::AmbiguousRole("reduction position")),
    };
    // 5. Induction counter (unit stride contract).
    let induction = find_induction(function, &outputs, &carried_inputs)?;

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
        term_inputs[1]
    } else if term_inputs[1] == induction.carry {
        term_inputs[0]
    } else {
        return Err(BindingError::UnsupportedShape(
            "termination does not compare the induction counter",
        ));
    };
    if bound == induction.carry {
        return Err(BindingError::AmbiguousRole("bound"));
    }
    let trip_count = derive_trip_count(
        function,
        facts,
        roles.loop_node,
        termination,
        &induction,
        bound,
        roles.collection,
    )?;

    // 7. Element access bound to the collection. The loop body is
    //    treated as a SET: compiler lowering may list one access node
    //    twice (D4/F9 body normalization artifact) and a duplicate must
    //    not fabricate an "ambiguous element access" role.
    let mut element_accesses = Vec::new();
    let mut seen_body = HashSet::new();
    for node_id in body.iter().copied() {
        if !seen_body.insert(node_id) {
            continue;
        }
        let Some(node) = function.get_node(node_id) else {
            continue;
        };
        match &node.kind {
            NodeKind::ArrayAccess { base, index }
                if *base == roles.collection && *index == induction.carry =>
            {
                element_accesses.push(node.id);
            }
            NodeKind::Load { ptr } => {
                if let Some(ptr_node) = function.get_node(*ptr) {
                    if let NodeKind::ArrayAccess { base, index } = ptr_node.kind {
                        if base == roles.collection && index == induction.carry {
                            element_accesses.push(node.id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let element_access = match element_accesses.as_slice() {
        [element_access] => *element_access,
        [] => {
            return Err(BindingError::UnsupportedShape(
                "no element access bound to the collection",
            ));
        }
        _ => return Err(BindingError::AmbiguousRole("element access")),
    };

    // 8. Identity: the accumulator's carried input must be a constant
    //    AND the monoid identity of the recurrence (D4/S1). The Any
    //    law is `acc' = acc ∨ predicate` with identity `false` (Or).
    //    An identity `true` computes an always-true reduction that is
    //    NOT Any: rewriting it corrupts the program (H3 h3a23/h3a24).
    //    The validated identity remains part of the role-map digest,
    //    so mutating it changes the concrete artifact.
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
    if !monoid_identity_ok(&reduction.reduction_kind, &identity) {
        return Err(BindingError::UnsupportedShape(
            "accumulator identity is not the recurrence's monoid identity",
        ));
    }

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

    // 8b. Predicate roles: the comparison operator node and scalar
    //     operand bound from the structural role (the binder resolves
    //     the op; a recipe must never hardcode or rediscover it).
    let (predicate_op, predicate_scalar, predicate_scalar_class) =
        match (roles.predicate_operator, roles.predicate_scalar) {
            (Some(op_node), Some(scalar)) => {
                let op_node_ref = function
                    .get_node(op_node)
                    .ok_or(BindingError::UnsupportedShape(
                        "predicate operator node missing",
                    ))?;
            let op = match &op_node_ref.kind {
                sir_nodes::NodeKind::Eq { .. } => sir_nodes::CmpOperator::Eq,
                sir_nodes::NodeKind::Ne { .. } => sir_nodes::CmpOperator::Ne,
                sir_nodes::NodeKind::Lt { .. } => sir_nodes::CmpOperator::Lt,
                sir_nodes::NodeKind::Le { .. } => sir_nodes::CmpOperator::Le,
                sir_nodes::NodeKind::Gt { .. } => sir_nodes::CmpOperator::Gt,
                sir_nodes::NodeKind::Ge { .. } => sir_nodes::CmpOperator::Ge,
                _ => {
                    return Err(BindingError::UnsupportedShape(
                        "predicate operator node is not a comparison",
                    ));
                }
            };
            let predicate_inputs = op_node_ref.kind.input_nodes();
            if predicate_inputs.len() != 2
                || predicate_inputs[0] != element_access
                || predicate_inputs[1] != scalar
            {
                return Err(BindingError::UnsupportedShape(
                    "predicate operator is not bound to the element access and scalar",
                ));
            }
            // D4/S2: the broadcast scalar must be certified LOOP-INVARIANT
            // by transitive dependency analysis — it must not depend on the
            // induction counter, the accumulator, any other loop-carried
            // value, loop outputs, mutable memory, or unknown state. The
            // former membership test passed the poison case
            // `values[i] > Convert(carried-const)` because the Convert node
            // wrapping the per-iteration carried value was itself neither in
            // the body nor equal to the induction carry (H3 h3a21). The
            // certification below walks the scalar's full dataflow closure.
            let scalar_class = certify_invariant_scalar(
                function,
                &body,
                &outputs,
                &carried_inputs,
                scalar,
                roles.collection,
                roles.loop_node,
            )?;
            (Some(op), Some(scalar), scalar_class)
        }
            _ => (None, None, PredicateScalarClass::None),
        };

    let loop_node = roles.loop_node;
    let map = ReductionRoleMap {
        region: roles.region,
        loop_node,
        collection: roles.collection,
        element_access,
        induction: Some(induction.carry),
        start: Some(induction.carry),
        bound: Some(bound),
        stride: induction.stride,
        predicate: Some(reduction.invariant_value),
        predicate_op,
        predicate_scalar,
        predicate_scalar_class,
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
        Some(trip_count),
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
