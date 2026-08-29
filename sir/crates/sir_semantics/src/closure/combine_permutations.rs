use crate::closure::ImplicationRule;
use crate::concepts::SemanticConcept;
use crate::semantics::SemanticDatabase;
use crate::truth::{Provenance, SemanticTruth, TruthParameter};

/// Rule: compose chains of `MaskedShiftSwap` truths into a single
/// permutation concept.
///
/// A `MaskedShiftSwap{mask, shift}` truth exchanges bit `i` and `i + shift`
/// for every set bit `i` in `mask`. When the output of one stage feeds the
/// input of the next, the stages form a pipeline that composes into one
/// permutation of bit positions. This rule recognises the two canonical
/// compositions:
///
/// * **BytePermutation** — the composed map reverses groups of 8 bits
///   (byte-order reversal over at least two bytes).
/// * **BitPermutation** — the composed map is a full bit reversal
///   (`mapping[i] == width - 1 - i`).
///
/// The composed truth carries its map as a `TruthParameter::BitPermutation`
/// so downstream consumers (role derivation, recipes) never re-derive the
/// physical stages.
pub struct CombinePermutations;

impl ImplicationRule for CombinePermutations {
    fn name(&self) -> &'static str {
        "CombinePermutations"
    }

    fn apply(&self, db: &SemanticDatabase) -> Vec<SemanticTruth> {
        let mut new_truths = Vec::new();

        // Collect the physical swap stages with their parameters.
        let stages: Vec<&SemanticTruth> = db
            .truths()
            .filter(|t| t.concept == SemanticConcept::MaskedShiftSwap)
            .filter(|t| {
                t.parameters
                    .iter()
                    .any(|p| matches!(p, TruthParameter::MaskedShiftSwap { .. }))
            })
            .collect();

        if stages.is_empty() {
            return new_truths;
        }

        // Map each produced value to the stage that produces it.
        let mut produced_by: std::collections::HashMap<crate::truth::ValueId, &SemanticTruth> =
            std::collections::HashMap::new();
        for stage in &stages {
            if let Some(out) = stage.outputs.first() {
                produced_by.insert(*out, stage);
            }
        }

        // A stage is a chain root when its input is not produced by another stage.
        for root in &stages {
            let root_input = match root.inputs.first() {
                Some(v) => *v,
                None => continue,
            };
            if produced_by.contains_key(&root_input) {
                continue;
            }

            // Walk the linear chain root -> ... -> top.
            let mut chain: Vec<&SemanticTruth> = vec![root];
            let mut cur = root;
            loop {
                let cur_out = match cur.outputs.first() {
                    Some(v) => *v,
                    None => break,
                };
                let next = stages.iter().find(|s| s.inputs.first() == Some(&cur_out));
                match next {
                    Some(n) => {
                        chain.push(n);
                        cur = n;
                    }
                    None => break,
                }
            }


            // Build each stage's bit map and require a shared width.
            let mut width: Option<u32> = None;
            let mut maps: Vec<Vec<u32>> = Vec::new();
            let mut ok = true;
            for stage in &chain {
                let param = stage.parameters.iter().find_map(|p| match p {
                    TruthParameter::MaskedShiftSwap { mask, shift } => Some((*mask, *shift)),
                    _ => None,
                });
                let (mask, shift) = match param {
                    Some(p) => p,
                    None => {
                        ok = false;
                        break;
                    }
                };
                let (w, map) = stage_map(mask, shift);
                match width {
                    None => width = Some(w),
                    Some(prev) => {
                        if prev != w {
                            ok = false;
                            break;
                        }
                    }
                }
                maps.push(map);
            }
            if !ok {
                continue;
            }
            let width = match width {
                Some(w) => w,
                None => continue,
            };
            if width == 0 || width > 64 {
                continue;
            }

            // Compose left-to-right: combined[i] = acc[map[i]].
            let mut combined: Vec<u32> = (0..width).collect();
            for map in &maps {
                combined = compose(&combined, map);
            }

            // Classify the composed permutation.
            let concept = classify(&combined, width);
            let Some(concept) = concept else {
                continue;
            };

            let top = chain.last().unwrap();
            let root_value = match root.inputs.first() {
                Some(v) => *v,
                None => continue,
            };
            let top_output = match top.outputs.first() {
                Some(v) => *v,
                None => continue,
            };

            new_truths.push(SemanticTruth {
                parameters: vec![TruthParameter::BitPermutation {
                    width,
                    mapping: combined,
                }],
                id: crate::truth::TruthId::new(0),
                concept,
                inputs: vec![root_value],
                outputs: vec![top_output],
                origin: top.origin,
                provenance: Provenance::Derived {
                    from_truths: chain.iter().map(|t| t.id).collect(),
                },
            });
        }

        new_truths
    }
}

/// Build the width and bit map for one `MaskedShiftSwap{mask, shift}` stage.
///
/// The map has `width` entries where `map[i]` is the source bit feeding
/// result bit `i`. Bits `i` and `i + shift` are exchanged for every set bit
/// `i` in `mask`; the width is the highest affected bit plus one.
fn stage_map(mask: u64, shift: u32) -> (u32, Vec<u32>) {
    let mut highest: u32 = 0;
    for i in 0..64 {
        if mask & (1u64 << i) != 0 {
            highest = highest.max(i);
            if let Some(hi) = i.checked_add(shift) {
                highest = highest.max(hi);
            }
        }
    }
    let width = highest + 1;
    let mut map: Vec<u32> = (0..width).collect();
    for i in 0..width {
        if mask & (1u64 << i) != 0 {
            let j = i + shift;
            if j < width {
                map.swap(i as usize, j as usize);
            }
        }
    }
    (width, map)
}

/// Compose two maps `a` then `b`: result bit `i` reads `a[b[i]]`.
fn compose(a: &[u32], b: &[u32]) -> Vec<u32> {
    b.iter().map(|&i| a[i as usize]).collect()
}

/// Classify a composed map into a permutation concept, or `None` when the
/// map is not one of the canonical permutations.
fn classify(map: &[u32], width: u32) -> Option<SemanticConcept> {
    let w = width as usize;

    // Byte-order reversal: groups of 8 bits reversed over at least two bytes.
    if width >= 16 && width % 8 == 0 {
        let bytes = width / 8;
        let is_byte_reversal = (0..w).all(|i| {
            let byte = (i / 8) as u32;
            let offset = (i % 8) as u32;
            map[i] == (bytes - 1 - byte) * 8 + offset
        });
        if is_byte_reversal {
            return Some(SemanticConcept::BytePermutation);
        }
    }

    // Full bit reversal over at least two bits.
    if width >= 2 {
        let is_bit_reversal = (0..w).all(|i| map[i] == (w - 1 - i) as u32);
        if is_bit_reversal {
            return Some(SemanticConcept::BitPermutation);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::RegionId;
    use crate::truth::ValueId;

    fn mk_swap(mask: u64, shift: u32, input: u64, output: u64, origin: RegionId, id: usize) -> SemanticTruth {
        SemanticTruth {
            parameters: vec![TruthParameter::MaskedShiftSwap { mask, shift }],
            id: crate::truth::TruthId::new(id),
            concept: SemanticConcept::MaskedShiftSwap,
            inputs: vec![ValueId::new(input)],
            outputs: vec![ValueId::new(output)],
            origin,
            provenance: Provenance::Physical { nodes: vec![] },
        }
    }

    #[test]
    fn single_stage_byte_swap_derives_byte_permutation() {
        let mut db = SemanticDatabase::new();
        db.add_truth(mk_swap(0xFF, 8, 1, 2, RegionId::new(0), 0));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 1);
        let t = &derived[0];
        assert_eq!(t.concept, SemanticConcept::BytePermutation);
        assert_eq!(t.inputs, vec![ValueId::new(1)]);
        assert_eq!(t.outputs, vec![ValueId::new(2)]);
        assert_eq!(t.origin, RegionId::new(0));
        let param = t
            .parameters
            .iter()
            .find_map(|p| match p {
                TruthParameter::BitPermutation { width, mapping } => Some((*width, mapping.clone())),
                _ => None,
            })
            .expect("derived truth carries BitPermutation parameter");
        assert_eq!(param.0, 16);
        let expected: Vec<u32> = (8..16).chain(0..8).collect();
        assert_eq!(param.1, expected);
    }

    #[test]
    fn three_stage_chain_derives_bit_permutation() {
        let mut db = SemanticDatabase::new();
        // Stage 1: 0x55 << 1, Stage 2: 0x33 << 2, Stage 3: 0x0F << 4
        db.add_truth(mk_swap(0x55, 1, 10, 11, RegionId::new(0), 0));
        db.add_truth(mk_swap(0x33, 2, 11, 12, RegionId::new(1), 1));
        db.add_truth(mk_swap(0x0F, 4, 12, 13, RegionId::new(2), 2));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 1);
        let t = &derived[0];
        assert_eq!(t.concept, SemanticConcept::BitPermutation);
        assert_eq!(t.inputs, vec![ValueId::new(10)]);
        assert_eq!(t.outputs, vec![ValueId::new(13)]);
        assert_eq!(t.origin, RegionId::new(2)); // top stage's region
        let param = t
            .parameters
            .iter()
            .find_map(|p| match p {
                TruthParameter::BitPermutation { width, mapping } => Some((*width, mapping.clone())),
                _ => None,
            })
            .expect("derived truth carries BitPermutation parameter");
        assert_eq!(param.0, 8);
        let expected: Vec<u32> = (0..8).rev().collect();
        assert_eq!(param.1, expected);
        // Provenance links all three stages.
        match &t.provenance {
            Provenance::Derived { from_truths } => assert_eq!(from_truths.len(), 3),
            _ => panic!("expected Derived provenance"),
        }
    }

    #[test]
    fn lone_stage_is_not_promoted() {
        let mut db = SemanticDatabase::new();
        // A single swap of the low nibble is a permutation, but it is
        // neither a byte nor a bit reversal, so it is not promoted.
        db.add_truth(mk_swap(0x0F, 4, 1, 2, RegionId::new(0), 0));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert!(derived.is_empty());
    }

    #[test]
    fn independent_stages_derive_separate_permutations() {
        let mut db = SemanticDatabase::new();
        // Two byte swaps on independent values: 1 -> 2 and 3 -> 4. Each is
        // its own single-stage permutation; they must not cross-chain.
        db.add_truth(mk_swap(0xFF, 8, 1, 2, RegionId::new(0), 0));
        db.add_truth(mk_swap(0xFF, 8, 3, 4, RegionId::new(1), 1));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert_eq!(derived.len(), 2);
        for t in &derived {
            assert_eq!(t.concept, SemanticConcept::BytePermutation);
        }
        let inputs: Vec<ValueId> = derived.iter().map(|t| t.inputs[0]).collect();
        assert!(inputs.contains(&ValueId::new(1)));
        assert!(inputs.contains(&ValueId::new(3)));
    }

    #[test]
    fn unrelated_nonclassifying_stages_derive_nothing() {
        let mut db = SemanticDatabase::new();
        // Nibble swaps on independent values: neither classifies, and the
        // rule must not invent a chain between them.
        db.add_truth(mk_swap(0x0F, 4, 1, 2, RegionId::new(0), 0));
        db.add_truth(mk_swap(0x0F, 4, 3, 4, RegionId::new(1), 1));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert!(derived.is_empty());
    }

    #[test]
    fn mismatched_widths_do_not_chain() {
        let mut db = SemanticDatabase::new();
        // Stage 1 covers 16 bits, stage 2 covers 8 bits: not composable.
        db.add_truth(mk_swap(0xFF, 8, 10, 11, RegionId::new(0), 0));
        db.add_truth(mk_swap(0x55, 1, 11, 12, RegionId::new(1), 1));

        let rule = CombinePermutations;
        let derived = rule.apply(&db);

        assert!(derived.is_empty());
    }
}
