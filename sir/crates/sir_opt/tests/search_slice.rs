#[test]
fn fixed_extent_inequality_search_builds_select_chain() {
    let ll = std::fs::read_to_string("../../../gate6a/v9_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p02_first_ne_key_u8_64").expect("lower");
    let (optimized, definition) =
        sir_opt::optimize_search(&function).expect("search must optimize");
    assert_eq!(definition, 200, "inequality search is definition 200");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the search loop"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Select { .. }))
            .count()
            >= 64,
        "reverse-overwrite chain must cover the extent"
    );
    assert!(
        sir_opt::optimize_memchr(&function).is_none(),
        "inequality search must not use the memchr primitive"
    );
}
