use std::collections::HashMap;
use std::fmt;

use sir_semantics::region::RegionId;
use sir_semantics::semantics::SemanticDatabase;

use crate::evidence::{EvidenceRegistry, Polarity};
use sir_transform::representation::Representation;
use crate::hypothesis::{Hypothesis, Support};

/// The hypothesis database — stores representation beliefs per region.
#[derive(Clone, Debug, Default)]
pub struct HypothesisDatabase {
    hypotheses: HashMap<RegionId, Vec<Hypothesis>>,
}

impl HypothesisDatabase {
    pub fn new() -> Self {
        Self {
            hypotheses: HashMap::new(),
        }
    }

    /// Get all hypotheses for a region.
    pub fn hypotheses(&self, region: RegionId) -> &[Hypothesis] {
        self.hypotheses
            .get(&region)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the highest-scoring hypothesis for a region.
    pub fn best(&self, region: RegionId) -> Option<&Hypothesis> {
        self.hypotheses
            .get(&region)
            .and_then(|v| v.iter().max_by_key(|h| h.support.score()))
    }

    /// Find all regions that have at least one hypothesis for the
    /// given representation.
    pub fn regions_supporting(&self, rep: Representation) -> Vec<RegionId> {
        self.hypotheses
            .iter()
            .filter(|(_, hyps)| hyps.iter().any(|h| h.representation == rep))
            .map(|(&rid, _)| rid)
            .collect()
    }

    /// Add a hypothesis to a region.
    pub fn add_hypothesis(&mut self, region: RegionId, hypothesis: Hypothesis) {
        self.hypotheses
            .entry(region)
            .or_insert_with(Vec::new)
            .push(hypothesis);
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
    pub const STRONG: u16 = 30;
    pub const MODERATE: u16 = 20;
    pub const WEAK: u16 = 10;
}

/// The inference engine — transforms semantic truths into representation beliefs.
pub struct InferenceEngine {
    db: HypothesisDatabase,
    evidence_registry: EvidenceRegistry,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            db: HypothesisDatabase::new(),
            evidence_registry: EvidenceRegistry::new(),
        }
    }

    /// Access the hypothesis database (read-only after inference).
    pub fn database(&self) -> &HypothesisDatabase {
        &self.db
    }

    /// Run inference: generate evidence from semantic truths, aggregate
    /// into support scores, and form hypotheses.
    ///
    /// This consumes only the `SemanticDatabase` — never SIR or compiler facts.
    pub fn infer(&mut self, semantic_db: &SemanticDatabase) {
        // Reset state to ensure idempotency
        self.evidence_registry = EvidenceRegistry::new();
        self.db = HypothesisDatabase::new();

        // 1. Generate evidence from all regions
        for (_, region) in semantic_db.regions() {
            let evidence = crate::sources::bitset_evidence::contribute(region);
            for e in evidence {
                self.evidence_registry.add(e);
            }
        }

        // 2. Aggregate evidence per (region, representation)
        // Build a map: (RegionId, Representation) -> (positive_sum, negative_sum, evidence_ids)
        let mut aggregation: HashMap<(RegionId, Representation), (u32, u32, Vec<usize>)> =
            HashMap::new();

        for (evidence_id, evidence) in self.evidence_registry.all().iter().enumerate() {
            let key = (evidence.region, evidence.representation);
            let entry = aggregation.entry(key).or_insert_with(|| (0, 0, Vec::new()));
            match evidence.polarity {
                Polarity::Supports => entry.0 += evidence.weight as u32,
                Polarity::Against => entry.1 += evidence.weight as u32,
            }
            entry.2.push(evidence_id);
        }

        // 3. Form hypotheses for any representation with non-zero support
        for ((region_id, representation), (positive, negative, evidence_ids)) in aggregation {
            if positive > 0 || negative > 0 {
                let hypothesis = Hypothesis {
                    representation,
                    support: Support { positive: positive as u16, negative: negative as u16 },
                    evidence: evidence_ids,
                };
                self.db.add_hypothesis(region_id, hypothesis);
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
