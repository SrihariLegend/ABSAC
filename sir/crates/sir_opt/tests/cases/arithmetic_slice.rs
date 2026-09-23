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

/// A constant in the **left** slot of a non-commutative op must not be
/// rewritten: `udiv 8, x` is not `x >> 3`, and `urem 8, x` is not `x & 7`.
/// The obligation in `prove` binds its free variable to the left slot, so it
/// never licensed this case — the hole was in the application binding, not in
/// the law. Constant-left `mul` is commutative and must still fire.
fn constant_left_division_is_refused_and_commutative_mul_still_fires() {
    let ll = r#"
define dso_local i32 @const_left_ops(i32 noundef %0) {
  %2 = udiv i32 8, %0
  %3 = urem i32 8, %0
  %4 = add i32 %2, %3
  %5 = mul i32 8, %0
  %6 = add i32 %4, %5
  ret i32 %6
}
"#;
    let mut function = sir_lower::lower_function(ll, "const_left_ops").expect("lower");
    let has_div = |function: &sir_nodes::Function| {
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Div { .. }))
    };
    let has_rem = |function: &sir_nodes::Function| {
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Rem { .. }))
    };
    assert!(
        has_div(&function) && has_rem(&function),
        "fixture must lower constant-left division and remainder"
    );
    while let Some(next) = sir_opt::optimize_arithmetic(&function) {
        function = next;
    }
    assert!(
        has_div(&function),
        "constant-left division must be refused, not rewritten to a shift"
    );
    assert!(
        has_rem(&function),
        "constant-left remainder must be refused, not rewritten to a mask"
    );
    assert!(
        !function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Mul { .. })),
        "commuted multiply is commutative and must still rewrite"
    );
}
