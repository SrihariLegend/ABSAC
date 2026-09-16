//! Kernel-term model of a recognized generated kernel.
//!
//! The model is the bridge between the bytes on disk and the object
//! theory:
//!
//! - memory is a free function `load(addr) : Bv8` (byte at an index);
//! - each kernel's element semantics is built from the *recognized*
//!   scalar tail, so the loops and the specification share one source
//!   of truth;
//! - the range fold `S(a, b) = Σ_{i=b}^{a-1} elem(i)` (or the boolean
//!   fold `A` for `All`) is defined by guarded schemas: each recurrence
//!   is only available inside its range, which keeps the theory
//!   consistent (see `docs/GATE4B_PROOF.md`);
//! - block contributions apply the SDM models of the intrinsics to the
//!   lanes the recognized code actually loads.

use sir_mech::kernel::{Kernel, KernelError, SchemaId};
use sir_mech::term::{BvOp, FuncId, Sort, Symbol, Tid};

use crate::emitted::{EmittedKernel, KernelKind};

/// Guarded recurrences of the range fold.
#[derive(Clone, Copy, Debug)]
pub struct FoldSchemas {
    pub base: SchemaId,
    pub step_right: SchemaId,
    pub step_left: SchemaId,
}

/// A kernel-term model of one recognized generated kernel.
#[derive(Clone, Debug)]
pub struct Model {
    pub kind: KernelKind,
    /// `load(addr) : Bv8` — the byte at index `addr`.
    pub load: FuncId,
    /// `S(a, b) : Int` for cardinality/sum kernels.
    pub fold_int: Option<FuncId>,
    /// `A(a, b) : Bool` for the `All` kernel.
    pub fold_bool: Option<FuncId>,
    pub schemas: FoldSchemas,
    /// Source parameters, in declaration order.
    pub params: Vec<(String, Symbol)>,
    pub n: Symbol,
    pub mask: Option<Symbol>,
    pub target: Option<Symbol>,
    pub val: Option<Symbol>,
}

fn elem_term(
    k: &mut Kernel,
    kind: &KernelKind,
    load: FuncId,
    mask: Option<Symbol>,
    target: Option<Symbol>,
    val: Option<Symbol>,
    addr: Tid,
) -> Tid {
    let byte = k.arena_mut().app(load, &[addr]);
    match kind {
        KernelKind::CardinalityMasked { .. } => {
            let m = k.arena_mut().var(mask.expect("mask"));
            let masked = k.arena_mut().bv_bin(BvOp::And, byte, m);
            let t = k.arena_mut().var(target.expect("target"));
            let eq = k.arena_mut().bv_bin(BvOp::Eq, masked, t);
            k.arena_mut().bool_to_int(eq)
        }
        KernelKind::SumAscii { .. } => k.arena_mut().bv_to_int(byte),
        KernelKind::AllEquality { .. } => {
            let v = k.arena_mut().var(val.expect("val"));
            k.arena_mut().bv_bin(BvOp::Eq, byte, v)
        }
    }
}

impl Model {
    /// Build the model for a recognized kernel in a fresh kernel arena.
    pub fn build(kernel: &EmittedKernel, k: &mut Kernel) -> Result<Model, KernelError> {
        let params: Vec<(String, Symbol)> = kernel
            .params
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    k.arena_mut().declare_var(p.name.clone(), Sort::Int),
                )
            })
            .collect();
        let load = k
            .arena_mut()
            .declare_func("load", vec![Sort::Int], Sort::Bv(8));

        let find = |params: &[(String, Symbol)], name: &str| -> Symbol {
            params
                .iter()
                .find(|(p, _)| p == name)
                .map(|(_, s)| *s)
                .unwrap_or_else(|| panic!("parameter {name} not recognized"))
        };
        let (n_name, mask_name, target_name, val_name) = match &kernel.kind {
            KernelKind::AllEquality { n, val, .. } => (n.clone(), None, None, Some(val.clone())),
            KernelKind::CardinalityMasked {
                n, mask, target, ..
            } => (
                n.clone(),
                Some(mask.clone()),
                Some(target.clone()),
                None,
            ),
            KernelKind::SumAscii { n, .. } => (n.clone(), None, None, None),
        };
        let n = find(&params, &n_name);
        let declare_byte = |k: &mut Kernel, name: &str| -> Symbol {
            k.arena_mut().declare_var(name, Sort::Bv(8))
        };
        let mask = mask_name
            .as_deref()
            .map(|m| declare_byte(k, m));
        let target = target_name
            .as_deref()
            .map(|t| declare_byte(k, t));
        let val = val_name.as_deref().map(|v| declare_byte(k, v));

        let fold_int = matches!(
            kernel.kind,
            KernelKind::CardinalityMasked { .. } | KernelKind::SumAscii { .. }
        )
        .then(|| {
            k.arena_mut()
                .declare_func("S", vec![Sort::Int, Sort::Int], Sort::Int)
        });
        let fold_bool = matches!(kernel.kind, KernelKind::AllEquality { .. })
            .then(|| {
                k.arena_mut()
                    .declare_func("A", vec![Sort::Int, Sort::Int], Sort::Bool)
            });

        let (x, y, y1, x1) = {
            let ar = k.arena_mut();
            (
                ar.declare_var("fold_x", Sort::Int),
                ar.declare_var("fold_y", Sort::Int),
                ar.declare_var("fold_y1", Sort::Int),
                ar.declare_var("fold_x1", Sort::Int),
            )
        };

        let schemas = match (fold_int, fold_bool) {
            (Some(f), None) => {
                // S(x, x) = 0
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let lhs = ar.app(f, &[xv, xv]);
                    let rhs = ar.int(0);
                    let guard = ar.le(xv, xv);
                    (lhs, rhs, guard)
                };
                let base = k.schema_guarded("S(x,x)=0", &[x], Some(guard), lhs, rhs);

                // S(x, y) = S(x, y1) + elem(y)   [y1 = y+1, y < x]
                //
                // Peeling the *lowest* element is what makes the axioms
                // true in the intended model S(x,y) = sum_{k=y}^{x-1}
                // elem(k): the successor sum is the predecessor sum
                // minus elem(y).
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let yv = ar.var(y);
                    let y1v = ar.var(y1);
                    let one = ar.int(1);
                    let succ = ar.add(yv, one);
                    let eq = ar.int_eq(y1v, succ);
                    let lt = ar.lt(yv, xv);
                    let guard = ar.bool_and(eq, lt);
                    let lhs = ar.app(f, &[xv, yv]);
                    let inner = ar.app(f, &[xv, y1v]);
                    let e = {
                        // element at y
                        let byte = ar.app(load, &[yv]);
                        match &kernel.kind {
                            KernelKind::CardinalityMasked { .. } => {
                                let m = ar.var(mask.expect("mask"));
                                let t = ar.var(target.expect("target"));
                                let masked = ar.bv_bin(BvOp::And, byte, m);
                                let eq = ar.bv_bin(BvOp::Eq, masked, t);
                                ar.bool_to_int(eq)
                            }
                            KernelKind::SumAscii { .. } => ar.bv_to_int(byte),
                            KernelKind::AllEquality { .. } => unreachable!(),
                        }
                    };
                    let rhs = ar.add(inner, e);
                    (lhs, rhs, guard)
                };
                let step_right =
                    k.schema_guarded("S(x,y)=S(x,y1)+elem(y)", &[x, y, y1], Some(guard), lhs, rhs);

                // S(x1, y) = S(x, y) + elem(x)   [x1 = x+1, y <= x]
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let x1v = ar.var(x1);
                    let yv = ar.var(y);
                    let one = ar.int(1);
                    let succ = ar.add(xv, one);
                    let eq = ar.int_eq(x1v, succ);
                    let le = ar.le(yv, xv);
                    let guard = ar.bool_and(eq, le);
                    let lhs = ar.app(f, &[x1v, yv]);
                    let inner = ar.app(f, &[xv, yv]);
                    let e = {
                        // element at x
                        let byte = ar.app(load, &[xv]);
                        match &kernel.kind {
                            KernelKind::CardinalityMasked { .. } => {
                                let m = ar.var(mask.expect("mask"));
                                let t = ar.var(target.expect("target"));
                                let masked = ar.bv_bin(BvOp::And, byte, m);
                                let eq = ar.bv_bin(BvOp::Eq, masked, t);
                                ar.bool_to_int(eq)
                            }
                            KernelKind::SumAscii { .. } => ar.bv_to_int(byte),
                            KernelKind::AllEquality { .. } => unreachable!(),
                        }
                    };
                    let rhs = ar.add(inner, e);
                    (lhs, rhs, guard)
                };
                let step_left = k.schema_guarded(
                    "S(x1,y)=S(x,y)+elem(x)",
                    &[x, x1, y],
                    Some(guard),
                    lhs,
                    rhs,
                );
                FoldSchemas {
                    base,
                    step_right,
                    step_left,
                }
            }
            (None, Some(f)) => {
                // A(x, x) = true
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let lhs = ar.app(f, &[xv, xv]);
                    let rhs = ar.bool(true);
                    let guard = ar.le(xv, xv);
                    (lhs, rhs, guard)
                };
                let base = k.schema_guarded("A(x,x)=true", &[x], Some(guard), lhs, rhs);

                // A(x, y) = A(x, y1) && elem(y)   [y1 = y+1, y < x]
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let yv = ar.var(y);
                    let y1v = ar.var(y1);
                    let one = ar.int(1);
                    let succ = ar.add(yv, one);
                    let eq = ar.int_eq(y1v, succ);
                    let lt = ar.lt(yv, xv);
                    let guard = ar.bool_and(eq, lt);
                    let lhs = ar.app(f, &[xv, yv]);
                    let inner = ar.app(f, &[xv, y1v]);
                    let v = ar.var(val.expect("val"));
                    let byte = ar.app(load, &[yv]);
                    let e = ar.bv_bin(BvOp::Eq, byte, v);
                    let rhs = ar.bool_and(inner, e);
                    (lhs, rhs, guard)
                };
                let step_right =
                    k.schema_guarded("A(x,y)=A(x,y1)&elem(y)", &[x, y, y1], Some(guard), lhs, rhs);

                // A(x1, y) = A(x, y) && elem(x)   [x1 = x+1, y <= x]
                let (lhs, rhs, guard) = {
                    let ar = k.arena_mut();
                    let xv = ar.var(x);
                    let x1v = ar.var(x1);
                    let yv = ar.var(y);
                    let one = ar.int(1);
                    let succ = ar.add(xv, one);
                    let eq = ar.int_eq(x1v, succ);
                    let le = ar.le(yv, xv);
                    let guard = ar.bool_and(eq, le);
                    let lhs = ar.app(f, &[x1v, yv]);
                    let inner = ar.app(f, &[xv, yv]);
                    let v = ar.var(val.expect("val"));
                    let byte = ar.app(load, &[xv]);
                    let e = ar.bv_bin(BvOp::Eq, byte, v);
                    let rhs = ar.bool_and(inner, e);
                    (lhs, rhs, guard)
                };
                let step_left = k.schema_guarded(
                    "A(x1,y)=A(x,y)&elem(x)",
                    &[x, x1, y],
                    Some(guard),
                    lhs,
                    rhs,
                );
                FoldSchemas {
                    base,
                    step_right,
                    step_left,
                }
            }
            _ => return Err(KernelError::Shape("model: fold ambiguity".into())),
        };

        Ok(Model {
            kind: kernel.kind.clone(),
            load,
            fold_int,
            fold_bool,
            schemas,
            params,
            n,
            mask,
            target,
            val,
        })
    }

    pub fn int(&self, k: &mut Kernel, v: i128) -> Tid {
        k.arena_mut().int(v)
    }

    pub fn add(&self, k: &mut Kernel, a: Tid, b: Tid) -> Tid {
        k.arena_mut().add(a, b)
    }

    pub fn byte(&self, k: &mut Kernel, addr: Tid) -> Tid {
        let load = self.load;
        k.arena_mut().app(load, &[addr])
    }

    /// `S(a, b)` (or `A(a, b)` for `All`).
    pub fn fold(&self, k: &mut Kernel, a: Tid, b: Tid) -> Tid {
        if let Some(f) = self.fold_int {
            k.arena_mut().app(f, &[a, b])
        } else {
            let f = self.fold_bool.expect("fold bool");
            k.arena_mut().app(f, &[a, b])
        }
    }

    /// The per-element contribution, exactly as the recognized scalar
    /// tail computes it.
    pub fn elem(&self, k: &mut Kernel, i: Tid) -> Tid {
        elem_term(k, &self.kind, self.load, self.mask, self.target, self.val, i)
    }

    /// The bytes a `width`-wide block load reads: `load(start + j)`.
    pub fn lanes(&self, k: &mut Kernel, width: usize, start: Tid) -> Vec<Tid> {
        (0..width)
            .map(|j| {
                let off = k.arena_mut().int(j as i128);
                let addr = k.arena_mut().add(start, off);
                self.byte(k, addr)
            })
            .collect()
    }

    /// The value a single vector block contributes, built by applying
    /// the SDM models of the intrinsics the recognized code executes:
    ///
    /// - `pcmpeqb`: lane is `0xFF` when equal, `0x00` otherwise
    /// - `pmovmskb`: bit `i` is the msb of lane `i`
    /// - `popcount`: number of set bits (cardinality)
    /// - `psadbw` against zero: sum of the 8 bytes in each 64-bit lane
    /// - `paddq`: 64-bit lane-wise addition
    pub fn contribution(&self, k: &mut Kernel, width: usize, start: Tid) -> Tid {
        let lanes = self.lanes(k, width, start);
        match &self.kind {
            KernelKind::CardinalityMasked { .. } => {
                let bits: Vec<Tid> = lanes
                    .iter()
                    .map(|&b| self.lane_msb_compare(k, b, true))
                    .collect();
                let movemask = k.arena_mut().from_bits(&bits).expect("movemask width");
                let pc = k.arena_mut().bv_popcount(movemask);
                k.arena_mut().bv_to_int(pc)
            }
            KernelKind::AllEquality { .. } => {
                let bits: Vec<Tid> = lanes
                    .iter()
                    .map(|&b| self.lane_msb_compare(k, b, false))
                    .collect();
                let movemask = k.arena_mut().from_bits(&bits).expect("movemask width");
                let full = if width == 32 {
                    0xFFFF_FFFFu128
                } else {
                    0xFFFFu128
                };
                let full = k.arena_mut().bv(full, width as u32);
                k.arena_mut().bv_bin(BvOp::Eq, movemask, full)
            }
            KernelKind::SumAscii { .. } => {
                // PSADBW gives one 64-bit lane sum per 8 bytes and PADDQ
                // accumulates them; the epilogue adds the four lanes.
                // The *value* a block contributes is therefore the sum
                // of its bytes (u64 addition is associative and
                // commutative whenever no lane wraps, which the
                // overflow precondition of the end-to-end theorem
                // discharges). Building that sum directly keeps the
                // chunk lemma a per-byte identity plus linear
                // arithmetic instead of one 64-bit adder-tree query.
                let mut total = k.arena_mut().int(0);
                for byte in lanes {
                    let wide = k.arena_mut().bv_zero_ext(byte, 64);
                    let wide = k.arena_mut().bv_to_int(wide);
                    total = k.arena_mut().add(total, wide);
                }
                total
            }
        }
    }

    /// For the `Sum` kernel: the per-byte contribution leaves as
    /// `(zero-extended leaf, plain leaf)` pairs, in block order.
    pub fn sum_leaf_pairs(
        &self,
        k: &mut Kernel,
        width: usize,
        start: Tid,
    ) -> Vec<(Tid, Tid)> {
        self.lanes(k, width, start)
            .into_iter()
            .map(|byte| {
                let wide = k.arena_mut().bv_zero_ext(byte, 64);
                let wide = k.arena_mut().bv_to_int(wide);
                let plain = k.arena_mut().bv_to_int(byte);
                (wide, plain)
            })
            .collect()
    }

    /// `msb(pcmpeqb(...))` — the SDM model of PCMPEQB + PMOVMSKB for one
    /// lane, masked or unmasked.
    fn lane_msb_compare(&self, k: &mut Kernel, byte: Tid, masked: bool) -> Tid {
        let lhs = if masked {
            let m = k.arena_mut().var(self.mask.expect("mask"));
            k.arena_mut().bv_bin(BvOp::And, byte, m)
        } else {
            byte
        };
        let rhs = if masked {
            k.arena_mut().var(self.target.expect("target"))
        } else {
            k.arena_mut().var(self.val.expect("val"))
        };
        let eq = k.arena_mut().bv_bin(BvOp::Eq, lhs, rhs);
        let ones = k.arena_mut().bv(0xFF, 8);
        let zero = k.arena_mut().bv(0x00, 8);
        let cmp = k.arena_mut().bv_ite(eq, ones, zero);
        k.arena_mut().bv_extract(cmp, 7, 7)
    }

    /// `S(0, n)` (or `A(0, n)`): the specification value.
    pub fn spec(&self, k: &mut Kernel) -> Tid {
        let n = k.arena_mut().var(self.n);
        let zero = k.arena_mut().int(0);
        self.fold(k, n, zero)
    }
}
