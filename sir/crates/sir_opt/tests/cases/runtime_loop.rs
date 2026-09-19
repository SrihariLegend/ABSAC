fn rotated_zero_test_loop_lowers() {
    let ir = r#"
define dso_local i32 @naive_popcount(i64 noundef %0) local_unnamed_addr {
  %2 = icmp eq i64 %0, 0
  br i1 %2, label %10, label %3
3:
  %4 = phi i32 [ %8, %3 ], [ 0, %2 ]
  %5 = phi i64 [ %7, %3 ], [ %0, %2 ]
  %6 = add i64 %5, -1
  %7 = and i64 %6, %5
  %8 = add i32 %4, 1
  %9 = icmp eq i64 %7, 0
  br i1 %9, label %10, label %3
10:
  %11 = phi i32 [ 0, %2 ], [ %8, %3 ]
  ret i32 %11
}
"#;
    let function = sir_lower::lower_function(ir, "naive_popcount")
        .expect("rotated zero-test loop must lower");
    assert!(
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "lowered function must contain the runtime loop"
    );
}
