use crate::recipe::RecipeRegistry;
use crate::recipes::all::AllRecipe;
use crate::recipes::any::AnyRecipe;
use crate::recipes::bitscan_forward::BitScanForwardRecipe;
use crate::recipes::bitscan_reverse::BitScanReverseRecipe;
use crate::recipes::byte_swap::ByteSwapRecipe;
use crate::recipes::divide_shift::DivideShiftRecipe;
use crate::recipes::leading_zero_count::LeadingZeroCountRecipe;
use crate::recipes::modulo_and::BitwiseAndModuloRecipe;
use crate::recipes::multiply_shift::MultiplyShiftRecipe;
use crate::recipes::parity::ParityRecipe;
use crate::recipes::popcount::PopcountRecipe;
use crate::recipes::reverse_bits::ReverseBitsRecipe;
use crate::recipes::rotate::RotateRecipe;
use crate::recipes::shift_mask::ShiftMaskRecipe;
use crate::recipes::trailing_zero_count::TrailingZeroCountRecipe;
use sir_transform::ids::DefinitionId;

/// Create the C3 freeze registry: ONLY the narrow role-bound Any
/// reduction recipe is enabled. This is intentionally separate from
/// `default_registry`.
///
/// QUARANTINE POLICY (D4 phase, advisor directive): the Any
/// transformation is QUARANTINED from trusted/default modes while the
/// soundness remediation is in flight. The H3 adversarial evaluation
/// (h3-eval-1) found two committed-corruption classes (S1: reduction
/// identity not part of the contract; S2: broadcast predicate scalar
/// not proven loop-invariant) plus an analysis panic (R1). This
/// registry therefore exists ONLY as the explicit EXPERIMENTAL test
/// mode used by the evaluation harness and the remediation corpus.
/// No caller outside that apparatus may use it, and the default
/// registry below no longer contains Any.
pub fn any_only_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::new();
    registry.register(Box::new(AnyRecipe::new(DefinitionId::new(4))));
    registry
}

/// Create a default recipe registry populated with all known recipes
/// EXCEPT quarantined definitions. Any (DefinitionId 4) is quarantined
/// here (H3 safety findings S1/S2 — see `docs/H3_RESULTS.md`); it is
/// reachable only through the explicit experimental registry
/// (`any_only_registry`) used by the evaluation harness.
pub fn default_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::new();

    // ID 0: Popcount
    registry.register(Box::new(PopcountRecipe::new(DefinitionId::new(0))));

    // NOTE: ID 4 (Any) is QUARANTINED from the default registry.
    // Re-enable only after the D4 remediation lands and the regression
    // corpus (h3 tier A/B, promoted to D4) passes with zero corrupt
    // commits and zero panics.

    // ID 5: All
    registry.register(Box::new(AllRecipe::new(DefinitionId::new(5))));

    // ID 6: Parity
    registry.register(Box::new(ParityRecipe::new(DefinitionId::new(6))));

    // ID 100: BitwiseAndModulo
    registry.register(Box::new(BitwiseAndModuloRecipe::new(DefinitionId::new(
        100,
    ))));

    // ID 101: DivideShift
    registry.register(Box::new(DivideShiftRecipe::new(DefinitionId::new(101))));

    // ID 102: MultiplyShift
    registry.register(Box::new(MultiplyShiftRecipe::new(DefinitionId::new(102))));

    // ID 103: ShiftMask
    registry.register(Box::new(ShiftMaskRecipe::new(DefinitionId::new(103))));

    // ID 200: BitScanForward
    registry.register(Box::new(BitScanForwardRecipe::new(DefinitionId::new(200))));

    // ID 201: BitScanReverse
    registry.register(Box::new(BitScanReverseRecipe::new(DefinitionId::new(201))));

    // ID 202: TrailingZeroCount
    registry.register(Box::new(TrailingZeroCountRecipe::new(DefinitionId::new(
        202,
    ))));

    // ID 203: LeadingZeroCount
    registry.register(Box::new(LeadingZeroCountRecipe::new(DefinitionId::new(
        203,
    ))));

    // ID 300: ClearLowestSetBit
    registry.register(Box::new(
        crate::recipes::clear_lowest_set_bit::ClearLowestSetBitRecipe::new(DefinitionId::new(300)),
    ));

    // ID 301: IsolateLowestSetBit
    registry.register(Box::new(
        crate::recipes::isolate_lowest_set_bit::IsolateLowestSetBitRecipe::new(DefinitionId::new(
            301,
        )),
    ));

    // ID 302: IsolateLowestClearBit
    registry.register(Box::new(
        crate::recipes::isolate_lowest_clear_bit::IsolateLowestClearBitRecipe::new(
            DefinitionId::new(302),
        ),
    ));

    // ID 303: SetLowestClearBit
    registry.register(Box::new(
        crate::recipes::set_lowest_clear_bit::SetLowestClearBitRecipe::new(DefinitionId::new(303)),
    ));

    // ID 310: RotateLeft
    registry.register(Box::new(RotateRecipe::new_left(DefinitionId::new(310))));

    // ID 311: RotateRight
    registry.register(Box::new(RotateRecipe::new_right(DefinitionId::new(311))));

    // ID 312: ByteSwap
    registry.register(Box::new(ByteSwapRecipe::new(DefinitionId::new(312))));

    // ID 313: ReverseBits
    registry.register(Box::new(ReverseBitsRecipe::new(DefinitionId::new(313))));

    registry
}
