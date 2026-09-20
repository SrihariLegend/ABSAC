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
        /// The node performs a volatile memory access.
        /// Volatile accesses must not be merged, reordered, or eliminated.
        const VOLATILE     = 0b0010_0000;
    }
}

impl Effects {
    /// Returns true if the node is pure (has no side effects).
    pub fn is_pure(self) -> bool {
        self.is_empty()
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
        if self.contains(Effects::VOLATILE) {
            parts.push("Volatile");
        }
        parts.join(" | ")
    }
}

impl std::fmt::Display for Effects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.describe())
    }
}
