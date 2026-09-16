//! Requirement 7: memory and overflow semantics.
//!
//! Two families of lemmas, all mechanically checked:
//!
//! - **No over-read**: every address the generated code loads is inside
//!   the buffer. For a `w`-wide stage guarded by `i + w <= n`, all of
//!   `i, i+1, ..., i+w-1` are `< n`.
//! - **No wraparound**: the accumulator the C code keeps in `u64` never
//!   exceeds `u64::MAX`. The abstract model's accumulator is the
//!   mathematical sum `S`; the bound lemma shows
//!   `S(m,i) <= B * (m - i)` with `B = 1` for indicators and `B = 255`
//!   for bytes, so the precondition `B * n < 2^64` makes every
//!   intermediate value fit — the `u64` arithmetic in the artifact and
//!   the integer model agree.

use sir_mech::kernel::{Kernel, KernelError, Statement, Theorem};
use sir_mech::term::{Sort, Symbol, Tid};

use crate::loops::{add, declare_int, find, fresh_hole_of, int, sub, v};
use crate::model::Model;

/// `[i + w <= n] ⊢ (i < n) ∧ (i+1 < n) ∧ ... ∧ (i+w-1 < n) = true`.
pub fn memory_bounds_lemma(
    k: &mut Kernel,
    width: usize,
    tag: &str,
) -> Result<Theorem, KernelError> {
    let iv = declare_int(k, format!("{tag}_i"));
    let nv = declare_int(k, format!("{tag}_n"));
    let (it, nt) = (v(k, iv), v(k, nv));
    let w = int(k, width as i128);
    let iw = add(k, it, w);
    let hyp = k.arena_mut().le(iw, nt);
    // conjunction of i+k < n for k < width
    let mut conj = k.arena_mut().bool(true);
    for lane in 0..width {
        let off = int(k, lane as i128);
        let addr = add(k, it, off);
        let lt = k.arena_mut().lt(addr, nt);
        conj = k.arena_mut().bool_and(conj, lt);
    }
    k.arith_entail(&[hyp], conj)
}

/// `[i < n] ⊢ i < n = true` for the scalar tail (trivially the guard).
pub fn scalar_memory_bound(k: &mut Kernel, tag: &str) -> Result<Theorem, KernelError> {
    let iv = declare_int(k, format!("{tag}_i"));
    let nv = declare_int(k, format!("{tag}_n"));
    let (it, nt) = (v(k, iv), v(k, nv));
    let hyp = k.arena_mut().lt(it, nt);
    k.arith_entail(&[hyp], hyp)
}

/// The per-element value bound: indicators are `<= 1`, bytes are
/// `<= 255`. Proved by bit-blasting the conversion's width.
pub fn element_bound(k: &mut Kernel, model: &Model, tag: &str) -> Result<Theorem, KernelError> {
    let iv = declare_int(k, format!("{tag}_i"));
    let it = v(k, iv);
    let e = model.elem(k, it);
    let bound = match model.kind {
        crate::emitted::KernelKind::CardinalityMasked { .. } => 1,
        crate::emitted::KernelKind::SumAscii { .. } => 255,
        crate::emitted::KernelKind::AllEquality { .. } => {
            return Err(KernelError::Shape(
                "bounds: predicate kernel has no numeric element".into(),
            ))
        }
    };
    let b = int(k, bound);
    let g = k.arena_mut().le(e, b);
    let t = k.arena_mut().bool(true);
    k.bitblast_eq(g, t)
}

/// `[i + r = m] ⊢ S(m,i) <= B * r = true`, by induction on `r`.
pub struct BoundLemma {
    pub theorem: Theorem,
    /// (m, i) template variables; the conclusion is `S(m,i) <= B*(m-i)`.
    pub vars: (Symbol, Symbol),
    pub bound: i128,
}

pub fn accumulator_bound(
    k: &mut Kernel,
    model: &Model,
    tag: &str,
) -> Result<BoundLemma, KernelError> {
    let bound = match model.kind {
        crate::emitted::KernelKind::CardinalityMasked { .. } => 1,
        crate::emitted::KernelKind::SumAscii { .. } => 255,
        crate::emitted::KernelKind::AllEquality { .. } => {
            return Err(KernelError::Shape(
                "bounds: predicate kernel has no accumulator".into(),
            ))
        }
    };
    let b = int(k, bound);
    let mvar = declare_int(k, format!("{tag}_m"));
    let ivar = declare_int(k, format!("{tag}_i"));
    let rvar = declare_int(k, format!("{tag}_r"));
    let (mt, it, rt) = (v(k, mvar), v(k, ivar), v(k, rvar));
    let one = int(k, 1);
    let r1 = add(k, rt, one);
    let i_plus_r = add(k, it, rt);
    let hyp = k.arena_mut().int_eq(i_plus_r, mt);
    // conclusion: S(m,i) <= B*r
    let s = model.fold(k, mt, it);
    let br = k.arena_mut().mul(b, rt);
    let goal = k.arena_mut().le(s, br);
    let template = Statement::new(vec![hyp], goal, k.arena_mut().bool(true));
    let e_bound = element_bound(k, model, &format!("{tag}_e"))?;

    // ── base: r = 0 ─────────────────────────────────────────────────
    let base = {
        let zero = int(k, 0);
        let i0 = add(k, it, zero);
        let hyp0 = k.arena_mut().int_eq(i0, mt);
        // S(m,i) = 0
        let fold_base = k.apply_schema(model.schemas.base, &[(find(k, "fold_x"), it)])?;
        let fb_guard = fold_base.statement.hyps[0];
        let s_ii_zero = k.discharge_arith(&fold_base, fb_guard)?;
        let h_hyp = k.hypothesis(hyp0)?;
        let i0_eq_i = k.arith_eq(i0, it)?;
        let i_eq_m = k.trans(&k.sym(&i0_eq_i), &h_hyp)?;
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = model.fold(k, hv, it);
        let s_mi_eq_s_ii = k.congr(sir_mech::kernel::Context { hole, template: t }, &i_eq_m)?;
        // congr gives S(i,i) = S(m,i); flip to get S(m,i) = S(i,i) = 0.
        let mut s_mi_zero = k.trans(&k.sym(&s_mi_eq_s_ii), &s_ii_zero)?;
        s_mi_zero = k.weaken(&s_mi_zero, &[hyp0]);

        // 0 <= B*0, then rewrite 0 -> S(m,i).
        let zero_le = {
            let z = int(k, 0);
            let bz = k.arena_mut().mul(b, z);
            let g = k.arena_mut().le(z, bz);
            k.arith_entail(&[], g)?
        };
        let bz0 = {
            let z = int(k, 0);
            k.arena_mut().mul(b, z)
        };
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = k.arena_mut().le(hv, bz0);
        let rewritten = k.congr(sir_mech::kernel::Context { hole, template: t }, &s_mi_zero)?;
        let out = k.trans(&rewritten, &zero_le)?;
        k.weaken(&out, &[hyp0])
    };

    // Step: r -> r+1
    // ── step: r → r+1 ───────────────────────────────────────────────
    let step = {
        let s_mi = model.fold(k, mt, it);
        let one = int(k, 1);
        let i1 = add(k, it, one);
        let s_mi1 = model.fold(k, mt, i1);
        let br_r = k.arena_mut().mul(b, rt);
        let ih_goal = k.arena_mut().le(s_mi1, br_r);
        let tr = k.arena_mut().bool(true);
        let ih_eq = k.arena_mut().bool_eq(ih_goal, tr);
        let ih = k.hypothesis(ih_eq)?;
        let e_i = model.elem(k, it);
        // fold step: S(m,i) = S(m,i+1) + E(i)
        let fs = k.apply_schema(
            model.schemas.step_right,
            &[
                (find(k, "fold_x"), mt),
                (find(k, "fold_y"), it),
                (find(k, "fold_y1"), i1),
            ],
        )?;
        let fs_guard = fs.statement.hyps[0];
        let sum_term = add(k, s_mi1, e_i);
        let r1v = add(k, rt, one);
        let br_r1 = k.arena_mut().mul(b, r1v);
        // the template's goal at r+1: S(m,i) <= B*(r+1)
        let template_goal = k.arena_mut().le(s_mi, br_r1);
        // rewrite its left side with the fold equation
        let hole = fresh_hole_of(k, Sort::Int);
        let hv = v(k, hole);
        let t = k.arena_mut().le(hv, br_r1);
        let goal_eq = k.congr(sir_mech::kernel::Context { hole, template: t }, &fs)?;
        // element bound for this i: E(i) <= B
        let g_e = k.arena_mut().le(e_i, b);
        let tr = k.arena_mut().bool(true);
        let e_bound_here = k.bitblast_eq(g_e, tr)?;
        // entail: S(m,i+1) <= B*r and E(i) <= B  ⊢  S(m,i+1)+E(i) <= B*(r+1)
        let target = k.arena_mut().le(sum_term, br_r1);
        let hyps = vec![ih_eq, g_e];
        let entail = k.arith_entail(&hyps, target)?;
        // goal_eq: template_goal = (sum_term <= B*(r+1)); entail: the
        // latter = true; chain gives the template goal.
        let combined = k.trans(&goal_eq, &entail)?;
        let i_plus_r1 = add(k, it, r1v);
        let hyp_next = k.arena_mut().int_eq(i_plus_r1, mt);
        let z = int(k, 0);
        let zero_le_r = k.arena_mut().le(z, rt);
        let mut w = k.weaken(&combined, &[hyp_next, zero_le_r]);
        if w.statement.hyps.contains(&fs_guard) {
            w = k.discharge_arith(&w, fs_guard)?;
        }
        // the element bound is a lemma, not an assumption
        if w.statement.hyps.contains(&g_e) {
            w = k.discharge_lemma(&w, g_e, &e_bound_here)?;
        }
        let _ = (ih, template_goal);
        w
    };

    let induction = k.nat_induction(rvar, &template, &base, &step)?;
    // Generalize: r := m - i.
    let m_inst = sub(k, mt, it);
    let inst = k.instantiate(&induction, &[(rvar, m_inst)])?;
    let hyp_ge = k.arena_mut().le(it, mt);
    let t1 = k.weaken(&inst, &[hyp_ge]);
    let i_plus_m = add(k, it, m_inst);
    let h_eq = k.arena_mut().int_eq(i_plus_m, mt);
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
    Ok(BoundLemma {
        theorem,
        vars: (mvar, ivar),
        bound,
    })
}

/// `[0 <= n, B*n <= 2^64 - 1] ⊢ S(n,0) <= 2^64 - 1 = true`.
///
/// This is the no-wraparound fact: with the accumulator bounded by the
/// total, the `u64` arithmetic in the artifact never wraps, so the
/// integer model and the emitted code agree on every intermediate
/// value.
pub fn no_overflow_lemma(
    k: &mut Kernel,
    model: &Model,
    bound: &BoundLemma,
) -> Result<Theorem, KernelError> {
    let n_t = v(k, model.n);
    let zero = int(k, 0);
    let b = int(k, bound.bound);
    let inst = k.instantiate(
        &bound.theorem,
        &[(bound.vars.0, n_t), (bound.vars.1, zero)],
    )?;
    let nonneg = inst.statement.hyps[0]; // 0 <= n
    let bound_term = inst.statement.lhs; // S(n,0) <= B*(n-0)

    let two64 = {
        // 2^64 - 1 as an integer constant.
        k.arena_mut().int((1i128 << 64) - 1)
    };
    let bn = k.arena_mut().mul(b, n_t);
    let precond = k.arena_mut().le(bn, two64);
    let s_n0 = model.fold(k, n_t, zero);
    let goal = k.arena_mut().le(s_n0, two64);

    let widened = k.weaken(&inst, &[precond]);
    let entail = k.arith_entail(&[bound_term, precond], goal)?;
    let mut out = k.weaken(&entail, &[nonneg, precond]);
    if out.statement.hyps.contains(&bound_term) {
        out = k.discharge_lemma(&out, bound_term, &inst)?;
    }
    Ok(out)
}
