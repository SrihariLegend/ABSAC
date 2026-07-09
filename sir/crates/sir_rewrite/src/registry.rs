use crate::recipe::{RecipeRegistry, RewriteRecipe};
use crate::recipes::popcount::PopcountRecipe;
use crate::recipes::any::AnyRecipe;
use crate::recipes::all::AllRecipe;
use crate::recipes::parity::ParityRecipe;
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
    
    registry
}
