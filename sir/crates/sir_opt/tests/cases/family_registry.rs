fn registry_exposes_expected_families_and_definitions() {
    let families = sir_opt::family::registry();
    assert_eq!(families.len(), 5, "one entry per optimization family");
    let mut definitions: Vec<u64> = families
        .iter()
        .flat_map(|family| family.definitions().iter().copied())
        .collect();
    definitions.sort_unstable();
    assert_eq!(definitions, vec![0, 4, 5, 200, 400, 401, 402]);
}

fn driver_runs_through_the_registry() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let (optimized, rewrites, definitions) =
        sir_opt::optimize(&function, &|_| true).expect("Any must optimize");
    assert_eq!(rewrites, 1);
    assert_eq!(definitions, vec![4]);
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "driver result must not contain the Any loop"
    );
}
