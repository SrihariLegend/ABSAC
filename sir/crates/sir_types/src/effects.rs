use bitflags::bitflags;
use serde::{Deserialize, Serialize};

// Side effects produced by a node.
//
// `Effects` uses a bitflags representation. `Effects::empty()` represents
// purity — a node with no side effects. Each flag marks a category of
// observable effect.
//
// Multiple effects can be combined with the `|` operator:
//   let both = Effects::READ_MEMORY | Effects::WRITE_MEMORY;
bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Effects: u32 {
        /// The node reads from memory (e.g., `Load`).
        const READ_MEMORY  = 0b0000_0001;
        /// The node writes to memory (e.g., `Store`, `Deallocate`).
        const WRITE_MEMORY = 0b0000_0010;
        /// The node allocates memory (e.g., `Allocate`).
        const ALLOCATE     = 0b0000_0100;
        /// The node performs I/O (e.g., `println!`, file operations).
        const IO           = 0b0000_1000;
        /// The node performs an atomic operation.
        const ATOMIC       = 0b0001_0000;
    }
}

impl Effects {
    /// Returns true if the node is pure (has no side effects).
    pub fn is_pure(self) -> bool {
        self.is_empty()
    }

    /// Returns true if the node has any memory-related effect.
    pub fn touches_memory(self) -> bool {
        self.intersects(Effects::READ_MEMORY | Effects::WRITE_MEMORY | Effects::ALLOCATE)
    }

    /// Return a human-readable string describing the effects.
    pub fn describe(self) -> String {
        if self.is_empty() {
            return "Pure".to_string();
        }
        let mut parts: Vec<&str> = Vec::new();
        if self.contains(Effects::READ_MEMORY) {
            parts.push("ReadMemory");
        }
        if self.contains(Effects::WRITE_MEMORY) {
            parts.push("WriteMemory");
        }
        if self.contains(Effects::ALLOCATE) {
            parts.push("Allocate");
        }
        if self.contains(Effects::IO) {
            parts.push("IO");
        }
        if self.contains(Effects::ATOMIC) {
            parts.push("Atomic");
        }
        parts.join(" | ")
    }
}

impl std::fmt::Display for Effects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.describe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_is_empty() {
        let e = Effects::empty();
        assert!(e.is_pure());
        assert!(!e.touches_memory());
        assert_eq!(e.describe(), "Pure");
    }

    #[test]
    fn combine_effects() {
        let e = Effects::READ_MEMORY | Effects::WRITE_MEMORY;
        assert!(!e.is_pure());
        assert!(e.touches_memory());
        assert!(e.contains(Effects::READ_MEMORY));
        assert!(e.contains(Effects::WRITE_MEMORY));
    }

    #[test]
    fn display_effects() {
        assert_eq!(
            format!("{}", Effects::empty()),
            "Pure"
        );
        let e = Effects::READ_MEMORY | Effects::IO;
        assert!(format!("{e}").contains("ReadMemory"));
        assert!(format!("{e}").contains("IO"));
    }

    #[test]
    fn serde_roundtrip() {
        let e = Effects::READ_MEMORY | Effects::WRITE_MEMORY;
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Effects = serde_json::from_str(&json).unwrap();
        assert_eq!(e, parsed);
    }
}
