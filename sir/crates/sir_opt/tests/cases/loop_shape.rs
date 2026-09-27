// The trip count a family consumes must be an exact count, not the
// constant bound the counter is compared against. `loops.rs` reports the
// bound and never reads the counter's initial value, so `for (i = 5; i <
// 64; i++)` and `for (i = 0; i < 64; i++)` both report 64. Families
// compare that value against the array length, so the first is accepted
// and unrolled as if it read `a[0..64]` when it reads `a[5..64]`.
//
// `loop_shape::checked_counted_loop` reads the initial value, the step and
// the relation, states the count exactly, and refuses anything it cannot
// state.

/// The same reduction as the corpus's `p07_any_or_const_u32`, but the
/// counter starts at 5. It runs 59 times and reads `a[5..64]`.
const START_5: &str = r#"
define dso_local range(i64 0, 2) i64 @s5_any(ptr nocapture noundef readonly %0) local_unnamed_addr {
  br label %5

2:
  %3 = icmp ne i32 %10, 0
  %4 = zext i1 %3 to i64
  ret i64 %4

5:
  %6 = phi i64 [ 5, %1 ], [ %11, %5 ]
  %7 = phi i32 [ 0, %1 ], [ %10, %5 ]
  %8 = getelementptr inbounds i32, ptr %0, i64 %6
  %9 = load i32, ptr %8, align 4
  %10 = or i32 %9, %7
  %11 = add nuw nsw i64 %6, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %2, label %5
}
"#;

fn only_loop(function: &sir_nodes::Function) -> sir_types::NodeId {
    function
        .arena
        .iter()
        .find(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. }))
        .expect("fixture has a loop")
        .id
}

fn counted_count(function: &sir_nodes::Function) -> Option<u64> {
    sir_analysis::loop_shape::checked_counted_loop(function, only_loop(function))
        .map(|shape| shape.trip_count)
}

pub fn checked_trip_counts_are_exact() {
    // The canonical loop really does run 64 times, and the checker agrees
    // with the bound because the counter starts where the bound implies.
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let canonical = sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let shape = sir_analysis::loop_shape::checked_counted_loop(&canonical, only_loop(&canonical))
        .expect("canonical counted loop is described");
    assert_eq!(shape.trip_count, 64, "canonical loop runs 0..64");
    assert_eq!(shape.counter_init, 0, "canonical counter starts at 0");
    assert_eq!(shape.step, 1, "canonical step is 1");

    // The counter starts at 5. The bound is still 64, but the loop runs 59
    // times and reads a[5..64]. The bound-only extractor reports 64 here,
    // which is the array length, so the family accepts it.
    let five = sir_lower::lower_function(START_5, "s5_any").expect("lower start-5");
    assert_eq!(
        counted_count(&five),
        Some(59),
        "a counter starting at 5 against bound 64 runs 59 times, not 64"
    );

    // Why the family refuses start-5, traced: the lowerer's
    // `strict_counted_bound` requires the counter to start at exactly 0
    // and step by exactly 1 before it will promote a `Pointer` parameter to
    // `Array { length: K }`. With a start of 5 it refuses, the parameter
    // stays a pointer, and `any`'s requirement that the base be
    // `Type::Array` is what rejects the function.
    //
    // So the refusal is principled — and it is not the trip count that
    // provides it. It is a *type*, inferred by a different, stricter
    // extractor in a different crate.
    let param_ty = |f: &sir_nodes::Function| f.arena.iter().next().map(|n| n.ty.clone());
    assert!(
        matches!(param_ty(&canonical), Some(sir_types::Type::Array { length: 64, .. })),
        "the canonical buffer is promoted to a 64-element array"
    );
    assert!(
        !matches!(param_ty(&five), Some(sir_types::Type::Array { .. })),
        "start-5 is not promoted — its parameter stays a pointer, which is \
         the requirement that actually refuses it"
    );

    // The gate the family actually consumes. `any` refuses unless
    // `fact.trip_count == Some(extent)`, and `extent` is the array length,
    // 64. The bound-only extractor reports 64 for *both* functions, so the
    // gate passes on the 59-iteration loop as readily as on the
    // 64-iteration one — it cannot tell them apart.
    //
    // This is pinned rather than asserted-away: it is the reason the
    // checked extractor exists, and if `loops.rs` is ever repaired this
    // assertion is the one that should fail.
    let facts = sir_analysis::facts::FactDatabase {
        loops: sir_analysis::loops::run_loops(&five),
    };
    assert_eq!(
        facts.loops.values().next().and_then(|f| f.constant_bound),
        Some(64),
        "the fact database reports the constant *bound*, not the count: a \
         counter starting at 5 against bound 64 runs 59 times, and it says 64. \
         Nothing consumes it as a count any more — but the field used to be \
         called `trip_count`, which is how it came to be read as one"
    );

    // Start-5 is refused, and the refusal is not attributable to the
    // trip-count gate — that gate passes. The safety here lives in the
    // lowerer's type inference, which happens to be strictly stronger
    // because it pins the counter to start at 0 and step by 1. The loose
    // count is therefore latent rather than live, and stays latent only as
    // long as promotion stays this strict.
    assert!(
        sir_opt::optimize_any(&five).is_none(),
        "start-5 is refused — by the base's type, not by the trip count"
    );
    assert!(
        sir_opt::optimize_any(&canonical).is_some(),
        "the canonical fixture still fires"
    );

    // The guard's actual job. Retype start-5's buffer as a 64-element array,
    // which is what `promote_constant_extent_buffers` would produce if it
    // accepted a non-zero loop start. The type obstacle is now gone, the
    // array length is 64, the loose extractor still reports 64 — and the
    // only thing left refusing this function is the *exact* count.
    let mut forced = five.clone();
    let param = forced.arena.iter().next().expect("parameter node").id;
    let element = forced
        .arena
        .iter()
        .find_map(|node| match &node.ty {
            sir_types::Type::Integer { .. } => Some(node.ty.clone()),
            _ => None,
        })
        .expect("an integer element type");
    let array_ty = sir_types::Type::Array {
        element: Box::new(element),
        length: 64,
    };
    if let Some(node) = forced.arena.get_mut(param) {
        node.ty = array_ty.clone();
    }
    assert!(
        sir_opt::optimize_any(&forced).is_none(),
        "with the type obstacle removed, a 59-iteration loop over a \
         64-element array must be refused by its count — otherwise the \
         family would unroll a[0..64] for a loop that reads a[5..64]"
    );
}
