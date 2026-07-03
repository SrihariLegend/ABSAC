pub mod cost_model;
pub mod database;
pub mod score;
pub mod selector;

pub use cost_model::{CostModel, DefaultCostModel};
pub use database::{SelectedCandidateOwned, SelectionDatabase, SelectionResultOwned};
pub use score::{CostModelReport, ScoreBreakdown, TransformationScore};
pub use selector::{SelectedCandidate, SelectionResult, Selector, VerifiedCandidate};

#[cfg(test)]
mod tests {
    use super::*;
    use sir_generation::candidate::{
        Candidate, CandidateEffects, CandidateExplanation, CandidateId, ImplementationStrategy,
    };
    use sir_transform::context::ContextId;
    use sir_transform::ids::DefinitionId;
    use sir_types::{CostProfile, RegionId};
    use sir_verification::{Proof, VerificationBackend};

    fn make_candidate(id: u64, expected_cost: CostProfile) -> Candidate {
        Candidate {
            id: CandidateId::new(id),
            region: RegionId::new(1),
            context_id: ContextId::new(1),
            definition_id: DefinitionId::new(0),
            strategy: ImplementationStrategy::Popcount,
            explanation: CandidateExplanation {
                source_concepts: vec![],
                rationale: "mock",
            },
            effects: vec![CandidateEffects::CountingStrategyChange],
            expected_cost,
        }
    }

    fn make_verified_candidate(id: u64, expected_cost: CostProfile) -> VerifiedCandidate {
        use sir_types::ConstantData;
        use sir_verification::semantic::expression::SemanticExpression;
        use sir_verification::semantic::theorem::Theorem;

        let dummy_expr = SemanticExpression::Constant(ConstantData::u64(0));

        VerifiedCandidate {
            candidate: make_candidate(id, expected_cost),
            proof: Proof {
                theorem: Theorem::new(dummy_expr.clone(), dummy_expr.clone()),
                normalized_theorem: Theorem::new(dummy_expr.clone(), dummy_expr.clone()),
                backend: VerificationBackend::Exhaustive,
                steps: vec![],
            },
        }
    }

    fn cheap_cost() -> CostProfile {
        CostProfile {
            instruction_count: 5,
            select_count: 0,
            memory_accesses: 3,
            critical_path_depth: 2,
        }
    }

    fn medium_cost() -> CostProfile {
        CostProfile {
            instruction_count: 7,
            select_count: 1,
            memory_accesses: 5,
            critical_path_depth: 3,
        }
    }

    fn expensive_cost() -> CostProfile {
        CostProfile {
            instruction_count: 15,
            select_count: 5,
            memory_accesses: 15,
            critical_path_depth: 10,
        }
    }

    fn original_cost() -> CostProfile {
        CostProfile {
            instruction_count: 10,
            select_count: 2,
            memory_accesses: 10,
            critical_path_depth: 5,
        }
    }

    fn bs001_original_cost() -> CostProfile {
        CostProfile {
            instruction_count: 9,
            select_count: 1,
            memory_accesses: 64,
            critical_path_depth: 5,
        }
    }

    fn bs001_popcount_cost() -> CostProfile {
        CostProfile {
            instruction_count: 5,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 3,
        }
    }

    // ── Tier 1: Delta computation ─────────────────────────

    #[test]
    fn test_cost_profile_diff() {
        let original = CostProfile {
            instruction_count: 10,
            select_count: 5,
            memory_accesses: 4,
            critical_path_depth: 8,
        };
        let expected = CostProfile {
            instruction_count: 6,
            select_count: 4,
            memory_accesses: 4,
            critical_path_depth: 6,
        };

        let candidate = make_candidate(1, expected.clone());
        let model = DefaultCostModel;
        let score = model.evaluate(&candidate, &original, &expected);

        assert_eq!(score.breakdown.instruction_delta, 4);
        assert_eq!(score.breakdown.select_delta, 1);
        assert_eq!(score.breakdown.memory_delta, 0);
        assert_eq!(score.breakdown.depth_delta, 2);
    }

    // ── Tier 2: Default model total matches sum ───────────

    #[test]
    fn test_default_model_total_matches_sum() {
        let original = CostProfile {
            instruction_count: 10,
            select_count: 5,
            memory_accesses: 4,
            critical_path_depth: 8,
        };
        let expected = CostProfile {
            instruction_count: 6,
            select_count: 4,
            memory_accesses: 4,
            critical_path_depth: 6,
        };

        let candidate = make_candidate(1, expected.clone());
        let model = DefaultCostModel;
        let score = model.evaluate(&candidate, &original, &expected);

        assert_eq!(
            score.total,
            score.breakdown.instruction_delta
                + score.breakdown.select_delta
                + score.breakdown.memory_delta
                + score.breakdown.depth_delta
        );
        assert_eq!(score.total, 7);
    }

    // ── Tier 3: Highest score wins ────────────────────────

    #[test]
    fn test_selector_highest_wins() {
        let selector = Selector::new(DefaultCostModel);

        let verified = vec![
            make_verified_candidate(1, cheap_cost()),
            make_verified_candidate(2, medium_cost()),
        ];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_some());
        assert_eq!(result.chosen.unwrap().candidate.id, CandidateId::new(1));
    }

    // ── Tier 4: Tie → lowest CandidateId wins ─────────────

    #[test]
    fn test_selector_tie_lowest_id() {
        let selector = Selector::new(DefaultCostModel);

        let verified = vec![
            make_verified_candidate(2, cheap_cost()),
            make_verified_candidate(1, cheap_cost()), // Same cost profile, lower ID
        ];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_some());
        // Candidate 1 should win (lower ID)
        assert_eq!(result.chosen.unwrap().candidate.id, CandidateId::new(1));
    }

    // ── Tier 5: Empty input ───────────────────────────────

    #[test]
    fn test_selector_empty_input() {
        let selector = Selector::new(DefaultCostModel);
        let result = selector.select(RegionId::new(1), &[], &original_cost());
        assert!(result.chosen.is_none());
    }

    // ── Tier 6: All negative → None ───────────────────────

    #[test]
    fn test_selector_all_negative() {
        let selector = Selector::new(DefaultCostModel);
        let verified = vec![make_verified_candidate(1, expensive_cost())];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_none());
        assert_eq!(result.rejected.len(), 1);
        assert_eq!(result.rejected[0], CandidateId::new(1));
    }

    // ── Tier 7: Zero delta → accepted ─────────────────────

    #[test]
    fn test_selector_zero_wins() {
        let selector = Selector::new(DefaultCostModel);
        let verified = vec![make_verified_candidate(1, original_cost())];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_some());
        assert_eq!(result.chosen.unwrap().candidate.id, CandidateId::new(1));
    }

    // ── Tier 8: Positive beats zero ───────────────────────

    #[test]
    fn test_selector_positive_beats_zero() {
        let selector = Selector::new(DefaultCostModel);
        let verified = vec![
            make_verified_candidate(2, original_cost()), // zero delta
            make_verified_candidate(1, cheap_cost()),    // +4 improvement
        ];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_some());
        assert_eq!(result.chosen.unwrap().candidate.id, CandidateId::new(1));
    }

    // ── Tier 9: Positive beats negative ───────────────────

    #[test]
    fn test_selector_positive_beats_negative() {
        let selector = Selector::new(DefaultCostModel);
        // slight improvement: instructions 9 (delta +1), otherwise same as original
        let slightly_better = CostProfile {
            instruction_count: 9,
            select_count: 2,
            memory_accesses: 10,
            critical_path_depth: 5,
        };
        let verified = vec![
            make_verified_candidate(2, expensive_cost()), // negative delta
            make_verified_candidate(1, slightly_better),  // positive delta (+1)
        ];

        let result = selector.select(RegionId::new(1), &verified, &original_cost());
        assert!(result.chosen.is_some());
        assert_eq!(result.chosen.unwrap().candidate.id, CandidateId::new(1));
        assert_eq!(result.rejected.len(), 1);
        assert_eq!(result.rejected[0], CandidateId::new(2));
    }

    // ── Tier 10: Deterministic ────────────────────────────

    #[test]
    fn test_deterministic_selection() {
        let selector = Selector::new(DefaultCostModel);

        let verified_a = vec![
            make_verified_candidate(1, cheap_cost()),
            make_verified_candidate(2, cheap_cost()),
        ];
        let verified_b = vec![
            make_verified_candidate(2, cheap_cost()),
            make_verified_candidate(1, cheap_cost()),
        ];

        let result_a = selector.select(RegionId::new(1), &verified_a, &original_cost());
        let result_b = selector.select(RegionId::new(1), &verified_b, &original_cost());

        assert_eq!(
            result_a.chosen.unwrap().candidate.id,
            result_b.chosen.unwrap().candidate.id
        );
    }

    // ── Tier 11: Report formatting ────────────────────────

    #[test]
    fn test_report_formatting() {
        let report = CostModelReport {
            region: RegionId::new(5),
            scores: vec![
                TransformationScore {
                    candidate: CandidateId::new(1),
                    strategy: ImplementationStrategy::Popcount,
                    total: 70,
                    breakdown: ScoreBreakdown {
                        instruction_delta: 4,
                        select_delta: 1,
                        memory_delta: 63,
                        depth_delta: 2,
                    },
                },
                TransformationScore {
                    candidate: CandidateId::new(2),
                    strategy: ImplementationStrategy::BitIteration,
                    total: 66,
                    breakdown: ScoreBreakdown {
                        instruction_delta: 2,
                        select_delta: 0,
                        memory_delta: 63,
                        depth_delta: 1,
                    },
                },
            ],
        };

        let output = format!("{}", report);
        assert!(output.contains("Region 5"));
        assert!(output.contains("Popcount"));
        assert!(output.contains("+70"));
        assert!(output.contains("BitIteration"));
        assert!(output.contains("+66"));
        assert!(output.contains("Winner: Popcount"));
    }

    // ── Tier 12: BS001 acceptance ─────────────────────────

    #[test]
    fn test_bs001_selection() {
        let selector = Selector::new(DefaultCostModel);

        // BitIteration: instructions=7, select=1, mem=1, depth=4
        let bit_iter_cost = CostProfile {
            instruction_count: 7,
            select_count: 1,
            memory_accesses: 1,
            critical_path_depth: 4,
        };

        // MaskConstruction: instructions=9, select=0, mem=2, depth=5
        let mask_cost = CostProfile {
            instruction_count: 9,
            select_count: 0,
            memory_accesses: 2,
            critical_path_depth: 5,
        };

        // PackedBitfield: instructions=11, select=0, mem=2, depth=6
        let packed_cost = CostProfile {
            instruction_count: 11,
            select_count: 0,
            memory_accesses: 2,
            critical_path_depth: 6,
        };

        let mut c1 = make_verified_candidate(1, bs001_popcount_cost());
        c1.candidate.strategy = ImplementationStrategy::Popcount;

        let mut c2 = make_verified_candidate(2, bit_iter_cost);
        c2.candidate.strategy = ImplementationStrategy::BitIteration;

        let mut c3 = make_verified_candidate(3, mask_cost);
        c3.candidate.strategy = ImplementationStrategy::MaskConstruction;

        let mut c4 = make_verified_candidate(4, packed_cost);
        c4.candidate.strategy = ImplementationStrategy::PackedBitfield;

        let verified = vec![c1, c2, c3, c4];
        let result = selector.select(RegionId::new(1), &verified, &bs001_original_cost());

        assert!(result.chosen.is_some());
        let winner = result.chosen.unwrap();
        assert_eq!(winner.candidate.strategy, ImplementationStrategy::Popcount);

        // Verify scores match expected BS001 deltas:
        // Popcount: (9-5)=+4, (1-0)=+1, (64-1)=+63, (5-3)=+2 => total +70
        assert_eq!(winner.score.total, 70);
        assert_eq!(winner.score.breakdown.instruction_delta, 4);
        assert_eq!(winner.score.breakdown.select_delta, 1);
        assert_eq!(winner.score.breakdown.memory_delta, 63);
        assert_eq!(winner.score.breakdown.depth_delta, 2);

        // Rejected count
        assert_eq!(result.rejected.len(), 3);
    }
}
