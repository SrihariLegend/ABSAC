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
use sir_types::{ConstantData, Effects, NodeId, RegionId};

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
// Definedness gate (advisor directive — scalar domains fail closed)
// ─────────────────────────────────────────────────────────────────

/// True when the operation's divisor is a constant known to be
/// nonzero — the only nonzero-divisor proof available before a
/// full DefinednessCertificate exists.
fn divisor_is_proven_nonzero(func: &Function, kind: &NodeKind) -> bool {
    let rhs = match kind {
        NodeKind::Div { rhs, .. } | NodeKind::Rem { rhs, .. } => *rhs,
        _ => return false,
    };
    match func.get_node(rhs).map(|n| &n.kind) {
        Some(NodeKind::Constant(c)) => {
            // Nonzero is provable for any literal: unsigned decodes via
            // as_u64, signed via i64 parse. Zero of either sign fails.
            let nonzero = |v: &str| v.parse::<i64>().map(|d| d != 0).unwrap_or(false);
            match c {
                ConstantData::Integer { value, .. } => Some(nonzero(value)),
                _ => None,
            }
            .unwrap_or(false)
        }
        _ => false,
    }
}

/// Signed division/remainder has an exceptional second case:
/// `INT_MIN / -1` (and `INT_MIN % -1`) overflow the result type and
/// trap in C / produce poison-flagged UB in LLVM. The udiv/sdiv/
/// urem/srem distinction is taken from the operand type's signedness
/// (SIR types carry signedness). Until a DefinednessCertificate can
/// prove `x != INT_MIN`, signed division and remainder by ANY
/// constant are refused — the transformation families that rewrite
/// them (divide→shift, modulo→mask) are also invalid for negative
/// operands (e.g. -1 % 8 = -1 but -1 & 7 = 7), so no signed div/rem
/// region qualifies for scalar authorization today.
fn signed_div_or_rem(node: &sir_nodes::Node) -> bool {
    matches!(node.kind, NodeKind::Div { .. } | NodeKind::Rem { .. })
        && matches!(&node.ty, sir_types::Type::Integer { signed: true, .. })
}

/// Whitelist scan for scalar-expression transformations.
///
/// "Pure" (no observable effects) does NOT imply total or safely
/// transformable. Until a DefinednessCertificate exists (nonzero
/// divisors, valid shift ranges, no-overflow proofs, poison
/// preconditions — Gate 4B is open), only operations with fully
/// modeled behavior may be transformed:
///
///   and/or/xor/not, well-defined add/sub/mul, safe constants,
///   well-constrained shifts, total casts, comparisons, selects.
///
/// A region containing any partially-modeled operation is refused:
///   - Div / Rem — no nonzero-divisor proof (trap/UB risk)
///   - shifts by a non-constant amount or an out-of-range constant
///   - nsw / nuw operations — overflow produces POISON, and no
///     poison-precondition proof exists yet (X02 machinery, reused)
///
/// This gate applies to the ScalarExpression domain and to the
/// derived Composition grant inside reduction regions.
fn scalar_expression_is_defined(func: &Function, region_nodes: &[NodeId]) -> Result<(), String> {
    // Region nodes plus their transitive inputs: the scalar expression
    // may consume values computed outside the region's own node set.
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
        match &node.kind {
            // Division/remainder — two separate questions (advisor
            // audit):
            //   1. Definedness: unsigned div/rem needs divisor != 0;
            //    signed additionally has the INT_MIN / -1 trap.
            // 2. Transformation legality: modulo→mask and divide→shift
            //    are only equivalent for unsigned (or proven
            //    non-negative) operands. Signed div/rem regions are
            //    refused outright.
            NodeKind::Div { .. } | NodeKind::Rem { .. } => {
                if signed_div_or_rem(node) {
                    return Err(format!(
                        "region contains signed div/rem (sdiv/srem: \
                         INT_MIN/-1 unmodeled; modulo-to-mask invalid \
                         for negative operands): node %{}",
                        id.0
                    ));
                }
                if !divisor_is_proven_nonzero(func, &node.kind) {
                    return Err(format!(
                        "region contains {} without a nonzero-divisor proof \
                         (no DefinednessCertificate yet)",
                        if matches!(node.kind, NodeKind::Div { .. }) {
                            "Div"
                        } else {
                            "Rem"
                        }
                    ));
                }
            }
            NodeKind::Shl { rhs, .. } | NodeKind::Shr { rhs, .. } => {
                // A shift is well-constrained only when the amount is a
                // constant strictly below the operand bit width. The
                // amount may be a constant EXPRESSION (the canonical
                // rotate writes `W - k` as a subtraction of literals);
                // a variable amount still refuses.
                let amount = constant_amount(func, *rhs);
                let width = match &node.ty {
                    sir_types::Type::Integer { width, .. } => width.bits() as u64,
                    _ => 0,
                };
                match amount {
                    Some(amt) if width > 0 && amt < width => {}
                    other => {
                        return Err(format!(
                            "shift amount not provably in range (amount={:?}, width={}): node %{}",
                            other.map(|a| a.to_string()),
                            width,
                            id.0
                        ));
                    }
                }
            }
            _ => {}
        }
        // Poison-possible arithmetic (nsw/nuw): the scalar expression
        // families rewrite arithmetic; without a no-overflow proof the
        // refinement relation is not maintained. Abstain.
        if node.metadata.contains_key("llvm.overflow") {
            return Err(format!(
                "region carries an nsw/nuw operation (poison semantics unmodeled): node %{}",
                id.0
            ));
        }
    }
    Ok(())
}

/// Statically evaluate an integer amount expression: literals and
/// constant add/sub chains (the canonical `W - k` rotate amount).
/// Anything involving a parameter or a non-constant node returns None —
/// the definedness gate must not guess.
fn constant_amount(func: &Function, id: NodeId) -> Option<u64> {
    match &func.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        NodeKind::Add { lhs, rhs } => {
            let a = constant_amount(func, *lhs)?;
            let b = constant_amount(func, *rhs)?;
            Some(a.wrapping_add(b))
        }
        NodeKind::Sub { lhs, rhs } => {
            let a = constant_amount(func, *lhs)?;
            let b = constant_amount(func, *rhs)?;
            Some(a.wrapping_sub(b))
        }
        _ => None,
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
    /// Authorization derived from another certificate: a scalar
    /// transformation used INSIDE a certified reduction region.
    /// Explicit provenance — the blanket "reduction cert grants every
    /// scalar op" capability escalation is not permitted (P0A
    /// hardening, advisor item 5).
    Composition {
        /// The base certificate whose closed world hosts the rewrite.
        base: Box<DomainCertificate>,
        /// The derived scalar subdomain (always ScalarExpression).
        derived: Box<DomainCertificate>,
    },
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

/// Serializable tag of a domain certificate — travels with the
/// candidate as authorization provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DomainKind {
    Reduction,
    PositionSearch,
    ScalarExpression,
    /// Scalar transformation derived from (and hosted by) a
    /// reduction certificate's closed world.
    CompositionScalarWithinReduction,
}

impl DomainCertificate {
    pub fn kind(&self) -> DomainKind {
        match self {
            DomainCertificate::Composition { .. } => {
                DomainKind::CompositionScalarWithinReduction
            }
            DomainCertificate::Reduction { .. } => DomainKind::Reduction,
            DomainCertificate::PositionSearch { .. } => DomainKind::PositionSearch,
            DomainCertificate::ScalarExpression => DomainKind::ScalarExpression,
        }
    }
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
    /// Immutable identity assigned by the issuing AuthorizationDatabase
    /// (advisor directive: the database, not any candidate-carried
    /// copy, is the source of authority). Candidates carry this id and
    /// the optimizer/rewrite layers retrieve the original authorization
    /// by it for exact comparison.
    pub id: AuthorizationId,
    pub region: RegionId,
    pub function_fingerprint: FunctionFingerprint,
    pub domain: DomainCertificate,
    /// Operation concepts this authorization covers.
    pub authorized_concepts: Vec<SemanticConcept>,
    /// Node provenance (region nodes + certificate-relevant nodes).
    pub provenance: Vec<NodeId>,
    /// Concrete binding facts (advisor P0A item 2, step 1): the exact
    /// memory bases, accumulator, and effects this certificate covers.
    /// The rewrite layer compares the nodes a recipe binds (collection,
    /// accumulator, replaced values) against these facts — authorization
    /// must bind the exact candidate, not merely its domain.
    pub concrete: ConcreteFacts,
}

/// Immutable identity of an issued authorization. Assigned by the
/// AuthorizationDatabase at derivation time; monotonically increasing
/// within a database. Carried by candidates as a REFERENCE — the
/// authoritative record stays in the database.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AuthorizationId(pub u64);

impl AuthorizationId {
    /// Sentinel for unit-test AuthorizationRefs (no database record).
    /// The exact-comparison path skips database retrieval for this id.
    pub const UNIT_TEST: AuthorizationId = AuthorizationId(u64::MAX);
}

/// Concrete facts of the certified region: what the certificate
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcreteFacts {
    /// Memory bases read (resolved to root parameters), in region
    /// layout order — from the interface certificate's footprint.
    pub memory_bases: Vec<NodeId>,
    /// The reassociable accumulator's carried variable, when the
    /// region carries a certified reduction recurrence.
    pub accumulator: Option<NodeId>,
    /// The region writes memory (closed-world refuses these today).
    pub has_memory_writes: bool,
    /// Effects the closed world permits for this region: READ_MEMORY
    /// when the region reads a bound base, empty for pure regions.
    /// A candidate requesting effects beyond this exceeds authorization.
    pub authorized_effects: Effects,
}
impl Default for ConcreteFacts {
    fn default() -> Self {
        Self {
            memory_bases: Vec::new(),
            accumulator: None,
            has_memory_writes: false,
            authorized_effects: Effects::empty(),
        }
    }
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
///
/// This database is the SOURCE OF AUTHORITY (advisor directive): a
/// candidate's carried facts are copies, advisory only. The
/// optimizer/rewrite layers retrieve the immutable original by
/// AuthorizationId and compare exactly.
#[derive(Default, Debug)]
pub struct AuthorizationDatabase {
    map: HashMap<RegionId, Vec<TransformationAuthorization>>,
    by_id: HashMap<AuthorizationId, RegionId>,
    next_id: u64,
}

impl AuthorizationDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, mut auth: TransformationAuthorization) {
        auth.id = AuthorizationId(self.next_id);
        self.next_id += 1;
        let region = auth.region;
        self.by_id.insert(auth.id, auth.region);
        self.map.entry(region).or_default().push(auth);
    }

    /// Retrieve the immutable original authorization by id. Returns
    /// None for unknown ids — a candidate citing an unknown id is
    /// forged or stale and must be rejected.
    pub fn authorization(&self, id: AuthorizationId) -> Option<&TransformationAuthorization> {
        let region = self.by_id.get(&id)?;
        self.map
            .get(region)
            .and_then(|v| v.iter().find(|a| a.id == id))
    }

    /// Test-support: insert a fully-formed authorization (used by
    /// adversarial binding tests). Production code must use
    /// `derive_authorizations` — which is the only issuer of
    /// certificate-backed authorizations.
    pub fn grant_raw(&mut self, auth: TransformationAuthorization) {
        self.add(auth);
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
            id: AuthorizationId::UNIT_TEST, // reassigned by add()
            region,
            function_fingerprint: FunctionFingerprint(0),
            domain: DomainCertificate::ScalarExpression,
            authorized_concepts: concepts,
            provenance: vec![],
            concrete: ConcreteFacts::default(),
        });
    }
}

// ─────────────────────────────────────────────────────────────────
// Derivation
// ─────────────────────────────────────────────────────────────────

/// Is this reduction the unit counter (stride-1 induction variable)?
/// The counter is exempt from overflow-flag checks: it is the index,
/// not an accumulated value. Same definition as the recognizers' stride
/// check: kind "sum" with a constant-1 invariant — plus the index-use
/// test: the carried variable must actually serve as the traversal
/// index (element-access index or termination operand). A bare
/// `count += 1` accumulator that never indexes the collection is a
/// real reduction, NOT the induction counter.
pub(crate) fn is_unit_counter(func: &Function, r: &sir_analysis::facts::ReductionVar) -> bool {
    // A unit counter is a contiguous-traversal index: forward (+1, kind
    // "sum") or reverse (-1, kind "sub" from a reverse scan).
    if r.reduction_kind != "sum" && r.reduction_kind != "sub" {
        return false;
    }
    let Some(node) = func.get_node(r.invariant_value) else {
        return false;
    };
    if !matches!(&node.kind, NodeKind::Constant(data) if data.as_u64() == Some(1)) {
        return false;
    }
    // The carried variable must be used as an element-access index or
    // as the loop-termination operand WITHIN the loop that carries it
    // (a cross-loop usage is not a traversal counter for this loop).
    let mut used_as_index = false;
    let mut used_in_termination = false;
    for n in func.arena.iter() {
        let NodeKind::Loop {
            body,
            termination,
            carried_inputs,
            ..
        } = &n.kind
        else {
            continue;
        };
        if !carried_inputs.contains(&r.variable) {
            continue; // not this reduction's loop
        }
        for &body_id in body {
            if let Some(body_node) = func.get_node(body_id) {
                if let NodeKind::ArrayAccess { index, .. } = &body_node.kind {
                    if *index == r.variable {
                        used_as_index = true;
                    }
                }
            }
        }
        if let Some(term) = func.get_node(*termination) {
            if let NodeKind::Lt { lhs, rhs }
            | NodeKind::Le { lhs, rhs }
            | NodeKind::Gt { lhs, rhs }
            | NodeKind::Ge { lhs, rhs } = &term.kind
            {
                if *lhs == r.variable || *rhs == r.variable {
                    used_in_termination = true;
                }
            }
        }
    }
    used_as_index || used_in_termination
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
pub(crate) fn derives_from_memory(func: &Function, id: NodeId) -> bool {
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
) -> Option<NodeId> {
    // Returns the reassociable accumulator's carried variable (the
    // accumulated value — never the induction counter), or None when
    // the recurrence is not certifiable.

    // A unit-increment counter must exist (contiguous traversal).
    let has_counter = lf
        .reductions
        .iter()
        .any(|r| is_unit_counter(func, r));
    if !has_counter {
        return None;
    }

    // Exactly ONE non-counter recurrence is certifiable. A loop with
    // two legitimate accumulators (e.g. `sum += board[i]; count += 1`)
    // has no unique accumulator — selecting by scan order is forbidden
    // (mirrors the binding layer's AmbiguousRole rule).
    let mut non_counter: Vec<&sir_analysis::facts::ReductionVar> = lf
        .reductions
        .iter()
        .filter(|r| !is_unit_counter(func, r))
        .collect();
    if non_counter.len() != 1 {
        return None;
    }
    let r = non_counter.pop().expect("length checked");
    let slot = carried_inputs
        .iter()
        .position(|&c| c == r.variable)?;
    let out = outputs.get(slot)?;
    let sem = overflow_semantics_of(func, *out);
    if !sem.allows_reassociation() {
        return None; // poison semantics + no range proof → abstain
    }
    Some(r.variable)
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

        // Concrete binding facts (advisor P0A item 2, step 1): shared
        // across the authorizations issued for this region. All derive
        // from the same interface certificate.
        let mut concrete = ConcreteFacts {
            memory_bases: interface.memory_bases.clone(),
            accumulator: None, // set below when a reduction certifies
            has_memory_writes: interface.has_memory_writes,
            authorized_effects: if interface.memory_bases.is_empty() {
                Effects::empty()
            } else {
                Effects::READ_MEMORY
            },
        };

        // Concept lists, one per domain — issued as SEPARATE
        // authorizations below (never a blended list).
        let mut reduction_concepts: Vec<SemanticConcept> = Vec::new();
        let mut position_concepts: Vec<SemanticConcept> = Vec::new();
        let mut scalar_concepts: Vec<SemanticConcept> = Vec::new();

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
                            } => {
                                let acc = accumulators_are_reassociable(
                                    func, lf, carried_inputs, outputs,
                                );
                                if acc.is_some() {
                                    concrete.accumulator = acc;
                                }
                                acc.is_some()
                            }
                            _ => false,
                        },
                        None => false,
                    };
                    if loop_ok {
                        for c in region.concepts() {
                            if is_reduction_concept(c) && !reduction_concepts.contains(c) {
                                reduction_concepts.push(*c);
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
                if !position_concepts.contains(c) {
                    position_concepts.push(*c);
                }
            }
        }

        // ── Domain 3: closed-world pure scalar region ──
        // No memory reads, no calls, no volatile/atomic (interface
        // certificate): scalar bit-manipulation families are
        // authorized on the closed-world guarantee alone — PLUS a
        // definedness gate (advisor: fail closed NOW). Loops over
        // scalars (Kernighan, tzcnt) qualify — their state is fully
        // visible in the carried values.
        let definedness = if has_memory_reads {
            Err("region reads memory".to_string())
        } else {
            scalar_expression_is_defined(func, &region_nodes)
        };
        if !has_memory_reads && definedness.is_ok() {
            for c in scalar_op_concepts() {
                if !scalar_concepts.contains(c) {
                    scalar_concepts.push(*c);
                }
            }
        }

        // ── Issue authorizations: one per domain, never a blended list ──
        // Each entry is independently auditable: a consumer can see
        // exactly which certificate authorizes which concepts.
        if !reduction_concepts.is_empty() {
            let reduction_domain = DomainCertificate::Reduction {
                operator: ReductionOperator::Sum,
                integer_semantics: IntegerSemantics::Modular,
                stride_is_unit: true,
            };
            db.add(TransformationAuthorization {
                // id: assigned by the database on insert
                id: AuthorizationId(0),
                region: region_id,
                function_fingerprint: fingerprint,
                domain: reduction_domain.clone(),
                authorized_concepts: reduction_concepts,
                provenance: region_nodes.clone(),
                concrete: concrete.clone(),
            });

            // Composition authorization (derived, explicit): scalar
            // node rewrites hosted INSIDE the certified reduction's
            // closed world (e.g. blsr inside a counting loop); they
            // are equivalence-proved downstream. Issued only when the
            // reduction certificate itself was granted — never as a
            // capability escalation from it (P0A hardening item 5) —
            // AND only when the region's scalar expression is fully
            if scalar_expression_is_defined(func, &region_nodes).is_ok() {
                db.add(TransformationAuthorization {
                    // id: assigned by the database on insert
                    id: AuthorizationId(0),
                    region: region_id,
                    function_fingerprint: fingerprint,
                    domain: DomainCertificate::Composition {
                        base: Box::new(reduction_domain),
                        derived: Box::new(DomainCertificate::ScalarExpression),
                    },
                    authorized_concepts: scalar_bit_ops().to_vec(),
                    provenance: region_nodes.clone(),
                    concrete: concrete.clone(),
                });
            }
        }

        if !position_concepts.is_empty() {
            db.add(TransformationAuthorization {
                // id: assigned by the database on insert
                id: AuthorizationId(0),
                region: region_id,
                function_fingerprint: fingerprint,
                domain: DomainCertificate::PositionSearch { result_binds_index: true },
                authorized_concepts: position_concepts,
                provenance: region_nodes.clone(),
                concrete: concrete.clone(),
            });
        }

        if !scalar_concepts.is_empty() {
            db.add(TransformationAuthorization {
                // id: assigned by the database on insert
                id: AuthorizationId(0),
                region: region_id,
                function_fingerprint: fingerprint,
                domain: DomainCertificate::ScalarExpression,
                authorized_concepts: scalar_concepts,
                provenance: region_nodes.clone(),
                concrete: concrete.clone(),
            });
        }
    }

    db
}
