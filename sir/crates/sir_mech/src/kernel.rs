//! The proof kernel: LCF-style rules over kernel terms.
//!
//! Every [`Theorem`] is either an axiom from a fixed, hashed table, or
//! the result of a checked rule application. Constructors validate
//! their inputs, so a theorem cannot be fabricated by construction
//! sites — only by the rules below, each of which is re-checkable by
//! [`Kernel::replay`].
//!
//! Judgements are conditional equations:
//!
//! ```text
//!   h1, .., hn  ⊢  lhs = rhs
//! ```
//!
//! with all free variables implicitly universally quantified. The
//! Gate 4B statements are exactly of this form, with the well-formedness
//! preconditions appearing as hypotheses.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::arith::{prove_entailment, ArithOutcome};
use crate::equiv::{prove_equal as bv_prove_equal, CheckResult};
use crate::term::{Arena, Node, Sort, Symbol, Tid};

/// A definitional schema `lhs = rhs`, instantiable at any terms for its
/// parameters. Schemas are the object theory's definitions — range
/// folds, array reads, intrinsic models — and each use is pinned by
/// digest in the derivation artifact.
#[derive(Clone, Debug)]
pub struct Schema {
    pub name: String,
    pub params: Vec<Symbol>,
    /// Optional side condition. Applying the schema yields a theorem
    /// conditional on the guard (instantiated).
    pub guard: Option<Tid>,
    pub lhs: Tid,
    pub rhs: Tid,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SchemaId(pub usize);

/// A conditional equation judgement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub hyps: Vec<Tid>,
    pub lhs: Tid,
    pub rhs: Tid,
}

impl Statement {
    pub fn new(hyps: Vec<Tid>, lhs: Tid, rhs: Tid) -> Self {
        let mut hyps = hyps;
        hyps.sort_unstable();
        hyps.dedup();
        Statement { hyps, lhs, rhs }
    }

    /// Canonical comparison: hypotheses are a set.
    pub fn same_as(&self, other: &Statement) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs && self.hyps == other.hyps
    }
}

/// A context with a distinguished hole: `template[hole]`.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub hole: Symbol,
    pub template: Tid,
}

/// How a theorem was obtained. Kept as a tree so the artifact can be
/// replayed rather than trusted.
#[derive(Clone, Debug)]
pub enum Derivation {
    Refl(Tid),
    Sym(Box<Derivation>),
    Trans(Box<Derivation>, Box<Derivation>),
    Congr {
        context: Context,
        inner: Box<Derivation>,
    },
    Hypothesis(Tid),
    HypothesisBool(Tid),
    CaseSplit {
        cond: Tid,
        case_true: Box<Derivation>,
        case_false: Box<Derivation>,
    },
    Weaken {
        inner: Box<Derivation>,
        extra: Vec<Tid>,
    },
    Instantiate {
        inner: Box<Derivation>,
        map: Vec<(Symbol, Tid)>,
    },
    Schema {
        id: SchemaId,
        map: Vec<(Symbol, Tid)>,
        digest: u64,
    },
    Arith {
        goal: Statement,
        note: String,
    },
    BitBlast {
        goal: Statement,
        note: String,
    },
    DischargeLemma {
        inner: Box<Derivation>,
        hyp: Tid,
        lemma: Box<Derivation>,
    },
    DischargeArith {
        inner: Box<Derivation>,
        hyp: Tid,
    },
    Induction {
        var: Symbol,
        template: Statement,
        base: Box<Derivation>,
        /// One derivation per case: the optional literal is the case
        /// hypothesis the step assumes (e.g. the loop guard).
        cases: Vec<(Option<Tid>, Box<Derivation>)>,
    },
    Axiom {
        name: String,
        statement: Statement,
    },
}

#[derive(Clone, Debug)]
pub struct Theorem {
    pub statement: Statement,
    pub derivation: Derivation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelError {
    Shape(String),
    ArithFailed(String),
    SolverFailed(String),
    UnknownSchema(String),
    UnknownAxiom(String),
    AxiomMismatch {
        name: String,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// A trusted axiom: name plus its exact statement digest.
#[derive(Clone, Debug)]
struct AxiomRecord {
    name: String,
    digest: u64,
}

#[derive(Clone, Debug)]
pub struct Kernel {
    arena: Arena,
    schemas: Vec<Schema>,
    axioms: Vec<AxiomRecord>,
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl Kernel {
    pub fn new() -> Self {
        Kernel {
            arena: Arena::new(),
            schemas: Vec::new(),
            axioms: Vec::new(),
        }
    }

    pub fn arena(&self) -> &Arena {
        &self.arena
    }

    pub fn arena_mut(&mut self) -> &mut Arena {
        &mut self.arena
    }

    pub fn schemas(&self) -> &[Schema] {
        &self.schemas
    }

    /// A digest of a statement, used to pin axioms in artifacts.
    pub fn statement_digest(&self, s: &Statement) -> u64 {
        let mut acc: u64 = 0xcbf29ce484222325;
        let mut mix = |x: u64| {
            for i in 0..8 {
                acc ^= (x >> (8 * i)) & 0xff;
                acc = acc.wrapping_mul(0x100000001b3);
            }
        };
        for &h in &s.hyps {
            mix(h as u64);
        }
        mix(0xffff_ffff);
        mix(s.lhs as u64);
        mix(s.rhs as u64);
        acc
    }

    /// Register a definitional schema.
    pub fn schema(
        &mut self,
        name: impl Into<String>,
        params: &[Symbol],
        lhs: Tid,
        rhs: Tid,
    ) -> SchemaId {
        self.schema_guarded(name, params, None, lhs, rhs)
    }

    /// Register a definitional schema with a side condition.
    pub fn schema_guarded(
        &mut self,
        name: impl Into<String>,
        params: &[Symbol],
        guard: Option<Tid>,
        lhs: Tid,
        rhs: Tid,
    ) -> SchemaId {
        assert_eq!(
            self.arena.sort(lhs),
            self.arena.sort(rhs),
            "schema must equate same-sorted terms"
        );
        if let Some(g) = guard {
            assert_eq!(self.arena.sort(g), Sort::Bool, "schema guard must be Bool");
        }
        let name = name.into();
        for &p in params {
            assert!(
                (p.0 as usize) < self.arena.symbol_count(),
                "schema {name}: parameter {p:?} is not declared"
            );
        }
        let id = SchemaId(self.schemas.len());
        self.schemas.push(Schema {
            name,
            params: params.to_vec(),
            guard,
            lhs,
            rhs,
        });
        id
    }

    /// Digest pinning a schema definition into the artifact.
    pub fn schema_digest(&self, id: SchemaId) -> u64 {
        let s = &self.schemas[id.0];
        let mut d = self.statement_digest(&Statement::new(vec![], s.lhs, s.rhs));
        if let Some(g) = s.guard {
            d ^= self.statement_digest(&Statement::new(vec![], g, g));
        }
        for &p in &s.params {
            d ^= u64::from(p.0).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
        d
    }

    /// Declare a trusted axiom. Its statement is fixed at declaration
    /// and can only be used by name afterwards; replay checks the
    /// digest.
    pub fn declare_axiom(&mut self, name: impl Into<String>, statement: Statement) -> Theorem {
        let name = name.into();
        let digest = self.statement_digest(&statement);
        self.axioms.push(AxiomRecord {
            name: name.clone(),
            digest,
        });
        Theorem {
            statement: statement.clone(),
            derivation: Derivation::Axiom { name, statement },
        }
    }

    // ── Primitive rules ─────────────────────────────────────────────

    pub fn refl(&mut self, t: Tid) -> Theorem {
        Theorem {
            statement: Statement::new(vec![], t, t),
            derivation: Derivation::Refl(t),
        }
    }

    pub fn sym(&self, t: &Theorem) -> Theorem {
        Theorem {
            statement: Statement::new(t.statement.hyps.clone(), t.statement.rhs, t.statement.lhs),
            derivation: Derivation::Sym(Box::new(t.derivation.clone())),
        }
    }

    pub fn trans(&self, a: &Theorem, b: &Theorem) -> Result<Theorem, KernelError> {
        if a.statement.rhs != b.statement.lhs {
            return Err(KernelError::Shape(format!(
                "trans: {} != {}",
                self.arena.display(a.statement.rhs),
                self.arena.display(b.statement.lhs)
            )));
        }
        let mut hyps = a.statement.hyps.clone();
        hyps.extend(b.statement.hyps.iter().copied());
        Ok(Theorem {
            statement: Statement::new(hyps, a.statement.lhs, b.statement.rhs),
            derivation: Derivation::Trans(
                Box::new(a.derivation.clone()),
                Box::new(b.derivation.clone()),
            ),
        })
    }

    /// Congruence: from `a = b` derive `template[hole := a] =
    /// template[hole := b]`.
    pub fn congr(&mut self, context: Context, inner: &Theorem) -> Result<Theorem, KernelError> {
        let hole_term = self.arena.var(context.hole);
        if self.arena.sort(hole_term) != self.arena.sort(inner.statement.lhs) {
            return Err(KernelError::Shape("congr: hole sort mismatch".into()));
        }
        let mut map = HashMap::new();
        map.insert(hole_term, inner.statement.lhs);
        let lhs = self.arena.substitute(context.template, &map);
        let mut map2 = HashMap::new();
        map2.insert(hole_term, inner.statement.rhs);
        let rhs = self.arena.substitute(context.template, &map2);
        Ok(Theorem {
            statement: Statement::new(inner.statement.hyps.clone(), lhs, rhs),
            derivation: Derivation::Congr {
                context,
                inner: Box::new(inner.derivation.clone()),
            },
        })
    }

    pub fn weaken(&self, inner: &Theorem, extra: &[Tid]) -> Theorem {
        let mut hyps = inner.statement.hyps.clone();
        hyps.extend(extra.iter().copied());
        Theorem {
            statement: Statement::new(hyps, inner.statement.lhs, inner.statement.rhs),
            derivation: Derivation::Weaken {
                inner: Box::new(inner.derivation.clone()),
                extra: extra.to_vec(),
            },
        }
    }

    /// Build a conditional equation from an equality hypothesis:
    /// `h ⊢ lhs = rhs` where `h` is `lhs = rhs` as a boolean term.
    pub fn hypothesis(&self, eq_term: Tid) -> Result<Theorem, KernelError> {
        let (lhs, rhs) = self
            .equality_parts(eq_term)
            .ok_or_else(|| KernelError::Shape("hypothesis is not an equality".into()))?;
        Ok(Theorem {
            statement: Statement::new(vec![eq_term], lhs, rhs),
            derivation: Derivation::Hypothesis(eq_term),
        })
    }

    /// Introduce an arbitrary boolean hypothesis: `h ⊢ h = true`.
    pub fn hypothesis_bool(&mut self, h: Tid) -> Result<Theorem, KernelError> {
        if self.arena.sort(h) != Sort::Bool {
            return Err(KernelError::Shape("hypothesis_bool: not Bool".into()));
        }
        let t = self.arena.bool(true);
        Ok(Theorem {
            statement: Statement::new(vec![h], h, t),
            derivation: Derivation::HypothesisBool(h),
        })
    }

    /// Classical case split: from `c ⊢ C` and `¬c ⊢ C` derive `C`.
    ///
    /// The two cases may carry extra hypotheses; the conclusion keeps
    /// the union of everything except the split literal itself.
    pub fn case_split(
        &mut self,
        cond: Tid,
        case_true: &Theorem,
        case_false: &Theorem,
    ) -> Result<Theorem, KernelError> {
        if self.arena.sort(cond) != Sort::Bool {
            return Err(KernelError::Shape("case_split: condition not Bool".into()));
        }
        if case_true.statement.lhs != case_false.statement.lhs
            || case_true.statement.rhs != case_false.statement.rhs
        {
            return Err(KernelError::Shape(
                "case_split: branches prove different conclusions".into(),
            ));
        }
        let neg = self.arena.bool_not(cond);
        if !case_true.statement.hyps.contains(&cond) {
            return Err(KernelError::Shape(
                "case_split: true branch does not assume the condition".into(),
            ));
        }
        if !case_false.statement.hyps.contains(&neg) {
            return Err(KernelError::Shape(
                "case_split: false branch does not assume the negated condition".into(),
            ));
        }
        let mut hyps: Vec<Tid> = case_true
            .statement
            .hyps
            .iter()
            .copied()
            .filter(|&h| h != cond)
            .collect();
        hyps.extend(
            case_false
                .statement
                .hyps
                .iter()
                .copied()
                .filter(|&h| h != neg),
        );
        Ok(Theorem {
            statement: Statement::new(
                hyps,
                case_true.statement.lhs,
                case_true.statement.rhs,
            ),
            derivation: Derivation::CaseSplit {
                cond,
                case_true: Box::new(case_true.derivation.clone()),
                case_false: Box::new(case_false.derivation.clone()),
            },
        })
    }

    /// Instantiate free variables of a theorem.
    pub fn instantiate(
        &mut self,
        inner: &Theorem,
        map: &[(Symbol, Tid)],
    ) -> Result<Theorem, KernelError> {
        let mut tid_map = HashMap::new();
        for &(sym, term) in map {
            let var = self.arena.var(sym);
            if self.arena.sort(var) != self.arena.sort(term) {
                return Err(KernelError::Shape(format!(
                    "instantiate: sort mismatch for {}",
                    self.arena.var_name(sym)
                )));
            }
            tid_map.insert(var, term);
        }
        let hyps = inner
            .statement
            .hyps
            .iter()
            .map(|&h| self.arena.substitute(h, &tid_map))
            .collect();
        let lhs = self.arena.substitute(inner.statement.lhs, &tid_map);
        let rhs = self.arena.substitute(inner.statement.rhs, &tid_map);
        Ok(Theorem {
            statement: Statement::new(hyps, lhs, rhs),
            derivation: Derivation::Instantiate {
                inner: Box::new(inner.derivation.clone()),
                map: map.to_vec(),
            },
        })
    }

    /// Instantiate a definitional schema at the given parameter terms.
    pub fn apply_schema(
        &mut self,
        id: SchemaId,
        map: &[(Symbol, Tid)],
    ) -> Result<Theorem, KernelError> {
        let s = self
            .schemas
            .get(id.0)
            .cloned()
            .ok_or_else(|| KernelError::UnknownSchema(format!("{id:?}")))?;
        if map.len() != s.params.len() {
            return Err(KernelError::Shape(format!(
                "schema {}: expected {} parameters, got {}",
                s.name,
                s.params.len(),
                map.len()
            )));
        }
        let mut tid_map: HashMap<Tid, Tid> = HashMap::new();
        for &(param, arg) in map {
            let var = self.arena.var(param);
            if self.arena.sort(var) != self.arena.sort(arg) {
                return Err(KernelError::Shape(format!(
                    "schema {}: argument sort mismatch for {}",
                    s.name,
                    self.arena.var_name(param)
                )));
            }
            tid_map.insert(var, arg);
        }
        // Every declared parameter must be bound exactly once.
        for &param in &s.params {
            if map.iter().filter(|&&(p, _)| p == param).count() != 1 {
                return Err(KernelError::Shape(format!(
                    "schema {}: parameter {} not bound exactly once",
                    s.name,
                    self.arena.var_name(param)
                )));
            }
        }
        let lhs = self.arena.substitute(s.lhs, &tid_map);
        let rhs = self.arena.substitute(s.rhs, &tid_map);
        let hyps: Vec<Tid> = match s.guard {
            Some(g) => vec![self.arena.substitute(g, &tid_map)],
            None => vec![],
        };
        let digest = self.schema_digest(id);
        Ok(Theorem {
            statement: Statement::new(hyps, lhs, rhs),
            derivation: Derivation::Schema {
                id,
                map: map.to_vec(),
                digest,
            },
        })
    }

    /// Prove an arithmetic goal: either a linear integer identity, or
    /// an implication over linear constraints.
    pub fn arith_eq(&mut self, lhs: Tid, rhs: Tid) -> Result<Theorem, KernelError> {
        let (sl, sr) = (self.arena.sort(lhs), self.arena.sort(rhs));
        if sl != sr {
            return Err(KernelError::Shape("arith_eq: sort mismatch".into()));
        }
        let valid = match sl {
            Sort::Int => {
                crate::arith::linearize(&self.arena, lhs)
                    == crate::arith::linearize(&self.arena, rhs)
            }
            Sort::Bool => {
                let a = prove_entailment(&self.arena, &[lhs], rhs);
                let b = prove_entailment(&self.arena, &[rhs], lhs);
                a == ArithOutcome::Valid && b == ArithOutcome::Valid
            }
            Sort::Bv(_) => return Err(KernelError::Shape("arith_eq: use bitblast for Bv".into())),
        };
        if !valid {
            return Err(KernelError::ArithFailed(format!(
                "{} = {} not proved",
                self.arena.display(lhs),
                self.arena.display(rhs)
            )));
        }
        Ok(Theorem {
            statement: Statement::new(vec![], lhs, rhs),
            derivation: Derivation::Arith {
                goal: Statement::new(vec![], lhs, rhs),
                note: "linear integer normalization".to_string(),
            },
        })
    }

    /// Prove `hyps ⇒ goal` where `goal` is boolean, via linear
    /// arithmetic; result is the conditional equation `goal = true`.
    pub fn arith_entail(&mut self, hyps: &[Tid], goal: Tid) -> Result<Theorem, KernelError> {
        if self.arena.sort(goal) != Sort::Bool {
            return Err(KernelError::Shape("arith_entail: goal must be Bool".into()));
        }
        if prove_entailment(&self.arena, hyps, goal) != ArithOutcome::Valid {
            return Err(KernelError::ArithFailed(format!(
                "cannot prove {} under {} hypotheses",
                self.arena.display(goal),
                hyps.len()
            )));
        }
        let t = self.arena.bool(true);
        Ok(Theorem {
            statement: Statement::new(hyps.to_vec(), goal, t),
            derivation: Derivation::Arith {
                goal: Statement::new(hyps.to_vec(), goal, t),
                note: "linear entailment".to_string(),
            },
        })
    }

    /// Prove `lhs = rhs` for boolean/bitvector terms by bit-blasting.
    pub fn bitblast_eq(&mut self, lhs: Tid, rhs: Tid) -> Result<Theorem, KernelError> {
        let (sl, sr) = (self.arena.sort(lhs), self.arena.sort(rhs));
        if sl != sr {
            return Err(KernelError::Shape("bitblast_eq: sort mismatch".into()));
        }
        if !matches!(sl, Sort::Bool | Sort::Bv(_)) {
            if !matches!(sl, Sort::Int) {
                return Err(KernelError::Shape(
                    "bitblast_eq: only Bool/Bv/Int-conversion supported".into(),
                ));
            }
        }
        let (bv, l, r) = self.translate_pair(lhs, rhs)?;
        match bv_prove_equal(&bv, l, r) {
            CheckResult::Valid => Ok(Theorem {
                statement: Statement::new(vec![], lhs, rhs),
                derivation: Derivation::BitBlast {
                    goal: Statement::new(vec![], lhs, rhs),
                    note: format!("bitblast UNSAT ({} solver terms)", bv.len()),
                },
            }),
            CheckResult::Counterexample(model) => {
                let mut entries: Vec<_> = model.into_iter().collect();
                entries.sort();
                let shown: Vec<String> = entries
                    .into_iter()
                    .map(|(v, x)| format!("v{}={x}", v.0))
                    .collect();
                Err(KernelError::SolverFailed(format!(
                    "bitblast counterexample: {}",
                    shown.join(", ")
                )))
            }
            CheckResult::Unknown(m) => Err(KernelError::SolverFailed(m)),
        }
    }

    /// Prove `assumptions ⇒ lhs = rhs` by bit-blasting with the
    /// assumptions asserted. The result carries the assumptions as
    /// hypotheses; it is what propositional case obligations need.
    pub fn bitblast_implies(
        &mut self,
        assumptions: &[Tid],
        lhs: Tid,
        rhs: Tid,
    ) -> Result<Theorem, KernelError> {
        for &a in assumptions {
            if self.arena.sort(a) != Sort::Bool {
                return Err(KernelError::Shape(
                    "bitblast_implies: assumptions must be Bool".into(),
                ));
            }
        }
        let sl = self.arena.sort(lhs);
        if sl != self.arena.sort(rhs) {
            return Err(KernelError::Shape("bitblast_implies: sort mismatch".into()));
        }
        let mut terms: Vec<Tid> = assumptions.to_vec();
        terms.push(lhs);
        terms.push(rhs);
        let (bv, translated) = self.translate_terms(&terms)?;
        let n = assumptions.len();
        let outcome = crate::equiv::prove_implied_equal(
            &bv,
            &translated[..n],
            translated[n],
            translated[n + 1],
        );
        match outcome {
            CheckResult::Valid => Ok(Theorem {
                statement: Statement::new(assumptions.to_vec(), lhs, rhs),
                derivation: Derivation::BitBlast {
                    goal: Statement::new(assumptions.to_vec(), lhs, rhs),
                    note: "bitblast under assumptions: UNSAT".to_string(),
                },
            }),
            CheckResult::Counterexample(m) => Err(KernelError::SolverFailed(format!(
                "bitblast_implies counterexample: {m:?}"
            ))),
            CheckResult::Unknown(msg) => Err(KernelError::SolverFailed(msg)),
        }
    }

    /// Translate several terms into one solver arena.
    fn translate_terms(
        &self,
        terms: &[Tid],
    ) -> Result<(crate::bv::Bv, Vec<crate::bv::Term>), KernelError> {
        let mut bv = crate::bv::Bv::new();
        let mut memo = HashMap::new();
        let mut out = Vec::with_capacity(terms.len());
        for &t in terms {
            out.push(self.translate(t, &mut bv, &mut memo)?);
        }
        Ok((bv, out))
    }

    /// Discharge a hypothesis that is implied by the remaining
    /// hypotheses in linear arithmetic.
    pub fn discharge_arith(&self, inner: &Theorem, hyp: Tid) -> Result<Theorem, KernelError> {
        if !inner.statement.hyps.contains(&hyp) {
            return Err(KernelError::Shape(
                "discharge_arith: hypothesis absent".into(),
            ));
        }
        let remaining: Vec<Tid> = inner
            .statement
            .hyps
            .iter()
            .copied()
            .filter(|&h| h != hyp)
            .collect();
        if !self.entails(&remaining, hyp) {
            return Err(KernelError::ArithFailed(format!(
                "cannot discharge {}",
                self.arena.display(hyp)
            )));
        }
        Ok(Theorem {
            statement: Statement::new(remaining, inner.statement.lhs, inner.statement.rhs),
            derivation: Derivation::DischargeArith {
                inner: Box::new(inner.derivation.clone()),
                hyp,
            },
        })
    }

    /// Does the hypothesis set entail `goal`? Syntactic presence
    /// counts, conjunctions are decomposed, and the rest is handed to
    /// linear arithmetic. This is what lets a case literal (a boolean
    /// atom) discharge itself inside its own case.
    pub fn entails(&self, hyps: &[Tid], goal: Tid) -> bool {
        if hyps.contains(&goal) {
            return true;
        }
        match self.arena.node(goal).clone() {
            Node::BoolConst(true) => true,
            Node::BoolConst(false) => false,
            Node::BoolAnd(a, b) => self.entails(hyps, a) && self.entails(hyps, b),
            Node::BoolNot(inner) => {
                if hyps.contains(&inner) {
                    return false;
                }
                prove_entailment(&self.arena, hyps, goal) == ArithOutcome::Valid
            }
            _ => prove_entailment(&self.arena, hyps, goal) == ArithOutcome::Valid,
        }
    }

    /// Discharge a hypothesis using a lemma whose hypotheses are
    /// available in the target theorem.
    pub fn discharge_lemma(
        &mut self,
        inner: &Theorem,
        hyp: Tid,
        lemma: &Theorem,
    ) -> Result<Theorem, KernelError> {
        if !inner.statement.hyps.contains(&hyp) {
            return Err(KernelError::Shape(
                "discharge_lemma: hypothesis absent".into(),
            ));
        }
        // The lemma must prove the hypothesis: either `hyp` is the
        // lemma's lhs with rhs = true, or the lemma is `hyp` itself
        // (equality hypothesis), or `hyp` is an equality term matching
        // the lemma's equation.
        let true_t = self.arena.bool(true);
        let matches = (lemma.statement.lhs == hyp && lemma.statement.rhs == true_t)
            || self.equality_parts(hyp).map(|(l, r)| (l, r))
                == Some((lemma.statement.lhs, lemma.statement.rhs));
        if !matches {
            return Err(KernelError::Shape(format!(
                "discharge_lemma: lemma does not establish {}",
                self.arena.display(hyp)
            )));
        }
        for &lh in &lemma.statement.hyps {
            if !inner.statement.hyps.contains(&lh) {
                return Err(KernelError::Shape(
                    "discharge_lemma: lemma has unavailable hypotheses".into(),
                ));
            }
        }
        let remaining: Vec<Tid> = inner
            .statement
            .hyps
            .iter()
            .copied()
            .filter(|&h| h != hyp)
            .collect();
        Ok(Theorem {
            statement: Statement::new(remaining, inner.statement.lhs, inner.statement.rhs),
            derivation: Derivation::DischargeLemma {
                inner: Box::new(inner.derivation.clone()),
                hyp,
                lemma: Box::new(lemma.derivation.clone()),
            },
        })
    }

    /// Natural-number induction on `var` (Int).
    ///
    /// Given a template `P(k)`, a base proof of `P(0)` and a step proof
    /// of `k ≥ 0, P(k) ⊢ P(k+1)`, derive `k ≥ 0 ⊢ P(k)`.
    ///
    /// The step may assume not only `P(k)` itself but any *instance* of
    /// it: an extra hypothesis is accepted when it matches the template
    /// equality with the induction variable frozen, and the instance's
    /// hypotheses are entailed by the remaining step hypotheses. This is
    /// what makes loop invariants provable — the recursive call's state
    /// is a different instance of the same invariant at the same `k`.
    pub fn nat_induction(
        &mut self,
        var: Symbol,
        template: &Statement,
        base: &Theorem,
        step: &Theorem,
    ) -> Result<Theorem, KernelError> {
        self.nat_induction_cases(var, template, base, &[(None, step.clone())])
    }

    /// Case-split natural-number induction.
    ///
    /// Each case is `(literal, step)`: the step proves `P(k+1)` under
    /// the case literal (e.g. the loop guard) plus the successor
    /// template's hypotheses and `k >= 0`. Instances of the invariant
    /// used in a case are licensed with that case's hypotheses
    /// available, which is what lets a loop's recursive call use the
    /// induction hypothesis under its guard. The cases must be
    /// exhaustive: either one case with no literal, or a literal and
    /// its syntactic negation.
    pub fn nat_induction_cases(
        &mut self,
        var: Symbol,
        template: &Statement,
        base: &Theorem,
        cases: &[(Option<Tid>, Theorem)],
    ) -> Result<Theorem, KernelError> {
        let k = self.arena.var(var);
        if self.arena.sort(k) != Sort::Int {
            return Err(KernelError::Shape("induction: var must be Int".into()));
        }
        let zero = self.arena.int(0);
        let one = self.arena.int(1);
        let k_plus_1 = self.arena.add(k, one);

        // Base: P(0)
        let base_expected = self.instantiate_statement(template, &[(var, zero)])?;
        if !base.statement.same_as(&base_expected) {
            return Err(KernelError::Shape(format!(
                "induction base mismatch: expected {}, got {}",
                self.describe(&base_expected),
                self.describe(&base.statement)
            )));
        }

        // Exhaustiveness of the cases.
        match cases.len() {
            1 => {
                if cases[0].0.is_some() {
                    return Err(KernelError::Shape(
                        "induction: a single case must be unconditional".into(),
                    ));
                }
            }
            2 => {
                let (a, b) = (cases[0].0, cases[1].0);
                let complementary = match (a, b) {
                    (Some(x), Some(y)) => {
                        y == self.arena.bool_not(x) || x == self.arena.bool_not(y)
                    }
                    _ => false,
                };
                if !complementary {
                    return Err(KernelError::Shape(
                        "induction: two cases must be complementary literals".into(),
                    ));
                }
            }
            _ => {
                return Err(KernelError::Shape(
                    "induction: need one unconditional or two complementary cases".into(),
                ))
            }
        }

        // Step: hypotheses(template[k+1]) ∪ {k >= 0, literal} ⊢ P(k+1)
        let step_conclusion = self.instantiate_statement(template, &[(var, k_plus_1)])?;
        let ih_stmt = self.instantiate_statement(template, &[(var, k)])?;
        let ih_eq = self.equality_term(&ih_stmt)?;
        let guard = self.arena.le(zero, k);

        // Strong induction: an instance of P(r') is licensed when
        // r' >= 0 and r' < r + 1 follow from the available hypotheses.
        let frozen = HashSet::new();
        let k_plus_one = k_plus_1;
        let template_hyps_at_k: Vec<Tid> = template
            .hyps
            .iter()
            .map(|&h| {
                self.instantiate_statement(&Statement::new(vec![], h, h), &[(var, k)])
                    .map(|s| s.lhs)
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (literal, step) in cases {
            if !step.statement.lhs.eq(&step_conclusion.lhs)
                || !step.statement.rhs.eq(&step_conclusion.rhs)
            {
                return Err(KernelError::Shape(format!(
                    "induction step mismatch: expected {}, got {}",
                    self.describe(&step_conclusion),
                    self.describe(&step.statement)
                )));
            }
            // Hypotheses the step must carry, and the context in which
            // the invariant instances are checked.
            let mut required: Vec<Tid> = step_conclusion.hyps.clone();
            required.push(guard);
            let mut available: Vec<Tid> = step.statement.hyps.clone();
            if !available.contains(&guard) {
                available.push(guard);
            }
            // Hypothesis preservation for the invariant itself.
            let mut ctx = available.clone();
            if let Some(case_lit) = literal {
                if !ctx.contains(case_lit) {
                    ctx.push(*case_lit);
                }
            }
            for &h in &template.hyps {
                let h_next = self
                    .instantiate_statement(&Statement::new(vec![], h, h), &[(var, k_plus_1)])?
                    .lhs;
                if ctx.contains(&h_next) {
                    continue;
                }
                if prove_entailment(&self.arena, &ctx, h_next) != ArithOutcome::Valid {
                    return Err(KernelError::Shape(format!(
                        "induction: hypothesis {} is not preserved at the successor step",
                        self.arena.display(h)
                    )));
                }
            }
            let extras: Vec<Tid> = step
                .statement
                .hyps
                .iter()
                .copied()
                .filter(|h| !required.contains(h) && Some(*h) != *literal)
                .collect();
            for h in extras {
                if h == ih_eq {
                    continue;
                }
                let mut subst: HashMap<Tid, Tid> = HashMap::new();
                if !self.match_terms(ih_eq, h, &frozen, &mut subst) {
                    return Err(KernelError::Shape(format!(
                        "induction: hypothesis {} is neither the invariant nor a licensed instance",
                        self.arena.display(h)
                    )));
                }
                let check_ctx: Vec<Tid> = ctx.iter().copied().filter(|&x| x != h).collect();
                // The invariant equality usually does not mention the
                // induction variable (it lives in the template's
                // hypothesis). Recover the instance's measure from that
                // hypothesis: for `i + r = n`, r' = s(n) - s(i).
                let r_instance = match subst.get(&k).copied() {
                    Some(t) => t,
                    None => match self.induction_measure_split(template, var) {
                        Some((i_t, n_t)) => {
                            let i_inst = self.apply_subst(i_t, &subst);
                            let n_inst = self.apply_subst(n_t, &subst);
                            self.arena.sub(n_inst, i_inst)
                        }
                        None => k,
                    },
                };
                subst.insert(k, r_instance);
                let zero = self.arena.int(0);
                let nonneg = self.arena.le(zero, r_instance);
                if !check_ctx.contains(&nonneg)
                    && prove_entailment(&self.arena, &check_ctx, nonneg) != ArithOutcome::Valid
                {
                    return Err(KernelError::Shape(format!(
                        "induction: instance measure {} is not known to be non-negative",
                        self.arena.display(r_instance)
                    )));
                }
                let below = self.arena.lt(r_instance, k_plus_one);
                if !check_ctx.contains(&below)
                    && prove_entailment(&self.arena, &check_ctx, below) != ArithOutcome::Valid
                {
                    return Err(KernelError::Shape(format!(
                        "induction: instance measure {} is not smaller than the step",
                        self.arena.display(r_instance)
                    )));
                }
                for &premise in &template_hyps_at_k {
                    let inst = self.apply_subst(premise, &subst);
                    if check_ctx.contains(&inst) {
                        continue;
                    }
                    if prove_entailment(&self.arena, &check_ctx, inst) != ArithOutcome::Valid {
                        return Err(KernelError::Shape(format!(
                            "induction: instance premise {} is not available for {}",
                            self.arena.display(inst),
                            self.arena.display(h)
                        )));
                    }
                }
            }
        }

        // Result: k >= 0 ⊢ P(k)
        let mut hyps = template.hyps.clone();
        hyps.push(guard);
        Ok(Theorem {
            statement: Statement::new(
                hyps,
                self.instantiate_statement(template, &[(var, k)])?.lhs,
                self.instantiate_statement(template, &[(var, k)])?.rhs,
            ),
            derivation: Derivation::Induction {
                var,
                template: template.clone(),
                base: Box::new(base.derivation.clone()),
                cases: cases
                    .iter()
                    .map(|(lit, t)| (*lit, Box::new(t.derivation.clone())))
                    .collect(),
            },
        })
    }

    /// Structural matching of `pattern` against `target`: pattern
    /// variables bind to terms (consistently), except the frozen ones,
    /// which must match themselves. Used to license induction-hypothesis
    /// instances and to guarantee the induction variable is fixed.
    pub fn match_terms(
        &self,
        pattern: Tid,
        target: Tid,
        frozen: &HashSet<Tid>,
        subst: &mut HashMap<Tid, Tid>,
    ) -> bool {
        if pattern == target {
            return true;
        }
        if let Node::Var(_) = self.arena.node(pattern) {
            if frozen.contains(&pattern) {
                return false;
            }
            match subst.get(&pattern) {
                Some(&bound) => return bound == target,
                None => {
                    if self.arena.sort(pattern) != self.arena.sort(target) {
                        return false;
                    }
                    subst.insert(pattern, target);
                    return true;
                }
            }
        }
        let p = self.arena.node(pattern).clone();
        let t = self.arena.node(target).clone();
        if !p.same_form(&t) {
            return false;
        }
        let pc = p.children();
        let tc = t.children();
        if pc.len() != tc.len() {
            return false;
        }
        pc.iter()
            .zip(tc.iter())
            .all(|(&a, &b)| self.match_terms(a, b, frozen, subst))
    }

    /// Apply a term substitution map (pattern term id -> term).
    pub fn apply_subst(&mut self, t: Tid, subst: &HashMap<Tid, Tid>) -> Tid {
        self.arena.substitute(t, subst)
    }

    /// For a template whose measure hypothesis is `a + r = b` (with `r`
    /// the induction variable), return `(a, b)` so a rule can recover
    /// the instance's measure.
    fn induction_measure_split(&self, template: &Statement, var: Symbol) -> Option<(Tid, Tid)> {
        let is_var = |t: Tid| matches!(self.arena.node(t), Node::Var(s) if *s == var);
        for &h in &template.hyps {
            if let Node::IntEq(x, y) = self.arena.node(h).clone() {
                for (l, r) in [(x, y), (y, x)] {
                    if let Node::IntAdd(a, b) = self.arena.node(l).clone() {
                        if is_var(b) {
                            return Some((a, r));
                        }
                        if is_var(a) {
                            return Some((b, r));
                        }
                    }
                }
            }
        }
        None
    }

    // ── Helpers ─────────────────────────────────────────────────────

    pub fn instantiate_statement(
        &mut self,
        s: &Statement,
        map: &[(Symbol, Tid)],
    ) -> Result<Statement, KernelError> {
        let thm = Theorem {
            statement: s.clone(),
            derivation: Derivation::Refl(s.lhs),
        };
        let out = self.instantiate(&thm, map)?;
        Ok(out.statement)
    }

    /// The boolean term expressing `lhs = rhs` for the statement's sort.
    pub fn equality_term(&mut self, s: &Statement) -> Result<Tid, KernelError> {
        let sort = self.arena.sort(s.lhs);
        let out = match sort {
            Sort::Int => self.arena.int_eq(s.lhs, s.rhs),
            Sort::Bool => self.arena.bool_eq(s.lhs, s.rhs),
            Sort::Bv(_) => self.arena.bv_bin(crate::term::BvOp::Eq, s.lhs, s.rhs),
        };
        Ok(out)
    }

    /// Decompose an equality term into its sides.
    pub fn equality_parts(&self, t: Tid) -> Option<(Tid, Tid)> {
        match self.arena.node(t) {
            Node::IntEq(a, b) => Some((*a, *b)),
            Node::BoolEq(a, b) => Some((*a, *b)),
            Node::BvBin(crate::term::BvOp::Eq, a, b) => Some((*a, *b)),
            _ => None,
        }
    }

    pub fn describe(&self, s: &Statement) -> String {
        let hyps: Vec<String> = s.hyps.iter().map(|&h| self.arena.display(h)).collect();
        format!(
            "[{}] ⊢ {} = {}",
            hyps.join(", "),
            self.arena.display(s.lhs),
            self.arena.display(s.rhs)
        )
    }

    /// Translate two Bool/Bv fragment terms into one bitvector solver
    /// arena. Int subterms are rejected (they belong to arithmetic).
    fn translate_pair(
        &self,
        lhs: Tid,
        rhs: Tid,
    ) -> Result<(crate::bv::Bv, crate::bv::Term, crate::bv::Term), KernelError> {
        let mut bv = crate::bv::Bv::new();
        let mut memo: HashMap<Tid, crate::bv::Term> = HashMap::new();
        let l = self.translate(lhs, &mut bv, &mut memo)?;
        let r = self.translate(rhs, &mut bv, &mut memo)?;
        Ok((bv, l, r))
    }

    fn translate(
        &self,
        t: Tid,
        bv: &mut crate::bv::Bv,
        memo: &mut HashMap<Tid, crate::bv::Term>,
    ) -> Result<crate::bv::Term, KernelError> {
        if let Some(&out) = memo.get(&t) {
            return Ok(out);
        }
        let node = self.arena.node(t).clone();
        let width = match self.arena.sort(t) {
            Sort::Bv(w) => w,
            Sort::Bool => 1,
            // Int-sorted terms enter bit-blasting only through the
            // value-preserving conversions; every value fits 64 bits.
            Sort::Int => 64,
        };
        let out = match node {
            Node::BoolConst(c) => bv.constant(u64::from(c), 1),
            Node::BvConst(v, w) => {
                assert!(w <= 64);
                bv.constant(v as u64, w)
            }
            Node::Var(sym) => {
                if matches!(self.arena.sort(t), Sort::Int) {
                    return Err(KernelError::Shape(format!(
                        "bitblast: bare Int variable {} not supported",
                        self.arena.var_name(sym)
                    )));
                }
                let id = crate::bv::VarId(sym.0);
                bv.var(id, width)
            }
            Node::BoolNot(a) => {
                let a = self.translate(a, bv, memo)?;
                bv.not(a)
            }
            Node::BoolAnd(a, b) => {
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.and(a, b)
            }
            Node::BoolOr(a, b) => {
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.or(a, b)
            }
            Node::BoolIte(c, x, y) => {
                let c = self.translate(c, bv, memo)?;
                let x = self.translate(x, bv, memo)?;
                let y = self.translate(y, bv, memo)?;
                bv.ite(c, x, y)
            }
            Node::BoolEq(a, b) => {
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.eq(a, b)
            }
            Node::BvBin(op, a, b) => {
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                let sop = match op {
                    crate::term::BvOp::And => crate::bv::BinOp::And,
                    crate::term::BvOp::Or => crate::bv::BinOp::Or,
                    crate::term::BvOp::Xor => crate::bv::BinOp::Xor,
                    crate::term::BvOp::Add => crate::bv::BinOp::Add,
                    crate::term::BvOp::Sub => crate::bv::BinOp::Sub,
                    crate::term::BvOp::Mul => crate::bv::BinOp::Mul,
                    crate::term::BvOp::Shl => crate::bv::BinOp::Shl,
                    crate::term::BvOp::Lshr => crate::bv::BinOp::Lshr,
                    crate::term::BvOp::Ashr => crate::bv::BinOp::Ashr,
                    crate::term::BvOp::Ult => crate::bv::BinOp::Ult,
                    crate::term::BvOp::Ule => crate::bv::BinOp::Ule,
                    crate::term::BvOp::Eq => crate::bv::BinOp::Eq,
                };
                bv.bin(sop, a, b)
            }
            Node::BvNot(a) => {
                let a = self.translate(a, bv, memo)?;
                bv.not(a)
            }
            Node::BvNeg(a) => {
                let a = self.translate(a, bv, memo)?;
                bv.neg(a)
            }
            Node::BvIte(c, x, y) => {
                let c = self.translate(c, bv, memo)?;
                let x = self.translate(x, bv, memo)?;
                let y = self.translate(y, bv, memo)?;
                bv.ite(c, x, y)
            }
            Node::BvExtract(a, hi, lo) => {
                let a = self.translate(a, bv, memo)?;
                bv.extract(a, hi, lo)
            }
            Node::BvConcat(a, b) => {
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.concat(a, b)
            }
            Node::BvZeroExt(a, w) => {
                let a = self.translate(a, bv, memo)?;
                bv.zero_ext(a, w)
            }
            Node::BvSignExt(a, w) => {
                let a = self.translate(a, bv, memo)?;
                bv.sign_ext(a, w)
            }
            Node::BvPopcount(a) => {
                let a = self.translate(a, bv, memo)?;
                bv.popcount(a)
            }
            Node::BvToInt(a) => {
                let at = self.translate(a, bv, memo)?;
                bv.zero_ext(at, 64)
            }
            Node::BoolToInt(a) => {
                let at = self.translate(a, bv, memo)?;
                bv.zero_ext(at, 64)
            }
            Node::IntToBv(a, w) => {
                let at = self.translate(a, bv, memo)?;
                if w < 64 {
                    bv.extract(at, w - 1, 0)
                } else {
                    at
                }
            }
            Node::IntConst(v) => {
                if v < 0 || v > u64::MAX as i128 {
                    return Err(KernelError::Shape(
                        "bitblast: integer constant out of 64-bit range".into(),
                    ));
                }
                bv.constant(v as u64, 64)
            }
            Node::IntAdd(a, b) => {
                self.require_exact_int(t)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.add(a, b)
            }
            Node::IntSub(a, b) => {
                self.require_exact_int(t)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.sub(a, b)
            }
            Node::IntMul(a, b) => {
                self.require_exact_int(t)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.mul(a, b)
            }
            Node::IntNeg(a) => {
                self.require_exact_int(t)?;
                let a = self.translate(a, bv, memo)?;
                bv.neg(a)
            }
            Node::IntIte(c, x, y) => {
                self.require_exact_int(t)?;
                let c = self.translate(c, bv, memo)?;
                let x = self.translate(x, bv, memo)?;
                let y = self.translate(y, bv, memo)?;
                bv.ite(c, x, y)
            }
            Node::App(_, _) => {
                // A free symbol: one fresh solver variable per term.
                // This is the free model, so an identity proven under
                // it holds under every interpretation. Int-sorted
                // applications belong to the arithmetic engine.
                match self.arena.sort(t) {
                    Sort::Bool | Sort::Bv(_) => {
                        let fresh = crate::bv::VarId(1_000_000 + t);
                        bv.var(fresh, width)
                    }
                    Sort::Int => {
                        return Err(KernelError::Shape(format!(
                            "bitblast: Int application {} belongs to arithmetic",
                            self.arena.display(t)
                        )))
                    }
                }
            }
            Node::IntLe(a, b) => {
                // Both sides are non-negative and fit 64 bits (checked),
                // so an unsigned comparison on the encodings is faithful.
                self.require_exact_int(a)?;
                self.require_exact_int(b)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.bin(crate::bv::BinOp::Ule, a, b)
            }
            Node::IntLt(a, b) => {
                self.require_exact_int(a)?;
                self.require_exact_int(b)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.bin(crate::bv::BinOp::Ult, a, b)
            }
            Node::IntEq(a, b) => {
                self.require_exact_int(a)?;
                self.require_exact_int(b)?;
                let a = self.translate(a, bv, memo)?;
                let b = self.translate(b, bv, memo)?;
                bv.bin(crate::bv::BinOp::Eq, a, b)
            }
        };
        memo.insert(t, out);
        Ok(out)
    }

    /// Reject Int arithmetic whose value range cannot be proven to fit
    /// in an unsigned 64-bit word. Without this check, translating
    /// integer arithmetic to wrapping bitvectors would not be faithful.
    fn require_exact_int(&self, t: Tid) -> Result<(), KernelError> {
        match self.int_bounds(t) {
            Some((lo, hi)) if lo >= 0 && (hi as u128) < (1u128 << 64) => Ok(()),
            Some((lo, hi)) => Err(KernelError::Shape(format!(
                "bitblast: integer range [{lo}, {hi}] does not fit in 64 bits exactly"
            ))),
            None => Err(KernelError::Shape(format!(
                "bitblast: cannot bound integer term {}",
                self.arena.display(t)
            ))),
        }
    }

    /// Conservative inclusive bounds for Int-sorted terms that can be
    /// translated.
    fn int_bounds(&self, t: Tid) -> Option<(i128, i128)> {
        match self.arena.node(t).clone() {
            Node::IntConst(v) => Some((v, v)),
            Node::BoolToInt(_) => Some((0, 1)),
            Node::BvToInt(a) => {
                // Track the value range through the bitvector structure:
                // zero-extension does not widen the represented value.
                self.bv_bounds(a)
            }
            Node::IntToBv(a, w) => {
                let _ = self.int_bounds(a)?;
                Some((0, (1i128 << w) - 1))
            }
            Node::IntAdd(a, b) => {
                let (la, ha) = self.int_bounds(a)?;
                let (lb, hb) = self.int_bounds(b)?;
                Some((la.checked_add(lb)?, ha.checked_add(hb)?))
            }
            Node::IntSub(a, b) => {
                let (la, ha) = self.int_bounds(a)?;
                let (lb, hb) = self.int_bounds(b)?;
                Some((la.checked_sub(hb)?, ha.checked_sub(lb)?))
            }
            Node::IntMul(a, b) => {
                let (la, ha) = self.int_bounds(a)?;
                let (lb, hb) = self.int_bounds(b)?;
                let candidates = [
                    la.checked_mul(lb)?,
                    la.checked_mul(hb)?,
                    ha.checked_mul(lb)?,
                    ha.checked_mul(hb)?,
                ];
                Some((*candidates.iter().min()?, *candidates.iter().max()?))
            }
            Node::IntNeg(a) => {
                let (lo, hi) = self.int_bounds(a)?;
                Some((hi.checked_neg()?, lo.checked_neg()?))
            }
            Node::IntIte(_, x, y) => {
                let (lx, hx) = self.int_bounds(x)?;
                let (ly, hy) = self.int_bounds(y)?;
                Some((lx.min(ly), hx.max(hy)))
            }
            _ => None,
        }
    }

    /// Conservative inclusive bounds for Bv-sorted terms.
    fn bv_bounds(&self, t: Tid) -> Option<(i128, i128)> {
        let width = match self.arena.sort(t) {
            Sort::Bv(w) => w,
            _ => return None,
        };
        if width >= 127 {
            return None;
        }
        let full = (0i128, (1i128 << width) - 1);
        match self.arena.node(t).clone() {
            Node::BvConst(v, _) => Some((v as i128, v as i128)),
            Node::Var(_) => Some(full),
            Node::BvZeroExt(a, _) => self.bv_bounds(a),
            Node::BvSignExt(a, w) => {
                let (lo, hi) = self.bv_bounds(a)?;
                let signed = |v: i128| -> i128 {
                    let sign = 1i128 << (w - 1);
                    if v & sign != 0 {
                        v - (1i128 << w)
                    } else {
                        v
                    }
                };
                Some((signed(lo), signed(hi)))
            }
            Node::BvBin(crate::term::BvOp::And, a, b) => {
                let (_, ha) = self.bv_bounds(a)?;
                let (_, hb) = self.bv_bounds(b)?;
                Some((0, ha.min(hb)))
            }
            Node::BvPopcount(a) => {
                let w = match self.arena.sort(a) {
                    Sort::Bv(w) => w,
                    _ => return None,
                };
                Some((0, w as i128))
            }
            Node::BvIte(_, x, y) => {
                let (lx, hx) = self.bv_bounds(x)?;
                let (ly, hy) = self.bv_bounds(y)?;
                Some((lx.min(ly), hx.max(hy)))
            }
            _ => Some(full),
        }
    }

    /// Replay a theorem: re-check every derivation step, then compare
    /// the replayed statement with the claimed one.
    pub fn replay(&mut self, thm: &Theorem) -> Result<(), KernelError> {
        let replayed = self.replay_derivation(&thm.derivation)?;
        if replayed.same_as(&thm.statement) {
            Ok(())
        } else {
            Err(KernelError::Shape(format!(
                "replay mismatch: claimed {}, replayed {}",
                self.describe(&thm.statement),
                self.describe(&replayed)
            )))
        }
    }

    fn replay_derivation(&mut self, d: &Derivation) -> Result<Statement, KernelError> {
        match d.clone() {
            Derivation::Refl(t) => Ok(Statement::new(vec![], t, t)),
            Derivation::Sym(inner) => {
                let s = self.replay_box(&inner)?;
                Ok(Statement::new(s.hyps, s.rhs, s.lhs))
            }
            Derivation::Trans(a, b) => {
                let sa = self.replay_box(&a)?;
                let sb = self.replay_box(&b)?;
                if sa.rhs != sb.lhs {
                    return Err(KernelError::Shape("replay trans mismatch".into()));
                }
                let mut hyps = sa.hyps.clone();
                hyps.extend(sb.hyps.iter().copied());
                Ok(Statement::new(hyps, sa.lhs, sb.rhs))
            }
            Derivation::Congr { context, inner } => {
                let s = self.replay_box(&inner)?;
                let hole_term = self.arena.var(context.hole);
                let mut m = HashMap::new();
                m.insert(hole_term, s.lhs);
                let lhs = self.arena.substitute(context.template, &m);
                let mut m2 = HashMap::new();
                m2.insert(hole_term, s.rhs);
                let rhs = self.arena.substitute(context.template, &m2);
                Ok(Statement::new(s.hyps, lhs, rhs))
            }
            Derivation::Hypothesis(h) => {
                let (l, r) = self
                    .equality_parts(h)
                    .ok_or_else(|| KernelError::Shape("replay hypothesis".into()))?;
                Ok(Statement::new(vec![h], l, r))
            }
            Derivation::HypothesisBool(h) => {
                if self.arena.sort(h) != Sort::Bool {
                    return Err(KernelError::Shape("replay bool hypothesis".into()));
                }
                let t = self.arena.bool(true);
                Ok(Statement::new(vec![h], h, t))
            }
            Derivation::CaseSplit {
                cond,
                case_true,
                case_false,
            } => {
                let st = self.replay_box(&case_true)?;
                let sf = self.replay_box(&case_false)?;
                if st.lhs != sf.lhs || st.rhs != sf.rhs {
                    return Err(KernelError::Shape("replay case_split branches".into()));
                }
                let neg = self.arena.bool_not(cond);
                if !st.hyps.contains(&cond) || !sf.hyps.contains(&neg) {
                    return Err(KernelError::Shape("replay case_split hypotheses".into()));
                }
                let mut hyps: Vec<Tid> =
                    st.hyps.iter().copied().filter(|&h| h != cond).collect();
                hyps.extend(sf.hyps.iter().copied().filter(|&h| h != neg));
                Ok(Statement::new(hyps, st.lhs, st.rhs))
            }
            Derivation::Weaken { inner, extra } => {
                let s = self.replay_box(&inner)?;
                let mut hyps = s.hyps.clone();
                hyps.extend(extra.iter().copied());
                Ok(Statement::new(hyps, s.lhs, s.rhs))
            }
            Derivation::Instantiate { inner, map } => {
                let s = self.replay_box(&inner)?;
                let thm = Theorem {
                    statement: s,
                    derivation: Derivation::Refl(0),
                };
                let out = self.instantiate(&thm, &map)?;
                Ok(out.statement)
            }
            Derivation::Schema { id, map, digest } => {
                let recorded = self
                    .schemas
                    .get(id.0)
                    .cloned()
                    .ok_or_else(|| KernelError::UnknownSchema(format!("{id:?}")))?;
                if self.schema_digest(id) != digest {
                    return Err(KernelError::AxiomMismatch {
                        name: format!("schema:{}", recorded.name),
                        expected: format!("{:016x}", digest),
                        actual: format!("{:016x}", self.schema_digest(id)),
                    });
                }
                let mut tid_map = HashMap::new();
                for &(param, arg) in &map {
                    tid_map.insert(self.arena.var(param), arg);
                }
                let lhs = self.arena.substitute(recorded.lhs, &tid_map);
                let rhs = self.arena.substitute(recorded.rhs, &tid_map);
                let hyps: Vec<Tid> = match recorded.guard {
                    Some(g) => vec![self.arena.substitute(g, &tid_map)],
                    None => vec![],
                };
                Ok(Statement::new(hyps, lhs, rhs))
            }
            Derivation::Arith { goal, .. } => {
                let valid = match self.arena.sort(goal.lhs) {
                    Sort::Int => {
                        crate::arith::linearize(&self.arena, goal.lhs)
                            == crate::arith::linearize(&self.arena, goal.rhs)
                    }
                    Sort::Bool => {
                        if goal.rhs == self.arena.bool(true) {
                            prove_entailment(&self.arena, &goal.hyps, goal.lhs)
                                == ArithOutcome::Valid
                        } else {
                            false
                        }
                    }
                    Sort::Bv(_) => false,
                };
                if !valid {
                    return Err(KernelError::ArithFailed("replay arith".into()));
                }
                Ok(goal)
            }
            Derivation::BitBlast { goal, .. } => {
                if goal.hyps.is_empty() {
                    let (bv, l, r) = self.translate_pair(goal.lhs, goal.rhs)?;
                    match bv_prove_equal(&bv, l, r) {
                        CheckResult::Valid => Ok(goal),
                        other => Err(KernelError::SolverFailed(format!(
                            "replay bitblast: {other:?}"
                        ))),
                    }
                } else {
                    let mut terms: Vec<Tid> = goal.hyps.clone();
                    terms.push(goal.lhs);
                    terms.push(goal.rhs);
                    let (bv, tr) = self.translate_terms(&terms)?;
                    let n = goal.hyps.len();
                    match crate::equiv::prove_implied_equal(&bv, &tr[..n], tr[n], tr[n + 1]) {
                        CheckResult::Valid => Ok(goal),
                        other => Err(KernelError::SolverFailed(format!(
                            "replay bitblast (assumed): {other:?}"
                        ))),
                    }
                }
            }
            Derivation::DischargeLemma { inner, hyp, lemma } => {
                let s = self.replay_box(&inner)?;
                let l = self.replay_box(&lemma)?;
                let true_t = self.arena.bool(true);
                let matches = (l.lhs == hyp && l.rhs == true_t)
                    || self.equality_parts(hyp) == Some((l.lhs, l.rhs));
                if !matches {
                    return Err(KernelError::Shape("replay discharge_lemma".into()));
                }
                if !s.hyps.contains(&hyp) {
                    return Err(KernelError::Shape("replay discharge_lemma: no hyp".into()));
                }
                if l.hyps.iter().any(|h| !s.hyps.contains(h)) {
                    return Err(KernelError::Shape("replay discharge_lemma: hyps".into()));
                }
                let hyps = s.hyps.into_iter().filter(|&h| h != hyp).collect();
                Ok(Statement::new(hyps, s.lhs, s.rhs))
            }
            Derivation::DischargeArith { inner, hyp } => {
                let s = self.replay_box(&inner)?;
                if !s.hyps.contains(&hyp) {
                    return Err(KernelError::Shape("replay discharge_arith: no hyp".into()));
                }
                let remaining: Vec<Tid> = s.hyps.iter().copied().filter(|&h| h != hyp).collect();
                if !self.entails(&remaining, hyp) {
                    return Err(KernelError::ArithFailed("replay discharge_arith".into()));
                }
                Ok(Statement::new(remaining, s.lhs, s.rhs))
            }
            Derivation::Induction {
                var,
                template,
                base,
                cases,
            } => {
                let bs = self.replay_box(&base)?;
                let base_thm = Theorem {
                    statement: bs,
                    derivation: Derivation::Refl(0),
                };
                let mut replayed = Vec::new();
                for (lit, d) in &cases {
                    let s = self.replay_box(d)?;
                    replayed.push((
                        *lit,
                        Theorem {
                            statement: s,
                            derivation: Derivation::Refl(0),
                        },
                    ));
                }
                let out = self.nat_induction_cases(var, &template, &base_thm, &replayed)?;
                Ok(out.statement)
            }
            Derivation::Axiom { name, statement } => {
                let rec = self
                    .axioms
                    .iter()
                    .find(|a| a.name == name)
                    .ok_or_else(|| KernelError::UnknownAxiom(name.clone()))?;
                let digest = self.statement_digest(&statement);
                if digest != rec.digest {
                    return Err(KernelError::AxiomMismatch {
                        name: name.clone(),
                        expected: format!("{:016x}", rec.digest),
                        actual: format!("{:016x}", digest),
                    });
                }
                Ok(statement)
            }
        }
    }

    fn replay_box(&mut self, d: &Derivation) -> Result<Statement, KernelError> {
        self.replay_derivation(d)
    }
}
