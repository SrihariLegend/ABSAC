/// Properties already established by analysis or semantics.
///
/// A Constraint is already established. It cannot become false unless
/// the underlying analysis changes. Constraints are NOT assumptions
/// waiting to be proven — they are facts that have been determined.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    /// The structure has a statically known size.
    FixedLength(usize),
    /// The structure is not mutated (read-only access).
    ReadOnly,
    /// The structure does not escape the function.
    NoEscape,
    /// The structure is not aliased.
    NoAlias,
    /// The computation iterates a finite, known number of times.
    FiniteIteration,
}
