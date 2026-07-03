use serde::{Deserialize, Serialize};

use crate::Function;

/// A module containing one or more functions.
///
/// `Module` is the top-level compilation unit in SIR. It corresponds
/// roughly to a `.rs` file, a `.c` file, or a single translation unit.
///
/// In v0.1, modules contain only functions. Future versions will add
/// support for global variables, type definitions, and constants.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    /// The module's name (typically the file or crate name).
    pub name: String,
    /// The functions defined in this module.
    pub functions: Vec<Function>,
}

impl Module {
    /// Create a new empty module.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
        }
    }

    /// Add a function to the module.
    pub fn add_function(&mut self, func: Function) {
        self.functions.push(func);
    }

    /// Get a function by name.
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Get a mutable reference to a function by name.
    pub fn get_function_mut(&mut self, name: &str) -> Option<&mut Function> {
        self.functions.iter_mut().find(|f| f.name == name)
    }

    /// Return the number of functions in the module.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Return true if the module has no functions.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_types::Type;

    #[test]
    fn module_creation() {
        let m = Module::new("test_module");
        assert_eq!(m.name, "test_module");
        assert!(m.is_empty());
    }

    #[test]
    fn add_and_retrieve_function() {
        let mut m = Module::new("math");
        let f = Function::new("add", Type::i32());
        m.add_function(f);
        assert_eq!(m.function_count(), 1);
        assert!(m.get_function("add").is_some());
        assert!(m.get_function("sub").is_none());
    }

    #[test]
    fn get_function_mut() {
        let mut m = Module::new("mod");
        m.add_function(Function::new("f", Type::Unit));
        {
            let f = m.get_function_mut("f").unwrap();
            f.return_ty = Type::Bool;
        }
        assert_eq!(m.get_function("f").unwrap().return_ty, Type::Bool);
    }

    #[test]
    fn multiple_functions() {
        let mut m = Module::new("multi");
        m.add_function(Function::new("one", Type::Unit));
        m.add_function(Function::new("two", Type::Unit));
        m.add_function(Function::new("three", Type::Unit));
        assert_eq!(m.function_count(), 3);
    }

    #[test]
    fn serde_roundtrip() {
        let mut m = Module::new("test");
        m.add_function(Function::new("f", Type::i32()));
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Module = serde_json::from_str(&json).unwrap();
        assert_eq!(m.name, parsed.name);
        assert_eq!(m.function_count(), parsed.function_count());
    }
}
