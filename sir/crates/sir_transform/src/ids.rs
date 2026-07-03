/// Identifies a variable in a SemanticExpression.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableId(pub u64);

impl VariableId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl std::fmt::Display for VariableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// Identifies a transformation definition in the registry.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefinitionId(pub u64);

impl DefinitionId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl std::fmt::Display for DefinitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "def#{}", self.0)
    }
}

/// Identifies a proof obligation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObligationId(pub u64);

impl ObligationId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl std::fmt::Display for ObligationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "obl#{}", self.0)
    }
}
