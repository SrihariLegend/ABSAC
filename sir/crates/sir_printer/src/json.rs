use sir_nodes::{Function, Module};

/// JSON serialization wrapper for SIR graphs.
///
/// Since all data types already derive `Serialize`/`Deserialize` via serde,
/// this is a thin convenience wrapper providing helper methods.
pub struct JsonPrinter;

impl JsonPrinter {
    /// Serialize a Function to a JSON string.
    pub fn function_to_string(func: &Function) -> serde_json::Result<String> {
        serde_json::to_string_pretty(func)
    }

    /// Serialize a Function to a JSON writer.
    pub fn function_to_writer(
        func: &Function,
        w: &mut impl std::io::Write,
    ) -> serde_json::Result<()> {
        serde_json::to_writer_pretty(w, func)
    }

    /// Deserialize a Function from a JSON string.
    pub fn function_from_str(s: &str) -> serde_json::Result<Function> {
        serde_json::from_str(s)
    }

    /// Serialize a Module to a JSON string.
    pub fn module_to_string(module: &Module) -> serde_json::Result<String> {
        serde_json::to_string_pretty(module)
    }

    /// Serialize a Module to a JSON writer.
    pub fn module_to_writer(
        module: &Module,
        w: &mut impl std::io::Write,
    ) -> serde_json::Result<()> {
        serde_json::to_writer_pretty(w, module)
    }

    /// Deserialize a Module from a JSON string.
    pub fn module_from_str(s: &str) -> serde_json::Result<Module> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::Type;

    fn build_add_function() -> Function {
        let mut b = Builder::new("add", &[("a", Type::i32()), ("b", Type::i32())], Type::i32());
        let a = b.parameter_index(0).unwrap();
        let b_ = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_, sir_types::Span::unknown()).unwrap();
        b.return_value(sum, sir_types::Span::unknown()).unwrap();
        b.build()
    }

    #[test]
    fn function_json_roundtrip() {
        let func = build_add_function();
        let json = JsonPrinter::function_to_string(&func).unwrap();
        let parsed = JsonPrinter::function_from_str(&json).unwrap();
        assert_eq!(func.name, parsed.name);
        assert_eq!(func.params.len(), parsed.params.len());
        assert_eq!(func.return_ty, parsed.return_ty);
        assert_eq!(func.node_count(), parsed.node_count());
    }

    #[test]
    fn module_json_roundtrip() {
        let mut module = Module::new("test_module");
        module.add_function(build_add_function());
        let json = JsonPrinter::module_to_string(&module).unwrap();
        let parsed = JsonPrinter::module_from_str(&json).unwrap();
        assert_eq!(module.name, parsed.name);
        assert_eq!(module.function_count(), parsed.function_count());
    }
}
