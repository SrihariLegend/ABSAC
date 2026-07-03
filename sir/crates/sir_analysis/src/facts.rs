//! Fact types and the unified FactDatabase.
//!
//! Every analysis produces facts keyed by `NodeId`. Facts are stored in
//! the `FactDatabase` — never inside SIR `Node` structs. This keeps the
//! IR clean and analyses disposable.

use std::collections::{BTreeSet, HashMap};
use sir_types::{ConstantData, NodeId};

// ── Constant Lattice ───────────────────────────────────────

/// Three-level lattice for constant propagation.
///
/// Values flow: `Top` (unknown) → `Constant(v)` → `Bottom` (overdefined, conflicting).
/// This is a meet-semilattice: `meet(Top, x) = x`, `meet(Constant(a), Constant(b)) = Bottom if a!=b`.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstantLattice {
    /// Unknown value (initial state for non-constant nodes).
    Top,
    /// Known constant.
    Constant(ConstantData),
    /// Overdefined — value is not constant (multiple possible values).
    Bottom,
}

impl ConstantLattice {
    /// Meet (join in lattice parlance) two lattice values.
    pub fn meet(&self, other: &Self) -> Self {
        match (self, other) {
            (ConstantLattice::Top, x) | (x, ConstantLattice::Top) => x.clone(),
            (ConstantLattice::Bottom, _) | (_, ConstantLattice::Bottom) => ConstantLattice::Bottom,
            (ConstantLattice::Constant(a), ConstantLattice::Constant(b)) => {
                if a == b {
                    ConstantLattice::Constant(a.clone())
                } else {
                    ConstantLattice::Bottom
                }
            }
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, ConstantLattice::Constant(_))
    }

    pub fn is_top(&self) -> bool {
        matches!(self, ConstantLattice::Top)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, ConstantLattice::Bottom)
    }
}

// ── Purity ─────────────────────────────────────────────────

/// Expression-level purity classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PurityLevel {
    /// No side effects whatsoever.
    Pure,
    /// May read from memory.
    ReadsMemory,
    /// May write to memory.
    WritesMemory,
    /// May allocate memory.
    Allocates,
    /// May perform I/O.
    IO,
    /// May perform atomic operations.
    Atomic,
    /// Unknown — conservative default.
    Unknown,
}

// ── Alias ──────────────────────────────────────────────────

/// Alias relationship between two pointer values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AliasKind {
    /// The two pointers definitely reference the same allocation.
    MustAlias,
    /// The two pointers may reference the same allocation.
    MayAlias(BTreeSet<NodeId>),
    /// The two pointers definitely reference different allocations.
    NoAlias,
    /// Cannot determine aliasing.
    Unknown,
}

// ── Escape ─────────────────────────────────────────────────

/// How a value escapes (or doesn't) from the function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscapeKind {
    /// Does not escape — purely local.
    NeverEscapes,
    /// Returned to the caller.
    Returned,
    /// Stored to a globally-visible memory location.
    StoredGlobally,
    /// Passed as an argument to an external function.
    PassedExternally,
    /// Captured by a closure or loop construct.
    Captured,
}

// ── Reduction ──────────────────────────────────────────────

/// A reduction variable detected in a loop.
#[derive(Clone, Debug, PartialEq)]
pub struct ReductionVar {
    /// The NodeId of the carried variable.
    pub variable: NodeId,
    /// The kind of reduction (e.g., "sum", "product", "xor", "or", "and").
    pub reduction_kind: String,
    /// The loop-invariant value combined in each iteration.
    pub invariant_value: NodeId,
}

// ── Fact Types ─────────────────────────────────────────────

/// Use-Definition facts for a single node.
#[derive(Clone, Debug, PartialEq)]
pub struct UseDefFact {
    /// Nodes that produce values consumed by this node (dataflow inputs).
    pub definitions: Vec<NodeId>,
    /// Nodes that consume the value produced by this node.
    pub users: Vec<NodeId>,
    /// Whether this node is dead (zero uses, not a return node).
    pub is_dead: bool,
    /// Number of dataflow users.
    pub use_count: usize,
}

/// Dominance facts for a single node.
#[derive(Clone, Debug, PartialEq)]
pub struct DominanceFact {
    /// The immediate dominator of this node (None for roots).
    pub idom: Option<NodeId>,
    /// Nodes dominated by this node.
    pub dominates: BTreeSet<NodeId>,
    /// Nodes that dominate this node.
    pub dominators: BTreeSet<NodeId>,
    /// Children in the dominator tree.
    pub dom_tree_children: Vec<NodeId>,
}

/// Constant propagation facts.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstantFact {
    /// The constant value, or Top/Bottom.
    pub value: ConstantLattice,
}

/// Purity facts. Includes both the node-level purity and
/// the subgraph (transitive) purity.
#[derive(Clone, Debug, PartialEq)]
pub struct PurityFact {
    /// This node's own purity level.
    pub purity: PurityLevel,
    /// Whether the entire subgraph rooted at this node is pure.
    pub subgraph_is_pure: bool,
}

/// Integer range facts.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeFact {
    /// Inclusive lower bound (None = unbounded below).
    pub lower: Option<i128>,
    /// Inclusive upper bound (None = unbounded above).
    pub upper: Option<i128>,
    /// The value is known to be non-zero.
    pub is_nonzero: bool,
    /// The value is known to be a power of two.
    pub is_power_of_two: bool,
    /// The value is known to be aligned to this byte boundary.
    pub alignment: Option<u64>,
}

/// Alias facts for a pointer-valued node.
#[derive(Clone, Debug, PartialEq)]
pub struct AliasFact {
    /// The kind of aliasing for this pointer.
    pub kind: AliasKind,
    /// The Allocate node this pointer targets, if known.
    pub allocation_site: Option<NodeId>,
}

/// Escape facts for a node.
#[derive(Clone, Debug, PartialEq)]
pub struct EscapeFact {
    /// How this value escapes.
    pub kind: EscapeKind,
}

/// Loop analysis facts.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopFact {
    /// Whether the loop is known to terminate.
    pub is_finite: bool,
    /// Known trip count, if statically determinable.
    pub trip_count: Option<u64>,
    /// Whether this loop is nested inside another loop.
    pub is_nested: bool,
    /// Carried variables (values passed from one iteration to the next).
    pub carried: Vec<NodeId>,
    /// Reduction variables detected in the loop.
    pub reductions: Vec<ReductionVar>,
}

/// Value numbering facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueNumberFact {
    /// The congruence class identifier.
    pub congruence_class: u64,
    /// The canonical representative of this class (smallest NodeId).
    pub canonical: NodeId,
}

// ── Fact Database ──────────────────────────────────────────

/// The unified fact database.
///
/// Each analysis stores its facts in a dedicated `HashMap<NodeId, FactType>`.
/// New fact types are added as new fields — this is a concrete struct
/// rather than type-erased to keep v0.1 simple and correct.
#[derive(Clone, Debug, Default)]
pub struct FactDatabase {
    /// Use-definition facts.
    pub use_def: HashMap<NodeId, UseDefFact>,
    /// Dominance facts.
    pub dominance: HashMap<NodeId, DominanceFact>,
    /// Constant propagation facts.
    pub constants: HashMap<NodeId, ConstantFact>,
    /// Purity facts.
    pub purity: HashMap<NodeId, PurityFact>,
    /// Range facts.
    pub ranges: HashMap<NodeId, RangeFact>,
    /// Alias facts.
    pub aliases: HashMap<NodeId, AliasFact>,
    /// Escape facts.
    pub escapes: HashMap<NodeId, EscapeFact>,
    /// Loop facts.
    pub loops: HashMap<NodeId, LoopFact>,
    /// Value numbering facts.
    pub value_numbers: HashMap<NodeId, ValueNumberFact>,
}

impl FactDatabase {
    /// Create an empty fact database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true if no facts are stored.
    pub fn is_empty(&self) -> bool {
        self.use_def.is_empty()
            && self.dominance.is_empty()
            && self.constants.is_empty()
            && self.purity.is_empty()
            && self.ranges.is_empty()
            && self.aliases.is_empty()
            && self.escapes.is_empty()
            && self.loops.is_empty()
            && self.value_numbers.is_empty()
    }

    /// Total number of facts across all analyses.
    pub fn total_facts(&self) -> usize {
        self.use_def.len()
            + self.dominance.len()
            + self.constants.len()
            + self.purity.len()
            + self.ranges.len()
            + self.aliases.len()
            + self.escapes.len()
            + self.loops.len()
            + self.value_numbers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_lattice_meet_top() {
        let top = ConstantLattice::Top;
        let c = ConstantLattice::Constant(ConstantData::i32(42));
        assert_eq!(top.meet(&c), c);
        assert_eq!(c.meet(&top), c);
    }

    #[test]
    fn constant_lattice_meet_same_constant() {
        let a = ConstantLattice::Constant(ConstantData::i32(42));
        let b = ConstantLattice::Constant(ConstantData::i32(42));
        assert_eq!(a.meet(&b), a);
    }

    #[test]
    fn constant_lattice_meet_different_constant() {
        let a = ConstantLattice::Constant(ConstantData::i32(42));
        let b = ConstantLattice::Constant(ConstantData::i32(17));
        assert!(a.meet(&b).is_bottom());
    }

    #[test]
    fn constant_lattice_meet_bottom_absorbs() {
        let a = ConstantLattice::Constant(ConstantData::boolean(true));
        assert!(a.meet(&ConstantLattice::Bottom).is_bottom());
    }

    #[test]
    fn fact_database_empty() {
        let db = FactDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.total_facts(), 0);
    }

    #[test]
    fn fact_database_insert_use_def() {
        let mut db = FactDatabase::new();
        let fact = UseDefFact {
            definitions: vec![NodeId::new(0)],
            users: vec![NodeId::new(2)],
            is_dead: false,
            use_count: 1,
        };
        db.use_def.insert(NodeId::new(1), fact.clone());
        assert_eq!(db.total_facts(), 1);
        assert!(!db.is_empty());
        assert_eq!(db.use_def.get(&NodeId::new(1)), Some(&fact));
    }

    #[test]
    fn use_def_fact_defaults() {
        let fact = UseDefFact {
            definitions: vec![],
            users: vec![],
            is_dead: true, // no users = dead
            use_count: 0,
        };
        assert!(fact.is_dead);
        assert_eq!(fact.use_count, 0);
    }

    #[test]
    fn range_fact_properties() {
        let fact = RangeFact {
            lower: Some(0),
            upper: Some(63),
            is_nonzero: false, // 0 is in range
            is_power_of_two: false,
            alignment: Some(4),
        };
        assert_eq!(fact.lower, Some(0));
        assert_eq!(fact.upper, Some(63));
        assert!(!fact.is_nonzero);
        assert_eq!(fact.alignment, Some(4));
    }
}
