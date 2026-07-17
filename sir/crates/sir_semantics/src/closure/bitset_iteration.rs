use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::{SemanticTruth, TruthId, Provenance};

/// Rule: `LoopUntilZero(ClearLowestSetBit(X))` => `BitsetIteration`
pub struct ClearLowestToBitsetIteration;

impl ImplicationRule for ClearLowestToBitsetIteration {
    fn name(&self) -> &'static str {
        "ClearLowestToBitsetIteration"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        // Find all LoopUntilZero truths
        for loop_truth in db.truths().filter(|t| t.concept == SemanticConcept::LoopUntilZero) {
            // LoopUntilZero has one input (the value being compared to 0)
            if let Some(loop_checked_val) = loop_truth.inputs.first() {
                // Find a ClearLowestSetBit truth that produces this value
                for clear_truth in db.truths().filter(|t| t.concept == SemanticConcept::ClearLowestSetBit) {
                    if clear_truth.outputs.contains(loop_checked_val) {
                        // The loop is a BitsetIteration over the original value.
                        if let Some(original_value) = clear_truth.inputs.first() {
                            new_truths.push(SemanticTruth {
                                id: TruthId::new(0),
                                concept: SemanticConcept::BitsetIteration,
                                inputs: vec![*original_value],
                                outputs: loop_truth.outputs.clone(), // The loop node
                                origin: loop_truth.origin,
                                provenance: Provenance::Derived {
                                    from_truths: vec![loop_truth.id, clear_truth.id],
                                },
                            });
                        }
                    }
                }
            }
        }

        new_truths
    }
}
