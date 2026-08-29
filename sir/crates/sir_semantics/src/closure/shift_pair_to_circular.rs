use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::{Provenance, SemanticTruth, TruthParameter};

/// Rule: `ShiftPairLeft` / `ShiftPairRight` => `CircularPermutation`.
///
/// A shift pair `(x << k) | (x >> (w - k))` is exactly a circular rotation
/// of `x` by `k` bits. The rule promotes the physical pair into the
/// permutation concept, carrying the direction forward so the generator and
/// recipe can select rotate-left vs rotate-right without re-examining the
/// graph.
pub struct ShiftPairToCircularPermutation;

impl ImplicationRule for ShiftPairToCircularPermutation {
    fn name(&self) -> &'static str {
        "Shift pair -> CircularPermutation"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        for truth in db.truths() {
            let direction = match truth.concept {
                SemanticConcept::ShiftPairLeft => Some(sir_transform::roles::ShiftDirection::Left),
                SemanticConcept::ShiftPairRight => Some(sir_transform::roles::ShiftDirection::Right),
                _ => None,
            };
            let Some(direction) = direction else { continue };

            let width = truth
                .parameters
                .iter()
                .find_map(|p| match p {
                    TruthParameter::ShiftPair { width, .. } => Some(*width),
                    _ => None,
                })
                .unwrap_or(0);

            new_truths.push(SemanticTruth {
                parameters: vec![TruthParameter::ShiftPair { width, direction }],
                id: crate::truth::TruthId::new(0),
                concept: SemanticConcept::CircularPermutation,
                inputs: truth.inputs.clone(),
                outputs: truth.outputs.clone(),
                origin: truth.origin,
                provenance: Provenance::Derived {
                    from_truths: vec![truth.id],
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
    use sir_transform::roles::ShiftDirection;

    fn mk_shift_pair(
        concept: SemanticConcept,
        width: u32,
        direction: ShiftDirection,
        origin: RegionId,
        id: usize,
    ) -> SemanticTruth {
        SemanticTruth {
            parameters: vec![TruthParameter::ShiftPair { width, direction }],
            id: crate::truth::TruthId::new(id),
            concept,
            inputs: vec![ValueId::new(1)],
            outputs: vec![ValueId::new(2)],
            origin,
            provenance: Provenance::Physical { nodes: vec![] },
        }
    }

    #[test]
    fn promotes_shift_pair_left_to_circular_permutation() {
        let mut db = SemanticDatabase::new();
        db.add_truth(mk_shift_pair(
            SemanticConcept::ShiftPairLeft,
            64,
            ShiftDirection::Left,
            RegionId::new(0),
            0,
        ));

        let rule = ShiftPairToCircularPermutation;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 1);
        let t = &derived[0];
        assert_eq!(t.concept, SemanticConcept::CircularPermutation);
        assert_eq!(t.inputs, vec![ValueId::new(1)]);
        assert_eq!(t.outputs, vec![ValueId::new(2)]);
        match &t.parameters[0] {
            TruthParameter::ShiftPair { width, direction } => {
                assert_eq!(*width, 64);
                assert_eq!(*direction, ShiftDirection::Left);
            }
            _ => panic!("expected ShiftPair parameter"),
        }
        assert!(matches!(t.provenance, Provenance::Derived { .. }));
    }

    #[test]
    fn promotes_shift_pair_right() {
        let mut db = SemanticDatabase::new();
        db.add_truth(mk_shift_pair(
            SemanticConcept::ShiftPairRight,
            64,
            ShiftDirection::Right,
            RegionId::new(1),
            0,
        ));

        let rule = ShiftPairToCircularPermutation;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].concept, SemanticConcept::CircularPermutation);
        match &derived[0].parameters[0] {
            TruthParameter::ShiftPair { direction, .. } => {
                assert_eq!(*direction, ShiftDirection::Right);
            }
            _ => panic!("expected ShiftPair parameter"),
        }
    }

    #[test]
    fn unrelated_concepts_are_ignored() {
        let mut db = SemanticDatabase::new();
        db.add_truth(SemanticTruth {
            parameters: vec![],
            id: crate::truth::TruthId::new(0),
            concept: SemanticConcept::ShiftMask,
            inputs: vec![],
            outputs: vec![],
            origin: RegionId::new(0),
            provenance: Provenance::Physical { nodes: vec![] },
        });

        let rule = ShiftPairToCircularPermutation;
        let derived = rule.apply(&db);

        assert!(derived.is_empty());
    }
}
