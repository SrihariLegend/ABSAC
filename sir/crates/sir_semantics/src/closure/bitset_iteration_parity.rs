use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::{Provenance, SemanticTruth, TruthId};

/// Rule: `BitsetIteration(loop over x)` + `ExclusiveReduction(same loop)` => `Parity(x)`
///
/// A Kernighan-style loop clears one set bit per iteration and toggles a
/// parity accumulator (the XOR reduction). Since the loop visits each set
/// bit exactly once, the final parity equals the parity of the original
/// value: `popcount(x) mod 2`. This is the roadmap's
/// `BitsetIteration + Modulo 2 -> Parity` closure.
pub struct BitsetIterationParity;

impl ImplicationRule for BitsetIterationParity {
    fn name(&self) -> &'static str {
        "BitsetIterationParity"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        // Collect the ExclusiveReduction truths that are XOR toggles inside loops.
        let xor_toggles: Vec<_> = db
            .truths()
            .filter(|t| t.concept == SemanticConcept::ExclusiveReduction)
            .collect();

        for bitset_truth in db
            .truths()
            .filter(|t| t.concept == SemanticConcept::BitsetIteration)
        {
            // BitsetIteration outputs the loop node; its input is the original value.
            let loop_node = match bitset_truth.outputs.first() {
                Some(v) => *v,
                None => continue,
            };
            let original_value = match bitset_truth.inputs.first() {
                Some(v) => *v,
                None => continue,
            };

            // The same loop must carry the XOR toggle.
            let xor_toggle = xor_toggles
                .iter()
                .find(|t| t.origin == bitset_truth.origin || t.outputs.contains(&loop_node));
            let xor_truth_id = match xor_toggle {
                Some(t) => t.id,
                None => continue,
            };

            new_truths.push(SemanticTruth {
                id: TruthId::new(0),
                concept: SemanticConcept::Parity,
                inputs: vec![original_value],
                outputs: vec![loop_node],
                origin: bitset_truth.origin,
                provenance: Provenance::Derived {
                    from_truths: vec![bitset_truth.id, xor_truth_id],
                },
            });
        }

        new_truths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::RegionId;
    use crate::truth::ValueId;

    fn mk_truth(
        concept: SemanticConcept,
        inputs: Vec<u64>,
        outputs: Vec<u64>,
        origin: RegionId,
        id: usize,
    ) -> SemanticTruth {
        SemanticTruth {
            id: TruthId::new(id),
            concept,
            inputs: inputs.into_iter().map(ValueId::new).collect(),
            outputs: outputs.into_iter().map(ValueId::new).collect(),
            origin,
            provenance: Provenance::Physical { nodes: vec![] },
        }
    }

    #[test]
    fn derives_parity_from_bitset_iteration_with_xor_toggle() {
        let mut db = SemanticDatabase::new();
        let origin = RegionId::new(0);

        let bitset = mk_truth(SemanticConcept::BitsetIteration, vec![10], vec![20], origin, 0);
        let xor_toggle = mk_truth(SemanticConcept::ExclusiveReduction, vec![], vec![20], origin, 1);
        db.add_truth(bitset);
        db.add_truth(xor_toggle);

        let rule = BitsetIterationParity;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 1, "expected exactly one Parity truth");
        let parity = &derived[0];
        assert_eq!(parity.concept, SemanticConcept::Parity);
        assert_eq!(parity.inputs, vec![ValueId::new(10)]);
        assert_eq!(parity.outputs, vec![ValueId::new(20)]);
        assert_eq!(parity.origin, origin);
        assert!(matches!(parity.provenance, Provenance::Derived { .. }));
    }

    #[test]
    fn does_not_derive_parity_without_xor_toggle() {
        let mut db = SemanticDatabase::new();
        let origin = RegionId::new(0);

        // BitsetIteration loop but NO ExclusiveReduction toggle (a pure count loop).
        let bitset = mk_truth(SemanticConcept::BitsetIteration, vec![10], vec![20], origin, 0);
        let count = mk_truth(SemanticConcept::CardinalityReduction, vec![], vec![20], origin, 1);
        db.add_truth(bitset);
        db.add_truth(count);

        let rule = BitsetIterationParity;
        let derived = rule.apply(&db);
        assert!(derived.is_empty(), "count loops must not derive Parity");
    }

    #[test]
    fn does_not_derive_parity_without_bitset_iteration() {
        let mut db = SemanticDatabase::new();
        let origin = RegionId::new(0);

        // XOR toggle without a BitsetIteration loop (array parity loop).
        let xor_toggle = mk_truth(SemanticConcept::ExclusiveReduction, vec![], vec![20], origin, 0);
        db.add_truth(xor_toggle);

        let rule = BitsetIterationParity;
        let derived = rule.apply(&db);
        assert!(derived.is_empty(), "array parity must not derive scalar Parity");
    }
}
