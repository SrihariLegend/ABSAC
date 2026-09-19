fn pow2_arithmetic_rewrites_to_shifts_and_masks() {
    assert!(sir_opt::prove::divide_pow2_laws(32, 3));
    assert!(sir_opt::prove::modulo_pow2_laws(32, 4));
    assert!(sir_opt::prove::multiply_pow2_laws(32, 4));
    let ll = std::fs::read_to_string("../../../gate6a/v12_corpus.ll").expect("corpus");
    let mut function = sir_lower::lower_function(&ll, "m02_merged_pow2_u32").expect("lower");
    let has_shift_mask = |function: &sir_nodes::Function| {
        function.arena.iter().any(|node| {
            let sir_nodes::NodeKind::Shr { lhs, .. } = &node.kind else {
                return false;
            };
            matches!(
                function.get_node(*lhs).map(|inner| &inner.kind),
                Some(sir_nodes::NodeKind::Shl { .. })
            )
        })
    };
    assert!(has_shift_mask(&function), "fixture must contain a shift-mask chain");
    assert!(
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Div { .. }))
            && function
                .arena
                .iter()
                .any(|node| matches!(node.kind, sir_nodes::NodeKind::Rem { .. })),
        "fixture must contain division and remainder"
    );
    while let Some(next) = sir_opt::optimize_arithmetic(&function) {
        function = next;
    }
    assert!(
        !has_shift_mask(&function),
        "shift-mask chain must lower to a mask"
    );
    assert!(
        !function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Div { .. })),
        "power-of-two division must lower to a shift"
    );
    assert!(
        !function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Rem { .. })),
        "power-of-two remainder must lower to a mask"
    );
    assert!(
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Shr { .. })),
        "shift node must be present"
    );
    assert!(
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::And { .. })),
        "mask node must be present"
    );
}
