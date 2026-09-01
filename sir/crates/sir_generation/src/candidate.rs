use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use sir_semantics::concepts::SemanticConcept;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::ContextId;
use sir_transform::ids::DefinitionId;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::{NodeId, RegionId};

/// Unique identifier for a candidate plan.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CandidateId(pub u64);

impl CandidateId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "candidate#{}", self.0)
    }
}

/// How a bitset transformation might be implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImplementationStrategy {
    /// Iterate over set bits: while bb != 0 { tzcnt; bb &= bb-1 }
    BitIteration,
    /// Compute cardinality directly: popcount(bb)
    Popcount,
    /// Change data representation: bool[64] → u64
    PackedBitfield,
    /// Replace boolean predicates with mask operations: AND/OR/XOR
    MaskConstruction,
    /// Check if any bit is set: bb != 0
    Any,
    /// Check if all bits are set: bb == full_mask
    All,
    /// Check if odd number of bits are set: popcount(bb) & 1
    Parity,
    /// Bitwise AND operation (for modulo).
    BitwiseAnd,
    /// Right shift operation (for division).
    ShiftRight,
    /// Left shift operation (for multiplication).
    ShiftLeft,
    /// Shift right and shift left sequence extracting a mask.
    MaskExtract,
    /// Hardware bit scan forward (find first set bit).
    BitScanForward,
    /// Hardware bit scan reverse (find last set bit).
    BitScanReverse,
    /// Hardware trailing zero count.
    TrailingZeroCount,
    /// Hardware leading zero count.
    LeadingZeroCount,
    /// Clear the lowest set bit.
    ClearLowestBit,
    /// Isolate the lowest set bit: `x & -x`.
    IsolateLowestBit,
    /// Isolate the lowest clear bit: `~x & (x + 1)`.
    IsolateLowestClearBit,
    /// Set the lowest clear bit: `x | (x + 1)`.
    SetLowestClearBit,
    /// Hardware rotate left: `rol(x, k)`.
    RotateLeft,
    /// Hardware rotate right: `ror(x, k)`.
    RotateRight,
    /// Byte-order reversal via a byte-swap instruction.
    ByteSwap,
    /// Full bit reversal via a reverse-bits instruction.
    ReverseBits,
}

impl fmt::Display for ImplementationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImplementationStrategy::BitIteration => write!(f, "BitIteration"),
            ImplementationStrategy::Popcount => write!(f, "Popcount"),
            ImplementationStrategy::PackedBitfield => write!(f, "PackedBitfield"),
            ImplementationStrategy::MaskConstruction => write!(f, "MaskConstruction"),
            ImplementationStrategy::Any => write!(f, "Any"),
            ImplementationStrategy::All => write!(f, "All"),
            ImplementationStrategy::Parity => write!(f, "Parity"),
            ImplementationStrategy::BitwiseAnd => write!(f, "BitwiseAnd"),
            ImplementationStrategy::ShiftRight => write!(f, "ShiftRight"),
            ImplementationStrategy::ShiftLeft => write!(f, "ShiftLeft"),
            ImplementationStrategy::MaskExtract => write!(f, "MaskExtract"),
            ImplementationStrategy::BitScanForward => write!(f, "BitScanForward"),
            ImplementationStrategy::BitScanReverse => write!(f, "BitScanReverse"),
            ImplementationStrategy::TrailingZeroCount => write!(f, "TrailingZeroCount"),
            ImplementationStrategy::LeadingZeroCount => write!(f, "LeadingZeroCount"),
            ImplementationStrategy::ClearLowestBit => write!(f, "ClearLowestBit"),
            ImplementationStrategy::IsolateLowestBit => write!(f, "IsolateLowestBit"),
            ImplementationStrategy::IsolateLowestClearBit => write!(f, "IsolateLowestClearBit"),
            ImplementationStrategy::SetLowestClearBit => write!(f, "SetLowestClearBit"),
            ImplementationStrategy::RotateLeft => write!(f, "RotateLeft"),
            ImplementationStrategy::RotateRight => write!(f, "RotateRight"),
            ImplementationStrategy::ByteSwap => write!(f, "ByteSwap"),
            ImplementationStrategy::ReverseBits => write!(f, "ReverseBits"),
        }
    }
}

/// What kind of change a candidate proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateEffect {
    /// The representation of data changes (e.g., bool[64] → u64)
    RepresentationChange,
    /// How the data is traversed changes (e.g., loop → trailing-zero scan)
    TraversalChange,
    /// How predicates test conditions changes (e.g., if → mask)
    PredicateEncodingChange,
    /// How counting is performed changes (e.g., accumulator → popcount)
    CountingStrategyChange,
    /// How reduction is performed changes (e.g., loop accumulator → bitwise math)
    ReductionStrategyChange,
    /// Replaces an arithmetic operation with a bitwise equivalent.
    InstructionSubstitution,
}

/// Human-readable explanation of a candidate plan.
#[derive(Clone, Debug)]
pub struct CandidateExplanation {
    pub source_concepts: Vec<SemanticConcept>,
    pub rationale: &'static str,
}

/// A candidate transformation plan — a proposed implementation strategy
/// for a region, derived from a TransformationContext.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    /// Reference to the context that produced this candidate.
    /// Multiple candidates may reference the same context.
    pub context_id: ContextId,
    pub definition_id: DefinitionId,
    pub strategy: ImplementationStrategy,
    pub explanation: CandidateExplanation,
    pub effects: Vec<CandidateEffect>,
    /// Expected cost profile after this transformation is applied.
    /// Set by the generator at creation time based on the implementation strategy.
    /// This is objective data — cost models assign meaning to it.
    pub expected_cost: sir_types::CostProfile,
    pub representation: Representation,
    pub source_structure: SourceStructure,
    pub constraints: HashSet<Constraint>,
    pub assumptions: HashSet<Assumption>,
    /// Authorization provenance (P0A hardening): every authorized
    /// candidate carries the certificates that admitted it. The
    /// selector/verifier/rewrite chain can therefore reject stale or
    /// mismatched authorizations instead of trusting the generator's
    /// filter alone.
    pub authorization: AuthorizationRef,
    /// ConcreteBindingDigest (advisor P0A item 2): FNV-1a over every
    /// field that participates in the authorization decision — the
    /// authorization version, region, definition, strategy, cited
    /// concepts, effects, representation, constraints, assumptions.
    /// Downstream stages recompute it before trusting the candidate:
    /// any in-flight mutation of a bound field after authorization is
    /// detected as a digest mismatch.
    pub binding_digest: u64,
}

/// Authorization provenance that travels with a candidate: which
/// certificate(s), for which region, derived from which function
/// version. Downstream stages (selector, verifier, rewrite engine)
/// check this before trusting a candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationRef {
    /// Immutable identity of the issuing authorization in the
    /// AuthorizationDatabase (advisor directive: the database is the
    /// source of authority; the candidate's copies are advisory).
    /// The optimizer/rewrite layers retrieve the original by this id
    /// and compare exactly. UNIT_TEST (u64::MAX) skips the lookup.
    pub authorization_id: sir_semantics::authorization::AuthorizationId,
    /// Fingerprint of the exact function the authorization was derived
    /// from. Hash mismatch ⇒ definitely stale (reject). Hash match ⇒
    /// proceed to exact structural checks — a finite hash is a version
    /// key, never a collision-free equivalence proof.
    pub function_fingerprint: u64,
    /// The region the authorization was issued for.
    pub region: RegionId,
    /// Domains whose certificates cover this candidate's cited concepts.
    pub domains: Vec<sir_semantics::authorization::DomainKind>,
    /// Concrete binding facts copied from the matched authorization:
    /// the exact memory bases, accumulator, and effects the
    /// certificate covers. Advisory copies — the authoritative record
    /// lives in the AuthorizationDatabase and is compared exactly by
    /// `exact_binding_matches`.
    pub concrete: sir_semantics::authorization::ConcreteFacts,
    /// The authorized region's node set (certificate provenance).
    pub region_nodes: Vec<NodeId>,
}

impl AuthorizationRef {
    /// Unit-test support: a provenance-less ref for tests that
    /// construct candidates for selection/cost-model testing without a
    /// pipeline. Production candidates always get gate-minted refs.
    pub fn for_unit_test() -> Self {
        Self {
            authorization_id: sir_semantics::authorization::AuthorizationId::UNIT_TEST,
            function_fingerprint: 0,
            region: RegionId::new(u64::MAX),
            domains: Vec::new(),
            concrete: Default::default(),
            region_nodes: Vec::new(),
        }
    }

    /// Stale-authorization rejection (advisor item 4). A finite hash is
    /// only ever a version key: mismatch ⇒ definitely reject; match ⇒
    /// downstream proof obligations still apply.
    pub fn matches_function(&self, func: &sir_nodes::Function) -> bool {
        sir_semantics::authorization::function_fingerprint(func)
            == self.function_fingerprint
            && self.region != RegionId::new(u64::MAX)
    }
}

/// An UNTRUSTED proposal: whatever a generator produced from raw
/// truths/beliefs and context structure, before any authorization
/// check. It cannot enter the candidate database, selection, or
/// rewriting — the ONLY way to obtain a `Candidate` from a proposal is
/// `authorize` (crate-private), and generators outside sir_generation
/// cannot construct proposals at all (private fields).
///
/// Pipeline shape (advisor hardening items 2+3):
///
/// ```text
/// truths/beliefs → untrusted proposals → match against authorization
///                → Candidate constructed only after the match
/// ```
#[derive(Clone, Debug)]
pub struct UntrustedProposal {
    pub(crate) region: RegionId,
    pub(crate) context_id: ContextId,
    pub(crate) definition_id: DefinitionId,
    pub(crate) strategy: ImplementationStrategy,
    pub(crate) explanation: CandidateExplanation,
    pub(crate) effects: Vec<CandidateEffect>,
    pub(crate) expected_cost: sir_types::CostProfile,
    pub(crate) representation: Representation,
    pub(crate) source_structure: SourceStructure,
    pub(crate) constraints: HashSet<Constraint>,
    pub(crate) assumptions: HashSet<Assumption>,
    pub(crate) source_concepts: Vec<SemanticConcept>,
}

impl UntrustedProposal {
    /// Construct a proposal from generator data. In-crate only:
    /// generators propose, the authorization gate disposes.
    pub(crate) fn new(
        region: RegionId,
        context_id: ContextId,
        definition_id: DefinitionId,
        strategy: ImplementationStrategy,
        explanation: CandidateExplanation,
        effects: Vec<CandidateEffect>,
        expected_cost: sir_types::CostProfile,
        representation: Representation,
        source_structure: SourceStructure,
        constraints: HashSet<Constraint>,
        assumptions: HashSet<Assumption>,
        source_concepts: Vec<SemanticConcept>,
    ) -> Self {
        Self {
            region,
            context_id,
            definition_id,
            strategy,
            explanation,
            effects,
            expected_cost,
            representation,
            source_structure,
            constraints,
            assumptions,
            source_concepts,
        }
    }

    /// Mint an authorized candidate. Crate-private by design: nothing
    /// outside sir_generation can turn a proposal into a candidate.
    /// The binding digest is computed here, over the proposal's declared
    /// fields and the matched authorization — once, at the trust boundary.
    pub(crate) fn authorize(self, id: CandidateId, auth: AuthorizationRef) -> Candidate {
        let mut candidate = Candidate {
            id,
            region: self.region,
            context_id: self.context_id,
            definition_id: self.definition_id,
            strategy: self.strategy,
            explanation: self.explanation,
            effects: self.effects,
            expected_cost: self.expected_cost,
            representation: self.representation,
            source_structure: self.source_structure,
            constraints: self.constraints,
            assumptions: self.assumptions,
            authorization: auth,
            binding_digest: 0,
        };
        candidate.binding_digest = compute_binding_digest(&candidate);
        candidate
    }
}

/// FNV-1a chunk mixer.
fn digest_chunk(h: u64, bytes: &[u8]) -> u64 {
    let mut h = h;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn digest_str(h: u64, s: &str) -> u64 {
    digest_chunk(h, s.as_bytes())
}

/// FNV-1a over every field that participates in the authorization
/// decision (advisor P0A item 2): authorization version, region,
/// context, definition, strategy, cited concepts, effects,
/// representation, constraints, assumptions. Set-typed fields are
/// hashed in sorted order (determinism). Downstream stages recompute
/// this from the candidate's own fields: any in-flight mutation of a
/// bound field after authorization is detected as a mismatch.
fn compute_binding_digest(c: &Candidate) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    h = digest_chunk(h, &c.authorization.function_fingerprint.to_le_bytes());
    h = digest_chunk(h, &c.region.0.to_le_bytes());
    h = digest_chunk(h, &c.context_id.0.to_le_bytes());
    h = digest_chunk(h, &c.definition_id.0.to_le_bytes());
    h = digest_str(h, &format!("{:?}", c.strategy));
    let mut names: Vec<String> = Vec::new();
    for concept in &c.explanation.source_concepts {
        names.push(format!("{:?}", concept));
    }
    for effect in &c.effects {
        names.push(format!("{:?}", effect));
    }
    names.push(format!("{:?}", c.representation));
    for constraint in &c.constraints {
        names.push(format!("{:?}", constraint));
    }
    for assumption in &c.assumptions {
        names.push(format!("{:?}", assumption));
    }
    names.sort();
    for s in names {
        h = digest_str(h, &s);
    }
    h
}

impl Candidate {
    /// Recompute and compare the concrete binding digest. False means
    /// a bound field changed after authorization — the candidate is
    /// misbound and must not proceed to rewriting.
    pub fn binding_digest_valid(&self) -> bool {
        compute_binding_digest(self) == self.binding_digest
    }
}

/// Exact authorization comparison (advisor directive: "the source of
/// authority should remain the immutable AuthorizationDatabase").
///
/// The FNV digest is only a fast preliminary check against accidental
/// mutation — it is NOT collision-resistant and never establishes that
/// a candidate matches an authorization. This function is the
/// authoritative check:
///
///   1. retrieve the immutable original authorization by the
///      candidate's AuthorizationId (unknown id ⇒ forged/stale ⇒ false);
///   2. compare the candidate's carried copies against the database
///      record field by field (region, fingerprint, concrete facts,
///      region nodes, cited concepts);
///
/// A candidate whose AuthorizationId is the UNIT_TEST sentinel skips
/// the database retrieval (unit tests have no issuer) — production
/// candidates are always minted with real ids.
pub fn exact_binding_matches(
    candidate: &Candidate,
    db: &sir_semantics::authorization::AuthorizationDatabase,
) -> bool {
    use sir_semantics::authorization::AuthorizationId;

    if candidate.authorization.authorization_id == AuthorizationId::UNIT_TEST {
        // Unit-test escape: no issuer exists. Only structural checks
        // (matches_function, digest) apply.
        return true;
    }

    let Some(auth) = db.authorization(candidate.authorization.authorization_id) else {
        return false; // unknown AuthorizationId: forged or stale
    };

    // Exact field comparison against the database record.
    candidate.region == auth.region
        && candidate.authorization.function_fingerprint == auth.function_fingerprint.0
        && candidate.authorization.concrete == auth.concrete
        && candidate.authorization.region_nodes == auth.provenance
        // Every concept the candidate cites must be covered by the
        // retrieved authorization (or be a descriptive data concept).
        && candidate.explanation.source_concepts.iter().all(|c| {
            sir_semantics::authorization::is_data_concept(c)
                || auth.authorized_concepts.contains(c)
        })
}
