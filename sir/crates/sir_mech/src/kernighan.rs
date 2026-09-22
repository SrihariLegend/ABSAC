//! Kernel induction proof for the Kernighan popcount recurrence.
//!
//! `K` is the loop's functional equation:
//!
//! ```text
//!   K(0) = 0
//!   x != 0  =>  K(x) = 1 + K(x & (x - 1))
//! ```
//!
//! The example proves `K(x) = popcount(x)` by strong induction on the
//! integer value of `x`, then replays the derivation. Widths above 32
//! exceed the 64-bit integer shadow used for measure checks and are not
//! attempted yet.
//!
//! The proof covers widths up to 32; wider widths need a >64-bit shadow
//! for the integer measure and are still contract-backed.

use crate::kernel::{Context, Kernel, KernelError, SchemaId, Statement, Theorem};
use crate::term::{BvOp, FuncId, Sort, Symbol, Tid};

struct Env {
    k: Kernel,
    width: u32,
    x: Symbol,
    v: Symbol,
    slot: Symbol,
    kfun: FuncId,
    zero_schema: SchemaId,
    step_schema: SchemaId,
    roundtrip_param: Symbol,
    roundtrip_schema: SchemaId,
}

fn fresh(k: &mut Kernel, name: &str, sort: Sort) -> Symbol {
    k.arena_mut().declare_var(name, sort)
}

fn bv_zero(k: &mut Kernel, width: u32) -> Tid {
    k.arena_mut().bv(0, width)
}

fn bv_one(k: &mut Kernel, width: u32) -> Tid {
    k.arena_mut().bv(1, width)
}

fn app_k(k: &mut Kernel, kfun: FuncId, arg: Tid) -> Tid {
    k.arena_mut().app(kfun, &[arg])
}

fn pc(k: &mut Kernel, arg: Tid) -> Tid {
    let count = k.arena_mut().bv_popcount(arg);
    k.arena_mut().bv_to_int(count)
}

impl Env {
    fn new(width: u32) -> Self {
        let mut k = Kernel::new();
        let x = fresh(&mut k, "x", Sort::Bv(width));
        let v = fresh(&mut k, "v", Sort::Int);
        let slot = fresh(&mut k, "slot", Sort::Bv(width));
        let kfun = k
            .arena_mut()
            .declare_func("K", vec![Sort::Bv(width)], Sort::Int);
        let zero_schema = {
            let ar = k.arena_mut();
            let zero = ar.bv(0, width);
            let lhs = ar.app(kfun, &[zero]);
            let rhs = ar.int(0);
            k.schema("K(0)=0", &[], lhs, rhs)
        };
        let step_schema = {
            let ar = k.arena_mut();
            let zero = ar.bv(0, width);
            let one = ar.bv(1, width);
            let xv = ar.var(slot);
            let decremented = ar.bv_bin(BvOp::Sub, xv, one);
            let cleared = ar.bv_bin(BvOp::And, xv, decremented);
            let is_zero = ar.bv_bin(BvOp::Eq, xv, zero);
            let guard = ar.bool_not(is_zero);
            let lhs = ar.app(kfun, &[xv]);
            let rec = ar.app(kfun, &[cleared]);
            let one_int = ar.int(1);
            let rhs = ar.add(one_int, rec);
            k.schema_guarded("K(x)=1+K(x&(x-1))", &[slot], Some(guard), lhs, rhs)
        };
        let roundtrip_param = fresh(&mut k, "roundtrip", Sort::Bv(width));
        let roundtrip_schema = {
            let ar = k.arena_mut();
            let bv = ar.var(roundtrip_param);
            let as_int = ar.bv_to_int(bv);
            let back = ar.int_to_bv(as_int, width);
            k.schema("bv_to_int_roundtrip", &[roundtrip_param], back, bv)
        };
        Env {
            k,
            width,
            x,
            v,
            slot,
            kfun,
            zero_schema,
            step_schema,
            roundtrip_param,
            roundtrip_schema,
        }
    }

    fn upper(&mut self) -> Tid {
        let value = (1i128 << self.width) - 1;
        self.k.arena_mut().int(value)
    }

    fn template(&mut self) -> Statement {
        let ar = self.k.arena_mut();
        let zero = ar.int(0);
        let vv = ar.var(self.v);
        let upper = ar.int((1i128 << self.width) - 1);
        let bound_lo = ar.le(zero, vv);
        let bound_hi = ar.le(vv, upper);
        let image = ar.int_to_bv(vv, self.width);
        let lhs = ar.app(self.kfun, &[image]);
        let count = ar.bv_popcount(image);
        let rhs = ar.bv_to_int(count);
        Statement::new(vec![bound_lo, bound_hi], lhs, rhs)
    }

    fn base(&mut self) -> Result<Theorem, KernelError> {
        let zero = self.k.arena_mut().int(0);
        let image = self.k.arena_mut().int_to_bv(zero, self.width);
        let zero_bv = self.k.arena_mut().bv(0, self.width);
        let schema = self.k.apply_schema(self.zero_schema, &[])?;
        let roundtrip = self.k.bitblast_eq(image, zero_bv)?;
        let hole = fresh(&mut self.k, "base_hole", Sort::Bv(self.width));
        let hole_term = self.k.arena_mut().var(hole);
        let k_hole = self.k.arena_mut().app(self.kfun, &[hole_term]);
        let rewritten = self.k.congr(
            Context {
                hole,
                template: k_hole,
            },
            &roundtrip,
        )?;
        let k_image_zero = self.k.trans(&rewritten, &schema)?;
        let pc_image = pc(&mut self.k, image);
        let pc_zero = self.k.bitblast_eq(pc_image, zero)?;
        let body = self.k.trans(&k_image_zero, &self.k.sym(&pc_zero))?;
        let bound_lo = {
            let ar = self.k.arena_mut();
            let z = ar.int(0);
            ar.le(z, z)
        };
        let bound_hi = {
            let ar = self.k.arena_mut();
            let z = ar.int(0);
            let upper = ar.int((1i128 << self.width) - 1);
            ar.le(z, upper)
        };
        Ok(self.k.weaken(&body, &[bound_lo, bound_hi]))
    }

    fn step_nonzero(&mut self) -> Result<Theorem, KernelError> {
        let one = self.k.arena_mut().int(1);
        let vv = self.k.arena_mut().var(self.v);
        let next = self.k.arena_mut().add(vv, one);
        let (bound_lo, bound_hi) = next_bounds(&mut self.k, self.v, self.width);
        let image = self.k.arena_mut().int_to_bv(next, self.width);
        let zero_bv = bv_zero(&mut self.k, self.width);
        let one_bv = bv_one(&mut self.k, self.width);
        let decremented = self.k.arena_mut().bv_bin(BvOp::Sub, image, one_bv);
        let cleared = self.k.arena_mut().bv_bin(BvOp::And, image, decremented);
        let is_zero = self.k.arena_mut().bv_bin(BvOp::Eq, image, zero_bv);
        let neq = self.k.arena_mut().bool_not(is_zero);
        let remaining = self.k.arena_mut().bv_to_int(cleared);
        let shadow_image = self.k.arena_mut().int_to_bv(remaining, self.width);
        let ih_lhs = app_k(&mut self.k, self.kfun, shadow_image);
        let ih_rhs = pc(&mut self.k, shadow_image);
        let ih_eq = self.k.arena_mut().int_eq(ih_lhs, ih_rhs);
        let ih = self.k.hypothesis(ih_eq)?;
        let roundtrip = self
            .k
            .apply_schema(self.roundtrip_schema, &[(self.roundtrip_param, cleared)])?;
        let hole_bv = fresh(&mut self.k, "ih_bv", Sort::Bv(self.width));
        let hole_bv_term = self.k.arena_mut().var(hole_bv);
        let k_hole = self.k.arena_mut().app(self.kfun, &[hole_bv_term]);
        let lhs_rewrite = self.k.congr(
            Context {
                hole: hole_bv,
                template: k_hole,
            },
            &roundtrip,
        )?;
        let lhs_rewrite = self.k.sym(&lhs_rewrite);
        let k_cleared = self.k.trans(&lhs_rewrite, &ih)?;
        let pc_hole = pc(&mut self.k, hole_bv_term);
        let rhs_rewrite = self.k.congr(
            Context {
                hole: hole_bv,
                template: pc_hole,
            },
            &roundtrip,
        )?;
        let ih_cleared = self.k.trans(&k_cleared, &rhs_rewrite)?;
        let step = self
            .k
            .apply_schema(self.step_schema, &[(self.slot, image)])?;
        let pc_image = pc(&mut self.k, image);
        let pc_cleared = pc(&mut self.k, cleared);
        let one_int = self.k.arena_mut().int(1);
        let pc_step_rhs = self.k.arena_mut().add(one_int, pc_cleared);
        let pc_step = self
            .k
            .bitblast_implies(&[neq, bound_lo, bound_hi], pc_image, pc_step_rhs)?;
        let hole_int = fresh(&mut self.k, "ih_int", Sort::Int);
        let hole_int_term = self.k.arena_mut().var(hole_int);
        let one_int = self.k.arena_mut().int(1);
        let plus_hole = self.k.arena_mut().add(one_int, hole_int_term);
        let rec_rewrite = self.k.congr(
            Context {
                hole: hole_int,
                template: plus_hole,
            },
            &ih_cleared,
        )?;
        let k_image = self.k.trans(&step, &rec_rewrite)?;
        let body = self.k.trans(&k_image, &self.k.sym(&pc_step))?;
        Ok(self.k.weaken(&body, &[bound_lo, bound_hi]))
    }

    fn step_zero(&mut self) -> Result<Theorem, KernelError> {
        let one = self.k.arena_mut().int(1);
        let vv = self.k.arena_mut().var(self.v);
        let next = self.k.arena_mut().add(vv, one);
        let (bound_lo, bound_hi) = next_bounds(&mut self.k, self.v, self.width);
        let image = self.k.arena_mut().int_to_bv(next, self.width);
        let zero_bv = bv_zero(&mut self.k, self.width);
        let is_zero = self.k.arena_mut().bv_bin(BvOp::Eq, image, zero_bv);
        let neq = self.k.arena_mut().bool_not(is_zero);
        let is_zero = self.k.arena_mut().bool_not(neq);
        let zero_schema = self.k.apply_schema(self.zero_schema, &[])?;
        let image_is_zero =
            self.k
                .bitblast_implies(&[is_zero, bound_lo, bound_hi], image, zero_bv)?;
        let hole = fresh(&mut self.k, "zero_hole", Sort::Bv(self.width));
        let hole_term = self.k.arena_mut().var(hole);
        let k_hole = self.k.arena_mut().app(self.kfun, &[hole_term]);
        let lhs_rewrite = self.k.congr(
            Context {
                hole,
                template: k_hole,
            },
            &image_is_zero,
        )?;
        let k_image_zero = self.k.trans(&lhs_rewrite, &zero_schema)?;
        let zero_int = self.k.arena_mut().int(0);
        let pc_image = pc(&mut self.k, image);
        let pc_zero =
            self.k
                .bitblast_implies(&[is_zero, bound_lo, bound_hi], pc_image, zero_int)?;
        let body = self.k.trans(&k_image_zero, &self.k.sym(&pc_zero))?;
        Ok(self.k.weaken(&body, &[is_zero, bound_lo, bound_hi]))
    }
}

fn next_bounds(k: &mut Kernel, v: Symbol, width: u32) -> (Tid, Tid) {
    let ar = k.arena_mut();
    let zero = ar.int(0);
    let one = ar.int(1);
    let vv = ar.var(v);
    let next = ar.add(vv, one);
    let upper = ar.int((1i128 << width) - 1);
    (ar.le(zero, next), ar.le(next, upper))
}

pub fn kernighan_theorem(width: u32) -> Result<Theorem, KernelError> {
    let mut env = Env::new(width);
    let template = env.template();
    let base = env.base()?;
    let step_nonzero = env.step_nonzero()?;
    let step_zero = env.step_zero()?;
    let neq = {
        let ar = env.k.arena_mut();
        let zero = ar.bv(0, width);
        let one = ar.int(1);
        let vv = ar.var(env.v);
        let next = ar.add(vv, one);
        let image = ar.int_to_bv(next, width);
        let is_zero = ar.bv_bin(BvOp::Eq, image, zero);
        ar.bool_not(is_zero)
    };
    let neg = env.k.arena_mut().bool_not(neq);
    let inducted = env.k.nat_induction_cases(
        env.v,
        &template,
        &base,
        &[(Some(neq), step_nonzero), (Some(neg), step_zero)],
    )?;
    // Instantiate the induction variable at the integer value of the
    // free bitvector `x`, then discharge the range hypotheses and
    // rewrite `int_to_bv(bv_to_int(x))` back to `x`.
    let xv = env.k.arena_mut().var(env.x);
    let value = env.k.arena_mut().bv_to_int(xv);
    let instantiated = env.k.instantiate(&inducted, &[(env.v, value)])?;
    let zero_int = env.k.arena_mut().int(0);
    let lower = env.k.arena_mut().le(zero_int, value);
    let upper = env.upper();
    let higher = env.k.arena_mut().le(value, upper);
    let true_term = env.k.arena_mut().bool(true);
    let lower_lemma = env.k.bitblast_implies(&[], lower, true_term)?;
    let higher_lemma = env.k.bitblast_implies(&[], higher, true_term)?;
    let instantiated = env.k.discharge_lemma(&instantiated, lower, &lower_lemma)?;
    let instantiated = env
        .k
        .discharge_lemma(&instantiated, higher, &higher_lemma)?;
    let roundtrip = env
        .k
        .apply_schema(env.roundtrip_schema, &[(env.roundtrip_param, xv)])?;
    let hole_bv = fresh(&mut env.k, "final_bv", Sort::Bv(width));
    let hole_bv_term = env.k.arena_mut().var(hole_bv);
    let k_hole = env.k.arena_mut().app(env.kfun, &[hole_bv_term]);
    let lhs = env.k.congr(
        Context {
            hole: hole_bv,
            template: k_hole,
        },
        &roundtrip,
    )?;
    let pc_hole = pc(&mut env.k, hole_bv_term);
    let rhs = env.k.congr(
        Context {
            hole: hole_bv,
            template: pc_hole,
        },
        &roundtrip,
    )?;
    let lhs = env.k.sym(&lhs);
    let lhs = env.k.trans(&lhs, &instantiated)?;
    let theorem = env.k.trans(&lhs, &rhs)?;
    env.k.replay(&theorem)?;
    Ok(theorem)
}

/// Machine-checked Kernighan law for widths the integer shadow covers.
pub fn kernighan_law(width: u32) -> bool {
    (1..=32).contains(&width) && kernighan_theorem(width).is_ok()
}
