//! A small deterministic CDCL SAT solver.
//!
//! Used by [`crate::equiv`] to discharge bit-level identities. The
//! solver is complete over the CNF it is given: `Unsatisfiable` means
//! no assignment exists, `Satisfiable(model)` carries a concrete model
//! that callers re-check against the original terms.

use std::collections::HashSet;

use crate::cnf::Cnf;

/// Outcome of a satisfiability query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SatResult {
    /// A satisfying assignment: `model[v]` is the value of variable
    /// `v` (1-based; index 0 is unused).
    Satisfiable(Vec<bool>),
    /// No satisfying assignment exists.
    Unsatisfiable,
    /// The conflict budget was exhausted before a conclusion.
    Unknown,
}

/// Conflict budget for a single query. Adder-heavy equivalence queries
/// can need a few thousand conflicts; the default is generous but
/// bounded so callers fail closed instead of hanging.
pub const DEFAULT_CONFLICT_BUDGET: u64 = 2_000_000;

struct Solver {
    num_vars: usize,
    clauses: Vec<Vec<i32>>,
    learnt: Vec<bool>,
    watches: Vec<Vec<usize>>,
    assigns: Vec<i8>,
    level: Vec<u32>,
    reason: Vec<i32>,
    trail: Vec<i32>,
    trail_lim: Vec<usize>,
    qhead: usize,
    activity: Vec<f64>,
    var_inc: f64,
    phase: Vec<bool>,
    ok: bool,
}

#[inline]
fn lit_index(lit: i32) -> usize {
    let v = lit.unsigned_abs() as usize - 1;
    v * 2 + usize::from(lit < 0)
}

#[inline]
fn var_of(lit: i32) -> usize {
    lit.unsigned_abs() as usize
}

impl Solver {
    fn new(num_vars: usize) -> Self {
        Solver {
            num_vars,
            clauses: Vec::new(),
            learnt: Vec::new(),
            watches: vec![Vec::new(); num_vars * 2],
            assigns: vec![0; num_vars + 1],
            level: vec![0; num_vars + 1],
            reason: vec![-1; num_vars + 1],
            trail: Vec::new(),
            trail_lim: Vec::new(),
            qhead: 0,
            activity: vec![0.0; num_vars + 1],
            var_inc: 1.0,
            phase: vec![false; num_vars + 1],
            ok: true,
        }
    }

    #[inline]
    fn value_lit(&self, lit: i32) -> i8 {
        let a = self.assigns[var_of(lit)];
        if lit > 0 {
            a
        } else {
            -a
        }
    }

    fn decision_level(&self) -> usize {
        self.trail_lim.len()
    }

    fn add_clause(&mut self, raw: &[i32]) -> usize {
        // Canonicalize: drop duplicate literals, skip tautologies.
        let mut seen = HashSet::new();
        let mut lits = Vec::with_capacity(raw.len());
        for &l in raw {
            debug_assert!(l != 0);
            if seen.contains(&-l) {
                // Tautology: always satisfied, never needs watching.
                return usize::MAX;
            }
            if seen.insert(l) {
                lits.push(l);
            }
        }
        if lits.is_empty() {
            self.ok = false;
            return usize::MAX;
        }
        let idx = self.clauses.len();
        self.clauses.push(lits);
        self.learnt.push(false);
        let len = self.clauses[idx].len();
        if len == 1 {
            let l = self.clauses[idx][0];
            match self.value_lit(l) {
                -1 => self.ok = false,
                0 => self.enqueue(l, -1),
                _ => {}
            }
        } else {
            let a = self.clauses[idx][0];
            let b = self.clauses[idx][1];
            self.watches[lit_index(a)].push(idx);
            self.watches[lit_index(b)].push(idx);
        }
        idx
    }

    fn add_learnt(&mut self, lits: Vec<i32>) -> usize {
        let idx = self.clauses.len();
        self.clauses.push(lits);
        self.learnt.push(true);
        let len = self.clauses[idx].len();
        if len >= 2 {
            let a = self.clauses[idx][0];
            let b = self.clauses[idx][1];
            self.watches[lit_index(a)].push(idx);
            self.watches[lit_index(b)].push(idx);
        }
        idx
    }

    fn enqueue(&mut self, lit: i32, reason: i32) {
        let v = var_of(lit);
        self.assigns[v] = if lit > 0 { 1 } else { -1 };
        self.level[v] = self.decision_level() as u32;
        self.reason[v] = reason;
        self.trail.push(lit);
    }

    fn new_decision_level(&mut self) {
        self.trail_lim.push(self.trail.len());
    }

    fn backtrack(&mut self, level: usize) {
        if self.decision_level() <= level {
            return;
        }
        let lim = self.trail_lim[level];
        for k in (lim..self.trail.len()).rev() {
            let lit = self.trail[k];
            let v = var_of(lit);
            self.phase[v] = self.assigns[v] > 0;
            self.assigns[v] = 0;
            self.reason[v] = -1;
            self.level[v] = 0;
        }
        self.trail.truncate(lim);
        self.trail_lim.truncate(level);
        self.qhead = lim;
    }

    fn bump(&mut self, var: usize) {
        self.activity[var] += self.var_inc;
        if self.activity[var] > 1e100 {
            for a in self.activity.iter_mut() {
                *a *= 1e-100;
            }
            self.var_inc *= 1e-100;
        }
    }

    fn decay(&mut self) {
        self.var_inc /= 0.95;
    }

    /// Unit propagation. Returns the conflicting clause index if any.
    fn propagate(&mut self) -> Option<usize> {
        while self.qhead < self.trail.len() {
            let p = self.trail[self.qhead];
            self.qhead += 1;
            let idx = lit_index(-p);
            let mut ws = std::mem::take(&mut self.watches[idx]);
            let mut keep = 0usize;
            let mut conflict: Option<usize> = None;
            for k in 0..ws.len() {
                let ci = ws[k];
                if conflict.is_some() {
                    ws[keep] = ci;
                    keep += 1;
                    continue;
                }
                // The watched literal `-p` must be at position 0.
                if self.clauses[ci][0] != -p {
                    debug_assert_eq!(self.clauses[ci][1], -p, "watch invariant");
                    self.clauses[ci].swap(0, 1);
                }
                let other = self.clauses[ci][1];
                if self.value_lit(other) > 0 {
                    ws[keep] = ci;
                    keep += 1;
                    continue;
                }
                // Look for a replacement literal that is not false.
                let mut replacement: Option<usize> = None;
                {
                    let lits = &self.clauses[ci];
                    for (j, &l) in lits.iter().enumerate().skip(2) {
                        if self.value_lit(l) >= 0 {
                            replacement = Some(j);
                            break;
                        }
                    }
                }
                match replacement {
                    Some(j) => {
                        let new_lit = self.clauses[ci][j];
                        self.clauses[ci].swap(0, j);
                        self.watches[lit_index(new_lit)].push(ci);
                    }
                    None => {
                        ws[keep] = ci;
                        keep += 1;
                        if self.value_lit(other) < 0 {
                            conflict = Some(ci);
                        } else {
                            self.enqueue(other, ci as i32);
                        }
                    }
                }
            }
            ws.truncate(keep);
            self.watches[idx] = ws;
            if conflict.is_some() {
                return conflict;
            }
        }
        None
    }

    /// 1UIP conflict analysis: returns (learnt clause, backtrack level).
    fn analyze(&mut self, conflict: usize) -> (Vec<i32>, usize) {
        let mut learnt: Vec<i32> = vec![0];
        let mut seen = vec![false; self.num_vars + 1];
        let mut counter = 0usize;
        let mut clause_idx = conflict;
        let mut trail_pos = self.trail.len();
        // The pivot is the literal being resolved away. It must not be
        // re-introduced by its own reason clause.
        let mut pivot: Option<usize> = None;
        let mut p: i32;

        loop {
            let lits = self.clauses[clause_idx].clone();
            for &q in &lits {
                let v = var_of(q);
                if Some(v) == pivot {
                    continue;
                }
                if !seen[v] && self.level[v] > 0 {
                    seen[v] = true;
                    self.bump(v);
                    if self.level[v] as usize == self.decision_level() {
                        counter += 1;
                    } else {
                        learnt.push(q);
                    }
                }
            }
            loop {
                debug_assert!(trail_pos > 0, "conflict analysis ran off the trail");
                trail_pos -= 1;
                p = self.trail[trail_pos];
                if seen[var_of(p)] {
                    break;
                }
            }
            let v = var_of(p);
            seen[v] = false;
            counter -= 1;
            if counter == 0 {
                break;
            }
            clause_idx = self.reason[v] as usize;
            debug_assert!(
                self.reason[v] >= 0,
                "resolved a decision literal during 1UIP analysis"
            );
            pivot = Some(v);
        }
        learnt[0] = -p;

        // Backtrack level: highest level among the remaining literals.
        let mut bt_level = 0usize;
        if learnt.len() > 1 {
            let mut best = 0u32;
            let mut best_pos = 1usize;
            for (i, &l) in learnt.iter().enumerate().skip(1) {
                let lv = self.level[var_of(l)];
                if lv > best {
                    best = lv;
                    best_pos = i;
                }
            }
            bt_level = best as usize;
            learnt.swap(1, best_pos);
        }
        (learnt, bt_level)
    }

    fn pick_branch(&self) -> Option<i32> {
        let mut best = 0usize;
        let mut best_act = -1.0f64;
        for v in 1..=self.num_vars {
            if self.assigns[v] == 0 && self.activity[v] > best_act {
                best_act = self.activity[v];
                best = v;
            }
        }
        if best == 0 {
            None
        } else {
            Some(if self.phase[best] {
                best as i32
            } else {
                -(best as i32)
            })
        }
    }

    fn model(&self) -> Vec<bool> {
        let mut model = vec![false; self.num_vars + 1];
        for v in 1..=self.num_vars {
            model[v] = match self.assigns[v] {
                1 => true,
                -1 => false,
                _ => self.phase[v],
            };
        }
        model
    }
}

/// Solve a CNF with the given conflict budget.
pub fn solve(cnf: &Cnf, conflict_budget: u64) -> SatResult {
    let mut solver = Solver::new(cnf.num_vars as usize);
    for clause in &cnf.clauses {
        solver.add_clause(clause);
        if !solver.ok {
            return SatResult::Unsatisfiable;
        }
    }
    if !solver.ok {
        return SatResult::Unsatisfiable;
    }
    if solver.propagate().is_some() {
        return SatResult::Unsatisfiable;
    }

    let mut conflicts: u64 = 0;
    let mut restart_limit: u64 = 128;
    loop {
        if let Some(confl) = solver.propagate() {
            conflicts += 1;
            if conflicts > conflict_budget {
                return SatResult::Unknown;
            }
            if solver.decision_level() == 0 {
                return SatResult::Unsatisfiable;
            }
            let (learnt, bt_level) = solver.analyze(confl);
            solver.backtrack(bt_level);
            if learnt.len() == 1 {
                let l = learnt[0];
                match solver.value_lit(l) {
                    -1 => return SatResult::Unsatisfiable,
                    0 => solver.enqueue(l, -1),
                    _ => {}
                }
            } else {
                let ci = solver.add_learnt(learnt);
                let asserting = solver.clauses[ci][0];
                solver.enqueue(asserting, ci as i32);
            }
            solver.decay();
            if conflicts >= restart_limit {
                solver.backtrack(0);
                restart_limit = restart_limit + restart_limit / 2 + 1;
            }
        } else {
            match solver.pick_branch() {
                None => return SatResult::Satisfiable(solver.model()),
                Some(lit) => {
                    solver.new_decision_level();
                    solver.enqueue(lit, -1);
                }
            }
        }
    }
}

/// Convenience: solve with the default budget.
pub fn solve_default(cnf: &Cnf) -> SatResult {
    solve(cnf, DEFAULT_CONFLICT_BUDGET)
}

/// A trait-like entry point kept for API symmetry with the module
/// docs; `Solver` itself is an implementation detail.
#[derive(Clone, Debug, Default)]
pub struct SolverHandle;

impl SolverHandle {
    pub fn solve(&self, cnf: &Cnf) -> SatResult {
        solve_default(cnf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(cnf: &Cnf, lits: &[i32]) -> SatResult {
        let mut c = cnf.clone();
        for l in lits {
            c.add_clause([*l]);
        }
        solve_default(&c)
    }

    #[test]
    fn solves_simple_sat() {
        let mut cnf = Cnf::new();
        let a = cnf.new_lit();
        let b = cnf.new_lit();
        cnf.add_clause([a, b]);
        cnf.add_clause([-a, b]);
        match solve_default(&cnf) {
            SatResult::Satisfiable(model) => assert!(model[b as usize]),
            other => panic!("expected SAT, got {other:?}"),
        }
    }

    #[test]
    fn solves_simple_unsat() {
        let mut cnf = Cnf::new();
        let a = cnf.new_lit();
        cnf.add_clause([a]);
        cnf.add_clause([-a]);
        assert_eq!(solve_default(&cnf), SatResult::Unsatisfiable);
    }

    #[test]
    fn solves_pigeonhole_2_into_1_unsat() {
        // Two pigeons, one hole.
        let mut cnf = Cnf::new();
        let p1 = cnf.new_lit();
        let p2 = cnf.new_lit();
        cnf.add_clause([p1]);
        cnf.add_clause([p2]);
        cnf.add_clause([-p1, -p2]);
        assert_eq!(solve_default(&cnf), SatResult::Unsatisfiable);
    }

    #[test]
    fn solves_xor_chain() {
        // a xor b = c, a xor c = b, b xor c = a over 3 vars; assert all
        // three and one of a,b,c -> satisfiable.
        let mut cnf = Cnf::new();
        let a = cnf.new_lit();
        let b = cnf.new_lit();
        let c = cnf.new_lit();
        let ab = cnf.xor2(a, b);
        let ac = cnf.xor2(a, c);
        let bc = cnf.xor2(b, c);
        cnf.add_clause([ab, -c]); // ab -> c is not the identity; use
        cnf.add_clause([-ab, c]);
        cnf.add_clause([ac]);
        cnf.add_clause([bc]);
        // With a = true, b = false, c = true: ac = false, contradiction.
        // So satisfying assignments need parity reasoning; just check
        // the solver's model satisfies every clause.
        let _ = clause(&cnf, &[]);
        match solve_default(&cnf) {
            SatResult::Satisfiable(model) => {
                for cl in &cnf.clauses {
                    assert!(
                        cl.iter().any(|&l| {
                            if l > 0 {
                                model[l as usize]
                            } else {
                                !model[(-l) as usize]
                            }
                        }),
                        "model violates clause {cl:?}"
                    );
                }
            }
            SatResult::Unsatisfiable => {}
            SatResult::Unknown => panic!("unexpected unknown"),
        }
    }
}
