use crate::recipe::{RecipeRegistry, RewriteRecipe};
use crate::recipes::all::AllRecipe;
use crate::recipes::any::AnyRecipe;
use crate::recipes::divide_shift::DivideShiftRecipe;
use crate::recipes::modulo_and::BitwiseAndModuloRecipe;
use crate::recipes::multiply_shift::MultiplyShiftRecipe;
use crate::recipes::parity::ParityRecipe;
use crate::recipes::popcount::PopcountRecipe;
use crate::recipes::shift_mask::ShiftMaskRecipe;
use sir_transform::ids::DefinitionId;

/// Create a default recipe registry populated with all known recipes.
pub fn default_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::new();

    // ID 0: Popcount
    registry.register(Box::new(PopcountRecipe::new(DefinitionId::new(0))));

    // ID 4: Any
    registry.register(Box::new(AnyRecipe::new(DefinitionId::new(4))));

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

    registry
}
