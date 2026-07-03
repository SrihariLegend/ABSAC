use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_semantics::recognizers::boolean_collection::recognize_boolean_collection;

/// Verify the recognizer is callable with a minimal function and returns
/// the expected result type.
#[test]
fn boolean_collection_recognizer_is_callable() {
    let mut func = Function::new("empty", sir_types::Type::Unit);
    func.add_param("x", sir_types::Type::i32(), sir_types::Span::unknown());
    let analysis = FactDatabase::new();
    let results = recognize_boolean_collection(&func, &analysis);

    // With no boolean arrays in the function, should return empty.
    assert!(
        results.is_empty(),
        "Expected no boolean collections in empty function"
    );
}
