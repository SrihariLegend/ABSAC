//! Semantic Zoo — Benchmark Suite for Generalization Testing.
//!
//! Generates variations of optimizable and non-optimizable programs
//! across different optimization families to verify generalized coverage
//! and capture architectural statistics.

use sir_builder::Builder;
use sir_nodes::{CmpOperator, Function};
use sir_optimizer::config::OptimizerConfig;
use sir_optimizer::optimizer::Optimizer;
use sir_rewrite::registry::default_registry;
use sir_types::{ConstantData, Span, Type};

fn i32_type() -> Type {
    Type::i32()
}
fn u32_type() -> Type {
    Type::u32()
}
fn u64_type() -> Type {
    Type::u64()
}
fn bool_type() -> Type {
    Type::Bool
}
fn bool_array(len: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length: len,
    }
}
fn unknown() -> Span {
    Span::unknown()
}

/// A program category in the semantic zoo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    BooleanReduction,
    PredicateReduction,
    BitwiseArithmetic,
    Mixed,
    Unoptimizable,
}

pub struct ZooProgram {
    pub name: String,
    pub family: Family,
    pub function: Function,
    pub expected_rewrites: usize,
}

/// Builds boolean reductions (count, any, all, parity)
fn build_bool_reduction(name: &str, len: usize, reduction: &str) -> ZooProgram {
    let ret_ty = match reduction {
        "count" => i32_type(),
        "any" | "all" | "parity" => bool_type(),
        _ => panic!("Unknown reduction"),
    };

    let mut b = Builder::new(name, &[("board", bool_array(len))], ret_ty.clone());
    let board = b.parameter_index(0).unwrap();
    let i_init = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let i_step = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let limit = b.constant(ConstantData::u64(len as u64), u64_type(), unknown());

    let acc_init = match reduction {
        "count" => b.constant(ConstantData::i32(0), i32_type(), unknown()),
        "any" | "all" => {
            let init = if reduction == "any" { false } else { true };
            b.constant(ConstantData::boolean(init), bool_type(), unknown())
        }
        "parity" => b.constant(ConstantData::boolean(false), bool_type(), unknown()),
        _ => unreachable!(),
    };

    let elem = b
        .array_access(board, i_init, bool_type(), unknown())
        .unwrap();
    let acc_next = match reduction {
        "count" => {
            let one = b.constant(ConstantData::i32(1), i32_type(), unknown());
            let zero = b.constant(ConstantData::i32(0), i32_type(), unknown());
            let inc = b.select(elem, one, zero, unknown()).unwrap();
            b.add(acc_init, inc, unknown()).unwrap()
        }
        "any" => b.bool_or(acc_init, elem, unknown()).unwrap(),
        "all" => b.bool_and(acc_init, elem, unknown()).unwrap(),
        "parity" => b.ne(acc_init, elem, unknown()).unwrap(), // parity = xor
        _ => unreachable!(),
    };

    let i_next = b.add(i_init, i_step, unknown()).unwrap();
    let cond = b.lt(i_init, limit, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, acc_next, i_next, cond],
            cond,
            &[acc_next, i_next],
            &[acc_init, i_init],
            Type::Tuple {
                elements: vec![ret_ty.clone(), u64_type()],
            },
            unknown(),
        )
        .unwrap();

    let res = b.field_access(loop_node, "0", ret_ty, unknown()).unwrap();
    b.return_value(res, unknown()).unwrap();

    ZooProgram {
        name: name.to_string(),
        family: Family::BooleanReduction,
        function: b.build(),
        expected_rewrites: 1,
    }
}

/// Builds arithmetic identities (modulo, divide, multiply, shift_mask)
fn build_arithmetic(name: &str, op: &str, divisor: u64, signed: bool) -> ZooProgram {
    let ty = if signed { Type::i32() } else { Type::u32() };
    let mut b = Builder::new(name, &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();

    let res = if op == "shift_mask" {
        let c = b.constant(
            if signed {
                ConstantData::i32(divisor as i32)
            } else {
                ConstantData::u32(divisor as u32)
            },
            ty.clone(),
            unknown(),
        );
        let shl = b.shl(x, c, unknown()).unwrap();
        b.shr(shl, c, unknown()).unwrap()
    } else {
        let c = b.constant(
            if signed {
                ConstantData::i32(divisor as i32)
            } else {
                ConstantData::u32(divisor as u32)
            },
            ty.clone(),
            unknown(),
        );
        match op {
            "modulo" => b.rem(x, c, unknown()).unwrap(),
            "divide" => b.div(x, c, unknown()).unwrap(),
            "multiply" => b.mul(x, c, unknown()).unwrap(),
            _ => panic!("unknown op"),
        }
    };

    b.return_value(res, unknown()).unwrap();

    // Modulo/Multiply works on signed/unsigned.
    // ShiftMask works on unsigned, fails verification on signed (due to sign extension).
    // Divide works on unsigned, fails verification on signed.
    let expected = 1;

    ZooProgram {
        name: name.to_string(),
        family: Family::BitwiseArithmetic,
        function: b.build(),
        expected_rewrites: expected,
    }
}

pub fn generate_semantic_zoo() -> Vec<ZooProgram> {
    let mut zoo = Vec::new();

    // 1. Boolean Reductions (Count, Any, All, Parity) x Different Lengths
    for len in [16, 32, 64] {
        zoo.push(build_bool_reduction(
            &format!("bool_count_{}", len),
            len,
            "count",
        ));
        zoo.push(build_bool_reduction(
            &format!("bool_any_{}", len),
            len,
            "any",
        ));
        zoo.push(build_bool_reduction(
            &format!("bool_all_{}", len),
            len,
            "all",
        ));
        zoo.push(build_bool_reduction(
            &format!("bool_parity_{}", len),
            len,
            "parity",
        ));
    }

    // 2. Arithmetic (Mod, Div, Mul, Mask) x Signed/Unsigned x Different Values
    for val in [2, 4, 8, 16, 32] {
        for signed in [false, true] {
            let s = if signed { "signed" } else { "unsigned" };
            zoo.push(build_arithmetic(
                &format!("arith_mod_{}_{}", s, val),
                "modulo",
                val,
                signed,
            ));
            zoo.push(build_arithmetic(
                &format!("arith_div_{}_{}", s, val),
                "divide",
                val,
                signed,
            ));
            zoo.push(build_arithmetic(
                &format!("arith_mul_{}_{}", s, val),
                "multiply",
                val,
                signed,
            ));
            zoo.push(build_arithmetic(
                &format!("arith_mask_{}_{}", s, val),
                "shift_mask",
                val,
                signed,
            ));
        }
    }

    // Add unoptimizable (non-power of two)
    zoo.push(build_arithmetic("arith_mod_unsigned_3", "modulo", 3, false));
    if let Some(last) = zoo.last_mut() {
        last.expected_rewrites = 0;
        last.family = Family::Unoptimizable;
    }

    zoo
}

#[test]
fn semantic_zoo_evaluation() {
    let zoo = generate_semantic_zoo();
    let optimizer = Optimizer::new(OptimizerConfig::default(), default_registry());

    let mut total_facts = 0;
    let mut total_truths = 0;
    let mut total_beliefs = 0;
    let mut total_candidates = 0;
    let mut total_proofs = 0;
    let mut total_proven = 0;
    let mut total_rewrites = 0;

    println!(
        "{:<30} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
        "Program", "Facts", "Truths", "Beliefs", "Cands", "Proven", "Rewrites"
    );
    println!(
        "{:-<30}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}",
        "", "", "", "", "", "", ""
    );

    for prog in &zoo {
        let result = optimizer.optimize(&prog.function);

        let mut facts = 0;
        let mut truths = 0;
        let mut beliefs = 0;
        let mut candidates = 0;
        let mut proofs = 0;
        let mut proven = 0;

        for rec in &result.iterations_detail {
            facts += rec.facts_discovered;
            truths += rec.truths_discovered;
            beliefs += rec.beliefs_inferred;
            candidates += rec.candidates_generated;
            proofs += rec.proofs_attempted;
            proven += rec.proofs_succeeded;
        }

        total_facts += facts;
        total_truths += truths;
        total_beliefs += beliefs;
        total_candidates += candidates;
        total_proofs += proofs;
        total_proven += proven;
        total_rewrites += result.rewrites_applied;

        println!(
            "{:<30} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
            prog.name, facts, truths, beliefs, candidates, proven, result.rewrites_applied
        );

        assert_eq!(
            result.rewrites_applied, prog.expected_rewrites,
            "Mismatch in {} (expected {}, got {})",
            prog.name, prog.expected_rewrites, result.rewrites_applied
        );
    }

    println!(
        "{:-<30}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}",
        "", "", "", "", "", "", ""
    );
    println!(
        "{:<30} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
        "TOTALS",
        total_facts,
        total_truths,
        total_beliefs,
        total_candidates,
        total_proven,
        total_rewrites
    );
}
