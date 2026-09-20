// Third-party cross-check: random bitvector equivalences decided by the
// `sir_mech` SAT kernel must agree with z3.
//
// z3 is optional (set `Z3_BIN` or rely on the vendored binary); the test
// skips when it is unavailable, matching the historical differential.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sir_mech::{Bv, CheckResult, Term, VarId};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}

fn expression(
    rng: &mut Rng,
    bv: &mut Bv,
    vars: &[Term],
    width: u32,
    depth: usize,
) -> (Term, String) {
    if depth == 0 || rng.below(3) == 0 {
        if rng.below(2) == 0 {
            let index = rng.below(vars.len());
            return (vars[index], format!("x{index}"));
        }
        let value = rng.next();
        let value = if width == 64 {
            value
        } else {
            value & ((1u64 << width) - 1)
        };
        return (
            bv.constant(value, width),
            format!("(_ bv{value} {width})"),
        );
    }
    let (lhs, lhs_smt) = expression(rng, bv, vars, width, depth - 1);
    let (rhs, rhs_smt) = expression(rng, bv, vars, width, depth - 1);
    match rng.below(8) {
        0 => (bv.add(lhs, rhs), format!("(bvadd {lhs_smt} {rhs_smt})")),
        1 => (bv.sub(lhs, rhs), format!("(bvsub {lhs_smt} {rhs_smt})")),
        2 => (bv.mul(lhs, rhs), format!("(bvmul {lhs_smt} {rhs_smt})")),
        3 => (bv.and(lhs, rhs), format!("(bvand {lhs_smt} {rhs_smt})")),
        4 => (bv.or(lhs, rhs), format!("(bvor {lhs_smt} {rhs_smt})")),
        5 => (bv.xor(lhs, rhs), format!("(bvxor {lhs_smt} {rhs_smt})")),
        6 => {
            let amount = (rng.next() as u32) % width;
            let amount_term = bv.constant(u64::from(amount), width);
            (
                bv.shl(lhs, amount_term),
                format!("(bvshl {lhs_smt} (_ bv{amount} {width}))"),
            )
        }
        _ => {
            let amount = (rng.next() as u32) % width;
            let amount_term = bv.constant(u64::from(amount), width);
            (
                bv.lshr(lhs, amount_term),
                format!("(bvlshr {lhs_smt} (_ bv{amount} {width}))"),
            )
        }
    }
}

fn z3_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("Z3_BIN") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let vendored = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../vendor/z3py/bin/z3");
    vendored.exists().then_some(vendored)
}

fn z3_batch(z3: &Path, queries: &[(String, String, String)]) -> Option<Vec<Option<bool>>> {
    let mut script = String::new();
    for (declarations, lhs, rhs) in queries {
        script.push_str("(reset)\n");
        script.push_str(declarations);
        script.push_str(&format!("(assert (not (= {lhs} {rhs})))\n(check-sat)\n"));
    }
    let path = std::env::temp_dir().join(format!(
        "sir_opt_z3_{}.smt2",
        std::process::id()
    ));
    let mut file = std::fs::File::create(&path).ok()?;
    file.write_all(script.as_bytes()).ok()?;
    let output = Command::new(z3).arg("-smt2").arg(&path).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(path);
    Some(
        stdout
            .lines()
            .filter_map(|line| match line.trim() {
                "unsat" => Some(Some(true)),
                "sat" => Some(Some(false)),
                _ => None,
            })
            .collect(),
    )
}

fn law_queries() -> Vec<(String, String, String)> {
    let mut queries = Vec::new();
    for width in [8u32, 16, 32, 64] {
        let declarations = (0..3)
            .map(|index| format!("(declare-const x{index} (_ BitVec {width}))\n"))
            .collect::<String>();
        let zero = format!("(_ bv0 {width})");
        let ones = if width == 64 {
            format!("#x{:016x}", u64::MAX)
        } else {
            format!("(_ bv{} {width})", (1u64 << width) - 1)
        };
        let mut push = |lhs: String, rhs: String| {
            queries.push((declarations.clone(), lhs, rhs));
        };
        push(
            "(bvadd (bvadd x0 x1) x2)".into(),
            "(bvadd x0 (bvadd x1 x2))".into(),
        );
        push(format!("(bvadd x0 {zero})"), "x0".into());
        push(
            "(bvor (bvor x0 x1) x2)".into(),
            "(bvor x0 (bvor x1 x2))".into(),
        );
        push(format!("(bvor x0 {zero})"), "x0".into());
        push("(bvor x0 x1)".into(), "(bvor x1 x0)".into());
        push(
            "(bvand (bvand x0 x1) x2)".into(),
            "(bvand x0 (bvand x1 x2))".into(),
        );
        push(format!("(bvand x0 {ones})"), "x0".into());
        push(
            "(bvxor (bvxor x0 x1) x2)".into(),
            "(bvxor x0 (bvxor x1 x2))".into(),
        );
        push(format!("(bvxor x0 {zero})"), "x0".into());
        push(format!("(bvxor x0 x0)"), zero);
    }

    // The first-match contract: the found-flag scan and the
    // reverse-overwrite scan agree on an 8-element predicate.
    let declarations = (0..8)
        .map(|index| format!("(declare-const b{index} (_ BitVec 1))\n"))
        .collect::<String>();
    let mut forward = "(_ bv8 8)".to_string();
    let mut found = "(_ bv0 1)".to_string();
    for index in 0..8 {
        let position = format!("(_ bv{index} 8)");
        let take = format!("(= (bvand (bvnot {found}) b{index}) (_ bv1 1))");
        forward = format!("(ite {take} {position} {forward})");
        found = format!("(bvor {found} b{index})");
    }
    let mut reverse = "(_ bv8 8)".to_string();
    for index in (0..8).rev() {
        reverse = format!("(ite (= b{index} (_ bv1 1)) (_ bv{index} 8) {reverse})");
    }
    queries.push((declarations, forward, reverse));
    queries
}

fn kernel_agrees_with_z3_on_random_bitvector_equivalences() {
    let Some(z3) = z3_binary() else {
        eprintln!("z3 unavailable — skipping kernel differential");
        return;
    };
    let mut rng = Rng(0x5EED_1234_5678_9ABC);
    let mut cases = Vec::new();
    // Forty random terms across all four widths plus the family laws
    // below already exercise every operator the kernel encodes; more
    // samples cost subprocess time without adding a distinct shape.
    for iteration in 0..40 {
        let width = [8u32, 16, 32, 64][iteration % 4];
        let mut bv = Bv::with_capacity(256);
        let vars: Vec<Term> = (0..3)
            .map(|index| bv.var(VarId(index), width))
            .collect();
        let lhs_vars = vars.clone();
        let rhs_vars = vars;
        let (lhs, lhs_smt) = expression(&mut rng, &mut bv, &lhs_vars, width, 3);
        let (rhs, rhs_smt) = expression(&mut rng, &mut bv, &rhs_vars, width, 3);
        let declarations = (0..3)
            .map(|index| format!("(declare-const x{index} (_ BitVec {width}))\n"))
            .collect::<String>();
        cases.push((width, bv, lhs, rhs, lhs_smt, rhs_smt, declarations));
    }
    let prove = |slice: &[(u32, Bv, Term, Term, String, String, String)]| {
        slice
            .iter()
            .map(|(_, bv, lhs, rhs, ..)| match sir_mech::prove_equal(bv, *lhs, *rhs) {
                CheckResult::Valid => Some(true),
                CheckResult::Counterexample(_) => Some(false),
                CheckResult::Unknown(_) => None,
            })
            .collect::<Vec<_>>()
    };
    let (left, right) = cases.split_at(cases.len() / 2);
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(|| prove(left));
        let right = scope.spawn(|| prove(right));
        (left.join().expect("kernel proofs"), right.join().expect("kernel proofs"))
    });
    let kernels: Vec<Option<bool>> = left.into_iter().chain(right).collect();
    let widths: Vec<u32> = cases.iter().map(|case| case.0).collect();
    let mut queries: Vec<(String, String, String)> = cases
        .iter()
        .map(|(_, _, _, _, lhs, rhs, declarations)| {
            (declarations.clone(), lhs.clone(), rhs.clone())
        })
        .collect();
    let random_len = queries.len();
    queries.extend(law_queries());
    let Some(external) = z3_batch(&z3, &queries) else {
        panic!("z3 batch failed");
    };
    assert_eq!(external.len(), queries.len(), "one z3 verdict per query");
    let mut agreements = 0usize;
    let mut decided = 0usize;
    for (index, (kernel, external)) in kernels
        .iter()
        .zip(external.iter().take(random_len))
        .enumerate()
    {
        let width = widths[index];
        let (_, lhs, rhs) = &queries[index];
        match (kernel, external) {
            (Some(kernel), Some(external)) => {
                decided += 1;
                assert_eq!(
                    kernel, external,
                    "kernel/z3 disagreement at width {width}: {lhs} == {rhs}"
                );
                agreements += 1;
            }
            _ => {}
        }
    }
    assert!(
        agreements >= 10,
        "cross-check must decide a useful number of cases, got {agreements} of {decided}"
    );
    for (index, verdict) in external.iter().enumerate().skip(random_len) {
        assert_eq!(
            *verdict,
            Some(true),
            "family law query {index} was not proved by z3: {}",
            queries[index].1
        );
    }
}
