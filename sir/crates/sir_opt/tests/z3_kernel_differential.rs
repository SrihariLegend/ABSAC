//! Third-party cross-check: random bitvector equivalences decided by the
//! `sir_mech` SAT kernel must agree with z3.
//!
//! z3 is optional (set `Z3_BIN` or rely on the vendored binary); the test
//! skips when it is unavailable, matching the historical differential.

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

fn z3_verdict(z3: &Path, width: u32, lhs: &str, rhs: &str) -> Option<bool> {
    let mut script = String::new();
    for index in 0..3 {
        script.push_str(&format!("(declare-const x{index} (_ BitVec {width}))\n"));
    }
    script.push_str(&format!("(assert (not (= {lhs} {rhs})))\n(check-sat)\n"));
    let path = std::env::temp_dir().join(format!(
        "sir_opt_z3_{}_{}.smt2",
        std::process::id(),
        width
    ));
    let mut file = std::fs::File::create(&path).ok()?;
    file.write_all(script.as_bytes()).ok()?;
    let output = Command::new(z3).arg("-smt2").arg(&path).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let verdict = if stdout.contains("unsat") {
        Some(true)
    } else if stdout.contains("sat") {
        Some(false)
    } else {
        None
    };
    let _ = std::fs::remove_file(path);
    verdict
}

#[test]
fn kernel_agrees_with_z3_on_random_bitvector_equivalences() {
    let Some(z3) = z3_binary() else {
        eprintln!("z3 unavailable — skipping kernel differential");
        return;
    };
    let mut rng = Rng(0x5EED_1234_5678_9ABC);
    let mut agreements = 0usize;
    let mut decided = 0usize;
    for iteration in 0..300 {
        let width = [8u32, 16, 32, 64][iteration % 4];
        let mut bv = Bv::with_capacity(256);
        let vars: Vec<Term> = (0..3)
            .map(|index| bv.var(VarId(index), width))
            .collect();
        let lhs_vars = vars.clone();
        let rhs_vars = vars;
        let (lhs, lhs_smt) = expression(&mut rng, &mut bv, &lhs_vars, width, 3);
        let (rhs, rhs_smt) = expression(&mut rng, &mut bv, &rhs_vars, width, 3);

        let kernel = match sir_mech::prove_equal(&bv, lhs, rhs) {
            CheckResult::Valid => Some(true),
            CheckResult::Counterexample(_) => Some(false),
            CheckResult::Unknown(_) => None,
        };
        let external = z3_verdict(&z3, width, &lhs_smt, &rhs_smt);
        match (kernel, external) {
            (Some(kernel), Some(external)) => {
                decided += 1;
                assert_eq!(
                    kernel, external,
                    "kernel/z3 disagreement at width {width}: {lhs_smt} == {rhs_smt}"
                );
                agreements += 1;
            }
            _ => {}
        }
    }
    assert!(
        agreements >= 20,
        "cross-check must decide a useful number of cases, got {agreements} of {decided}"
    );
}
