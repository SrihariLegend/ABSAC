//! Loop semantics and the fold lemmas the loop invariants need.
//!
//! The recognized code is a three-stage loop nest. Its semantics is
//! encoded as three mutually recursive functions built from the
//! *recognized* stage bodies:
//!
//! ```text
//!   W32(acc,i,n) = W16(acc,i,n)                     if n < i + 32
//!   W32(acc,i,n) = W32(acc + C32(i), i + 32, n)     if i + 32 <= n
//!   W16(acc,i,n) = W1(acc,i,n)                      if n < i + 16
//!   W16(acc,i,n) = W16(acc + C16(i), i + 16, n)     if i + 16 <= n
//!   W1(acc,i,n)  = acc                              if n <= i
//!   W1(acc,i,n)  = W1(acc + E(i), i + 1, n)         if i < n
//! ```
//!
//! where `E(i)` is the scalar element from the recognized tail and
//! `C16`/`C32` are the SDM-modeled block contributions the chunk
//! lemmas tie to the fold.

use sir_mech::kernel::{Context, Kernel, KernelError, SchemaId, Statement, Theorem};
use sir_mech::term::{FuncId, Sort, Symbol, Tid};

use crate::model::Model;

/// The three loop functions (Int-valued kernels).
#[derive(Clone, Debug)]
pub struct Program {
    pub w1: FuncId,
    pub w16: FuncId,
    pub w32: FuncId,
    pub schemas: ProgramSchemas,
}

#[derive(Clone, Copy, Debug)]
pub struct ProgramSchemas {
    pub w1_base: SchemaId,
    pub w1_step: SchemaId,
    pub w16_base: SchemaId,
    pub w16_step: SchemaId,
    pub w32_base: SchemaId,
    pub w32_step: SchemaId,
}

pub(crate) fn declare_int(k: &mut Kernel, name: impl Into<String>) -> Symbol {
    k.arena_mut().declare_var(name, Sort::Int)
}

pub(crate) fn v(k: &mut Kernel, s: Symbol) -> Tid {
    k.arena_mut().var(s)
}

pub(crate) fn int(k: &mut Kernel, x: i128) -> Tid {
    k.arena_mut().int(x)
}

pub(crate) fn add(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    k.arena_mut().add(a, b)
}

pub(crate) fn sub(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    k.arena_mut().sub(a, b)
}

pub(crate) fn fresh_hole(k: &mut Kernel) -> Symbol {
    let name = format!("hole{}", k.arena().symbol_count());
    k.arena_mut().declare_var(name, Sort::Int)
}

pub(crate) fn fresh_hole_of(k: &mut Kernel, sort: Sort) -> Symbol {
    let name = format!("hole{}", k.arena().symbol_count());
    k.arena_mut().declare_var(name, sort)
}

pub(crate) fn find(k: &Kernel, name: &str) -> Symbol {
    for i in 0..k.arena().symbol_count() as u32 {
        let s = Symbol(i);
        if k.arena().var_name(s) == name {
            return s;
        }
    }
    panic!("symbol {name} not found");
}

impl Program {
    /// Build the loop semantics for an Int-valued kernel.
    pub fn build(k: &mut Kernel, model: &Model) -> Result<Program, KernelError> {
        if !matches!(
            model.kind,
            crate::emitted::KernelKind::CardinalityMasked { .. }
                | crate::emitted::KernelKind::SumAscii { .. }
        ) {
            return Err(KernelError::Shape(
                "loops: predicate kernel needs the boolean program builder".into(),
            ));
        }
        let w1 = k
            .arena_mut()
            .declare_func("W1", vec![Sort::Int, Sort::Int, Sort::Int], Sort::Int);
        let w16 = k
            .arena_mut()
            .declare_func("W16", vec![Sort::Int, Sort::Int, Sort::Int], Sort::Int);
        let w32 = k
            .arena_mut()
            .declare_func("W32", vec![Sort::Int, Sort::Int, Sort::Int], Sort::Int);
        let (acc, i, n) = {
            let ar = k.arena_mut();
            (
                ar.declare_var("p_acc", Sort::Int),
                ar.declare_var("p_i", Sort::Int),
                ar.declare_var("p_n", Sort::Int),
            )
        };
        let params = [acc, i, n];

        // W1 base
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let (a, iv, nv) = (ar.var(acc), ar.var(i), ar.var(n));
            let lhs = ar.app(w1, &[a, iv, nv]);
            let guard = ar.le(nv, iv);
            (lhs, a, guard)
        };
        let w1_base = k.schema_guarded("W1_base", &params, Some(guard), lhs, rhs);

        // W1 step
        let (a, iv, nv) = {
            let ar = k.arena_mut();
            (ar.var(acc), ar.var(i), ar.var(n))
        };
        let e_i = model.elem(k, iv);
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let one = ar.int(1);
            let i1 = ar.add(iv, one);
            let guard = ar.lt(iv, nv);
            let lhs = ar.app(w1, &[a, iv, nv]);
            let acc1 = ar.add(a, e_i);
            let rhs = ar.app(w1, &[acc1, i1, nv]);
            (lhs, rhs, guard)
        };
        let w1_step = k.schema_guarded("W1_step", &params, Some(guard), lhs, rhs);

        // W16 base
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let (a, iv, nv) = (ar.var(acc), ar.var(i), ar.var(n));
            let sixteen = ar.int(16);
            let ip = ar.add(iv, sixteen);
            let guard = ar.lt(nv, ip);
            let lhs = ar.app(w16, &[a, iv, nv]);
            let rhs = ar.app(w1, &[a, iv, nv]);
            (lhs, rhs, guard)
        };
        let w16_base = k.schema_guarded("W16_base", &params, Some(guard), lhs, rhs);

        // W16 step
        let (a, iv, nv) = {
            let ar = k.arena_mut();
            (ar.var(acc), ar.var(i), ar.var(n))
        };
        let c16_i = model.contribution(k, 16, iv);
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let sixteen = ar.int(16);
            let ip = ar.add(iv, sixteen);
            let guard = ar.le(ip, nv);
            let lhs = ar.app(w16, &[a, iv, nv]);
            let acc1 = ar.add(a, c16_i);
            let rhs = ar.app(w16, &[acc1, ip, nv]);
            (lhs, rhs, guard)
        };
        let w16_step = k.schema_guarded("W16_step", &params, Some(guard), lhs, rhs);

        // W32 base
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let (a, iv, nv) = (ar.var(acc), ar.var(i), ar.var(n));
            let w = ar.int(32);
            let ip = ar.add(iv, w);
            let guard = ar.lt(nv, ip);
            let lhs = ar.app(w32, &[a, iv, nv]);
            let rhs = ar.app(w16, &[a, iv, nv]);
            (lhs, rhs, guard)
        };
        let w32_base = k.schema_guarded("W32_base", &params, Some(guard), lhs, rhs);

        // W32 step
        let (a, iv, nv) = {
            let ar = k.arena_mut();
            (ar.var(acc), ar.var(i), ar.var(n))
        };
        let c32_i = model.contribution(k, 32, iv);
        let (lhs, rhs, guard) = {
            let ar = k.arena_mut();
            let w = ar.int(32);
            let ip = ar.add(iv, w);
            let guard = ar.le(ip, nv);
            let lhs = ar.app(w32, &[a, iv, nv]);
            let acc1 = ar.add(a, c32_i);
            let rhs = ar.app(w32, &[acc1, ip, nv]);
            (lhs, rhs, guard)
        };
        let w32_step = k.schema_guarded("W32_step", &params, Some(guard), lhs, rhs);

        Ok(Program {
            w1,
            w16,
            w32,
            schemas: ProgramSchemas {
                w1_base,
                w1_step,
                w16_base,
                w16_step,
                w32_base,
                w32_step,
            },
        })
    }

    pub fn run(&self, k: &mut Kernel, level: usize, acc: Tid, i: Tid, n: Tid) -> Tid {
        let f = match level {
            1 => self.w1,
            16 => self.w16,
            _ => self.w32,
        };
        k.arena_mut().app(f, &[acc, i, n])
    }

    pub fn identity(&self, k: &mut Kernel) -> Tid {
        k.arena_mut().int(0)
    }
}

/// `[b <= c, c <= a] ⊢ S(a,b) = S(a,c) + S(c,b)` — the fold splitting
/// law, by induction on the offset `m` with `c = b + m`.
pub struct SplitLaw {
    pub theorem: Theorem,
    /// (a, b, c) template variables.
    pub vars: (Symbol, Symbol, Symbol),
}

pub fn fold_split_law(k: &mut Kernel, model: &Model) -> Result<SplitLaw, KernelError> {
    if model.fold_int.is_none() {
        return Err(KernelError::Shape(
            "fold_split_law: predicate folds not implemented yet".into(),
        ));
    }
    let a = declare_int(k, "sp_a");
    let b = declare_int(k, "sp_b");
    let m = declare_int(k, "sp_m");
    let (at, bt, mt) = (v(k, a), v(k, b), v(k, m));
    // c = b + m  (terms in `m`)
    let c_of = |k: &mut Kernel, mm: Tid| add(k, bt, mm);
    let c0 = c_of(k, mt);
    let one = int(k, 1);
    let m1 = add(k, mt, one);
    let c1 = c_of(k, m1);

    // Template: [b + m <= a]  ⊢  S(a,b) = S(a,b+m) + S(b+m,b)
    let lhs = model.fold(k, at, bt);
    let s_ac = model.fold(k, at, c0);
    let s_cb = model.fold(k, c0, bt);
    let (hyp, rhs) = {
        let ar = k.arena_mut();
        (ar.le(c0, at), ar.add(s_ac, s_cb))
    };
    let template = Statement::new(vec![hyp], lhs, rhs);

    // ── base: m = 0 ─────────────────────────────────────────────────
    let base = {
        let fold_x = find(k, "fold_x");
        let base_raw = k.apply_schema(model.schemas.base, &[(fold_x, bt)])?;
        let guard = base_raw.statement.hyps[0];
        let s_bb_zero = k.discharge_arith(&base_raw, guard)?;
        // c0 at m = 0 is b + 0
        let zero = int(k, 0);
        let b0 = add(k, bt, zero);
        let bridge = k.arith_eq(b0, bt)?;
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_a = model.fold(k, at, hv);
        let s_ab0_eq = k.congr(Context { hole, template: t_a }, &bridge)?;
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_hb = model.fold(k, hv, bt);
        let s_b0b_eq = k.congr(Context { hole, template: t_hb }, &bridge)?;
        let s_b0b_zero = k.trans(&s_b0b_eq, &s_bb_zero)?;
        let sab = model.fold(k, at, bt);
        // S(a,b) + S(b+0,b) = S(a,b) + 0 = S(a,b)
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_plus = add(k, sab, hv);
        let step_a = k.congr(Context { hole, template: t_plus }, &s_b0b_zero)?;
        let zero_const = int(k, 0);
        let sab_plus_zero = add(k, sab, zero_const);
        let step_b = k.arith_eq(sab_plus_zero, sab)?;
        let rhs_zero = k.trans(&step_a, &step_b)?; // rhs(0) = S(a,b)
        // Rewrite S(a, b+0) to S(a,b) in the target's first summand.
        let s_b0b = model.fold(k, b0, bt);
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_first = add(k, hv, s_b0b);
        let first = k.congr(Context { hole, template: t_first }, &s_ab0_eq)?;
        let rhs_zero_eq = k.trans(&first, &rhs_zero)?; // rhs(0) = S(a,b)
        let chained = k.sym(&rhs_zero_eq); // S(a,b) = rhs(0)
        let hyp0 = {
            let ar = k.arena_mut();
            ar.le(b0, at)
        };
        k.weaken(&chained, &[hyp0])
    };

    // ── step: m → m+1 ───────────────────────────────────────────────
    let step = {
        let sab = model.fold(k, at, bt);
        let s_ac = model.fold(k, at, c0);
        let s_cb = model.fold(k, c0, bt);
        let ih_rhs = add(k, s_ac, s_cb);
        let ih_eq = k.arena_mut().int_eq(sab, ih_rhs);
        let ih = k.hypothesis(ih_eq)?;

        let fold_x = find(k, "fold_x");
        let fold_y = find(k, "fold_y");
        let fold_y1 = find(k, "fold_y1");
        let fold_step = k.apply_schema(
            model.schemas.step_right,
            &[(fold_x, at), (fold_y, c0), (fold_y1, c1)],
        )?;
        let guard_right = fold_step.statement.hyps[0];
        let fold_left = k.apply_schema(
            model.schemas.step_left,
            &[(fold_x, c0), (find(k, "fold_x1"), c1), (fold_y, bt)],
        )?;
        let guard_left = fold_left.statement.hyps[0];

        let s_ac1 = model.fold(k, at, c1);
        let e_c = model.elem(k, c0);
        // S(a,b) = (S(a,c1) + E) + S(c,b)   [IH + fold_step]
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_expand = add(k, hv, s_cb);
        let expanded = k.congr(Context { hole, template: t_expand }, &fold_step)?;
        let ih_expanded = k.trans(&ih, &expanded)?;
        // = S(a,c1) + (S(c,b) + E)          [arithmetic]
        let lhs_assoc = {
            let inner = add(k, s_ac1, e_c);
            add(k, inner, s_cb)
        };
        let rhs_assoc = {
            let inner = add(k, s_cb, e_c);
            add(k, s_ac1, inner)
        };
        let assoc = k.arith_eq(lhs_assoc, rhs_assoc)?;
        let combined = k.trans(&ih_expanded, &assoc)?;
        // = S(a,c1) + S(c1,b)               [fold_left, reversed]
        let hole = fresh_hole(k);
        let hv = v(k, hole);
        let t_fold_left = add(k, s_ac1, hv);
        let left_rewrite = k.congr(Context { hole, template: t_fold_left }, &k.sym(&fold_left))?;
        let step_result = k.trans(&combined, &left_rewrite)?;
        // Hypotheses of the successor instance.
        let hyp_next = {
            let ar = k.arena_mut();
            ar.le(c1, at)
        };
        let zero_le_m = {
            let z = int(k, 0);
            let ar = k.arena_mut();
            ar.le(z, mt)
        };
        let widened = k.weaken(&step_result, &[hyp_next, zero_le_m]);
        let mut w = widened;
        if w.statement.hyps.contains(&guard_right) {
            w = k.discharge_arith(&w, guard_right)?;
        }
        if w.statement.hyps.contains(&guard_left) {
            w = k.discharge_arith(&w, guard_left)?;
        }
        w
    };

    let induction = k.nat_induction(m, &template, &base, &step)?;

    // Generalize to S(a,b) = S(a,c) + S(c,b) under b <= c <= a.
    let c = declare_int(k, "sp_c");
    let ct = v(k, c);
    let m_inst = sub(k, ct, bt);
    let inst = k.instantiate(&induction, &[(m, m_inst)])?;
    // inst: S(a,b) = S(a, b+(c-b)) + S(b+(c-b), b)
    // Bridge b + (c - b) -> c inside both summands.
    let b_plus_m = add(k, bt, m_inst);
    let bridge = k.arith_eq(b_plus_m, ct)?;
    let hole = fresh_hole(k);
    let hv = v(k, hole);
    let t_fold_a = model.fold(k, at, hv);
    let s_a_bridge = k.congr(Context { hole, template: t_fold_a }, &bridge)?;
    // S(b+(c-b), b) = S(c, b)
    let s_bmb = model.fold(k, b_plus_m, bt);
    let hole = fresh_hole(k);
    let hv = v(k, hole);
    let t_fold_hb = model.fold(k, hv, bt);
    let s_b_bridge = k.congr(Context { hole, template: t_fold_hb }, &bridge)?;
    // Rewrite the instantiated right-hand side.
    let hole = fresh_hole(k);
    let hv = v(k, hole);
    let t_sum1 = add(k, hv, s_bmb);
    let step1 = k.congr(Context { hole, template: t_sum1 }, &s_a_bridge)?;
    let s_ac = model.fold(k, at, ct);
    let hole = fresh_hole(k);
    let hv = v(k, hole);
    let t_sum2 = add(k, s_ac, hv);
    let step2 = k.congr(Context { hole, template: t_sum2 }, &s_b_bridge)?;
    let chained = k.trans(&inst, &step1)?;
    let t2 = k.trans(&chained, &step2)?;

    let hyp_bc = {
        let ar = k.arena_mut();
        ar.le(bt, ct)
    };
    let hyp_ca = {
        let ar = k.arena_mut();
        ar.le(ct, at)
    };
    let t3 = k.weaken(&t2, &[hyp_bc, hyp_ca]);
    let h_bound = {
        let ar = k.arena_mut();
        ar.le(b_plus_m, at)
    };
    let h_nonneg = {
        let z = int(k, 0);
        let ar = k.arena_mut();
        ar.le(z, m_inst)
    };
    let t4 = if t3.statement.hyps.contains(&h_bound) {
        k.discharge_arith(&t3, h_bound)?
    } else {
        t3
    };
    let t5 = if t4.statement.hyps.contains(&h_nonneg) {
        k.discharge_arith(&t4, h_nonneg)?
    } else {
        t4
    };
    Ok(SplitLaw {
        theorem: t5,
        vars: (a, b, c),
    })
}

/// A loop lemma together with the template variables it was proven
/// with, so callers can instantiate it at concrete states.
#[derive(Clone, Debug)]
pub struct LoopLemma {
    pub theorem: Theorem,
    /// (acc, i, n) template variables.
    pub vars: (Symbol, Symbol, Symbol),
}

/// Prove `i <= n ⊢ W_level(acc,i,n) = acc + S(n,i)`.
///
/// `sub_lemma` is the lemma for the immediately smaller stage (W1 for
/// W16, W16 for W32) and `chunk` is the chunk lemma for this stage's
/// width (`None` for the scalar stage).
pub fn loop_lemma(
    k: &mut Kernel,
    model: &Model,
    prog: &Program,
    level: usize,
    sub_lemma: Option<&LoopLemma>,
    chunk: Option<(&Symbol, &Theorem)>,
    tag: &str,
) -> Result<LoopLemma, KernelError> {
    assert!(level == 1 || level == 16 || level == 32);
    let width = level as i128;
    let acc = declare_int(k, format!("{tag}_acc"));
    let iv = declare_int(k, format!("{tag}_i"));
    let nv = declare_int(k, format!("{tag}_n"));
    let rv = declare_int(k, format!("{tag}_r"));
    let (a_t, i_t, n_t, r_t) = (v(k, acc), v(k, iv), v(k, nv), v(k, rv));
    let one = int(k, 1);
    let r1 = add(k, r_t, one);
    let i_plus_r = add(k, i_t, r_t);
    let hyp_eq = k.arena_mut().int_eq(i_plus_r, n_t);
    let lhs = prog.run(k, level, a_t, i_t, n_t);
    let s_ni = model.fold(k, n_t, i_t);
    let rhs = add(k, a_t, s_ni);
    let template = Statement::new(vec![hyp_eq], lhs, rhs);

    // ── base: P(0) ──────────────────────────────────────────────────
    let base = {
        let zero_const = int(k, 0);
        let i0 = add(k, i_t, zero_const);
        let hyp0 = k.arena_mut().int_eq(i0, n_t);
        let schema = match level {
            1 => prog.schemas.w1_base,
            16 => prog.schemas.w16_base,
            _ => prog.schemas.w32_base,
        };
        let base_inst = k.apply_schema(schema, &[(p_acc(k), a_t), (p_i(k), i_t), (p_n(k), n_t)])?;
        let guard = base_inst.statement.hyps[0];

        let target = if level == 1 {
            // S(n,i) = S(i,i) = 0 because i + 0 = n.
            let h_hyp = k.hypothesis(hyp0)?; // [hyp0] ⊢ i+0 = n
            let i0_eq_i = k.arith_eq(i0, i_t)?; // i+0 = i
            let i_eq_n = k.trans(&k.sym(&i0_eq_i), &h_hyp)?; // [hyp0] ⊢ i = n
            let fold_base = k.apply_schema(model.schemas.base, &[(find(k, "fold_x"), i_t)])?;
            let fb_guard = fold_base.statement.hyps[0];
            let s_ii_zero = k.discharge_arith(&fold_base, fb_guard)?;
            let hole = fresh_hole(k);
            let hv = v(k, hole);
            let t_fold = model.fold(k, hv, i_t);
            let s_ii_eq_s_ni = k.congr(Context { hole, template: t_fold }, &i_eq_n)?;
            let s_ni_zero = k.trans(&k.sym(&s_ii_eq_s_ni), &s_ii_zero)?;
            let hole = fresh_hole(k);
            let hv = v(k, hole);
            let t_plus = add(k, a_t, hv);
            let acc_plus = k.congr(Context { hole, template: t_plus }, &s_ni_zero)?;
            let zero_const2 = int(k, 0);
            let acc_plus_zero = add(k, a_t, zero_const2);
            let acc_zero_acc = k.arith_eq(acc_plus_zero, a_t)?;
            let acc_plus_eq_acc = k.trans(&acc_plus, &acc_zero_acc)?;
            k.trans(&base_inst, &k.sym(&acc_plus_eq_acc))?
        } else {
            // The stage's base branch calls the smaller stage: use its
            // lemma, instantiated at (acc, i, n).
            let sub = sub_lemma.expect("sub-lemma for a vector stage");
            let sub_inst = k.instantiate(
                &sub.theorem,
                &[(sub.vars.0, a_t), (sub.vars.1, i_t), (sub.vars.2, n_t)],
            )?;
            let sub_hyp = sub_inst.statement.hyps[0]; // i <= n
            let mut w = k.weaken(&sub_inst, &[hyp0]);
            if w.statement.hyps.contains(&sub_hyp) {
                w = k.discharge_arith(&w, sub_hyp)?;
            }
            k.trans(&base_inst, &w)?
        };
        let mut t = target;
        if t.statement.hyps.contains(&guard) {
            t = k.discharge_arith(&t, guard)?;
        }
        k.weaken(&t, &[hyp0])
    };

    // ── step: P(r) → P(r+1) ─────────────────────────────────────────
    let cases: Vec<(Option<Tid>, Theorem)> = {
        let c_i = if level == 1 {
            model.elem(k, i_t)
        } else {
            model.contribution(k, level, i_t)
        };
        let i_step = if level == 1 {
            let one = int(k, 1);
            add(k, i_t, one)
        } else {
            let w = int(k, width);
            add(k, i_t, w)
        };
        let acc_step = add(k, a_t, c_i);
        // IH instance: same invariant at the recursive state.
        let r_step = if level == 1 {
            r_t
        } else {
            let d = int(k, width - 1);
            sub(k, r_t, d)
        };
        let inst_stmt = k.instantiate_statement(
            &template,
            &[(acc, acc_step), (iv, i_step), (rv, r_step)],
        )?;
        let ih_eq = k.equality_term(&inst_stmt)?;
        let ih = k.hypothesis(ih_eq)?;
        let i_next = add(k, i_t, r1);
        let hyp_next = k.arena_mut().int_eq(i_next, n_t);

        let s_step = {
            let schema = match level {
                1 => prog.schemas.w1_step,
                16 => prog.schemas.w16_step,
                _ => prog.schemas.w32_step,
            };
            k.apply_schema(schema, &[(p_acc(k), a_t), (p_i(k), i_t), (p_n(k), n_t)])?
        };
        let guard = s_step.statement.hyps[0];
        // W(acc,i,n) = W(state')  ;  IH: W(state') = acc' + S(n,i')
        let chain = k.trans(&s_step, &ih)?;

        let build_target = |k: &mut Kernel| -> Result<Theorem, KernelError> {
            // acc + S(n,i) = (acc + C(i)) + S(n,i')
            let i_st = i_step;
            let c = c_i;
            let acc_c = add(k, a_t, c);
            let s_ni_st = model.fold(k, n_t, i_st);
            let target_rhs = add(k, acc_c, s_ni_st);
            if level == 1 {
                // Fold step: S(n,i) = S(n,i+1) + E(i)
                let fs = k.apply_schema(
                    model.schemas.step_right,
                    &[
                        (find(k, "fold_x"), n_t),
                        (find(k, "fold_y"), i_t),
                        (find(k, "fold_y1"), i_st),
                    ],
                )?;
                let fs_guard = fs.statement.hyps[0];
                let hole = fresh_hole(k);
                let hv = v(k, hole);
                let t = add(k, a_t, hv);
                let lhs_rew = k.congr(Context { hole, template: t }, &fs)?;
                let mid = {
                    let inner = add(k, s_ni_st, c);
                    add(k, a_t, inner)
                };
                let assoc = k.arith_eq(mid, target_rhs)?;
                let target_eq = k.trans(&lhs_rew, &assoc)?;
                let out = k.weaken(&target_eq, &[fs_guard]);
                Ok(out)
            } else {
                // Split law: S(n,i) = S(n,i+w) + S(i+w,i); chunk:
                // C(i) = S(i+w,i).
                let split = fold_split_law(k, model)?;
                let (sa, sb, sc) = split.vars;
                let split_inst = k.instantiate(
                    &split.theorem,
                    &[(sa, n_t), (sb, i_t), (sc, i_st)],
                )?;
                let hole = fresh_hole(k);
                let hv = v(k, hole);
                let t = add(k, a_t, hv);
                let step_a = k.congr(Context { hole, template: t }, &split_inst)?;
                let (chunk_start, chunk_thm) = chunk.expect("chunk lemma for a vector stage");
                let chunk_inst = k.instantiate(chunk_thm, &[(*chunk_start, i_t)])?;
                let mid = {
                    let inner = add(k, s_ni_st, c);
                    add(k, a_t, inner)
                };
                let hole = fresh_hole(k);
                let hv = v(k, hole);
                let t2 = {
                    let inner = add(k, s_ni_st, hv);
                    add(k, a_t, inner)
                };
                // chunk: C(i) = S(i+w, i); we need the reverse to replace
                // the split law's S(i+w,i) by C(i).
                let step_b = k.congr(Context { hole, template: t2 }, &k.sym(&chunk_inst))?;
                let assoc = k.arith_eq(mid, target_rhs)?;
                let t3 = k.trans(&step_a, &step_b)?;
                let target_eq = k.trans(&t3, &assoc)?;
                Ok(target_eq)
            }
        };

        let finish_case = |k: &mut Kernel, result: &Theorem, extra: &[Tid]| -> Result<Theorem, KernelError> {
            let mut w = k.weaken(result, extra);
            if w.statement.hyps.contains(&guard) {
                w = k.discharge_arith(&w, guard)?;
            }
            Ok(w)
        };

        if level == 1 {
            let target_eq = build_target(k)?;
            let result = k.trans(&chain, &k.sym(&target_eq))?;
            let zero = int(k, 0);
            let zero_le_r = k.arena_mut().le(zero, r_t);
            let mut w = k.weaken(&result, &[hyp_next, zero_le_r]);
            w = discharge_extras(k, &w, &[hyp_next, zero_le_r, ih_eq])?;
            let _ = finish_case;
            vec![(None, w)]
        } else {
            let w = int(k, width);
            let i_ge = add(k, i_t, w);
            let guard_ge = k.arena_mut().le(i_ge, n_t);
            let not_guard_ge = k.arena_mut().bool_not(guard_ge);
            // true branch
            let case_true = {
                let target_eq = build_target(k)?;
                let result = k.trans(&chain, &k.sym(&target_eq))?;
                let zero = int(k, 0);
                let zero_le_r = k.arena_mut().le(zero, r_t);
                let w = k.weaken(&result, &[hyp_next, zero_le_r, guard_ge]);
                let w = discharge_extras(k, &w, &[hyp_next, zero_le_r, guard_ge, ih_eq])?;
                w
            };
            // false branch: the stage does not run; use the sub lemma.
            let case_false = {
                let schema = match level {
                    16 => prog.schemas.w16_base,
                    _ => prog.schemas.w32_base,
                };
                let base_inst =
                    k.apply_schema(schema, &[(p_acc(k), a_t), (p_i(k), i_t), (p_n(k), n_t)])?;
                let base_guard = base_inst.statement.hyps[0];
                let sub = sub_lemma.expect("sub-lemma for a vector stage");
                let sub_inst = k.instantiate(
                    &sub.theorem,
                    &[(sub.vars.0, a_t), (sub.vars.1, i_t), (sub.vars.2, n_t)],
                )?;
                let sub_hyp = sub_inst.statement.hyps[0];
                let zero = int(k, 0);
                let zero_le_r = k.arena_mut().le(zero, r_t);
                let mut w = k.weaken(&sub_inst, &[hyp_next, zero_le_r, not_guard_ge]);
                w = discharge_extras(k, &w, &[hyp_next, zero_le_r, not_guard_ge, sub_hyp])?;
                let chained = k.trans(&base_inst, &w)?;
                let out = discharge_extras(
                    k,
                    &chained,
                    &[hyp_next, zero_le_r, not_guard_ge, ih_eq],
                )?;
                let _ = base_guard;
                out
            };
            vec![(Some(guard_ge), case_true), (Some(not_guard_ge), case_false)]
        }
    };

    let induction = k.nat_induction_cases(rv, &template, &base, &cases)?;
    // Generalize: r := n - i, then discharge the instantiated guards.
    let m_inst = sub(k, n_t, i_t);
    let inst = k.instantiate(&induction, &[(rv, m_inst)])?;
    let hyp_ge = {
        let ar = k.arena_mut();
        ar.le(i_t, n_t)
    };
    let t1 = k.weaken(&inst, &[hyp_ge]);
    let h_nonneg = {
        let z = int(k, 0);
        let ar = k.arena_mut();
        ar.le(z, m_inst)
    };
    let i_plus_m = add(k, i_t, m_inst);
    let h_eq = k.arena_mut().int_eq(i_plus_m, n_t);
    let t2 = if t1.statement.hyps.contains(&h_eq) {
        k.discharge_arith(&t1, h_eq)?
    } else {
        t1
    };
    let t3 = if t2.statement.hyps.contains(&h_nonneg) {
        k.discharge_arith(&t2, h_nonneg)?
    } else {
        t2
    };
    let theorem = k.weaken(&t3, &[hyp_ge]);
    Ok(LoopLemma {
        theorem,
        vars: (acc, iv, nv),
    })
}

/// End-to-end: `0 <= n ⊢ W32(0,0,n) = S(n,0)`.
pub fn end_to_end(
    k: &mut Kernel,
    model: &Model,
    w32: &LoopLemma,
) -> Result<Theorem, KernelError> {
    let n_t = v(k, model.n);
    let zero = int(k, 0);
    let inst = k.instantiate(
        &w32.theorem,
        &[(w32.vars.0, zero), (w32.vars.1, zero), (w32.vars.2, n_t)],
    )?;
    let lhs = model.fold(k, n_t, zero);
    let sum_zero = add(k, zero, lhs);
    let simplify = k.arith_eq(sum_zero, lhs)?;
    k.trans(&inst, &simplify)
}

fn p_acc(k: &Kernel) -> Symbol {
    find(k, "p_acc")
}

/// Try to discharge every hypothesis that is not explicitly kept, using
/// linear arithmetic. Hypotheses that cannot be discharged are left in
/// place so the surrounding rule reports them loudly.
fn discharge_extras(
    k: &mut Kernel,
    thm: &Theorem,
    keep: &[Tid],
) -> Result<Theorem, KernelError> {
    let mut out = thm.clone();
    let extras: Vec<Tid> = out
        .statement
        .hyps
        .iter()
        .copied()
        .filter(|h| !keep.contains(h))
        .collect();
    for h in extras {
        if !out.statement.hyps.contains(&h) {
            continue;
        }
        if let Ok(next) = k.discharge_arith(&out, h) {
            out = next;
        }
    }
    Ok(out)
}

fn p_i(k: &Kernel) -> Symbol {
    find(k, "p_i")
}

fn p_n(k: &Kernel) -> Symbol {
    find(k, "p_n")
}
