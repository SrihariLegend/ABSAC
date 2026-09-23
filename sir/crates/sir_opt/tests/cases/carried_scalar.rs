// S2 regression: a reduction whose predicate scalar is derived from the
// loop's induction variable must be refused, not unrolled.
//
// `Loop` has no phi nodes — the lowerer reuses the carried-input node as
// the in-body name of the carried variable. Reading that node inside the
// body yields the current iteration's value; copying it into an unrolled
// chain therefore substitutes the *initial* value at every index, so
// `values[i] > i` becomes `values[k] > 0`. The referee's random-byte
// driver cannot see the difference (the predicate is true on both sides),
// which is why this shipped; these cases are the difference.

const ANY: &str = r#"
define dso_local i64 @s2_any(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %2
2:
  %3 = phi i64 [ 0, %1 ], [ %11, %2 ]
  %4 = phi i64 [ 0, %1 ], [ %10, %2 ]
  %5 = getelementptr inbounds i32, ptr %0, i64 %3
  %6 = load i32, ptr %5, align 4
  %7 = zext i32 %6 to i64
  %8 = icmp ugt i64 %7, %3
  %9 = zext i1 %8 to i64
  %10 = or i64 %9, %4
  %11 = add nuw nsw i64 %3, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %13, label %2
13:
  ret i64 %10
}
"#;

const ANY_CONTROL: &str = r#"
define dso_local i64 @ctl_any(ptr nocapture noundef readonly %0, i64 %1) local_unnamed_addr {
  br label %3
3:
  %4 = phi i64 [ 0, %2 ], [ %12, %3 ]
  %5 = phi i64 [ 0, %2 ], [ %11, %3 ]
  %6 = getelementptr inbounds i32, ptr %0, i64 %4
  %7 = load i32, ptr %6, align 4
  %8 = zext i32 %7 to i64
  %9 = icmp ugt i64 %8, %1
  %10 = zext i1 %9 to i64
  %11 = or i64 %10, %5
  %12 = add nuw nsw i64 %4, 1
  %13 = icmp eq i64 %12, 64
  br i1 %13, label %14, label %3
14:
  ret i64 %11
}
"#;

const COUNT: &str = r#"
define dso_local i64 @s2_count(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %2
2:
  %3 = phi i64 [ 0, %1 ], [ %11, %2 ]
  %4 = phi i64 [ 0, %1 ], [ %10, %2 ]
  %5 = getelementptr inbounds i32, ptr %0, i64 %3
  %6 = load i32, ptr %5, align 4
  %7 = zext i32 %6 to i64
  %8 = icmp ugt i64 %7, %3
  %9 = zext i1 %8 to i64
  %10 = add i64 %9, %4
  %11 = add nuw nsw i64 %3, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %13, label %2
13:
  ret i64 %10
}
"#;

const PARITY: &str = r#"
define dso_local i64 @s2_parity(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %2
2:
  %3 = phi i64 [ 0, %1 ], [ %11, %2 ]
  %4 = phi i64 [ 0, %1 ], [ %10, %2 ]
  %5 = getelementptr inbounds i32, ptr %0, i64 %3
  %6 = load i32, ptr %5, align 4
  %7 = zext i32 %6 to i64
  %8 = icmp ugt i64 %7, %3
  %9 = zext i1 %8 to i64
  %10 = xor i64 %9, %4
  %11 = add nuw nsw i64 %3, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %13, label %2
13:
  ret i64 %10
}
"#;

const ALL: &str = r#"
define dso_local i64 @s2_all(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %2
2:
  %3 = phi i64 [ 0, %1 ], [ %10, %2 ]
  %4 = phi i64 [ 1, %1 ], [ %9, %2 ]
  %5 = getelementptr inbounds i32, ptr %0, i64 %3
  %6 = load i32, ptr %5, align 4
  %7 = zext i32 %6 to i64
  %8 = icmp ugt i64 %7, %3
  %9 = select i1 %8, i64 0, i64 %4
  %10 = add nuw nsw i64 %3, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %12, label %2
12:
  ret i64 %9
}
"#;

const SUM: &str = r#"
define dso_local i64 @s2_sum(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %2
2:
  %3 = phi i64 [ 0, %1 ], [ %10, %2 ]
  %4 = phi i64 [ 0, %1 ], [ %9, %2 ]
  %5 = getelementptr inbounds i32, ptr %0, i64 %3
  %6 = load i32, ptr %5, align 4
  %7 = zext i32 %6 to i64
  %8 = add i64 %7, %3
  %9 = add i64 %8, %4
  %10 = add nuw nsw i64 %3, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %12, label %2
12:
  ret i64 %9
}
"#;

fn lower(ll: &str, name: &str) -> sir_nodes::Function {
    sir_lower::lower_function(ll, name).unwrap_or_else(|error| panic!("lower {name}: {error}"))
}

/// Each family that hoists a per-element expression out of its loop must
/// refuse when that expression reads loop-carried state. `sum` is the one
/// the referee can already see (its unrolled `values[i] + i` loses the `i`
/// term on every case, not just a crafted one).
///
/// The five refusals are collected before asserting so that a failure names
/// every family that fired, rather than only the first — and so that
/// disabling `patch::carried_names` shows at once whether any of the five
/// snippets fails to reach its family in the first place.
fn carried_scalar_is_refused_across_families() {
    let fired: Vec<(&str, bool)> = vec![
        (
            "any",
            sir_opt::optimize_any(&lower(ANY, "s2_any")).is_some(),
        ),
        (
            "count",
            sir_opt::optimize_count(&lower(COUNT, "s2_count")).is_some(),
        ),
        (
            "parity",
            sir_opt::optimize_parity(&lower(PARITY, "s2_parity")).is_some(),
        ),
        (
            "all",
            sir_opt::optimize_all(&lower(ALL, "s2_all")).is_some(),
        ),
        (
            "sum",
            sir_opt::optimize_sum(&lower(SUM, "s2_sum")).is_some(),
        ),
    ];
    assert!(
        fired.iter().all(|(_, fired)| !fired),
        "these families unrolled an induction-derived scalar: {fired:?}"
    );

    // The refusal must be narrow: the same shape with a loop-invariant
    // scalar still fires. Without this half, a blanket family kill would
    // pass the assertion above.
    assert!(
        sir_opt::optimize_any(&lower(ANY_CONTROL, "ctl_any")).is_some(),
        "a loop-invariant predicate scalar must still unroll"
    );
}
