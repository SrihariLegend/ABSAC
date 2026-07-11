use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::SemanticTruth;

/// Rule: `ClearLowestSetBit(X) == 0` => `AtMostOneBitSet(X)`
pub struct ClearLowestIsZeroToAtMostOneBit;

impl ImplicationRule for ClearLowestIsZeroToAtMostOneBit {
    fn name(&self) -> &'static str {
        "ClearLowestIsZeroToAtMostOneBit"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        // Find all IsZero truths
        for is_zero_truth in db.truths().filter(|t| t.concept == SemanticConcept::IsZero) {
            // IsZero has one input (the value being compared to 0)
            if let Some(is_zero_input) = is_zero_truth.inputs.first() {
                // Find a ClearLowestSetBit truth that produces this value
                for clear_truth in db.truths().filter(|t| t.concept == SemanticConcept::ClearLowestSetBit) {
                    if clear_truth.outputs.contains(is_zero_input) {
                        // The input to the ClearLowestSetBit operation is the value
                        // that has at most one bit set.
                        if let Some(original_value) = clear_truth.inputs.first() {
                            new_truths.push(SemanticTruth {
                                concept: SemanticConcept::AtMostOneBitSet,
                                inputs: vec![*original_value],
                                outputs: is_zero_truth.outputs.clone(), // The boolean result
                                origin: is_zero_truth.origin, // Bind it to the same semantic origin region as the `== 0` check
                            });
                        }
                    }
                }
            }
        }

        new_truths
    }
}
