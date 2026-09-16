//! The `All` kernel: predicate loop semantics and early-exit lemmas.
//!
//! The generated code returns 0 as soon as a block fails and 1 at the
//! end. Modeled as boolean-valued functions:
//!
//! ```text
//!   V32(i,n) = V16(i,n)                     if n < i + 32
//!   V32(i,n) = V32(i + 32, n)               if i + 32 <= n && C32(i)
//!   V32(i,n) = false                        if i + 32 <= n && !C32(i)
//!   V16(i,n) = V1(i,n)                      if n < i + 16
//!   V16(i,n) = V16(i + 16, n)               if i + 16 <= n && C16(i)
//!   V16(i,n) = false                        if i + 16 <= n && !C16(i)
//!   V1(i,n)  = true                         if n <= i
//!   V1(i,n)  = V1(i + 1, n)                 if i < n && E(i)
//!   V1(i,n)  = false                        if i < n && !E(i)
//! ```
//!
//! and proven equal to the boolean range fold `A(n,i)` for all `i <= n`
//! by case-split induction. Propositional obligations are discharged by
//! bit-blasting *under the case's guard*.

use sir_mech::kernel::{Context, Kernel, KernelError, SchemaId, Statement, Theorem};
use sir_mech::term::{FuncId, Sort, Symbol, Tid};

use crate::loops::{add, declare_int, find, fresh_hole_of, int, sub as isub, v};
use crate::model::Model;

pub struct PredProgram {
    pub v1: FuncId,
    pub v16: FuncId,
    pub v32: FuncId,
    pub schemas: PredSchemas,
}

#[derive(Clone, Copy, Debug)]
pub struct PredSchemas {
    pub v1_base: SchemaId,
    pub v1_step: SchemaId,
    pub v1_exit: SchemaId,
    pub v16_base: SchemaId,
    pub v16_step: SchemaId,
    pub v16_exit: SchemaId,
    pub v32_base: SchemaId,
    pub v32_step: SchemaId,
    pub v32_exit: SchemaId,
}

/// A predicate loop lemma plus its `(i, n)` template variables.
#[derive(Clone, Debug)]
pub struct PredLemma {
    pub theorem: Theorem,
    pub vars: (Symbol, Symbol),
}

fn and_of(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    k.arena_mut().bool_and(a, b)
}

fn not_of(k: &mut Kernel, a: Tid) -> Tid {
    k.arena_mut().bool_not(a)
}

pub fn build(k: &mut Kernel, model: &Model) -> Result<PredProgram, KernelError> {
    if model.fold_bool.is_none() {
        return Err(KernelError::Shape(
            "pred: predicate program requires the boolean fold".into(),
        ));
    }
    let v1 = k
        .arena_mut()
        .declare_func("V1", vec![Sort::Int, Sort::Int], Sort::Bool);
    let v16 = k
        .arena_mut()
        .declare_func("V16", vec![Sort::Int, Sort::Int], Sort::Bool);
    let v32 = k
        .arena_mut()
        .declare_func("V32", vec![Sort::Int, Sort::Int], Sort::Bool);
    let (i, n) = {
        let ar = k.arena_mut();
        (
            ar.declare_var("pred_i", Sort::Int),
            ar.declare_var("pred_n", Sort::Int),
        )
    };
    let params = [i, n];
    let (iv, nv) = (v(k, i), v(k, n));

    // V1
    let (v1_base, v1_step, v1_exit) = {
        let t = k.arena_mut().bool(true);
        let g = k.arena_mut().le(nv, iv);
        let lhs = k.arena_mut().app(v1, &[iv, nv]);
        let base = k.schema_guarded("V1_base", &params, Some(g), lhs, t);

        let e = model.elem(k, iv);
        let one = int(k, 1);
        let i1 = add(k, iv, one);
        let rhs = k.arena_mut().app(v1, &[i1, nv]);
        let lt = k.arena_mut().lt(iv, nv);
        let g = and_of(k, lt, e);
        let lhs = k.arena_mut().app(v1, &[iv, nv]);
        let step = k.schema_guarded("V1_step", &params, Some(g), lhs, rhs);

        let f = k.arena_mut().bool(false);
        let lt = k.arena_mut().lt(iv, nv);
        let ne = not_of(k, e);
        let g = and_of(k, lt, ne);
        let lhs = k.arena_mut().app(v1, &[iv, nv]);
        let exit = k.schema_guarded("V1_exit", &params, Some(g), lhs, f);
        (base, step, exit)
    };

    // V16
    let (v16_base, v16_step, v16_exit) = {
        let sixteen = int(k, 16);
        let i16 = add(k, iv, sixteen);
        let c16 = model.contribution(k, 16, iv);
        let g = k.arena_mut().lt(nv, i16);
        let lhs = k.arena_mut().app(v16, &[iv, nv]);
        let rhs = k.arena_mut().app(v1, &[iv, nv]);
        let base = k.schema_guarded("V16_base", &params, Some(g), lhs, rhs);

        let rhs = k.arena_mut().app(v16, &[i16, nv]);
        let ge = k.arena_mut().le(i16, nv);
        let g = and_of(k, ge, c16);
        let lhs = k.arena_mut().app(v16, &[iv, nv]);
        let step = k.schema_guarded("V16_step", &params, Some(g), lhs, rhs);

        let f = k.arena_mut().bool(false);
        let ge = k.arena_mut().le(i16, nv);
        let nc = not_of(k, c16);
        let g = and_of(k, ge, nc);
        let lhs = k.arena_mut().app(v16, &[iv, nv]);
        let exit = k.schema_guarded("V16_exit", &params, Some(g), lhs, f);
        (base, step, exit)
    };

    // V32
    let (v32_base, v32_step, v32_exit) = {
        let thirty_two = int(k, 32);
        let i32_ = add(k, iv, thirty_two);
        let c32 = model.contribution(k, 32, iv);
        let g = k.arena_mut().lt(nv, i32_);
        let lhs = k.arena_mut().app(v32, &[iv, nv]);
        let rhs = k.arena_mut().app(v16, &[iv, nv]);
        let base = k.schema_guarded("V32_base", &params, Some(g), lhs, rhs);

        let rhs = k.arena_mut().app(v32, &[i32_, nv]);
        let ge = k.arena_mut().le(i32_, nv);
        let g = and_of(k, ge, c32);
        let lhs = k.arena_mut().app(v32, &[iv, nv]);
        let step = k.schema_guarded("V32_step", &params, Some(g), lhs, rhs);

        let f = k.arena_mut().bool(false);
        let ge = k.arena_mut().le(i32_, nv);
        let nc = not_of(k, c32);
        let g = and_of(k, ge, nc);
        let lhs = k.arena_mut().app(v32, &[iv, nv]);
        let exit = k.schema_guarded("V32_exit", &params, Some(g), lhs, f);
        (base, step, exit)
    };

    Ok(PredProgram {
        v1,
        v16,
        v32,
        schemas: PredSchemas {
            v1_base,
            v1_step,
            v1_exit,
            v16_base,
            v16_step,
            v16_exit,
            v32_base,
            v32_step,
            v32_exit,
        },
    })
}

impl PredProgram {
    pub fn run(&self, k: &mut Kernel, level: usize, i: Tid, n: Tid) -> Tid {
        let f = match level {
            1 => self.v1,
            16 => self.v16,
            _ => self.v32,
        };
        k.arena_mut().app(f, &[i, n])
    }
}

/// `[b <= c, c <= a] ⊢ A(a,b) = A(a,c) ∧ A(c,b)`.
pub struct BoolSplitLaw {
    pub theorem: Theorem,
    pub vars: (Symbol, Symbol, Symbol),
}

pub fn bool_split_law(k: &mut Kernel, model: &Model) -> Result<BoolSplitLaw, KernelError> {
    let a = declare_int(k, "ba_a");
    let b = declare_int(k, "ba_b");
    let m = declare_int(k, "ba_m");
    let (at, bt, mt) = (v(k, a), v(k, b), v(k, m));
    let c0 = add(k, bt, mt);
    let one = int(k, 1);
    let m1 = add(k, mt, one);
    let c1 = add(k, bt, m1);
    let a_ab = model.fold(k, at, bt);
    let a_ac = model.fold(k, at, c0);
    let a_cb = model.fold(k, c0, bt);
    let rhs = and_of(k, a_ac, a_cb);
    let hyp = k.arena_mut().le(c0, at);
    let template = Statement::new(vec![hyp], a_ab, rhs);

    // ── base: m = 0 ─────────────────────────────────────────────────
    let base = {
        let fold_x = find(k, "fold_x");
        let fb = k.apply_schema(model.schemas.base, &[(fold_x, bt)])?;
        let fb_guard = fb.statement.hyps[0];
        let a_bb_true = k.discharge_arith(&fb, fb_guard)?;
        let zero = int(k, 0);
        let b0 = add(k, bt, zero);
        let bridge = k.arith_eq(b0, bt)?;
        // A(a,b+0) = A(a,b)
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = model.fold(k, at, hv);
        let a_ab0_eq = k.congr(Context { hole, template: t }, &bridge)?;
        // A(b+0,b) = A(b,b) = true
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = model.fold(k, hv, bt);
        let a_b0b_eq = k.congr(Context { hole, template: t }, &bridge)?;
        let a_b0b_true = k.trans(&a_b0b_eq, &a_bb_true)?;
        // A(a,b+0) ∧ A(b+0,b) -> A(a,b) ∧ A(b+0,b) -> A(a,b) ∧ true -> A(a,b)
        let a_b0b = model.fold(k, b0, bt);
        let hole = fresh_hole_of(k, Sort::Bool);
        let hv = v(k, hole);
        let t = and_of(k, hv, a_b0b);
        let step1 = k.congr(Context { hole, template: t }, &a_ab0_eq)?;
        let hole = fresh_hole_of(k, Sort::Bool);
        let hv = v(k, hole);
        let t = and_of(k, a_ab, hv);
        let step2 = k.congr(Context { hole, template: t }, &a_b0b_true)?;
        let tr = k.arena_mut().bool(true);
        let ab_and_true = and_of(k, a_ab, tr);
        let simpl = k.bitblast_eq(ab_and_true, a_ab)?;
        let t1 = k.trans(&step1, &step2)?;
        let t2 = k.trans(&t1, &simpl)?; // rhs(0) = A(a,b)
        let out = k.sym(&t2);
        let hyp0 = k.arena_mut().le(b0, at);
        k.weaken(&out, &[hyp0])
    };

    // ── step ────────────────────────────────────────────────────────
    let step = {
        let ih_rhs = and_of(k, a_ac, a_cb);
        let ih_eq = k.arena_mut().bool_eq(a_ab, ih_rhs);
        let ih = k.hypothesis(ih_eq)?;
        let fold_x = find(k, "fold_x");
        let fold_y = find(k, "fold_y");
        let fold_y1 = find(k, "fold_y1");
        let fold_step = k.apply_schema(
            model.schemas.step_right,
            &[(fold_x, at), (fold_y, c0), (fold_y1, c1)],
        )?;
        let g_right = fold_step.statement.hyps[0];
        let fold_left = k.apply_schema(
            model.schemas.step_left,
            &[(fold_x, c0), (find(k, "fold_x1"), c1), (fold_y, bt)],
        )?;
        let g_left = fold_left.statement.hyps[0];

        let a_ac1 = model.fold(k, at, c1);
        let a_c1b = model.fold(k, c1, bt);
        let e_c = model.elem(k, c0);
        // A(a,b) = (A(a,c1) ∧ E) ∧ A(c,b)
        let hole = fresh_hole_of(k, Sort::Bool);
        let hv = v(k, hole);
        let t = and_of(k, hv, a_cb);
        let expanded = k.congr(Context { hole, template: t }, &fold_step)?;
        let ih_expanded = k.trans(&ih, &expanded)?;
        // = A(a,c1) ∧ (A(c,b) ∧ E)
        let lhs_assoc = {
            let inner = and_of(k, a_ac1, e_c);
            and_of(k, inner, a_cb)
        };
        let rhs_assoc = {
            let inner = and_of(k, a_cb, e_c);
            and_of(k, a_ac1, inner)
        };
        let assoc = k.bitblast_eq(lhs_assoc, rhs_assoc)?;
        let combined = k.trans(&ih_expanded, &assoc)?;
        // = A(a,c1) ∧ A(c1,b)
        let hole = fresh_hole_of(k, Sort::Bool);
        let hv = v(k, hole);
        let t = and_of(k, a_ac1, hv);
        let left_rewrite = k.congr(Context { hole, template: t }, &k.sym(&fold_left))?;
        let step_result = k.trans(&combined, &left_rewrite)?;
        let hyp_next = k.arena_mut().le(c1, at);
        let z = int(k, 0);
        let zero_le_m = k.arena_mut().le(z, mt);
        let mut w = k.weaken(&step_result, &[hyp_next, zero_le_m]);
        if w.statement.hyps.contains(&g_right) {
            w = k.discharge_arith(&w, g_right)?;
        }
        if w.statement.hyps.contains(&g_left) {
            w = k.discharge_arith(&w, g_left)?;
        }
        w
    };

    let induction = k.nat_induction(m, &template, &base, &step)?;

    // ── generalize to b <= c <= a ───────────────────────────────────
    let c = declare_int(k, "ba_c");
    let ct = v(k, c);
    let m_inst = isub(k, ct, bt);
    let inst = k.instantiate(&induction, &[(m, m_inst)])?;
    let bm = add(k, bt, m_inst);
    let bridge = k.arith_eq(bm, ct)?;
    let a_bm = model.fold(k, at, bm);
    let a_mb = model.fold(k, bm, bt);
    let s_ac = model.fold(k, at, ct);
    let s_cb = model.fold(k, ct, bt);
    // A(a, b+(c-b)) = A(a,c)
    let hole = fresh_hole_of(k, Sort::Int);
    let hv = v(k, hole);
    let t = model.fold(k, at, hv);
    let a_bm_eq = k.congr(Context { hole, template: t }, &bridge)?;
    // A(b+(c-b), b) = A(c,b)
    let hole = fresh_hole_of(k, Sort::Int);
    let hv = v(k, hole);
    let t = model.fold(k, hv, bt);
    let a_mb_eq = k.congr(Context { hole, template: t }, &bridge)?;
    // Rewrite the instantiated right-hand side with those boolean
    // equations.
    let hole = fresh_hole_of(k, Sort::Bool);
    let hv = v(k, hole);
    let t = and_of(k, hv, a_mb);
    let step_lhs = k.congr(Context { hole, template: t }, &a_bm_eq)?;
    let hole = fresh_hole_of(k, Sort::Bool);
    let hv = v(k, hole);
    let t = and_of(k, s_ac, hv);
    let step_rhs = k.congr(Context { hole, template: t }, &a_mb_eq)?;
    let _ = (a_bm, s_cb);
    let conv = k.trans(&step_lhs, &step_rhs)?;
    let final_eq = k.trans(&inst, &conv)?;
    let hyp_bc = k.arena_mut().le(bt, ct);
    let hyp_ca = k.arena_mut().le(ct, at);
    let w = k.weaken(&final_eq, &[hyp_bc, hyp_ca]);
    let h_bound = k.arena_mut().le(bm, at);
    let z = int(k, 0);
    let h_nonneg = k.arena_mut().le(z, m_inst);
    let w = if w.statement.hyps.contains(&h_bound) {
        k.discharge_arith(&w, h_bound)?
    } else {
        w
    };
    let w = if w.statement.hyps.contains(&h_nonneg) {
        k.discharge_arith(&w, h_nonneg)?
    } else {
        w
    };
    Ok(BoolSplitLaw {
        theorem: w,
        vars: (a, b, c),
    })
}

/// `i <= n ⊢ V_level(i,n) = A(n,i)` for the predicate kernel.
pub fn pred_loop_lemma(
    k: &mut Kernel,
    model: &Model,
    prog: &PredProgram,
    level: usize,
    sub: Option<&PredLemma>,
    chunk: Option<(&Symbol, &Theorem)>,
    tag: &str,
) -> Result<PredLemma, KernelError> {
    assert!(level == 1 || level == 16 || level == 32);
    let width = level as i128;
    let iscalar = declare_int(k, format!("{tag}_i"));
    let nvar = declare_int(k, format!("{tag}_n"));
    let rvar = declare_int(k, format!("{tag}_r"));
    let (it, nt, rt) = (v(k, iscalar), v(k, nvar), v(k, rvar));
    let one = int(k, 1);
    let r1 = add(k, rt, one);
    let i_plus_r = add(k, it, rt);
    let hyp_eq = k.arena_mut().int_eq(i_plus_r, nt);
    let mut run = prog.run(k, level, it, nt);
    let spec = model.fold(k, nt, it);
    let template = Statement::new(vec![hyp_eq], run, spec);
    let _ = &mut run;

    // ── base: P(0) ──────────────────────────────────────────────────
    let base = {
        let zero = int(k, 0);
        let i0 = add(k, it, zero);
        let hyp0 = k.arena_mut().int_eq(i0, nt);
        let (schema, is_scalar) = match level {
            1 => (prog.schemas.v1_base, true),
            _ => (prog.schemas.v1_base, false),
        };
        let s = if is_scalar {
            k.apply_schema(schema, &[(pred_i(k), it), (pred_n(k), nt)])?
        } else {
            let schema = match level {
                16 => prog.schemas.v16_base,
                _ => prog.schemas.v32_base,
            };
            k.apply_schema(schema, &[(pred_i(k), it), (pred_n(k), nt)])?
        };
        let guard = s.statement.hyps[0];
        // A(n,i) = true via A(i,i) = true.
        let h_hyp = k.hypothesis(hyp0)?;
        let i0_eq_i = k.arith_eq(i0, it)?;
        let i_eq_n = k.trans(&k.sym(&i0_eq_i), &h_hyp)?;
        let fold_base = k.apply_schema(model.schemas.base, &[(find(k, "fold_x"), it)])?;
        let fb_guard = fold_base.statement.hyps[0];
        let a_ii_true = k.discharge_arith(&fold_base, fb_guard)?;
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = model.fold(k, hv, it);
        let a_ii_eq = k.congr(Context { hole, template: t }, &i_eq_n)?;
        let a_ni_true = k.trans(&k.sym(&a_ii_eq), &a_ii_true)?;
        let target = if is_scalar {
            k.trans(&s, &k.sym(&a_ni_true))?
        } else {
            // vector base branch: use the sub lemma
            let sub = sub.expect("sub-lemma");
            let sub_inst = k.instantiate(&sub.theorem, &[(sub.vars.0, it), (sub.vars.1, nt)])?;
            let sub_hyp = sub_inst.statement.hyps[0];
            let mut w = k.weaken(&sub_inst, &[hyp0]);
            w = discharge_extras(k, &w, &[hyp0, sub_hyp])?;
            if w.statement.hyps.contains(&sub_hyp) {
                w = k.discharge_arith(&w, sub_hyp)?;
            }
            k.trans(&s, &w)?
        };
        let mut t = target;
        if t.statement.hyps.contains(&guard) {
            t = k.discharge_arith(&t, guard)?;
        }
        k.weaken(&t, &[hyp0])
    };

    // ── step ────────────────────────────────────────────────────────
    let cases: Vec<(Option<Tid>, Theorem)> = {
        let i_next = add(k, it, r1);
        let hyp_next = k.arena_mut().int_eq(i_next, nt);
        let zero = int(k, 0);
        let zero_le_r = k.arena_mut().le(zero, rt);
        if level == 1 {
            let e = model.elem(k, it);
            let not_e = k.arena_mut().bool_not(e);
            let one = int(k, 1);
            let i1 = add(k, it, one);
            let ih_stmt = k.instantiate_statement(&template, &[(iscalar, i1)])?;
            let ih_eq = k.equality_term(&ih_stmt)?;
            let ih = k.hypothesis(ih_eq)?;
            let a_ni1 = model.fold(k, nt, i1);
            // true case
            let case_true = {
                let s = k.apply_schema(
                    prog.schemas.v1_step,
                    &[(pred_i(k), it), (pred_n(k), nt)],
                )?;
                let guard = s.statement.hyps[0];
                let chain = k.trans(&s, &ih)?;
                let fs = k.apply_schema(
                    model.schemas.step_right,
                    &[
                        (find(k, "fold_x"), nt),
                        (find(k, "fold_y"), it),
                        (find(k, "fold_y1"), i1),
                    ],
                )?;
                let fs_guard = fs.statement.hyps[0];
                let lhs_simpl = and_of(k, a_ni1, e);
                let simpl = k.bitblast_implies(&[e], lhs_simpl, a_ni1)?;
                let t1 = k.trans(&fs, &simpl)?; // A(n,i) = A(n,i+1)
                let result = k.trans(&chain, &k.sym(&t1))?;
                let w = k.weaken(&result, &[hyp_next, zero_le_r, e]);
                let mut w = w;
                if w.statement.hyps.contains(&guard) {
                    w = k.discharge_arith(&w, guard)?;
                }
                if w.statement.hyps.contains(&fs_guard) {
                    w = k.discharge_arith(&w, fs_guard)?;
                }
                w
            };
            // false case
            let case_false = {
                let s = k.apply_schema(
                    prog.schemas.v1_exit,
                    &[(pred_i(k), it), (pred_n(k), nt)],
                )?;
                let guard = s.statement.hyps[0];
                let fs = k.apply_schema(
                    model.schemas.step_right,
                    &[
                        (find(k, "fold_x"), nt),
                        (find(k, "fold_y"), it),
                        (find(k, "fold_y1"), i1),
                    ],
                )?;
                let fs_guard = fs.statement.hyps[0];
                let lhs_simpl = and_of(k, a_ni1, e);
                let f = k.arena_mut().bool(false);
                let simpl = k.bitblast_implies(&[not_e], lhs_simpl, f)?;
                let t1 = k.trans(&fs, &simpl)?; // A(n,i) = false
                let result = k.trans(&s, &k.sym(&t1))?;
                let mut w = k.weaken(&result, &[hyp_next, zero_le_r, not_e]);
                if w.statement.hyps.contains(&guard) {
                    w = k.discharge_arith(&w, guard)?;
                }
                if w.statement.hyps.contains(&fs_guard) {
                    w = k.discharge_arith(&w, fs_guard)?;
                }
                w
            };
            let _ = ih;
            vec![(Some(e), case_true), (Some(not_e), case_false)]
        } else {
            let w = int(k, width);
            let iw = add(k, it, w);
            let guard_ge = k.arena_mut().le(iw, nt);
            let not_guard_ge = k.arena_mut().bool_not(guard_ge);
            let c = model.contribution(k, level, it);
            let not_c = k.arena_mut().bool_not(c);
            let r_step = {
                let d = int(k, width - 1);
                isub(k, rt, d)
            };
            let ih_stmt = k.instantiate_statement(
                &template,
                &[(iscalar, iw), (rvar, r_step)],
            )?;
            let ih_eq = k.equality_term(&ih_stmt)?;
            let ih = k.hypothesis(ih_eq)?;
            let a_niw = model.fold(k, nt, iw);
            // split law and chunk lemma instances
            let split = bool_split_law(k, model)?;
            let split_inst = k.instantiate(
                &split.theorem,
                &[(split.vars.0, nt), (split.vars.1, it), (split.vars.2, iw)],
            )?;
            let (chunk_start, chunk_thm) = chunk.expect("chunk lemma");
            let chunk_inst = k.instantiate(chunk_thm, &[(*chunk_start, it)])?;
            let a_iw_i = model.fold(k, iw, it);
            // A(n,i) = A(n,i+w) ∧ A(i+w,i) = A(n,i+w) ∧ C
            let hole = fresh_hole_of(k, Sort::Bool);
            let hv = v(k, hole);
            let t = and_of(k, a_niw, hv);
            let to_c = k.congr(Context { hole, template: t }, &k.sym(&chunk_inst))?;
            let split_to_c = k.trans(&split_inst, &to_c)?;
            // true case: V = V(i+w,n) = A(n,i+w) and A(n,i) = A(n,i+w)
            let case_true = {
                let schema = match level {
                    16 => prog.schemas.v16_step,
                    _ => prog.schemas.v32_step,
                };
                let s = k.apply_schema(schema, &[(pred_i(k), it), (pred_n(k), nt)])?;
                let guard = s.statement.hyps[0];
                let chain = k.trans(&s, &ih)?;
                let lhs_simpl = and_of(k, a_niw, c);
                let simpl = k.bitblast_implies(&[c], lhs_simpl, a_niw)?;
                let t1 = k.trans(&split_to_c, &simpl)?; // A(n,i) = A(n,i+w)
                let result = k.trans(&chain, &k.sym(&t1))?;
                let mut ww = k.weaken(&result, &[hyp_next, zero_le_r, guard_ge, c]);
                if ww.statement.hyps.contains(&guard) {
                    ww = k.discharge_arith(&ww, guard)?;
                }
                ww = discharge_extras(k, &ww, &[hyp_next, zero_le_r, guard_ge, c, ih_eq])?;
                ww
            };
            // false case: exit -> false, and A(n,i) = false
            let case_false_inner = {
                let schema = match level {
                    16 => prog.schemas.v16_exit,
                    _ => prog.schemas.v32_exit,
                };
                let s = k.apply_schema(schema, &[(pred_i(k), it), (pred_n(k), nt)])?;
                let guard = s.statement.hyps[0];
                let f = k.arena_mut().bool(false);
                let lhs_simpl = and_of(k, a_niw, c);
                let simpl = k.bitblast_implies(&[not_c], lhs_simpl, f)?;
                let t1 = k.trans(&split_to_c, &simpl)?; // A(n,i) = false
                let result = k.trans(&s, &k.sym(&t1))?;
                let mut ww = k.weaken(&result, &[hyp_next, zero_le_r, guard_ge, not_c]);
                if ww.statement.hyps.contains(&guard) {
                    ww = k.discharge_arith(&ww, guard)?;
                }
                ww = discharge_extras(k, &ww, &[hyp_next, zero_le_r, guard_ge, not_c, ih_eq])?;
                ww
            };
            let inner = k.case_split(c, &case_true, &case_false_inner)?;
            // base branch: the stage does not run
            let case_false = {
                let schema = match level {
                    16 => prog.schemas.v16_base,
                    _ => prog.schemas.v32_base,
                };
                let s = k.apply_schema(schema, &[(pred_i(k), it), (pred_n(k), nt)])?;
                let guard = s.statement.hyps[0];
                let sub = sub.expect("sub-lemma");
                let sub_inst =
                    k.instantiate(&sub.theorem, &[(sub.vars.0, it), (sub.vars.1, nt)])?;
                let sub_hyp = sub_inst.statement.hyps[0];
                let mut ww = k.weaken(&sub_inst, &[hyp_next, zero_le_r, not_guard_ge]);
                if ww.statement.hyps.contains(&sub_hyp) {
                    ww = k.discharge_arith(&ww, sub_hyp)?;
                }
                let chained = k.trans(&s, &ww)?;
                let mut out = chained;
                if out.statement.hyps.contains(&guard) {
                    out = k.discharge_arith(&out, guard)?;
                }
                out
            };
            let _ = (a_iw_i, ih_eq, ih);
            vec![
                (Some(guard_ge), inner),
                (Some(not_guard_ge), case_false),
            ]
        }
    };

    let induction = k.nat_induction_cases(rvar, &template, &base, &cases)?;
    // Generalize: r := n - i.
    let m_inst = isub(k, nt, it);
    let inst = k.instantiate(&induction, &[(rvar, m_inst)])?;
    let hyp_ge = k.arena_mut().le(it, nt);
    let t1 = k.weaken(&inst, &[hyp_ge]);
    let i_plus_m = add(k, it, m_inst);
    let h_eq = k.arena_mut().int_eq(i_plus_m, nt);
    let t2 = if t1.statement.hyps.contains(&h_eq) {
        k.discharge_arith(&t1, h_eq)?
    } else {
        t1
    };
    let z = int(k, 0);
    let h_nonneg = k.arena_mut().le(z, m_inst);
    let t3 = if t2.statement.hyps.contains(&h_nonneg) {
        k.discharge_arith(&t2, h_nonneg)?
    } else {
        t2
    };
    let theorem = k.weaken(&t3, &[hyp_ge]);
    Ok(PredLemma {
        theorem,
        vars: (iscalar, nvar),
    })
}

/// `0 <= n ⊢ V32(0,n) = A(n,0)`.
pub fn pred_end_to_end(
    k: &mut Kernel,
    model: &Model,
    w32: &PredLemma,
) -> Result<Theorem, KernelError> {
    let n_t = v(k, model.n);
    let zero = int(k, 0);
    k.instantiate(&w32.theorem, &[(w32.vars.0, zero), (w32.vars.1, n_t)])
}

fn pred_i(k: &Kernel) -> Symbol {
    find(k, "pred_i")
}

fn pred_n(k: &Kernel) -> Symbol {
    find(k, "pred_n")
}

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
