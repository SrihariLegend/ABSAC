//! Generic first-match scan equivalence over integer ranges.
//!
//! `F` is the forward found-flag fold (checks the right end last):
//!
//! ```text
//!   F(a,a) = a
//!   F(a,b+1) = if F(a,b) < b then F(a,b) else if bit(b) then b else b+1
//! ```
//!
//! `R` is the first-match fold that checks the left end first:
//!
//! ```text
//!   R(a,a) = a
//!   R(a,b) = if bit(a) then a else R(a+1,b)      (a < b)
//! ```
//!
//! `F(a,b) = R(a,b)` under `a <= b` is proved symbolically with the
//! kernel's primitive `IntIte` reduction rules, so it holds for every
//! extent and needs no bit-blasting or concrete unrolling.

use std::sync::{Mutex, OnceLock};

use crate::kernel::{Context, Kernel, KernelError, SchemaId, Statement, Theorem};
use crate::term::{FuncId, Sort, Symbol, Tid};

struct Scan {
    k: Kernel,
    a: Symbol,
    b: Symbol,
    n: Symbol,
    f: FuncId,
    r: FuncId,
    bit: FuncId,
    sx: Symbol,
    sy: Symbol,
    sy1: Symbol,
    sx1: Symbol,
    id_a: Symbol,
    id_p: Symbol,
    id_x: Symbol,
    id_y: Symbol,
    id_c: Symbol,
    f_zero: SchemaId,
    f_step: SchemaId,
    r_zero: SchemaId,
    r_step: SchemaId,
}

impl Scan {
    fn new() -> Self {
        let mut k = Kernel::new();
        let a = k.arena_mut().declare_var("a", Sort::Int);
        let b = k.arena_mut().declare_var("b", Sort::Int);
        let n = k.arena_mut().declare_var("n", Sort::Int);
        let f = k
            .arena_mut()
            .declare_func("F", vec![Sort::Int, Sort::Int], Sort::Int);
        let r = k
            .arena_mut()
            .declare_func("R", vec![Sort::Int, Sort::Int], Sort::Int);
        let bit = k
            .arena_mut()
            .declare_func("bit", vec![Sort::Int], Sort::Bool);
        let sx = k.arena_mut().declare_var("x", Sort::Int);
        let sy = k.arena_mut().declare_var("y", Sort::Int);
        let sy1 = k.arena_mut().declare_var("y1", Sort::Int);
        let sx1 = k.arena_mut().declare_var("x1", Sort::Int);
        let f_zero = {
            let (lhs, rhs) = {
                let ar = k.arena_mut();
                let xv = ar.var(sx);
                (ar.app(f, &[xv, xv]), xv)
            };
            k.schema("F(x,x)=x", &[sx], lhs, rhs)
        };
        let f_step = {
            let (guard, lhs, rhs) = {
                let ar = k.arena_mut();
                let xv = ar.var(sx);
                let yv = ar.var(sy);
                let y1v = ar.var(sy1);
                let one = ar.int(1);
                let succ = ar.add(yv, one);
                let guard = ar.int_eq(y1v, succ);
                let call = ar.app(f, &[xv, yv]);
                let found = ar.lt(call, yv);
                let bit_y = ar.app(bit, &[yv]);
                let innermost = ar.int_ite(bit_y, yv, y1v);
                let rhs = ar.int_ite(found, call, innermost);
                let lhs = ar.app(f, &[xv, y1v]);
                (guard, lhs, rhs)
            };
            k.schema_guarded("F(x,y+1)=...", &[sx, sy, sy1], Some(guard), lhs, rhs)
        };
        let r_zero = {
            let (lhs, rhs) = {
                let ar = k.arena_mut();
                let xv = ar.var(sx);
                (ar.app(r, &[xv, xv]), xv)
            };
            k.schema("R(x,x)=x", &[sx], lhs, rhs)
        };
        let r_step = {
            let (guard, lhs, rhs) = {
                let ar = k.arena_mut();
                let xv = ar.var(sx);
                let x1v = ar.var(sx1);
                let yv = ar.var(sy);
                let one = ar.int(1);
                let succ = ar.add(xv, one);
                let eq = ar.int_eq(x1v, succ);
                let lt = ar.lt(xv, yv);
                let guard = ar.bool_and(eq, lt);
                let lhs = ar.app(r, &[xv, yv]);
                let rec = ar.app(r, &[x1v, yv]);
                let bit_x = ar.app(bit, &[xv]);
                let rhs = ar.int_ite(bit_x, xv, rec);
                (guard, lhs, rhs)
            };
            k.schema_guarded("R(x,y)=...", &[sx, sx1, sy], Some(guard), lhs, rhs)
        };
        let id_a = k.arena_mut().declare_var("id_a", Sort::Int);
        let id_p = k.arena_mut().declare_var("id_p", Sort::Int);
        let id_x = k.arena_mut().declare_var("id_x", Sort::Int);
        let id_y = k.arena_mut().declare_var("id_y", Sort::Int);
        let id_c = k.arena_mut().declare_var("id_c", Sort::Bool);
        Scan {
            k,
            a,
            b,
            n,
            f,
            r,
            bit,
            sx,
            sy,
            sy1,
            sx1,
            id_a,
            id_p,
            id_x,
            id_y,
            id_c,
            f_zero,
            f_step,
            r_zero,
            r_step,
        }
    }

    fn term(&mut self, symbol: Symbol) -> Tid {
        self.k.arena_mut().var(symbol)
    }

    fn int(&mut self, value: i128) -> Tid {
        self.k.arena_mut().int(value)
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

    fn lt(&mut self, x: Tid, y: Tid) -> Tid {
        self.k.arena_mut().lt(x, y)
    }

    fn eq(&mut self, x: Tid, y: Tid) -> Tid {
        self.k.arena_mut().int_eq(x, y)
    }

    fn int_ite(&mut self, c: Tid, t: Tid, e: Tid) -> Tid {
        self.k.arena_mut().int_ite(c, t, e)
    }

    fn bool_not(&mut self, x: Tid) -> Tid {
        self.k.arena_mut().bool_not(x)
    }

    fn app_f(&mut self, x: Tid, y: Tid) -> Tid {
        let f = self.f;
        self.k.arena_mut().app(f, &[x, y])
    }

    fn app_r(&mut self, x: Tid, y: Tid) -> Tid {
        let r = self.r;
        self.k.arena_mut().app(r, &[x, y])
    }

    fn app_bit(&mut self, x: Tid) -> Tid {
        let bit = self.bit;
        self.k.arena_mut().app(bit, &[x])
    }

    fn f_zero(&mut self, x: Tid, ctx: &[Tid]) -> Result<Theorem, KernelError> {
        let applied = self.k.apply_schema(self.f_zero, &[(self.sx, x)])?;
        Ok(self.k.weaken(&applied, ctx))
    }

    fn f_step(&mut self, x: Tid, y: Tid, y1: Tid, ctx: &[Tid]) -> Result<Theorem, KernelError> {
        let applied = self
            .k
            .apply_schema(self.f_step, &[(self.sx, x), (self.sy, y), (self.sy1, y1)])?;
        let mut out = self.k.weaken(&applied, ctx);
        for &guard in &applied.statement.hyps {
            let lemma = self.k.arith_entail(ctx, guard)?;
            out = self.k.discharge_lemma(&out, guard, &lemma)?;
        }
        Ok(out)
    }

    fn r_zero(&mut self, x: Tid, ctx: &[Tid]) -> Result<Theorem, KernelError> {
        let applied = self.k.apply_schema(self.r_zero, &[(self.sx, x)])?;
        Ok(self.k.weaken(&applied, ctx))
    }

    fn r_step(&mut self, x: Tid, x1: Tid, y: Tid, ctx: &[Tid]) -> Result<Theorem, KernelError> {
        let applied = self
            .k
            .apply_schema(self.r_step, &[(self.sx, x), (self.sx1, x1), (self.sy, y)])?;
        let mut out = self.k.weaken(&applied, ctx);
        for &guard in &applied.statement.hyps {
            let lemma = self.k.arith_entail(ctx, guard)?;
            out = self.k.discharge_lemma(&out, guard, &lemma)?;
        }
        Ok(out)
    }

    fn eq_under(&mut self, ctx: &[Tid], lhs: Tid, rhs: Tid) -> Result<Theorem, KernelError> {
        let goal = self.eq(lhs, rhs);
        let lemma = self.k.arith_entail(ctx, goal)?;
        let base = self.k.hypothesis(goal)?;
        let base = self.k.weaken(&base, ctx);
        self.k.discharge_lemma(&base, goal, &lemma)
    }

    fn discharge(&mut self, thm: &Theorem, goal: Tid, ctx: &[Tid]) -> Result<Theorem, KernelError> {
        let lemma = self.k.arith_entail(ctx, goal)?;
        self.k.discharge_lemma(thm, goal, &lemma)
    }

    fn congr(
        &mut self,
        hole: Symbol,
        template: Tid,
        inner: &Theorem,
    ) -> Result<Theorem, KernelError> {
        self.k.congr(Context { hole, template }, inner)
    }

    fn trans(&self, a: &Theorem, b: &Theorem) -> Result<Theorem, KernelError> {
        self.k.trans(a, b)
    }

    fn sym(&self, t: &Theorem) -> Theorem {
        self.k.sym(t)
    }

    fn hyp_matching(&self, thm: &Theorem, expected: Tid) -> Result<Tid, KernelError> {
        thm.statement
            .hyps
            .iter()
            .copied()
            .find(|hyp| *hyp == expected)
            .ok_or_else(|| KernelError::Shape("expected hypothesis not present".into()))
    }
}

/// `a < p` ⊢ the symbolic peel identity.
fn peel_identity(scan: &mut Scan) -> Result<Theorem, KernelError> {
    let (av, pv, xv, yv, cv) = (
        scan.term(scan.id_a),
        scan.term(scan.id_p),
        scan.term(scan.id_x),
        scan.term(scan.id_y),
        scan.term(scan.id_c),
    );
    let x_below = scan.lt(xv, pv);
    let tail = scan.int_ite(x_below, xv, yv);
    let hyp = scan.lt(av, pv);
    let hole = scan.k.arena_mut().declare_var("id_hole", Sort::Int);
    let hole_term = scan.term(hole);
    let hole_below = scan.lt(hole_term, pv);
    let lhs_template = scan.int_ite(hole_below, hole_term, yv);

    let c_true = scan.k.int_ite_true(cv, av, xv)?;
    let lhs_true = scan.congr(hole, lhs_template, &c_true)?;
    let rhs_true = scan.k.int_ite_true(cv, av, tail)?;
    let cond_true = scan.k.int_ite_true(hyp, av, yv)?;
    let lhs_true = scan.trans(&lhs_true, &cond_true)?;
    let true_case = scan.trans(&lhs_true, &scan.sym(&rhs_true))?;

    let c_false = scan.k.int_ite_false(cv, av, xv)?;
    let lhs_false = scan.congr(hole, lhs_template, &c_false)?;
    let rhs_false = scan.k.int_ite_false(cv, av, tail)?;
    let false_case = scan.trans(&lhs_false, &scan.sym(&rhs_false))?;

    scan.k.case_split(cv, &true_case, &false_case)
}

/// Peel lemma: `b = (a+n)+1 ⊢ F(a,b) = if bit(a) then a else F(a+1,b)`.
fn peel_lemma(scan: &mut Scan) -> Result<Theorem, KernelError> {
    let identity = peel_identity(scan)?;
    let a = scan.term(scan.a);
    let b = scan.term(scan.b);
    let n = scan.term(scan.n);
    let one = scan.int(1);
    let zero = scan.int(0);
    let a1 = scan.add(a, one);
    let a_plus_n = scan.add(a, n);
    let bound = scan.add(a_plus_n, one);
    let template_hyp = scan.eq(b, bound);
    let bit_a = scan.app_bit(a);
    let f_a1_b = scan.app_f(a1, b);
    let rhs = scan.int_ite(bit_a, a, f_a1_b);
    let f_a_b = scan.app_f(a, b);
    let template = Statement::new(vec![template_hyp], f_a_b, rhs);

    // ---- base n = 0 ----
    let a_plus_zero = scan.add(a, zero);
    let bound0 = scan.add(a_plus_zero, one);
    let hyp0 = scan.eq(b, bound0);
    let ctx0 = vec![hyp0];
    let b_eq_a1 = scan.eq_under(&ctx0, b, a1)?;
    let fzz = scan.f_zero(a, &ctx0)?;
    let step0 = scan.f_step(a, a, b, &ctx0)?;
    let hole = scan.k.arena_mut().declare_var("peel_hole", Sort::Int);
    let hole_term = scan.term(hole);
    let bit_a = scan.app_bit(a);
    let inner = scan.int_ite(bit_a, a, b);
    let hole_lt_a = scan.lt(hole_term, a);
    let template_inner = scan.int_ite(hole_lt_a, hole_term, inner);
    let rewritten0 = scan.congr(hole, template_inner, &fzz)?;
    let step0 = scan.trans(&step0, &rewritten0)?;
    let a_lt_a = scan.lt(a, a);
    let cond_false = scan.bool_not(a_lt_a);
    let false_rule = scan.k.int_ite_false(a_lt_a, a, inner)?;
    let step0 = scan.trans(&step0, &false_rule)?;
    let step0 = scan.discharge(&step0, cond_false, &[])?;

    let fzz1 = scan.f_zero(a1, &ctx0)?;
    let hole_call = scan.k.arena_mut().declare_var("peel_call", Sort::Int);
    let hole_call_term = scan.term(hole_call);
    let call_template = scan.app_f(a1, hole_call_term);
    let call_rewrite = scan.congr(hole_call, call_template, &b_eq_a1)?;
    let call_rewrite = scan.trans(&call_rewrite, &fzz1)?;
    let call_rewrite = scan.trans(&call_rewrite, &scan.sym(&b_eq_a1))?;
    let bit_a = scan.app_bit(a);
    let target_template = scan.int_ite(bit_a, a, hole_call_term);
    let target0 = scan.congr(hole_call, target_template, &call_rewrite)?;
    let base = scan.trans(&step0, &scan.sym(&target0))?;

    // ---- step ----
    let n_plus_one = scan.add(n, one);
    let a_plus_n1 = scan.add(a, n_plus_one);
    let step_bound = scan.add(a_plus_n1, one);
    let step_hyp = scan.eq(b, step_bound);
    let guard = scan.le(zero, n);
    let b1 = scan.sub(b, one);
    let lit_zero = scan.eq(n, zero);
    let lit_pos = scan.bool_not(lit_zero);
    let f_a_b1 = scan.app_f(a, b1);
    let f_a1_b1 = scan.app_f(a1, b1);
    let bit_a = scan.app_bit(a);
    let ih_rhs = scan.int_ite(bit_a, a, f_a1_b1);
    let ih_term = scan.eq(f_a_b1, ih_rhs);
    let bit_b1 = scan.app_bit(b1);
    let y = scan.int_ite(bit_b1, b1, b);
    let step_ctx = vec![step_hyp, guard, lit_pos, ih_term];

    let step_schema = scan.f_step(a, b1, b, &step_ctx)?;
    let ih_hyp = scan.k.hypothesis(ih_term)?;
    let step_rewritten = {
        let hole2 = scan.k.arena_mut().declare_var("peel_hole2", Sort::Int);
        let hole2_term = scan.term(hole2);
        let below = scan.lt(hole2_term, b1);
        let template2 = scan.int_ite(below, hole2_term, y);
        scan.congr(hole2, template2, &ih_hyp)?
    };
    let step_lhs = scan.trans(&step_schema, &step_rewritten)?;

    let target_unfold = scan.f_step(a1, b1, b, &step_ctx)?;
    let target_rewrite = {
        let hole3 = scan.k.arena_mut().declare_var("peel_hole3", Sort::Int);
        let hole3_term = scan.term(hole3);
        let bit_a = scan.app_bit(a);
        let template3 = scan.int_ite(bit_a, a, hole3_term);
        scan.congr(hole3, template3, &target_unfold)?
    };
    let x_prime = scan.app_f(a1, b1);
    let bit_a = scan.app_bit(a);
    let identity_instance = scan.k.instantiate(
        &identity,
        &[
            (scan.id_a, a),
            (scan.id_p, b1),
            (scan.id_x, x_prime),
            (scan.id_y, y),
            (scan.id_c, bit_a),
        ],
    )?;
    let identity_instance = scan.k.weaken(&identity_instance, &step_ctx);
    let identity_expected = scan.lt(a, b1);
    let identity_hyp = scan.hyp_matching(&identity_instance, identity_expected)?;
    let identity_instance = scan.discharge(&identity_instance, identity_hyp, &step_ctx)?;
    let chain = scan.trans(&step_lhs, &identity_instance)?;
    let positive = scan.trans(&chain, &scan.sym(&target_rewrite))?;

    // zero case: replace the IH with the base lemma.
    let zero_ctx = vec![step_hyp, guard, lit_zero];
    let base_instance = scan.k.instantiate(&base, &[(scan.b, b1)])?;
    let base_instance = scan.k.weaken(&base_instance, &zero_ctx);
    let a_plus_zero = scan.add(a, zero);
    let a_plus_one = scan.add(a_plus_zero, one);
    let base_expected = scan.eq(b1, a_plus_one);
    let base_hyp = scan.hyp_matching(&base_instance, base_expected)?;
    let base_instance = scan.discharge(&base_instance, base_hyp, &zero_ctx)?;
    let zero_lhs = {
        let step = scan.f_step(a, b1, b, &zero_ctx)?;
        let hole4 = scan.k.arena_mut().declare_var("peel_hole4", Sort::Int);
        let hole4_term = scan.term(hole4);
        let below = scan.lt(hole4_term, b1);
        let template4 = scan.int_ite(below, hole4_term, y);
        let rewritten = scan.congr(hole4, template4, &base_instance)?;
        scan.trans(&step, &rewritten)?
    };
    let zero_target = {
        let unfold = scan.f_step(a1, b1, b, &zero_ctx)?;
        let hole5 = scan.k.arena_mut().declare_var("peel_hole5", Sort::Int);
        let hole5_term = scan.term(hole5);
        let bit_a = scan.app_bit(a);
        let template5 = scan.int_ite(bit_a, a, hole5_term);
        scan.congr(hole5, template5, &unfold)?
    };
    let identity0 = {
        let x_prime = scan.app_f(a1, b1);
        let bit_a = scan.app_bit(a);
        let inst = scan.k.instantiate(
            &identity,
            &[
                (scan.id_a, a),
                (scan.id_p, b1),
                (scan.id_x, x_prime),
                (scan.id_y, y),
                (scan.id_c, bit_a),
            ],
        )?;
        let inst = scan.k.weaken(&inst, &zero_ctx);
        let expected = scan.lt(a, b1);
        let hyp = scan.hyp_matching(&inst, expected)?;
        scan.discharge(&inst, hyp, &zero_ctx)?
    };
    let zero_chain = scan.trans(&zero_lhs, &identity0)?;
    let zero = scan.trans(&zero_chain, &scan.sym(&zero_target))?;

    scan.k.nat_induction_cases(
        scan.n,
        &template,
        &base,
        &[(Some(lit_zero), zero), (Some(lit_pos), positive)],
    )
}

/// `a <= b ⊢ F(a,b) = R(a,b)`.
fn equivalence(scan: &mut Scan, peel: &Theorem) -> Result<Theorem, KernelError> {
    let a = scan.term(scan.a);
    let b = scan.term(scan.b);
    let n = scan.term(scan.n);
    let one = scan.int(1);
    let zero = scan.int(0);
    let a1 = scan.add(a, one);
    let a_plus_n = scan.add(a, n);
    let template_hyp = scan.eq(b, a_plus_n);
    let f_a_b = scan.app_f(a, b);
    let r_a_b = scan.app_r(a, b);
    let template = Statement::new(vec![template_hyp], f_a_b, r_a_b);

    // base m = 0
    let a_plus_zero = scan.add(a, zero);
    let hyp0 = scan.eq(b, a_plus_zero);
    let ctx0 = vec![hyp0];
    let b_eq_a = scan.eq_under(&ctx0, b, a)?;
    let fz = scan.f_zero(a, &ctx0)?;
    let rz = scan.r_zero(a, &ctx0)?;
    let hole = scan.k.arena_mut().declare_var("eq_hole", Sort::Int);
    let hole_term = scan.term(hole);
    let f_template = scan.app_f(a, hole_term);
    let f_rewrite = scan.congr(hole, f_template, &b_eq_a)?;
    let f_rewrite = scan.trans(&f_rewrite, &fz)?;
    let r_template = scan.app_r(a, hole_term);
    let r_rewrite = scan.congr(hole, r_template, &b_eq_a)?;
    let r_rewrite = scan.trans(&r_rewrite, &rz)?;
    let base = scan.trans(&f_rewrite, &scan.sym(&r_rewrite))?;

    // step
    let n_plus_one = scan.add(n, one);
    let step_bound = scan.add(a, n_plus_one);
    let step_hyp = scan.eq(b, step_bound);
    let guard = scan.le(zero, n);
    let f_a1_b = scan.app_f(a1, b);
    let r_a1_b = scan.app_r(a1, b);
    let ih_term = scan.eq(f_a1_b, r_a1_b);
    let ih = scan.k.hypothesis(ih_term)?;
    let step_ctx = vec![step_hyp, guard, ih_term];

    let peel_inst = scan.k.instantiate(peel, &[(scan.n, n)])?;
    let peel_inst = scan.k.weaken(&peel_inst, &step_ctx);
    let a_plus_n = scan.add(a, n);
    let peel_bound = scan.add(a_plus_n, one);
    let peel_expected = scan.eq(b, peel_bound);
    let peel_hyp = scan.hyp_matching(&peel_inst, peel_expected)?;
    let peel_inst = scan.discharge(&peel_inst, peel_hyp, &step_ctx)?;
    let peeled = {
        let hole2 = scan.k.arena_mut().declare_var("eq_hole2", Sort::Int);
        let hole2_term = scan.term(hole2);
        let bit_a = scan.app_bit(a);
        let template2 = scan.int_ite(bit_a, a, hole2_term);
        scan.congr(hole2, template2, &ih)?
    };
    let lhs = scan.trans(&peel_inst, &peeled)?;
    let rev = scan.r_step(a, a1, b, &step_ctx)?;
    let step = scan.trans(&lhs, &scan.sym(&rev))?;

    let inducted = scan.k.nat_induction(scan.n, &template, &base, &step)?;

    let diff = scan.sub(b, a);
    let instantiated = scan.k.instantiate(&inducted, &[(scan.n, diff)])?;
    let a_plus_diff = scan.add(a, diff);
    let identity_expected = scan.eq(b, a_plus_diff);
    let identity = scan.hyp_matching(&instantiated, identity_expected)?;
    let instantiated = scan.discharge(&instantiated, identity, &[])?;
    let nonneg = scan.le(zero, diff);
    let nonneg = scan.hyp_matching(&instantiated, nonneg)?;
    let le_hyp = scan.le(a, b);
    let instantiated = scan.k.weaken(&instantiated, &[le_hyp]);
    scan.discharge(&instantiated, nonneg, &[le_hyp])
}

/// Build and replay the generic scan theorem.
pub fn first_match_theorem() -> Result<Theorem, KernelError> {
    let mut scan = Scan::new();
    let peel = peel_lemma(&mut scan)?;
    let theorem = equivalence(&mut scan, &peel)?;
    scan.k.replay(&theorem)?;
    Ok(theorem)
}

/// Cached decision form used by the optimizer.
pub fn first_match_law() -> bool {
    static CACHE: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(value) = *guard {
            return value;
        }
    }
    let value = first_match_theorem().is_ok();
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(value);
    }
    value
}
