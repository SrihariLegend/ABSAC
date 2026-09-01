//! Transformation authorization — the gate between inference and generation.
//!
//! Architecture (advisor directive, Gate 6A-v2 finding X06):
//!
//! ```text
//! Verified SIR → Truths → Beliefs → Complete concrete binding
//!     → Eligibility certificate → Transformation authorization
//!     → Candidate generation → Concrete equivalence → Rewrite
//! ```
//!
//! There is NO direct path Truth → Candidate or Belief → Candidate.
//! Truths and beliefs are EVIDENCE, not authorization. A candidate is
//! admitted only when a `TransformationAuthorization`, derived from a
//! complete certificate, covers its source concepts. Uncertified
//! regions generate zero candidates.
//!
//! Two certificate layers:
//!
//! 1. `RegionInterfaceCertificate` — the closed-world guarantee: ABSAC
//!    has bound every memory access, effect, and control-flow edge of
//!    the region. Required by every transformation.
//!
//! 2. `DomainCertificate` — domain-specific semantics:
//!    - `Reduction` — operator, accumulator recurrence, integer
//!      semantics (overflow/poison modeling — finding X02).
//!    - `PositionSearch` — the result must bind the INDEX (X06: a
//!      running-max loop's select binds memory values, not positions).
//!    - `ScalarExpression` — closed-world pure scalar region.

use std::collections::{HashMap, HashSet};

use sir_analysis::facts::FactDatabase;
use sir_analysis::graph;
use sir_nodes::{Function, NodeKind};
use sir_types::{Effects, NodeId, RegionId};

use crate::certificate::{memory_footprint, FootprintCheck};
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;

// ─────────────────────────────────────────────────────────────────
// Integer semantics (X02 — overflow/poison modeling)
// ─────────────────────────────────────────────────────────────────

/// Integer overflow semantics of an accumulator recurrence.
///
/// `nsw`/`nuw` do NOT mean "trap on overflow" — they mean overflow
/// produces POISON in LLVM semantics, with consequences depending on
/// use. A transformation must preserve the refinement relation for
/// defined source executions. Vector reassociation
/// (`((a+b)+c)+d` → `(a+b)+(c+d)`) is valid under Modular arithmetic
/// but can differ under poison semantics: one grouping may overflow
/// while the other does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerSemantics {
    /// Two's-complement wrapping — reassociation is valid.
    Modular,
    /// Overflow produces poison (LLVM `nsw`).
    PoisonSigned,
    /// Overflow produces poison (LLVM `nuw`).
    PoisonUnsigned,
    /// Overflow produces poison (LLVM `nsw nuw`).
    PoisonBoth,
    /// Semantics unknown or unsupported (trapping, saturating…).
    Unknown,
}

impl IntegerSemantics {
    /// Can this accumulation be freely reassociated (e.g., into a
    /// vector reduction tree) without changing defined behavior?
    /// Only modular arithmetic allows reassociation without a proof.
    /// Poison semantics require a range proof (not yet implemented),
    /// so they force abstention.
    pub fn allows_reassociation(&self) -> bool {
        matches!(self, IntegerSemantics::Modular)
    }
}

/// Read the overflow flag recorded by the lowerer from LLVM op flags.
fn overflow_semantics_of(func: &Function, id: NodeId) -> IntegerSemantics {
    let Some(node) = func.get_node(id) else {
        return IntegerSemantics::Unknown;
    };
    match node.metadata.get("llvm.overflow") {
        Some("nsw") => IntegerSemantics::PoisonSigned,
        Some("nuw") => IntegerSemantics::PoisonUnsigned,
        Some("nsw+nuw") => IntegerSemantics::PoisonBoth,
        _ => IntegerSemantics::Modular,
    }
}
// ─────────────────────────────────────────────────────────────────
// Region interface certificate
// ─────────────────────────────────────────────────────────────────

/// The closed-world guarantee: every observable effect of the region
/// is declared. Every authorization requires this certificate.
#[derive(Clone, Debug)]
pub struct RegionInterfaceCertificate {
    /// Exact source node set covered.
    pub region_nodes: Vec<NodeId>,
    /// Memory bases read (resolved to root parameters).
    pub memory_bases: Vec<NodeId>,
    /// Whether the region writes memory (currently never authorized).
    pub has_memory_writes: bool,
    pub has_calls: bool,
    pub has_volatile: bool,
    pub has_atomics: bool,
    /// True when every node/basis resolved — ABSAC has a closed-world view.
    pub complete: bool,
    /// Why the interface is incomplete (for audit trails).
    pub incompleteness_reason: Option<String>,
}

impl RegionInterfaceCertificate {
    /// Build a region interface certificate from a region's node set.
    ///
    /// Checks effects over the region nodes AND their transitive inputs
    /// (a call or volatile op reachable from region nodes is part of the
    /// region's behavior even if the call node itself is outside).
    pub fn build(func: &Function, region_nodes: &[NodeId]) -> Self {
        let mut cert = RegionInterfaceCertificate {
            region_nodes: region_nodes.to_vec(),
            memory_bases: Vec::new(),
            has_memory_writes: false,
            has_calls: false,
            has_volatile: false,
            has_atomics: false,
            complete: true,
            incompleteness_reason: None,
        };

        // Region nodes plus their transitive inputs.
        let mut all_nodes: Vec<NodeId> = region_nodes.to_vec();
        for &n in region_nodes {
            for input in graph::transitive_inputs(n, &func.arena) {
                if !all_nodes.contains(&input) {
                    all_nodes.push(input);
                }
            }
        }

        for id in &all_nodes {
            let Some(node) = func.get_node(*id) else { continue };
            if node.effects.contains(Effects::WRITE_MEMORY) {
                cert.has_memory_writes = true;
            }
            if node.effects.contains(Effects::IO) {
                cert.has_calls = true;
            }
            if node.effects.contains(Effects::VOLATILE) {
                cert.has_volatile = true;
            }
            if node.effects.contains(Effects::ATOMIC) {
                cert.has_atomics = true;
            }
            if matches!(
                node.kind,
                NodeKind::Call { .. } | NodeKind::ExternalCall { .. }
            ) {
                cert.has_calls = true;
            }
        }

        // Memory footprint (reads)
        match memory_footprint(func, region_nodes) {
            FootprintCheck::Complete(bases) => {
                cert.memory_bases = bases.into_iter().map(|b| b.base).collect();
            }
            FootprintCheck::Incomplete(why) => {
                cert.complete = false;
                cert.incompleteness_reason = Some(why);
            }
        }

        if cert.has_memory_writes || cert.has_calls || cert.has_volatile || cert.has_atomics {
            cert.complete = false;
            cert.incompleteness_reason = Some(format!(
                "region effects exceed the closed world (writes={}, calls={}, volatile={}, atomic={})",
                cert.has_memory_writes, cert.has_calls, cert.has_volatile, cert.has_atomics
            ));
        }

        cert
    }
}

// ─────────────────────────────────────────────────────────────────
// Domain certificates
// ─────────────────────────────────────────────────────────────────

/// Domain-specific certificate. The region interface certificate proves
/// the region is closed-world; the domain certificate states what the
/// region IS and which transformations that semantics supports.
#[derive(Clone, Debug)]
pub enum DomainCertificate {
    /// Reduction over an iteration domain.
    Reduction {
        operator: ReductionOperator,
        /// Overflow semantics of the accumulator recurrence (X02).
        integer_semantics: IntegerSemantics,
        /// Whether the iteration counter steps by constant 1.
        stride_is_unit: bool,
    },
    /// Position search. `result_binds_index` is the X06 invariant: the
    /// search's result select must bind the position/index — never a
    /// memory-derived value (which would indicate min/max or a
    /// value-returning recurrence, not a position search).
    PositionSearch { result_binds_index: bool },
    /// Closed-world pure scalar region — no memory, no calls, no
    /// hidden state. Authorizes scalar bit-manipulation families.
    ScalarExpression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReductionOperator {
    Cardinality,
    Sum,
    All,
    Any,
    Exclusive,
    Disjunctive,
}

/// Operation concepts a closed-world PURE-SCALAR region is authorized
/// for. Memory-touching regions never receive these via ScalarExpression;
/// they need a matching domain certificate (Reduction / PositionSearch).

/// Reduction-domain operation concepts.
fn is_reduction_concept(c: &SemanticConcept) -> bool {
    matches!(
        c,
        SemanticConcept::CardinalityReduction
            | SemanticConcept::SumReduction
            | SemanticConcept::ConjunctiveReduction
            | SemanticConcept::DisjunctiveReduction
            | SemanticConcept::ExclusiveReduction
    )
}

/// Concepts authorized by a PositionSearch certificate.
fn position_search_concepts() -> &'static [SemanticConcept] {
    const CONCEPTS: &[SemanticConcept] = &[
        SemanticConcept::FirstOccurrence,
        SemanticConcept::LastOccurrence,
    ];
    CONCEPTS
}

/// Scalar bit-manipulation op concepts grantable INSIDE a
/// reduction-certified region (node-level rewrites, equivalence-proved
/// downstream). PositionSearch is deliberately absent: it requires its
/// own certificate (X06).
fn scalar_bit_ops() -> &'static [SemanticConcept] {
    const CONCEPTS: &[SemanticConcept] = &[
        SemanticConcept::ModuloPowerOfTwo,
        SemanticConcept::MultiplyPowerOfTwo,
        SemanticConcept::DividePowerOfTwo,
        SemanticConcept::ShiftMask,
        SemanticConcept::Parity,
        SemanticConcept::TrailingZeroSearch,
        SemanticConcept::LeadingZeroSearch,
        SemanticConcept::BitsetIteration,
        SemanticConcept::LowestSetBit,
        SemanticConcept::LowestClearBitMask,
        SemanticConcept::SetLowestClearBit,
        SemanticConcept::ClearLowestSetBit,
        SemanticConcept::IsZero,
        SemanticConcept::CircularPermutation,
        SemanticConcept::BytePermutation,
        SemanticConcept::BitPermutation,
        SemanticConcept::AtMostOneBitSet,
        SemanticConcept::LoopUntilZero,
        SemanticConcept::ShiftPairLeft,
        SemanticConcept::ShiftPairRight,
        SemanticConcept::MaskedShiftSwap,
    ];
    CONCEPTS
}

/// Operation concepts a closed-world PURE-SCALAR region is authorized
/// for. Memory-touching regions never receive these via ScalarExpression;
/// they need a matching domain certificate (Reduction / PositionSearch).
fn scalar_op_concepts() -> &'static [SemanticConcept] {
    const CONCEPTS: &[SemanticConcept] = &[
        SemanticConcept::ModuloPowerOfTwo,
        SemanticConcept::MultiplyPowerOfTwo,
        SemanticConcept::DividePowerOfTwo,
        SemanticConcept::ShiftMask,
        SemanticConcept::Parity,
        SemanticConcept::TrailingZeroSearch,
        SemanticConcept::LeadingZeroSearch,
        SemanticConcept::BitsetIteration,
        SemanticConcept::LowestSetBit,
        SemanticConcept::LowestClearBitMask,
        SemanticConcept::SetLowestClearBit,
        SemanticConcept::ClearLowestSetBit,
        SemanticConcept::IsZero,
        SemanticConcept::CircularPermutation,
        SemanticConcept::BytePermutation,
        SemanticConcept::BitPermutation,
        SemanticConcept::SetMembership,
        SemanticConcept::SetUnion,
        SemanticConcept::SetIntersection,
        SemanticConcept::SetDifference,
        SemanticConcept::SetSymmetricDifference,
        SemanticConcept::SetSubset,
        SemanticConcept::SetEquality,
        SemanticConcept::SetEmpty,
        SemanticConcept::AtMostOneBitSet,
        SemanticConcept::LoopUntilZero,
        SemanticConcept::ShiftPairLeft,
        SemanticConcept::ShiftPairRight,
        SemanticConcept::MaskedShiftSwap,
    ];
    CONCEPTS
}

/// Concepts that DESCRIBE data/structure and never authorize a and never authorize a
/// transformation by themselves. Candidates may cite them freely; they
/// confer no transformation rights.
pub fn is_data_concept(c: &SemanticConcept) -> bool {
    matches!(
        c,
        SemanticConcept::LogicalSequence
            | SemanticConcept::FiniteCollection
            | SemanticConcept::FiniteSet
            | SemanticConcept::ElementSequence
            | SemanticConcept::PredicateMap
            | SemanticConcept::MembershipTraversal
    )
}

// ─────────────────────────────────────────────────────────────────
// Transformation authorization
// ─────────────────────────────────────────────────────────────────

/// A function fingerprint — invalidates authorizations after rewrites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionFingerprint(pub u64);

/// FNV-1a over the function name and a structural summary of every node.
pub fn function_fingerprint(func: &Function) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mix = |hash: &mut u64, byte: u8| {
        *hash ^= byte as u64;
        *hash = hash.wrapping_mul(0x100000001b3);
    };
    for b in func.name.bytes() {
        mix(&mut hash, b);
    }
    mix(&mut hash, 0xFF);
    for node in func.arena.iter() {
        mix(&mut hash, (node.id.0 & 0xFF) as u8);
        mix(&mut hash, ((node.id.0 >> 8) & 0xFF) as u8);
        // Kind identity: hash the debug-format discriminant of the kind.
        let kind_debug = format!("{:?}", std::mem::discriminant(&node.kind));
        for b in kind_debug.bytes() {
            mix(&mut hash, b);
        }
    }
    hash
}

// ─────────────────────────────────────────────────────────────────
// Transformation authorization
// ─────────────────────────────────────────────────────────────────

/// Authorization for a specific transformation family on a region,
/// derived from complete certificates. This is the ONLY artifact a
/// candidate generator may consume.
#[derive(Clone, Debug)]
pub struct TransformationAuthorization {
    pub region: RegionId,
    pub function_fingerprint: FunctionFingerprint,
    pub domain: DomainCertificate,
    /// Operation concepts this authorization covers.
    pub authorized_concepts: Vec<SemanticConcept>,
    /// Node provenance (region nodes + certificate-relevant nodes).
    pub provenance: Vec<NodeId>,
}

impl TransformationAuthorization {
    /// Version binding: an authorization is only valid against the exact
    /// function it was derived from. After a rewrite the function changes
    /// and this authorization is stale (fresh ones are derived per pass).
    pub fn is_valid_for(&self, func: &Function) -> bool {
        function_fingerprint(func) == self.function_fingerprint.0
    }
}

/// Per-region authorizations. Produced by `derive_authorizations` once
/// per pipeline pass; consumed by the candidate generator.
#[derive(Default, Debug)]
pub struct AuthorizationDatabase {
    map: HashMap<RegionId, Vec<TransformationAuthorization>>,
}

impl AuthorizationDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, auth: TransformationAuthorization) {
        self.map.entry(auth.region).or_default().push(auth);
    }

    pub fn for_region(&self, region: RegionId) -> &[TransformationAuthorization] {
        self.map.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Union of operation concepts authorized for a region.
    pub fn authorized_concepts(
        &self,
        region: RegionId,
    ) -> HashSet<SemanticConcept> {
        let mut set = HashSet::new();
        for a in self.for_region(region) {
            set.extend(a.authorized_concepts.iter().copied());
        }
        set
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn region_count(&self) -> usize {
        self.map.len()
    }

    /// Unit-test support: grant concepts directly without a function.
    ///
    /// Escape hatch for pure generator unit tests that have no SIR
    /// function. Production pipelines must use `derive_authorizations`
    /// — certificates are never skipped there.
    pub fn grant_for_unit_test(
        &mut self,
        region: RegionId,
        concepts: Vec<SemanticConcept>,
    ) {
        self.add(TransformationAuthorization {
            region,
            function_fingerprint: FunctionFingerprint(0),
            domain: DomainCertificate::ScalarExpression,
            authorized_concepts: concepts,
            provenance: vec![],
        });
    }
}

// ─────────────────────────────────────────────────────────────────
// Derivation
// ─────────────────────────────────────────────────────────────────

/// Is this reduction the unit counter (stride-1 induction variable)?
/// The counter is exempt from overflow-flag checks: it is the index,
/// not an accumulated value. Same definition as the recognizers' stride
/// check: kind "sum" with a constant-1 invariant.
fn is_unit_counter(func: &Function, r: &sir_analysis::facts::ReductionVar) -> bool {
    // A unit counter is a contiguous-traversal index: forward (+1, kind
    // "sum") or reverse (-1, kind "sub" from a reverse scan).
    if r.reduction_kind != "sum" && r.reduction_kind != "sub" {
        return false;
    }
    let Some(node) = func.get_node(r.invariant_value) else {
        return false;
    };
    if let NodeKind::Constant(data) = &node.kind {
        return data.as_u64() == Some(1);
    }
    false
}

/// Is the node a memory access (direct value read)?
fn is_memory_access(func: &Function, id: NodeId) -> bool {
    func.get_node(id)
        .map(|n| {
            matches!(
                n.kind,
                NodeKind::Load { .. } | NodeKind::ArrayAccess { .. }
            )
        })
        .unwrap_or(false)
}

/// Does any node reachable transitively from `id` read memory directly
/// (Load / ArrayAccess)? Used by the X06 invariant: a position result
/// must not be a memory-derived value.
fn derives_from_memory(func: &Function, id: NodeId) -> bool {
    if is_memory_access(func, id) {
        return true;
    }
    graph::transitive_inputs(id, &func.arena)
        .iter()
        .any(|&input| is_memory_access(func, input))
}

/// Integer-semantics gate for a loop's reduction accumulators (X02).
///
/// Returns Some(()) only when EVERY non-counter reduction accumulates
/// modularly. Any accumulator whose update node carries `nsw`/`nuw`
/// (poison-on-overflow) forces abstention: vector reassociation can
/// change which executions are defined, and no range-proof
/// infrastructure exists yet to rescue them.
fn accumulators_are_reassociable(
    func: &Function,
    lf: &sir_analysis::facts::LoopFact,
    carried_inputs: &[NodeId],
    outputs: &[NodeId],
) -> bool {
    // A unit-increment counter must exist (contiguous traversal).
    let has_counter = lf
        .reductions
        .iter()
        .any(|r| is_unit_counter(func, r));
    if !has_counter {
        return false;
    }

    for r in &lf.reductions {
        if is_unit_counter(func, r) {
            continue; // induction variable, not an accumulated value
        }
        let Some(slot) = carried_inputs.iter().position(|&c| c == r.variable) else {
            continue;
        };
        let Some(&out) = outputs.get(slot) else {
            continue;
        };
        let sem = overflow_semantics_of(func, out);
        if !sem.allows_reassociation() {
            return false; // poison semantics + no range proof → abstain
        }
    }
    true
}

/// The X06 invariant: a PositionSearch's result select must bind the
/// POSITION (the index/counter), never a memory-derived value. A
/// running max contains `select(gt(load, m), load, m)` — a
/// non-constant-armed select whose arm derives from memory. A real
/// search selects between the index and a sentinel. Loop regions with
/// NO non-constant-armed select (e.g. forward-termination searches
/// whose result is pure index arithmetic) pass vacuously.
fn position_selects_bind_index(func: &Function, region_nodes: &[NodeId]) -> bool {
    for &n in region_nodes {
        let Some(node) = func.get_node(n) else { continue };
        if let NodeKind::Select { true_val, false_val, .. } = &node.kind {
            // ANY select whose arm derives from memory binds a memory
            // VALUE (a running max/min, an element search), not a
            // position. Constant arms (cardinality/all identity selects)
            // never derive from memory and pass vacuously.
            if derives_from_memory(func, *true_val)
                || derives_from_memory(func, *false_val)
            {
                return false;
            }
        }
    }
    true
}


/// Derive transformation authorizations for every region of a function.
///
/// Called once per pipeline pass, right after semantics derivation.
/// Authorizations are version-bound to this exact function; after a
/// rewrite they are re-derived (never reused across function versions).
///
/// Issuance policy (all fail-closed):
/// - Region interface incomplete → no authorization of any kind.
/// - Reduction truths + complete interface + accumulators provably
///   reassociable → Reduction authorization for the region's reduction
///   concepts (X02 gate: poison-overflow accumulators abstain).
/// - FirstOccurrence/LastOccurrence truth + result binds the index →
///   PositionSearch authorization (X06 gate: memory-derived results
///   are refused).
/// - Region reads no memory → ScalarExpression domain authorizes all
///   scalar bit-manipulation families.
pub fn derive_authorizations(
    func: &Function,
    analysis: &FactDatabase,
    semantic: &SemanticDatabase,
) -> AuthorizationDatabase {
    let mut db = AuthorizationDatabase::new();
    let fingerprint = FunctionFingerprint(function_fingerprint(func));

    for (region_id, region) in semantic.regions() {
        let region_nodes: Vec<NodeId> = region.nodes().iter().copied().collect();
        if region_nodes.is_empty() {
            continue;
        }

        // ── Layer 1: region interface certificate (always required) ──
        let interface = RegionInterfaceCertificate::build(func, &region_nodes);
        if !interface.complete {
            continue; // closed-world guarantee fails → nothing authorized
        }

        let has_memory_reads = !interface.memory_bases.is_empty();

        // Locate the region's loop, if any.
        let loop_node = region_nodes.iter().copied().find(|&n| {
            func.get_node(n)
                .map(|node| matches!(&node.kind, NodeKind::Loop { .. }))
                .unwrap_or(false)
        });

        let mut authorized: Vec<SemanticConcept> = Vec::new();

        // ── Domain 1: Reduction (requires accumulator semantics) ──
        if let Some(loop_id) = loop_node {
            let has_reduction_truth = region
                .concepts()
                .iter()
                .any(is_reduction_concept);
            if has_reduction_truth {
                if let Some(lf) = analysis.loops.get(&loop_id) {
                    let loop_ok = match func.get_node(loop_id) {
                        Some(loop_node) => match &loop_node.kind {
                            NodeKind::Loop {
                                outputs,
                                carried_inputs,
                                ..
                            } => accumulators_are_reassociable(
                                func, lf, carried_inputs, outputs,
                            ),
                            _ => false,
                        },
                        None => false,
                    };
                    if loop_ok {
                        for c in region.concepts() {
                            if is_reduction_concept(c) && !authorized.contains(c) {
                                authorized.push(*c);
                            }
                        }
                        // A certified closed-world reduction region also
                        // hosts scalar node rewrites (e.g. blsr inside a
                        // counting loop); these are equivalence-proved
                        // downstream. PositionSearch is NOT granted here.
                        for c in scalar_bit_ops() {
                            if !authorized.contains(c) {
                                authorized.push(*c);
                            }
                        }
                    }
                }
            }
        }

        // ── Domain 2: Position search (X06 invariant) ──
        let wants_position = region
            .concepts()
            .contains(&SemanticConcept::FirstOccurrence)
            || region
                .concepts()
                .contains(&SemanticConcept::LastOccurrence);
        if wants_position && position_selects_bind_index(func, &region_nodes) {
            for c in position_search_concepts() {
                if !authorized.contains(c) {
                    authorized.push(*c);
                }
            }
        }

        // ── Domain 3: closed-world pure scalar region ──
        // No memory reads, no calls, no volatile/atomic (interface
        // certificate): scalar bit-manipulation families are
        // authorized on the closed-world guarantee alone. Loops over
        // scalars (Kernighan, tzcnt) qualify — their state is fully
        // visible in the carried values.
        if !has_memory_reads {
            for c in scalar_op_concepts() {
                if !authorized.contains(c) {
                    authorized.push(*c);
                }
            }
        }

        if !authorized.is_empty() {
            // Domain certificate: memory-touching regions carry the
            // specific memory domain; pure-scalar regions are
            // ScalarExpression.
            let domain = if has_memory_reads {
                DomainCertificate::Reduction {
                    operator: ReductionOperator::Sum,
                    integer_semantics: IntegerSemantics::Modular,
                    stride_is_unit: true,
                }
            } else {
                DomainCertificate::ScalarExpression
            };

            db.add(TransformationAuthorization {
                region: region_id,
                function_fingerprint: fingerprint,
                domain,
                authorized_concepts: authorized,
                provenance: region_nodes.clone(),
            });
        }
    }

    db
}
