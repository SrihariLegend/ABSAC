use std::collections::{HashMap, HashSet};

use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::{NodeId, Type};

use crate::concepts::SemanticConcept;
use crate::cost::CostDatabase;
use crate::cost_deriver::CostDeriver;
use crate::region::{RecognitionExplanation, Region, RegionId};
use crate::structure::StructuralDatabase;

/// The semantic knowledge database.
///
/// Stores regions and their recognized concepts. Immutable after
/// the `SemanticEngine::derive()` call completes.
#[derive(Clone, Debug, Default)]
pub struct SemanticDatabase {
    regions: HashMap<RegionId, Region>,
    next_region_id: u64,
}

impl SemanticDatabase {
    /// Create an empty semantic database.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            next_region_id: 0,
        }
    }

    /// Add a region to the database.
    pub fn add_region(&mut self, region: Region) {
        self.regions.insert(region.id, region);
    }

    /// Iterate over all regions.
    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &Region)> {
        self.regions.iter().map(|(&id, region)| (id, region))
    }

    /// Get a specific region by ID.
    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.regions.get(&id)
    }

    /// Get the explanation for why a concept was recognized in a region.
    pub fn explain(
        &self,
        region: RegionId,
        concept: SemanticConcept,
    ) -> Option<&RecognitionExplanation> {
        self.regions
            .get(&region)
            .and_then(|r| r.explanation(concept))
    }

    /// Number of regions in the database.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get the ID of the first/surviving region (after merging).
    /// Returns `None` if there are no regions.
    pub fn first_region_id(&self) -> Option<RegionId> {
        self.regions.keys().next().copied()
    }

    /// Allocate the next region ID.
    pub(crate) fn next_region_id(&mut self) -> RegionId {
        let id = RegionId::new(self.next_region_id);
        self.next_region_id += 1;
        id
    }

    /// Merge regions that share nodes into single regions.
    ///
    /// After all recognizers have run, related concepts (e.g., all concepts
    /// for the same loop + array computation) may end up in separate regions
    /// because each recognizer creates its own. This method finds overlapping
    /// regions (regions that share SIR node IDs) and merges them so that
    /// one computation maps to one region with all its semantic concepts.
    ///
    /// This is critical for evidence accumulation: a merged region with
    /// multiple concepts produces combined evidence weight, enabling
    /// strong support scores for the resulting representation hypothesis.
    ///
    /// Uses a reverse-index + union-find approach: O(total_nodes) instead
    /// of the naive O(n^3) nested pairwise comparison.
    pub(crate) fn merge_overlapping_regions(&mut self, func: &Function) {
        if self.regions.len() <= 1 {
            return;
        }

        // Build reverse index: node -> Vec<RegionId>
        let mut node_to_regions: HashMap<NodeId, Vec<RegionId>> = HashMap::new();
        for (&rid, region) in &self.regions {
            for &nid in &region.nodes {
                // Do not merge based on shared Parameters or Constants
                if let Some(node) = func.get_node(nid) {
                    if matches!(
                        node.kind,
                        sir_nodes::NodeKind::Parameter { .. } | sir_nodes::NodeKind::Constant(_)
                    ) {
                        continue;
                    }
                }
                node_to_regions.entry(nid).or_default().push(rid);
            }
        }

        // Build a graph of overlapping regions using a simple approach:
        // collect all pairs of regions that share a node, then merge.
        let mut merged: HashSet<RegionId> = HashSet::new();
        let mut merge_map: HashMap<RegionId, RegionId> = HashMap::new(); // source -> target

        for (_, rids) in &node_to_regions {
            if rids.len() <= 1 {
                continue;
            }
            // Choose the smallest as target, merge the rest into it
            let target = *rids.iter().min().unwrap();
            for &rid in rids.iter() {
                if rid != target && !merged.contains(&rid) {
                    merge_map.insert(rid, target);
                    merged.insert(rid);
                }
            }
        }

        // Resolve transitive chains in merge_map:
        // if A -> B and B -> C (B is also a source), resolve to A -> C.
        let mut resolved_map: HashMap<RegionId, RegionId> = HashMap::new();
        for (&src, &tgt) in &merge_map {
            let mut ultimate = tgt;
            while let Some(&next) = merge_map.get(&ultimate) {
                ultimate = next;
            }
            resolved_map.insert(src, ultimate);
        }

        // Merge regions according to the resolved merge map
        for (source_id, target_id) in &resolved_map {
            if let Some(source) = self.regions.remove(source_id) {
                if let Some(target_region) = self.regions.get_mut(target_id) {
                    for &nid in &source.nodes {
                        target_region.nodes.insert(nid);
                    }
                    let concepts: Vec<SemanticConcept> =
                        source.concepts().iter().copied().collect();
                    for concept in concepts {
                        if let Some(expl) = source.explanation(concept) {
                            target_region.add_concept(concept, expl.clone());
                        }
                    }
                }
            }
        }

        // Update next_region_id to avoid reusing IDs
        let max_id = self
            .regions
            .keys()
            .map(|rid| rid.as_u64())
            .max()
            .unwrap_or(0);
        self.next_region_id = max_id + 1;
    }
}

/// The semantic derivation engine.
///
/// Transforms compiler facts into semantic truths by running
/// deterministic recognizers over the function graph.
pub struct SemanticEngine {
    db: SemanticDatabase,
    structural_db: StructuralDatabase,
    cost_db: CostDatabase,
}

impl SemanticEngine {
    /// Create a new semantic engine with an empty database.
    pub fn new() -> Self {
        Self {
            db: SemanticDatabase::new(),
            structural_db: StructuralDatabase::new(),
            cost_db: CostDatabase::new(),
        }
    }

    /// Access the semantic database (read-only after derivation).
    pub fn database(&self) -> &SemanticDatabase {
        &self.db
    }

    /// Access the structural database (read-only after derivation).
    pub fn structural_database(&self) -> &StructuralDatabase {
        &self.structural_db
    }

    /// Access the cost database (read-only after derivation).
    pub fn cost_database(&self) -> &CostDatabase {
        &self.cost_db
    }

    /// Derive semantic truths from the function graph and compiler facts.
    ///
    /// This calls each recognizer, which inspects the function's graph
    /// structure (for node kinds, types, and connectivity) and the
    /// analysis fact database (for trip counts, purity, escape, etc.).
    ///
    /// Recognized concepts are grouped into regions and stored in the
    /// `SemanticDatabase`.
    pub fn derive(&mut self, func: &Function, analysis: &FactDatabase) {
        use crate::recognizers::{
            boolean_collection, cardinality_reduction, conjunctive_reduction,
            disjunctive_reduction, divide_power_of_two, exclusive_reduction, finite_collection,
            membership_traversal, modulo_power_of_two, multiply_power_of_two, predicate_collection,
            shift_mask, set_algebra, mask_algebra,
        };

        let mask_recs = mask_algebra::recognize_mask_algebra(func, analysis);
        for (_concept, explanation, node_ids) in mask_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let sa_recs = set_algebra::recognize_set_algebra(func, analysis);
        for (_concept, explanation, node_ids) in sa_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let bc_recs = boolean_collection::recognize_boolean_collection(func, analysis);
        for (_concept, explanation, node_ids) in bc_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let finite_recs = finite_collection::recognize_finite_collection(func, analysis);
        for (_concept, explanation, node_ids) in finite_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let membership_recs = membership_traversal::recognize_membership_traversal(func, analysis);
        for (_concept, explanation, node_ids) in membership_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let cardinality_recs =
            cardinality_reduction::recognize_cardinality_reduction(func, analysis);
        for (_concept, explanation, node_ids) in cardinality_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let disjunctive_recs =
            disjunctive_reduction::recognize_disjunctive_reduction(func, analysis);
        for (_concept, explanation, node_ids) in disjunctive_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let conjunctive_recs =
            conjunctive_reduction::recognize_conjunctive_reduction(func, analysis);
        for (_concept, explanation, node_ids) in conjunctive_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let exclusive_recs = exclusive_reduction::recognize_exclusive_reduction(func, analysis);
        for (_concept, explanation, node_ids) in exclusive_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let modulo_recs = modulo_power_of_two::recognize_modulo_power_of_two(func, analysis);
        for (_concept, explanation, node_ids) in modulo_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let divide_recs = divide_power_of_two::recognize_divide_power_of_two(func, analysis);
        for (_concept, explanation, node_ids) in divide_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let multiply_recs = multiply_power_of_two::recognize_multiply_power_of_two(func, analysis);
        for (_concept, explanation, node_ids) in multiply_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let shift_mask_recs = shift_mask::recognize_shift_mask(func, analysis);
        for (_concept, explanation, node_ids) in shift_mask_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let pred_recs = predicate_collection::recognize_predicate_collection(func, analysis);
        for (_concept, explanation, node_ids) in pred_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let pos_recs =
            crate::recognizers::position_search::recognize_position_search(func, analysis);
        for (_concept, explanation, node_ids) in pos_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        // Merge overlapping regions so that related concepts
        // (e.g., all concepts for the same loop/array computation)
        // end up in a single region. This enables combined evidence
        // accumulation in the inference engine.
        self.db.merge_overlapping_regions(func);

        // Structural recognizers
        use crate::recognizers::{
            bitmask, boolean_array, divide_power_of_two as struct_divide,
            modulo_power_of_two as struct_modulo, multiply_power_of_two as struct_multiply,
            shift_mask as struct_shift_mask,
        };

        // For mask algebra, we just set the structural description to MaskAlgebraExpression
        for (rid, region) in self.db.regions() {
            if region.contains(SemanticConcept::ClearLowestSetBit) || region.contains(SemanticConcept::LowestSetBit) {
                use sir_transform::structures::SourceStructure;
                let desc = crate::structure::StructuralDescription::new(
                    rid,
                    SourceStructure::MaskAlgebraExpression,
                );
                self.structural_db.add_description(desc);
            }
        }

        let bool_array_recs = boolean_array::recognize_boolean_array(func, analysis);
        for (_region_id, desc) in bool_array_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::LogicalSequence) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        let dyn_bool_seq_recs =
            predicate_collection::recognize_dynamic_boolean_sequence(func, analysis);
        for (_region_id, desc) in dyn_bool_seq_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::LogicalSequence) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        let mod_op_recs = struct_modulo::recognize_modulo_operator(func, analysis);
        for (_region_id, desc) in mod_op_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::ModuloPowerOfTwo) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        let div_op_recs = struct_divide::recognize_divide_operator(func, analysis);
        for (_region_id, desc) in div_op_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::DividePowerOfTwo) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        let mul_op_recs = struct_multiply::recognize_multiply_operator(func, analysis);
        for (_region_id, desc) in mul_op_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::MultiplyPowerOfTwo) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        let shift_mask_op_recs = struct_shift_mask::recognize_shift_mask_operator(func, analysis);
        for (_region_id, desc) in shift_mask_op_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::ShiftMask) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        // In v0.1 we reuse BooleanArray structure for array searches, and BitMask for scalar searches
        for (rid, region) in self.db.regions() {
            if region.contains(SemanticConcept::FirstOccurrence)
                || region.contains(SemanticConcept::LastOccurrence)
            {
                // If not already populated by BooleanArray recognizer
                if self.structural_db.region(rid).is_none() {
                    let desc = crate::structure::StructuralDescription::new(
                        rid,
                        sir_transform::structures::SourceStructure::LogicalSequence { length: 64 }, // stub
                    );
                    self.structural_db.add_description(desc);
                }
            } else if region.contains(SemanticConcept::TrailingZeroSearch)
                || region.contains(SemanticConcept::LeadingZeroSearch)
            {
                if self.structural_db.region(rid).is_none() {
                    let desc = crate::structure::StructuralDescription::new(
                        rid,
                        sir_transform::structures::SourceStructure::BitMask { width: 64 }, // stub
                    );
                    self.structural_db.add_description(desc);
                }
            }
        }

        let bitmask_recs = bitmask::recognize_bitmask(func, analysis);
        for (_region_id, desc) in bitmask_recs {
            for (rid, region) in self.db.regions() {
                if region.contains(SemanticConcept::FiniteCollection) {
                    let mut new_desc = desc.clone();
                    new_desc.region = rid;
                    if self.structural_db.region(rid).is_none() {
                        self.structural_db.add_description(new_desc);
                    }
                }
            }
        }

        // ── Role derivation ────────────────────────────────────
        // Populate RegionRoles on structural descriptions from
        // recognized semantic concepts. For v0.1, this handles
        // BooleanCollectionReduction by identifying collection,
        // accumulator, and result nodes from the function structure.
        self.derive_roles(func, analysis);

        // ── Cost derivation ────────────────────────────────────
        // Compute CostProfile for each region from the SIR nodes.
        // Runs after structural recognition so all regions are known.
        self.cost_db = CostDeriver::derive(func, &self.structural_db);
    }

    /// Derive RegionRoles on structural descriptions from recognized
    /// semantic concepts and function structure.
    ///
    /// For v0.1, handles BooleanCollectionReduction:
    /// - collection: function parameter with Array<Bool> type
    /// - accumulator: loop carried variable with "sum" reduction kind
    /// - result: return node of the function
    fn derive_roles(&mut self, func: &Function, analysis: &FactDatabase) {
        use sir_nodes::NodeKind;
        use sir_transform::roles::RegionRoles;

        for (region_id, region) in self.db.regions() {
            let is_reduction = region.contains(SemanticConcept::CardinalityReduction)
                || region.contains(SemanticConcept::DisjunctiveReduction)
                || region.contains(SemanticConcept::ConjunctiveReduction)
                || region.contains(SemanticConcept::ExclusiveReduction);

            if is_reduction {
                // Identify collection: function parameter with Array<Bool> type
                let mut collection: Option<NodeId> = None;
                let mut accumulator: Option<NodeId> = None;
                let mut result_node: Option<NodeId> = None;

                let mut predicate_scalar: Option<NodeId> = None;
                let mut predicate_op_node: Option<NodeId> = None;

                for node in func.arena.iter() {
                    // Collection: Parameter node
                    if let NodeKind::Parameter { .. } = &node.kind {
                        if let Type::Array { element, .. } = &node.ty {
                            // Only capture boolean array here. Dynamically generated will override.
                            if matches!(element.as_ref(), &Type::Bool) {
                                collection = Some(node.id);
                            }
                        }
                    }
                    // Comparison nodes inside the region indicating a dynamic predicate
                    if region.nodes.contains(&node.id) {
                        if matches!(
                            node.kind,
                            NodeKind::Eq { .. }
                                | NodeKind::Ne { .. }
                                | NodeKind::Lt { .. }
                                | NodeKind::Le { .. }
                                | NodeKind::Gt { .. }
                                | NodeKind::Ge { .. }
                        ) {
                            let inputs = node.kind.input_nodes();
                            if inputs.len() == 2 {
                                // Assume input 0 is array access, input 1 is scalar
                                if let Some(lhs) = func.get_node(inputs[0]) {
                                    if matches!(
                                        lhs.kind,
                                        NodeKind::ArrayAccess { .. } | NodeKind::Load { .. }
                                    ) {
                                        // Retrieve the base array from the access
                                        let mut base_array = None;
                                        if let NodeKind::ArrayAccess { base, .. } = lhs.kind {
                                            base_array = Some(base);
                                        } else if let NodeKind::Load { ptr } = lhs.kind {
                                            if let Some(ptr_node) = func.get_node(ptr) {
                                                if let NodeKind::ArrayAccess { base, .. } =
                                                    ptr_node.kind
                                                {
                                                    base_array = Some(base);
                                                }
                                            }
                                        }
                                        if let Some(b) = base_array {
                                            collection = Some(b);
                                            predicate_scalar = Some(inputs[1]);
                                            predicate_op_node = Some(node.id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Accumulator: loop carried variable with a supported reduction
                    // Result node is the Loop node itself, as it produces the final reduction value.
                    if let NodeKind::Loop { .. } = &node.kind {
                        if region.nodes.contains(&node.id) {
                            result_node = Some(node.id);
                            if let Some(loop_fact) = analysis.loops.get(&node.id) {
                                for reduction in &loop_fact.reductions {
                                    if matches!(
                                        reduction.reduction_kind.as_str(),
                                        "sum" | "bitwise_or" | "bitwise_and" | "bitwise_xor"
                                    ) {
                                        accumulator = Some(reduction.variable);
                                    }
                                }
                            }
                        }
                    }
                }

                if let (Some(collection), Some(result)) = (collection, result_node) {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        if let (Some(scalar), Some(operator)) =
                            (predicate_scalar, predicate_op_node)
                        {
                            desc.roles = Some(RegionRoles::PredicateCollectionReduction {
                                collection,
                                scalar,
                                operator,
                                accumulator,
                                result,
                            });
                        } else {
                            desc.roles = Some(RegionRoles::BooleanCollectionReduction {
                                collection,
                                accumulator,
                                result,
                            });
                        }
                    }
                }
            } else if region.contains(SemanticConcept::ModuloPowerOfTwo) {
                // Find Rem node
                let mut op_info = None;
                for node in func.arena.iter() {
                    if let NodeKind::Rem { lhs, rhs } = &node.kind {
                        op_info = Some((node.id, *lhs, *rhs));
                    }
                }
                if let Some((rem, lhs, rhs)) = op_info {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        desc.roles = Some(RegionRoles::ArithmeticOperation {
                            operator_node: rem,
                            lhs,
                            rhs,
                            result: rem,
                        });
                    }
                }
            } else if region.contains(SemanticConcept::DividePowerOfTwo) {
                let mut op_info = None;
                for node in func.arena.iter() {
                    if let NodeKind::Div { lhs, rhs } = &node.kind {
                        op_info = Some((node.id, *lhs, *rhs));
                    }
                }
                if let Some((div, lhs, rhs)) = op_info {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        desc.roles = Some(RegionRoles::ArithmeticOperation {
                            operator_node: div,
                            lhs,
                            rhs,
                            result: div,
                        });
                    }
                }
            } else if region.contains(SemanticConcept::MultiplyPowerOfTwo) {
                let mut op_info = None;
                for node in func.arena.iter() {
                    if let NodeKind::Mul { lhs, rhs } = &node.kind {
                        op_info = Some((node.id, *lhs, *rhs));
                    }
                }
                if let Some((mul, lhs, rhs)) = op_info {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        desc.roles = Some(RegionRoles::ArithmeticOperation {
                            operator_node: mul,
                            lhs,
                            rhs,
                            result: mul,
                        });
                    }
                }
            } else if region.contains(SemanticConcept::ClearLowestSetBit) {
                let mut op_info = None;
                for node in func.arena.iter() {
                    if let NodeKind::And { .. } = &node.kind {
                        if region.nodes.contains(&node.id) {
                            op_info = Some(node.id);
                            break;
                        }
                    }
                }
                if let Some(and_node) = op_info {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        let mut operand = and_node; 
                        if let NodeKind::And { lhs, rhs } = func.get_node(and_node).unwrap().kind {
                            if let Some(lhs_node) = func.get_node(lhs) {
                                if matches!(lhs_node.kind, NodeKind::Sub { .. }) {
                                    operand = rhs;
                                } else {
                                    operand = lhs;
                                }
                            }
                        }
                        desc.roles = Some(RegionRoles::MaskOperation {
                            operand,
                            result: and_node,
                        });
                    }
                }
            } else if region.contains(SemanticConcept::ShiftMask) {
                let mut op_info = None;
                for node in func.arena.iter() {
                    if let NodeKind::Shr { lhs, rhs } = &node.kind {
                        if let Some(lhs_node) = func.get_node(*lhs) {
                            if let NodeKind::Shl { .. } = &lhs_node.kind {
                                op_info = Some((node.id, *lhs, *rhs));
                            }
                        }
                    }
                }
                if let Some((shr, lhs, rhs)) = op_info {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        desc.roles = Some(RegionRoles::ArithmeticOperation {
                            operator_node: shr,
                            lhs,
                            rhs,
                            result: shr,
                        });
                    }
                }
            } else if region.contains(SemanticConcept::FirstOccurrence)
                || region.contains(SemanticConcept::LastOccurrence)
                || region.contains(SemanticConcept::TrailingZeroSearch)
                || region.contains(SemanticConcept::LeadingZeroSearch)
            {
                let mut collection = None;
                let mut scalar = None;
                let mut result_node = None;

                for node in func.arena.iter() {
                    if region.nodes.contains(&node.id) {
                        if let NodeKind::Loop { .. } = &node.kind {
                            result_node = Some(node.id);
                        }
                    }
                    if let NodeKind::Parameter { .. } = &node.kind {
                        if let Type::Array { element, .. } = &node.ty {
                            if matches!(element.as_ref(), &Type::Bool) {
                                collection = Some(node.id);
                            }
                        } else if matches!(node.ty, Type::Integer { .. }) {
                            println!("Found scalar parameter for PositionSearch: {:?}", node.id);
                            scalar = Some(node.id); // Hacky for v0.1
                        }
                    }
                }

                println!(
                    "PositionSearch roles assigned: collection={:?}, scalar={:?}, result={:?}",
                    collection, scalar, result_node
                );

                if let Some(result) = result_node {
                    if let Some(desc) = self.structural_db.region_mut(region_id) {
                        desc.roles = Some(RegionRoles::PositionSearch {
                            collection,
                            scalar,
                            result,
                        });
                    }
                }
            }
        }
    }
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}
