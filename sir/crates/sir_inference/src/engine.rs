use std::collections::{BTreeMap, HashSet};
use std::fmt;

use sir_semantics::region::RegionId;
use sir_semantics::semantics::SemanticDatabase;
use sir_semantics::structure::StructuralDatabase;
use sir_types::RegionMap;

use crate::evidence::{EvidenceRegistry, Polarity};
use crate::hypothesis::{Hypothesis, Support};
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::{TransformationContext, TransformationContextDatabase};
use sir_transform::representation::Representation;

/// The hypothesis database — stores representation beliefs per region.
#[derive(Clone, Debug, Default)]
pub struct HypothesisDatabase {
    map: RegionMap<Hypothesis>,
}

impl HypothesisDatabase {
    pub fn new() -> Self {
        Self {
            map: RegionMap::new(),
        }
    }

    /// Get all hypotheses for a region.
    pub fn hypotheses(&self, region: RegionId) -> &[Hypothesis] {
        self.map.get(region)
    }

    /// Get the highest-scoring hypothesis for a region.
    pub fn best(&self, region: RegionId) -> Option<&Hypothesis> {
        self.map
            .get(region)
            .iter()
            .max_by_key(|h| h.support.score())
    }

    /// Find all regions that have at least one hypothesis for the
    /// given representation.
    pub fn regions_supporting(&self, rep: Representation) -> Vec<RegionId> {
        self.map
            .iter()
            .filter(|(_, hyps)| hyps.iter().any(|h| h.representation == rep))
            .map(|(rid, _)| rid)
            .collect()
    }

    /// Add a hypothesis to a region.
    pub fn add_hypothesis(&mut self, region: RegionId, hypothesis: Hypothesis) {
        self.map.insert(region, hypothesis);
    }
}

/// A formatted explanation of why a hypothesis exists.
#[derive(Clone, Debug)]
pub struct Explanation {
    pub region: RegionId,
    pub representation: Representation,
    pub support: Support,
    pub evidence_lines: Vec<EvidenceLine>,
}

/// A single line in an explanation: the evidence entry and its contribution.
#[derive(Clone, Debug)]
pub struct EvidenceLine {
    pub polarity: crate::evidence::Polarity,
    pub weight: u16,
    pub source: String,
    pub explanation: String,
}

impl fmt::Display for Explanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Hypothesis: {}", self.representation)?;
        writeln!(
            f,
            "Support: +{} / -{} (net {})",
            self.support.positive,
            self.support.negative,
            self.support.score()
        )?;
        writeln!(f, "Confidence: {}", self.support.confidence_label())?;
        writeln!(f, "Evidence:")?;
        for line in &self.evidence_lines {
            let sign = match line.polarity {
                Polarity::Supports => '+',
                Polarity::Against => '-',
            };
            writeln!(
                f,
                "  {}{:<4} {:<22} \"{}\"",
                sign, line.weight, line.source, line.explanation
            )?;
        }
        Ok(())
    }
}

/// Evidence weight constants — relative strength categories.
pub mod weights {
    pub const ABSOLUTE: u16 = 100;
    pub const STRONG: u16 = 30;
    pub const MODERATE: u16 = 20;
    pub const WEAK: u16 = 10;
}

/// Default assumptions for all transformation contexts.
static DEFAULT_ASSUMPTIONS: [Assumption; 3] = [
    Assumption::EquivalentCardinality,
    Assumption::PreservesLayout,
    Assumption::PreservesIterationOrder,
];

/// The inference engine — transforms semantic truths into representation beliefs.
pub struct InferenceEngine {
    db: HypothesisDatabase,
    evidence_registry: EvidenceRegistry,
    context_db: TransformationContextDatabase,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            db: HypothesisDatabase::new(),
            evidence_registry: EvidenceRegistry::new(),
            context_db: TransformationContextDatabase::new(),
        }
    }

    /// Access the hypothesis database (read-only after inference).
    pub fn database(&self) -> &HypothesisDatabase {
        &self.db
    }

    /// Access the transformation context database (read-only after inference).
    pub fn context_database(&self) -> &TransformationContextDatabase {
        &self.context_db
    }

    /// Run inference: generate evidence from semantic truths, aggregate
    /// into support scores, and form hypotheses.
    ///
    /// Consumes both `SemanticDatabase` and `StructuralDatabase` to produce
    /// hypotheses and transformation contexts.
    pub fn infer(&mut self, semantic_db: &SemanticDatabase, structural_db: &StructuralDatabase) {
        // Reset state to ensure idempotency
        self.evidence_registry = EvidenceRegistry::new();
        self.db = HypothesisDatabase::new();
        self.context_db = TransformationContextDatabase::new();

        let truths: Vec<_> = semantic_db.truths().cloned().collect();

        // 1. Generate evidence from all regions
        for (_, region) in semantic_db.regions() {
            let evidence = crate::sources::bitset_evidence::contribute(region, &truths);
            for e in evidence {
                self.evidence_registry.add(e);
            }
            let arith_evidence = crate::sources::arithmetic_evidence::contribute(region);
            for e in arith_evidence {
                self.evidence_registry.add(e);
            }
            let scan_evidence = crate::sources::bitscan_evidence::contribute(region);
            for e in scan_evidence {
                self.evidence_registry.add(e);
            }
            let mask_evidence = crate::sources::mask_algebra_evidence::contribute(region);
            for e in mask_evidence {
                self.evidence_registry.add(e);
            }
        }

        // 2. Aggregate evidence per (region, representation)
        // Build a map: (RegionId, Representation) -> (positive_sum, negative_sum, evidence_ids)
        let mut aggregation: BTreeMap<(RegionId, Representation), (u32, u32, Vec<usize>)> =
            BTreeMap::new();

        for (evidence_id, evidence) in self.evidence_registry.all().iter().enumerate() {
            let key = (evidence.region, evidence.representation);
            let entry = aggregation.entry(key).or_insert_with(|| (0, 0, Vec::new()));
            match evidence.polarity {
                Polarity::Supports => entry.0 += evidence.weight as u32,
                Polarity::Against => entry.1 += evidence.weight as u32,
            }
            entry.2.push(evidence_id);
        }

        // 3. Form hypotheses and build TransformationContexts in a single pass.
        //    Consuming `aggregation` with into_iter() lets us move evidence_ids
        //    into the Hypothesis rather than cloning.
        for ((region_id, representation), (positive, negative, evidence_ids)) in
            aggregation.into_iter()
        {
            if positive > 0 || negative > 0 {
                let hypothesis = Hypothesis {
                    representation,
                    support: Support { positive, negative },
                    evidence: evidence_ids, // moved, not cloned
                };
                self.db.add_hypothesis(region_id, hypothesis);
            }

            // Build TransformationContext for every region+representation pair
            // that has a structural description
            if let Some(structural) = structural_db.region(region_id) {
                let mut constraints = structural.constraints.clone();
                constraints.insert(Constraint::FiniteIteration);

                let assumptions: HashSet<Assumption> =
                    DEFAULT_ASSUMPTIONS.iter().cloned().collect();

                let ctx = TransformationContext::new(
                    region_id,
                    representation,
                    structural.source_structure.clone(),
                    constraints,
                    assumptions,
                );
                let _ = ctx.validate();
                self.context_db.insert(region_id, ctx);
            }
        }
    }

    /// Explain why a hypothesis exists for a given region and representation.
    /// This is a first-class API, not a debug helper.
    pub fn explain(&self, region: RegionId, rep: Representation) -> Option<Explanation> {
        let hypothesis = self.db.best(region)?;
        if hypothesis.representation != rep {
            // Find the hypothesis for this specific representation
            return self
                .db
                .hypotheses(region)
                .iter()
                .find(|h| h.representation == rep)
                .map(|h| self.build_explanation(region, h));
        }
        Some(self.build_explanation(region, hypothesis))
    }

    fn build_explanation(&self, region: RegionId, hypothesis: &Hypothesis) -> Explanation {
        let lines: Vec<EvidenceLine> = hypothesis
            .evidence
            .iter()
            .filter_map(|&eid| self.evidence_registry.get(eid))
            .map(|e| EvidenceLine {
                polarity: e.polarity,
                weight: e.weight,
                source: e.source.to_string(),
                explanation: e.explanation.to_string(),
            })
            .collect();

        Explanation {
            region,
            representation: hypothesis.representation,
            support: hypothesis.support,
            evidence_lines: lines,
        }
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}
