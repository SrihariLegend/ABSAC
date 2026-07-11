use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::SemanticTruth;

/// Rule: `ElementSequence(Collection) -> Element` + `PredicateMap(Element) -> Bool` => `LogicalSequence(Collection) -> Bool`
pub struct PredicateMapToLogicalSequence;

impl ImplicationRule for PredicateMapToLogicalSequence {
    fn name(&self) -> &'static str {
        "PredicateMapToLogicalSequence"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        for pm_truth in db.truths().filter(|t| t.concept == SemanticConcept::PredicateMap) {
            if let Some(element_id) = pm_truth.inputs.first() {
                // Find the ElementSequence that produced this element
                for es_truth in db.truths().filter(|t| t.concept == SemanticConcept::ElementSequence) {
                    if es_truth.outputs.contains(element_id) {
                        if let Some(collection_id) = es_truth.inputs.first() {
                            new_truths.push(SemanticTruth {
                                concept: SemanticConcept::LogicalSequence,
                                inputs: vec![*collection_id],      // The underlying collection
                                outputs: pm_truth.outputs.clone(), // The boolean sequence
                                origin: pm_truth.origin,
                            });
                        }
                    }
                }
            }
        }

        new_truths
    }
}
