pub mod rules;
pub mod predicate_map_to_seq;
pub mod bitset_iteration;

use crate::semantics::SemanticDatabase;
use crate::truth::SemanticTruth;

/// A rule that infers new semantic truths from existing ones.
pub trait ImplicationRule {
    fn name(&self) -> &'static str;
    
    /// Apply the rule to the current database, returning any *new* truths discovered.
    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth>;
}

/// The engine that computes the semantic closure.
pub struct ClosureEngine {
    rules: Vec<Box<dyn ImplicationRule>>,
}

impl ClosureEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Box<dyn ImplicationRule>) {
        self.rules.push(rule);
    }

    /// Compute the semantic closure, adding derived truths to the database
    /// until a fixed point is reached.
    pub fn compute_closure(&self, db: &mut SemanticDatabase) {
        let mut changed = true;
        let mut iterations = 0;
        let max_iterations = 100;

        while changed && iterations < max_iterations {
            changed = false;
            iterations += 1;

            let mut new_truths = Vec::new();
            for rule in &self.rules {
                let inferred = rule.apply(db);
                for truth in inferred {
                    // Check if we already have this exact truth
                    let already_exists = db.truths().any(|existing| {
                        existing.concept == truth.concept &&
                        existing.inputs == truth.inputs &&
                        existing.outputs == truth.outputs &&
                        existing.origin == truth.origin
                    });

                    if !already_exists {
                        new_truths.push(truth);
                    }
                }
            }

            if !new_truths.is_empty() {
                changed = true;
                for truth in new_truths {
                    // Since it's a new truth, we could optionally generate a new RegionId
                    // or just attach it to an existing one. For now, the rule specifies `origin`.
                    db.add_truth(truth);
                }
            }
        }
    }
}
