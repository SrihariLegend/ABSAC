//! The Gate 4B proof itself, built on the [`sir_mech`] kernel.
//!
//! Per kernel the proof has this shape:
//!
//! ```text
//!   chunk lemma (width w, symbolic start i)
//!       contribution(w, i) = S(i + w, i)
//!
//!       = [bitblast]  the SDM-modeled intrinsic sequence computes the
//!                     same value as the scalar element fold over the
//!                     block, for *every* block content
//!       ∘ [unfold]    the guarded fold recursions expand S(i+w, i) to
//!                     the element sum, with each guard discharged by
//!                     linear arithmetic
//! ```

use sir_mech::kernel::{Context, Kernel, KernelError, Theorem};
use sir_mech::term::{Sort, Symbol, Tid};

use crate::emitted::{EmittedKernel, KernelKind};
use crate::model::Model;

/// One proved lemma, with a replayable derivation.
#[derive(Clone, Debug)]
pub struct LemmaEntry {
    pub name: String,
    pub statement: String,
    pub theorem: Theorem,
}

/// The Gate 4B proof for one generated kernel.
#[derive(Clone, Debug)]
pub struct Gate4bProof {
    pub kernel: EmittedKernel,
    pub model: Model,
    pub kernel_state: Kernel,
    pub lemmas: Vec<LemmaEntry>,
    pub notes: Vec<String>,
    /// Requirement 1: the pipeline binding of this artifact to its SIR
    /// region (present when the proof was checked with the LLVM IR).
    pub binding: Option<crate::binding::SirBinding>,
}

impl Gate4bProof {
    /// Parse a generated kernel from disk and discharge the per-chunk
    /// identities against the actual instruction sequence.
    pub fn check_file(path: impl AsRef<std::path::Path>) -> Result<Gate4bProof, String> {
        let kernel = crate::emitted::parse_file(path).map_err(|e| e.to_string())?;
        Self::check_kernel(kernel)
    }

    /// Check a generated artifact *and* bind it to the SIR region the
    /// real pipeline derives it from.
    pub fn check_file_with_ll(
        path: impl AsRef<std::path::Path>,
        ll_path: impl AsRef<std::path::Path>,
    ) -> Result<Gate4bProof, String> {
        let path = path.as_ref();
        let ll_path = ll_path.as_ref();
        let generated = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let ll_text = std::fs::read_to_string(ll_path).map_err(|e| e.to_string())?;
        let kernel = crate::emitted::parse(path.display().to_string().as_str(), &generated)
            .map_err(|e| e.to_string())?;
        let binding = crate::binding::bind(&kernel, &generated, ll_path, &ll_text)?;
        if !binding.all_ok() {
            let failed: Vec<String> = binding
                .checks
                .iter()
                .filter(|(_, ok)| !ok)
                .map(|(name, _)| name.clone())
                .collect();
            return Err(format!("SIR binding checks failed: {failed:?}"));
        }
        let mut proof = Self::check_kernel(kernel)?;
        proof.binding = Some(binding);
        Ok(proof)
    }

    pub fn check_kernel(kernel: EmittedKernel) -> Result<Gate4bProof, String> {
        let mut k = Kernel::new();
        let model = Model::build(&kernel, &mut k).map_err(|e| e.to_string())?;
        let mut lemmas = Vec::new();
        let mut notes = Vec::new();
        let mut chunk_by_width: Vec<(usize, Symbol, Theorem)> = Vec::new();

        // Chunk lemmas for the vector widths the code actually has.
        let widths: &[usize] = match model.kind {
            KernelKind::CardinalityMasked { .. } | KernelKind::AllEquality { .. } => &[32, 16, 1],
            KernelKind::SumAscii { .. } => &[32, 16, 1],
        };
        for &width in widths {
            let sym = k
                .arena_mut()
                .declare_var(format!("blk{width}"), Sort::Int);
            let start = k.arena_mut().var(sym);
            let thm = match chunk_lemma(&mut k, &model, width, start) {
                Ok(t) => t,
                Err(e) => {
                    notes.push(format!("chunk{width}: NOT PROVED: {e}"));
                    continue;
                }
            };
            if let Err(e) = k.replay(&thm) {
                return Err(format!("chunk{width} replay failed: {e}"));
            }
            lemmas.push(LemmaEntry {
                name: format!("chunk_lemma_{width}"),
                statement: k.describe(&thm.statement),
                theorem: thm,
            });
            chunk_by_width.push((width, sym, lemmas.last().unwrap().theorem.clone()));
        }

        // Loop lemmas for the Int-valued kernels (cardinality/sum).
        if model.fold_int.is_some() {
            let prog = crate::loops::Program::build(&mut k, &model).map_err(|e| e.to_string())?;
            let w1 = crate::loops::loop_lemma(&mut k, &model, &prog, 1, None, None, "l1")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "loop_w1", &w1.theorem);
            let chunk16 = chunk_by_width
                .iter()
                .find(|(w, _, _)| *w == 16)
                .map(|(_, s, t)| (*s, t.clone()));
            let w16 = match chunk16 {
                Some((sym, thm)) => {
                    crate::loops::loop_lemma(&mut k, &model, &prog, 16, Some(&w1), Some((&sym, &thm)), "l16")
                        .map_err(|e| e.to_string())?
                }
                None => return Err("missing 16-byte chunk lemma".into()),
            };
            record_loop_lemma(&k, &mut lemmas, "loop_w16", &w16.theorem);
            let chunk32 = chunk_by_width
                .iter()
                .find(|(w, _, _)| *w == 32)
                .map(|(_, s, t)| (*s, t.clone()));
            let w32 = match chunk32 {
                Some((sym, thm)) => {
                    crate::loops::loop_lemma(&mut k, &model, &prog, 32, Some(&w16), Some((&sym, &thm)), "l32")
                        .map_err(|e| e.to_string())?
                }
                None => return Err("missing 32-byte chunk lemma".into()),
            };
            record_loop_lemma(&k, &mut lemmas, "loop_w32", &w32.theorem);
            let e2e = crate::loops::end_to_end(&mut k, &model, &w32).map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "end_to_end", &e2e);

            // Requirement 7: memory safety and no wraparound.
            let mem32 = crate::bounds::memory_bounds_lemma(&mut k, 32, "mem32")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bounds_32", &mem32);
            let mem16 = crate::bounds::memory_bounds_lemma(&mut k, 16, "mem16")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bounds_16", &mem16);
            let mem1 = crate::bounds::scalar_memory_bound(&mut k, "mem1")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bound_scalar", &mem1);
            let bound =
                crate::bounds::accumulator_bound(&mut k, &model, "bnd").map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "accumulator_bound", &bound.theorem);
            let no_wrap = crate::bounds::no_overflow_lemma(&mut k, &model, &bound)
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "no_overflow", &no_wrap);
        } else {
            // Predicate (All) kernel: early-exit program semantics.
            let prog = crate::pred::build(&mut k, &model).map_err(|e| e.to_string())?;
            let v1 = crate::pred::pred_loop_lemma(&mut k, &model, &prog, 1, None, None, "p1")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "loop_v1", &v1.theorem);
            let chunk16 = chunk_by_width
                .iter()
                .find(|(w, _, _)| *w == 16)
                .map(|(_, s, t)| (*s, t.clone()));
            let chunk32 = chunk_by_width
                .iter()
                .find(|(w, _, _)| *w == 32)
                .map(|(_, s, t)| (*s, t.clone()));
            let v16 = match chunk16 {
                Some((sym, thm)) => crate::pred::pred_loop_lemma(
                    &mut k,
                    &model,
                    &prog,
                    16,
                    Some(&v1),
                    Some((&sym, &thm)),
                    "p16",
                )
                .map_err(|e| e.to_string())?,
                None => return Err("missing 16-byte chunk lemma".into()),
            };
            record_loop_lemma(&k, &mut lemmas, "loop_v16", &v16.theorem);
            let v32 = match chunk32 {
                Some((sym, thm)) => crate::pred::pred_loop_lemma(
                    &mut k,
                    &model,
                    &prog,
                    32,
                    Some(&v16),
                    Some((&sym, &thm)),
                    "p32",
                )
                .map_err(|e| e.to_string())?,
                None => return Err("missing 32-byte chunk lemma".into()),
            };
            record_loop_lemma(&k, &mut lemmas, "loop_v32", &v32.theorem);
            let e2e = crate::pred::pred_end_to_end(&mut k, &model, &v32)
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "end_to_end", &e2e);
            // Requirement 7 for the predicate kernel: reads stay in
            // bounds (there is no accumulator to overflow).
            let mem32 = crate::bounds::memory_bounds_lemma(&mut k, 32, "pmem32")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bounds_32", &mem32);
            let mem16 = crate::bounds::memory_bounds_lemma(&mut k, 16, "pmem16")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bounds_16", &mem16);
            let mem1 = crate::bounds::scalar_memory_bound(&mut k, "pmem1")
                .map_err(|e| e.to_string())?;
            record_loop_lemma(&k, &mut lemmas, "memory_bound_scalar", &mem1);
        }

        Ok(Gate4bProof {
            kernel,
            model,
            kernel_state: k,
            lemmas,
            notes,
            binding: None,
        })
    }

    pub fn replay_all(&mut self) -> Result<(), String> {
        for lemma in &self.lemmas {
            self.kernel_state
                .replay(&lemma.theorem)
                .map_err(|e| format!("{}: {e}", lemma.name))?;
        }
        Ok(())
    }
}

fn record_loop_lemma(
    k: &Kernel,
    lemmas: &mut Vec<LemmaEntry>,
    name: &str,
    thm: &Theorem,
) {
    lemmas.push(LemmaEntry {
        name: name.to_string(),
        statement: k.describe(&thm.statement),
        theorem: thm.clone(),
    });
}

/// The per-chunk identity: the SDM-modeled vector block contribution
/// equals the scalar element fold over the same block.
pub fn chunk_lemma(
    k: &mut Kernel,
    model: &Model,
    width: usize,
    start: Tid,
) -> Result<Theorem, KernelError> {
    assert!(width == 32 || width == 16 || width == 1);
    let contribution = model.contribution(k, width, start);
    let canonical = canonical_sum(k, model, start, width);
    // identity: contribution = canonical sum, by bit-blasting the SDM
    // models against the scalar element terms.
    let identity = match model.kind {
        KernelKind::SumAscii { .. } => sum_bridge(k, model, width, start, canonical)?,
        _ => k.bitblast_eq(contribution, canonical)?,
    };

    // Unfold S(start + width, start) into the canonical sum. The
    // recursion peels elements from the bottom, which produces a
    // decreasing-order sum; arithmetic reassociates it.
    let (unfolded, descending) = unfold_fold(k, model, start, width)?;
    // Reassociation is arithmetic for the sum folds and propositional
    // for the boolean fold.
    let reassoc = if model.fold_int.is_some() {
        k.arith_eq(descending, canonical)?
    } else {
        k.bitblast_eq(descending, canonical)?
    };
    let folded = k.trans(&unfolded, &reassoc)?;
    // fold(start + width, start) is written with the literal `start`
    // as the lower bound; the unfolding uses `start + 0`.
    let t0 = {
        let zero = k.arena_mut().int(0);
        k.arena_mut().add(start, zero)
    };
    let bridge = k.arith_eq(t0, start)?;
    let top = {
        let off = k.arena_mut().int(width as i128);
        k.arena_mut().add(start, off)
    };
    let hole = fresh_hole(k, Sort::Int);
    let hv = k.arena_mut().var(hole);
    let template = model.fold(k, top, hv);
    let lower_bridge = k.congr(Context { hole, template }, &bridge)?;
    let folded = k.trans(&k.sym(&lower_bridge), &folded)?;

    // contribution = fold(start + width, start)
    let sym = k.sym(&folded);
    k.trans(&identity, &sym)
}

/// `0 + elem(t_0) + elem(t_1) + ... + elem(t_{w-1})`, left-nested in
/// increasing order — the shape the bit-blasted identity talks about.
fn canonical_sum(k: &mut Kernel, model: &Model, start: Tid, width: usize) -> Tid {
    let is_int = model.fold_int.is_some();
    let mut acc = if is_int {
        k.arena_mut().int(0)
    } else {
        k.arena_mut().bool(true)
    };
    for j in 0..width {
        let off = k.arena_mut().int(j as i128);
        let idx = k.arena_mut().add(start, off);
        let e = model.elem(k, idx);
        acc = if is_int {
            k.arena_mut().add(acc, e)
        } else {
            k.arena_mut().bool_and(acc, e)
        };
    }
    acc
}

/// `Sum` kernels: prove `contribution = element_sum` compositionally.
///
/// Each byte contributes `bv_to_int(zero_ext_64(b))`, which equals
/// `bv_to_int(b)` (a trivial per-byte bitvector identity), and the two
/// sides are then the same left-nested sum of the same atoms, so
/// linear arithmetic closes the gap.
fn sum_bridge(
    k: &mut Kernel,
    model: &Model,
    width: usize,
    start: Tid,
    element_sum: Tid,
) -> Result<Theorem, KernelError> {
    let pairs = model.sum_leaf_pairs(k, width, start);
    let zero = k.arena_mut().int(0);
    let mut acc_wide = zero;
    let mut acc_plain = zero;
    let mut current = k.refl(zero);
    for (wide, plain) in pairs {
        let lemma = k.bitblast_eq(wide, plain)?; // wide = plain
        // acc_wide + wide = acc_plain + wide
        let hole = fresh_hole(k, Sort::Int);
        let hv = k.arena_mut().var(hole);
        let template = k.arena_mut().add(hv, wide);
        let left = k.congr(Context { hole, template }, &current)?;
        // acc_plain + wide = acc_plain + plain
        let hole = fresh_hole(k, Sort::Int);
        let hv = k.arena_mut().var(hole);
        let template = k.arena_mut().add(acc_plain, hv);
        let right = k.congr(Context { hole, template }, &lemma)?;
        current = k.trans(&left, &right)?;
        acc_wide = k.arena_mut().add(acc_wide, wide);
        acc_plain = k.arena_mut().add(acc_plain, plain);
    }
    // `current` proves contribution = Σ plain leaves; the fold's
    // element sum is the same term.
    let _ = element_sum;
    assert_eq!(current.statement.lhs, acc_wide);
    assert_eq!(current.statement.rhs, element_sum);
    Ok(current)
}

/// Unfold `S(start + width, start)` into the explicit element sum
/// (`A` conjunction for the `All` kernel). All recurrence guards are
/// discharged by linear arithmetic, so the returned theorem is
/// unconditional.
fn unfold_fold(
    k: &mut Kernel,
    model: &Model,
    start: Tid,
    width: usize,
) -> Result<(Theorem, Tid), KernelError> {
    // t_j = start + j
    let t: Vec<Tid> = (0..=width)
        .map(|j| {
            let c = k.arena_mut().int(j as i128);
            k.arena_mut().add(start, c)
        })
        .collect();
    // Base at the top: S(start + width, start + width) = 0 (or
    // A(...) = true).
    let fold_x = find(k, "fold_x");
    let fold_y = find(k, "fold_y");
    let fold_y1 = find(k, "fold_y1");
    let base_raw = k.apply_schema(model.schemas.base, &[(fold_x, t[width])])?;
    let mut pending_guards: Vec<Tid> = Vec::new();
    pending_guards.push(base_raw.statement.hyps[0]);
    let mut current = base_raw;

    // Peel the lowest element repeatedly:
    //   S(x, y) = S(x, y+1) + elem(y).
    let hole_sort = if model.fold_int.is_some() {
        Sort::Int
    } else {
        Sort::Bool
    };
    for j in (0..width).rev() {
        let step = k.apply_schema(
            model.schemas.step_right,
            &[
                (fold_x, t[width]),
                (fold_y, t[j]),
                (fold_y1, t[j + 1]),
            ],
        )?;
        let guard = step.statement.hyps[0];
        pending_guards.push(guard);
        // current: S(t_w, t_{j+1}) = rhs
        // congr (hole + elem(t_j)): S(t_w,t_{j+1}) + elem(t_j) = rhs + elem(t_j)
        // trans with the schema: S(t_w, t_j) = rhs + elem(t_j)
        let elem = model.elem(k, t[j]);
        let hole = fresh_hole(k, hole_sort);
        let hv = k.arena_mut().var(hole);
        let template = if model.fold_int.is_some() {
            k.arena_mut().add(hv, elem)
        } else {
            k.arena_mut().bool_and(hv, elem)
        };
        let expanded = k.congr(Context { hole, template }, &current)?;
        current = k.trans(&step, &expanded)?;
    }

    // Discharge the recurrence guards: each is a linear identity in the
    // index terms (t_{j+1} = t_j + 1 and t_j < start + width).
    for guard in pending_guards {
        if current.statement.hyps.contains(&guard) {
            current = k.discharge_arith(&current, guard)?;
        }
    }
    let sum = current.statement.rhs;
    Ok((current, sum))
}

fn fresh_hole(k: &mut Kernel, sort: Sort) -> Symbol {
    let name = format!("hole{}", k.arena().symbol_count());
    k.arena_mut().declare_var(name, sort)
}

fn find(k: &Kernel, name: &str) -> Symbol {
    for i in 0..k.arena().symbol_count() as u32 {
        let s = Symbol(i);
        if k.arena().var_name(s) == name {
            return s;
        }
    }
    panic!("symbol {name} not found")
}
