fn runtime_bound_search_lowers_and_uses_memchr() {
    let ir = r#"
define dso_local i64 @naive_first_zero(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %12, label %4
4:
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %12, label %9
9:
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4
12:
  %13 = phi i64 [ 0, %2 ], [ %1, %9 ], [ %5, %4 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  ret i64 %14
}
"#;
    let function = sir_lower::lower_function(ir, "naive_first_zero")
        .expect("runtime-bound search loop must lower");
    let (optimized, definition) =
        sir_opt::optimize_search(&function).expect("runtime search must optimize");
    assert_eq!(definition, 402, "runtime search is memchr definition 402");
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_memchr"
        )),
        "runtime search must use the sir_memchr primitive"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the search loop"
    );
}

fn runtime_zero_test_loop_lowers() {
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
