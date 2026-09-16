//! A genuine induction proof in the kernel.
//!
//! The object theory is a range fold:
//!
//! ```text
//!   S(x, x) = 0                      (empty range)
//!   S(x, y1) = S(x, y) + g(y)  if y1 = y + 1
//!   S(x1, y) = S(x, y) + g(x)  if x1 = x + 1
//! ```
//!
//! (the second and third schemas are the two directions of "extend the
//! range by one element"). From them we prove the splitting law
//!
//! ```text
//!   b <= c, c <= a  ⊢  S(a, c) = S(a, b) + S(c, b)
//! ```
//!
//! by induction on `m = c - b`, then instantiate and discharge the
//! guards with linear arithmetic. The final theorem is replayed by the
//! kernel, which re-checks every derivation step.

use sir_mech::kernel::{Context, Kernel, KernelError, SchemaId, Statement, Theorem};
use sir_mech::term::{FuncId, Sort, Symbol, Tid};

struct Env {
    k: Kernel,
    s: FuncId,
    g: FuncId,
    a: Symbol,
    b: Symbol,
    c: Symbol,
    m: Symbol,
    s_zero: SchemaId,
    s_step_right: SchemaId,
    s_step_left: SchemaId,
}

fn fresh_hole(k: &mut Kernel, sort: Sort) -> Symbol {
    let name = format!("hole{}", k.arena().symbol_count());
    k.arena_mut().declare_var(name, sort)
}

impl Env {
    fn new() -> Self {
        let mut k = Kernel::new();
        let (a, b, c, m) = {
            let ar = k.arena_mut();
            (
                ar.declare_var("a", Sort::Int),
                ar.declare_var("b", Sort::Int),
                ar.declare_var("c", Sort::Int),
                ar.declare_var("m", Sort::Int),
            )
        };
        let (s, g) = {
            let ar = k.arena_mut();
            (
                ar.declare_func("S", vec![Sort::Int, Sort::Int], Sort::Int),
                ar.declare_func("g", vec![Sort::Int], Sort::Int),
            )
        };
        // S(x, x) = 0
        let x = k.arena_mut().declare_var("x", Sort::Int);
        let s_zero = {
            let ar = k.arena_mut();
            let xv = ar.var(x);
            let lhs = ar.app(s, &[xv, xv]);
            let zero = ar.int(0);
            let guard = ar.le(xv, xv);
            k.schema_guarded("S(x,x)=0", &[x], Some(guard), lhs, zero)
        };
        // S(x, y1) = S(x, y) + g(y)   guarded by  y1 = y + 1
        let (y, y1) = {
            let ar = k.arena_mut();
            (
                ar.declare_var("y", Sort::Int),
                ar.declare_var("y1", Sort::Int),
            )
        };
        let s_step_right = {
            let ar = k.arena_mut();
            let xv = ar.var(x);
            let yv = ar.var(y);
            let y1v = ar.var(y1);
            let one = ar.int(1);
            let guard = {
                let succ = ar.add(yv, one);
                let lt = ar.lt(yv, xv);
                let eq = ar.int_eq(y1v, succ);
                ar.bool_and(eq, lt)
            };
            let lhs = ar.app(s, &[xv, y1v]);
            let sxy = ar.app(s, &[xv, yv]);
            let gy = ar.app(g, &[yv]);
            let rhs = ar.add(sxy, gy);
            k.schema_guarded("S(x,y1)=S(x,y)+g(y)", &[x, y, y1], Some(guard), lhs, rhs)
        };
        // S(x1, y) = S(x, y) + g(x)   guarded by  x1 = x + 1
        let x1 = k.arena_mut().declare_var("x1", Sort::Int);
        let s_step_left = {
            let ar = k.arena_mut();
            let xv = ar.var(x);
            let x1v = ar.var(x1);
            let yv = ar.var(y);
            let one = ar.int(1);
            let guard = {
                let succ = ar.add(xv, one);
                let le = ar.le(yv, xv);
                let eq = ar.int_eq(x1v, succ);
                ar.bool_and(eq, le)
            };
            let lhs = ar.app(s, &[x1v, yv]);
            let sxy = ar.app(s, &[xv, yv]);
            let gx = ar.app(g, &[xv]);
            let rhs = ar.add(sxy, gx);
            k.schema_guarded("S(x1,y)=S(x,y)+g(x)", &[x, x1, y], Some(guard), lhs, rhs)
        };
        Env {
            k,
            s,
            g,
            a,
            b,
            c,
            m,
            s_zero,
            s_step_right,
            s_step_left,
        }
    }

    fn var(&mut self, s: Symbol) -> Tid {
        self.k.arena_mut().var(s)
    }
    fn app_s(&mut self, x: Tid, y: Tid) -> Tid {
        let f = self.s;
        self.k.arena_mut().app(f, &[x, y])
    }
    fn app_g(&mut self, x: Tid) -> Tid {
        let f = self.g;
        self.k.arena_mut().app(f, &[x])
    }
    fn int(&mut self, v: i128) -> Tid {
        self.k.arena_mut().int(v)
    }
    fn add(&mut self, x: Tid, y: Tid) -> Tid {
        self.k.arena_mut().add(x, y)
    }
    fn sub(&mut self, x: Tid, y: Tid) -> Tid {
        self.k.arena_mut().sub(x, y)
    }
    fn le(&mut self, x: Tid, y: Tid) -> Tid {
        self.k.arena_mut().le(x, y)
    }
}

/// `S(a, b + m) = S(a, b) + S(b + m, b)` under `b + m <= a`, by
/// induction on `m`.
fn prove_fold_shift(env: &mut Env) -> Result<Theorem, KernelError> {
    let a = env.var(env.a);
    let b = env.var(env.b);
    let m = env.var(env.m);
    let zero = env.int(0);
    let one = env.int(1);
    let bm = env.add(b, m);
    let hyp = env.le(bm, a);
    let lhs = env.app_s(a, bm);
    let sab = env.app_s(a, b);
    let sbm_b = env.app_s(bm, b);
    let rhs = env.add(sab, sbm_b);
    let template = Statement::new(vec![hyp], lhs, rhs);

    // ── Base: P(0) ──────────────────────────────────────────────────
    let b0 = env.add(b, zero);
    let e_b0 = env.k.arith_eq(b0, b)?; // b + 0 = b

    // S(a, b+0) = S(a, b)
    let h1 = fresh_hole(&mut env.k, Sort::Int);
    let h1v = env.var(h1);
    let ctx_a_hole = Context {
        hole: h1,
        template: env.app_s(a, h1v),
    };
    let s_a_b0_eq = env.k.congr(ctx_a_hole, &e_b0)?;

    // S(b+0, b) = S(b, b)
    let h2 = fresh_hole(&mut env.k, Sort::Int);
    let h2v = env.var(h2);
    let ctx_hole_b = Context {
        hole: h2,
        template: env.app_s(h2v, b),
    };
    let s_b0b_eq_sbb = env.k.congr(ctx_hole_b, &e_b0)?;

    // S(b, b) = 0
    let x_sym = env_x(&env.k);
    let s_bb_zero = env.k.apply_schema(env.s_zero, &[(x_sym, b)])?;
    let s_b0b_zero = env.k.trans(&s_b0b_eq_sbb, &s_bb_zero)?;

    // S(a,b) + S(b+0,b) = S(a,b) + 0 = S(a,b)
    let h3 = fresh_hole(&mut env.k, Sort::Int);
    let h3v = env.var(h3);
    let ctx_sab_plus_hole = Context {
        hole: h3,
        template: env.add(sab, h3v),
    };
    let step_a = env.k.congr(ctx_sab_plus_hole, &s_b0b_zero)?;
    let sab_plus_zero = env.add(sab, zero);
    let step_b = env.k.arith_eq(sab_plus_zero, sab)?;
    let step_c = env.k.trans(&step_a, &step_b)?;
    let step_d = env.k.sym(&step_c); // S(a,b) = S(a,b) + S(b+0,b)

    let base_raw = env.k.trans(&s_a_b0_eq, &step_d)?;
    // Discharge the schema guard `b <= b` (trivially true).
    let guard_zero = env.le(b, b);
    let base_unguarded = env.k.discharge_arith(&base_raw, guard_zero)?;
    let base_hyp = env.le(b0, a);
    let base = env.k.weaken(&base_unguarded, &[base_hyp]);

    // ── Step: P(m) ⊢ P(m+1) ─────────────────────────────────────────
    let m1 = env.add(m, one);
    let bm1 = env.add(b, m1);
    let gbm = env.app_g(bm);
    let s_bm_b = env.app_s(bm, b);

    // u1: S(a, bm1) = S(a, bm) + g(bm), guard bm1 = bm + 1
    let (x_sym, y_sym, y1_sym, x1_sym) =
        (env_x(&env.k), env_y(&env.k), env_y1(&env.k), env_x1(&env.k));
    let u1 = env
        .k
        .apply_schema(env.s_step_right, &[(x_sym, a), (y_sym, bm), (y1_sym, bm1)])?;
    // u2: S(bm1, b) = S(bm, b) + g(bm), guard bm1 = bm + 1
    let u2 = env
        .k
        .apply_schema(env.s_step_left, &[(x_sym, bm), (x1_sym, bm1), (y_sym, b)])?;
    assert_eq!(u1.statement.hyps.len(), 1, "schema guard instantiated");
    let guard_right = u1.statement.hyps[0];
    let guard_left = u2.statement.hyps[0];

    // IH as a rewrite rule.
    let ih_stmt = template.clone();
    let ih_eq = env.k.equality_term(&ih_stmt)?;
    let ih = env.k.hypothesis(ih_eq)?;

    // S(a,bm) + g(bm) = (S(a,b) + S(bm,b)) + g(bm)
    let h4 = fresh_hole(&mut env.k, Sort::Int);
    let h4v = env.var(h4);
    let ctx_hole_plus_gbm = Context {
        hole: h4,
        template: env.add(h4v, gbm),
    };
    let step_ih = env.k.congr(ctx_hole_plus_gbm, &ih)?;

    // associativity: (S(a,b) + S(bm,b)) + g(bm) = S(a,b) + (S(bm,b) + g(bm))
    let sab_plus_sbm = env.add(sab, s_bm_b);
    let left_assoc = env.add(sab_plus_sbm, gbm);
    let sbm_plus_gbm = env.add(s_bm_b, gbm);
    let right_assoc = env.add(sab, sbm_plus_gbm);
    let assoc = env.k.arith_eq(left_assoc, right_assoc)?;

    // S(bm,b) + g(bm) = S(bm1,b)
    let u2_sym = env.k.sym(&u2);
    let h5 = fresh_hole(&mut env.k, Sort::Int);
    let h5v = env.var(h5);
    let ctx_sab_plus_hole2 = Context {
        hole: h5,
        template: env.add(sab, h5v),
    };
    let step_u2 = env.k.congr(ctx_sab_plus_hole2, &u2_sym)?;

    let c1 = env.k.trans(&u1, &step_ih)?;
    let c2 = env.k.trans(&c1, &assoc)?;
    let c3 = env.k.trans(&c2, &step_u2)?;
    // c3: S(a,bm1) = S(a,b) + S(bm1,b),
    // hyps {guard_right, guard_left, ih_eq}

    // The step is proven under the successor template's hypotheses:
    // bm1 <= a (the range bound) and 0 <= m (the range is nonempty).
    let zero_le_m = env.le(zero, m);
    let hyp_step = env.le(bm1, a);
    let c4 = env.k.weaken(&c3, &[zero_le_m, hyp_step]);

    // Discharge the schema guards. guard_right is
    // (bm1 = bm + 1) && (bm < a); guard_left is
    // (bm1 = bm + 1) && (b <= bm). Both follow from the step
    // hypotheses by linear arithmetic.
    let c5 = env.k.discharge_arith(&c4, guard_right)?;
    let step = env.k.discharge_arith(&c5, guard_left)?;

    env.k.nat_induction(env.m, &template, &base, &step)
}

// Parameter lookup helpers: schemas were declared with these symbols,
// which we recover by name so the test reads like the paper proof.
fn find(k: &Kernel, name: &str) -> Symbol {
    for i in 0..k.arena().symbol_count() as u32 {
        let s = Symbol(i);
        if k.arena().var_name(s) == name {
            return s;
        }
    }
    panic!("symbol {name} not found");
}

fn env_x(k: &Kernel) -> Symbol {
    find(k, "x")
}
fn env_y(k: &Kernel) -> Symbol {
    find(k, "y")
}
fn env_y1(k: &Kernel) -> Symbol {
    find(k, "y1")
}
fn env_x1(k: &Kernel) -> Symbol {
    find(k, "x1")
}

#[test]
fn fold_shift_lemma_holds_for_all_m() {
    let mut env = Env::new();
    let thm = prove_fold_shift(&mut env).expect("induction proof");
    let expected = {
        let a = env.var(env.a);
        let b = env.var(env.b);
        let m = env.var(env.m);
        let zero = env.int(0);
        let bm = env.add(b, m);
        let hyp_le = env.le(bm, a);
        let hyp_nonneg = env.le(zero, m);
        let lhs = env.app_s(a, bm);
        let sab = env.app_s(a, b);
        let sbm_b = env.app_s(bm, b);
        let rhs = env.add(sab, sbm_b);
        Statement::new(vec![hyp_le, hyp_nonneg], lhs, rhs)
    };
    assert!(
        thm.statement.same_as(&expected),
        "got {}",
        env.k.describe(&thm.statement)
    );
    env.k.replay(&thm).expect("replay of induction proof");
}

#[test]
fn fold_shift_instantiates_to_classic_split() {
    let mut env = Env::new();
    let shift = prove_fold_shift(&mut env).expect("induction proof");
    let (a, b, c, m) = (env.a, env.b, env.c, env.m);
    let a_t = env.var(a);
    let b_t = env.var(b);
    let c_t = env.var(c);
    let m_t = env.var(m);

    // m := c - b
    let c_minus_b = env.sub(c_t, b_t);
    let inst = env
        .k
        .instantiate(&shift, &[(m, c_minus_b)])
        .expect("instantiate induction result");

    // b + (c - b) = c
    let bm = env.add(b_t, c_minus_b);
    let e_bridge = env.k.arith_eq(bm, c_t).expect("bridge arithmetic");
    let h1 = fresh_hole(&mut env.k, Sort::Int);
    let h1v = env.var(h1);
    let ctx_lhs = Context {
        hole: h1,
        template: env.app_s(a_t, h1v),
    };
    let bridge_lhs = env.k.congr(ctx_lhs, &e_bridge).expect("congr lhs"); // S(a,bm) = S(a,c)
    let bridge_lhs_sym = env.k.sym(&bridge_lhs);
    let t1 = env.k.trans(&bridge_lhs_sym, &inst).expect("trans lhs"); // S(a,c) = S(a,b) + S(bm,b)
    let h2 = fresh_hole(&mut env.k, Sort::Int);
    let h2v = env.var(h2);
    let sab = env.app_s(a_t, b_t);
    let s_hole_b = env.app_s(h2v, b_t);
    let ctx_rhs = Context {
        hole: h2,
        template: env.add(sab, s_hole_b),
    };
    let bridge_rhs = env.k.congr(ctx_rhs, &e_bridge).expect("congr rhs"); // S(a,b)+S(bm,b) = S(a,b)+S(c,b)
    let t2 = env.k.trans(&t1, &bridge_rhs).expect("trans rhs");

    // Add the intended hypotheses, then discharge the two instantiated
    // side conditions.
    let h_bc = env.le(b_t, c_t);
    let h_ca = env.le(c_t, a_t);
    let t3 = env.k.weaken(&t2, &[h_bc, h_ca]);
    let hyp_shift_bound = env.le(bm, a_t);
    let zero_t = env.int(0);
    let hyp_nonneg = env.le(zero_t, c_minus_b);
    let t4 = env
        .k
        .discharge_arith(&t3, hyp_shift_bound)
        .expect("discharge shift bound");
    let t5 = env
        .k
        .discharge_arith(&t4, hyp_nonneg)
        .expect("discharge nonneg");

    let expected_lhs = env.app_s(a_t, c_t);
    let scb = env.app_s(c_t, b_t);
    let expected_rhs = env.add(sab, scb);
    assert_eq!(t5.statement.lhs, expected_lhs);
    assert_eq!(t5.statement.rhs, expected_rhs);
    assert_eq!(t5.statement.hyps.len(), 2);
    env.k.replay(&t5).expect("replay of instantiated split law");
    let _ = m_t;
}
