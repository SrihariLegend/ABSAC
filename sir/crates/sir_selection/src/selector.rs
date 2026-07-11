use sir_generation::candidate::Candidate;
use sir_semantics::cost::CostDatabase;
use sir_types::{CostProfile, RegionId};
use sir_verification::Proof;

use crate::cost_model::CostModel;
use crate::score::{CostModelReport, TransformationScore};

/// A candidate that has passed verification.
/// Carries everything needed for selection and rewriting.
#[derive(Clone, Debug)]
pub struct VerifiedCandidate {
    pub candidate: Candidate,
    pub proof: Proof,
}

/// The selector's output — no reconstruction needed by the caller.
pub struct SelectedCandidate<'a> {
    pub candidate: &'a Candidate,
    pub proof: &'a Proof,
    pub score: TransformationScore,
}

/// The result of selection for a single region.
pub struct SelectionResult<'a> {
    pub chosen: Option<SelectedCandidate<'a>>,
    pub rejected: Vec<sir_generation::candidate::CandidateId>,
    pub report: CostModelReport,
}

impl<'a> SelectionResult<'a> {
    /// Convert to the owned form for persistent storage.
    pub fn to_owned(&self) -> crate::database::SelectionResultOwned {
        crate::database::SelectionResultOwned {
            chosen: self
                .chosen
                .as_ref()
                .map(|s| crate::database::SelectedCandidateOwned {
                    candidate: s.candidate.clone(),
                    proof: s.proof.clone(),
                    score: s.score.clone(),
                }),
            rejected: self.rejected.clone(),
            report: self.report.clone(),
        }
    }
}

/// Selection result across all regions.
///
/// The selector owns region grouping — the optimizer doesn't need to
/// know that selection is per-region. It just calls select_all() and
/// receives a flat list of chosen candidates.
pub struct MultiRegionSelection<'a> {
    /// All chosen candidates (one per region, at most).
    pub chosen: Vec<SelectedCandidate<'a>>,
    /// Per-region reports for diagnostics.
    pub reports: Vec<CostModelReport>,
}

/// Deterministic selection of the best verified candidate.
///
/// Expected costs come directly from `Candidate.expected_cost`,
/// populated by `sir_generation` at candidate creation time.
pub struct Selector<M: CostModel> {
    cost_model: M,
}

impl<M: CostModel> Selector<M> {
    pub fn new(cost_model: M) -> Self {
        Self { cost_model }
    }

    /// Select the best candidate from verified options for a single region.
    ///
    /// All candidates in `verified` must belong to the same region.
    /// `original_cost` is the pre-computed cost profile of the original
    /// region (computed by the orchestrator by walking SIR region nodes —
    /// the selector never reads SIR).
    ///
    /// For each verified candidate, calls:
    ///   CostModel.evaluate(&candidate, original_cost, &candidate.expected_cost)
    ///
    /// Policy:
    ///   - Filter: total > 0 (strict improvement over original)
    ///   - Rank: highest total wins
    ///   - Tie: lowest CandidateId wins (deterministic, stable)
    ///   - Empty input: chosen is None
    pub fn select<'a>(
        &self,
        region: RegionId,
        verified: &'a [VerifiedCandidate],
        original_cost: &CostProfile,
    ) -> SelectionResult<'a> {
        // All candidates must belong to the same region
        debug_assert!(
            verified.iter().all(|vc| vc.candidate.region == region),
            "selector.select(): all VerifiedCandidates must belong to region {}",
            region
        );

        if verified.is_empty() {
            return SelectionResult {
                chosen: None,
                rejected: vec![],
                report: CostModelReport {
                    region,
                    scores: vec![],
                },
            };
        }

        let mut scored_candidates: Vec<(&VerifiedCandidate, TransformationScore)> = verified
            .iter()
            .map(|vc| {
                let score = self.cost_model.evaluate(
                    &vc.candidate,
                    original_cost,
                    &vc.candidate.expected_cost,
                );
                (vc, score)
            })
            .collect();

        // Sort by total descending, then by candidate ID ascending
        scored_candidates.sort_by(|a, b| {
            b.1.total
                .cmp(&a.1.total)
                .then_with(|| a.0.candidate.id.cmp(&b.0.candidate.id))
        });

        let mut chosen = None;
        let mut rejected = Vec::new();

        if let Some((vc, score)) = scored_candidates.first() {
            if score.total > 0 {
                chosen = Some(SelectedCandidate {
                    candidate: &vc.candidate,
                    proof: &vc.proof,
                    score: score.clone(),
                });

                // Add the rest to rejected
                for (other_vc, _) in scored_candidates.iter().skip(1) {
                    rejected.push(other_vc.candidate.id);
                }
            } else {
                // All rejected because highest score is < 0
                for (other_vc, _) in scored_candidates.iter() {
                    rejected.push(other_vc.candidate.id);
                }
            }
        }

        let scores: Vec<_> = scored_candidates.into_iter().map(|(_, s)| s).collect();

        SelectionResult {
            chosen,
            rejected,
            report: CostModelReport { region, scores },
        }
    }

    /// Select the best candidate per region across all verified candidates.
    ///
    /// Groups candidates by region internally, then applies the same
    /// per-region selection policy. The optimizer calls this once and
    /// receives a flat list of chosen candidates — it doesn't need to
    /// know selection is region-based.
    ///
    /// Policy (same as per-region):
    ///   - Filter: total > 0 (strict improvement)
    ///   - Rank: highest total wins
    ///   - Tie: lowest CandidateId wins
    pub fn select_all<'a>(
        &self,
        verified: &'a [VerifiedCandidate],
        cost_db: &CostDatabase,
    ) -> MultiRegionSelection<'a> {
        use std::collections::BTreeMap;

        // Group verified candidates by region
        let mut by_region: BTreeMap<RegionId, Vec<&'a VerifiedCandidate>> = BTreeMap::new();
        for vc in verified {
            by_region.entry(vc.candidate.region).or_default().push(vc);
        }

        let mut chosen = Vec::new();
        let mut reports = Vec::new();

        for (region, region_candidates) in by_region {
            let original_cost = cost_db.for_region(region);

            if let Some(cost) = original_cost {
                // ── Score each candidate ────────────────────────
                let mut scored: Vec<(&'a VerifiedCandidate, TransformationScore)> =
                    region_candidates
                        .into_iter()
                        .map(|vc| {
                            let score = self.cost_model.evaluate(
                                &vc.candidate,
                                cost,
                                &vc.candidate.expected_cost,
                            );
                            (vc, score)
                        })
                        .collect();

                // ── Sort by total descending, then candidate ID ascending ──
                scored.sort_by(|a, b| {
                    b.1.total
                        .cmp(&a.1.total)
                        .then_with(|| a.0.candidate.id.cmp(&b.0.candidate.id))
                });

                // ── Select winner (strict improvement) ──────────
                if let Some((vc, score)) = scored.first() {
                    if score.total > 0 {
                        chosen.push(SelectedCandidate {
                            candidate: &vc.candidate,
                            proof: &vc.proof,
                            score: score.clone(),
                        });
                    }
                }

                let scores: Vec<_> = scored.into_iter().map(|(_, s)| s).collect();
                reports.push(CostModelReport { region, scores });
            }
        }

        MultiRegionSelection { chosen, reports }
    }
}
