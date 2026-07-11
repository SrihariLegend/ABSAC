//! The AnalysisManager: lazy execution, caching, and invalidation.
//!
//! The manager owns the FactDatabase and orchestrates analysis
//! execution. Analyses are computed on demand and cached per-function.

use sir_nodes::Function;
use std::any::TypeId;
use std::time::Instant;

use crate::analysis::{Analysis, AnalysisResult};
use crate::cache::AnalysisCache;
use crate::facts::FactDatabase;
use crate::{alias, constants, dominance, escape, loops, purity, ranges, use_def, value_numbering};

/// Statistics collected across all analysis runs.
#[derive(Clone, Debug, Default)]
pub struct AnalysisStats {
    /// Total number of analysis runs (including cache hits).
    pub total_runs: usize,
    /// Number of cache hits.
    pub cache_hits: usize,
    /// Number of cache misses (actual computation).
    pub cache_misses: usize,
    /// Total wall-clock time spent in analysis.
    pub total_runtime_ms: u64,
    /// Total nodes processed.
    pub total_nodes_processed: usize,
}

/// The AnalysisManager coordinates lazy execution of all analyses.
///
/// Analyses are identified by their Rust type (via `TypeId`).
/// Calling `get::<UseDefAnalysis>(&func)` returns the cached result
/// or computes it if not yet cached.
pub struct AnalysisManager {
    db: FactDatabase,
    cache: AnalysisCache,
    stats: AnalysisStats,
}

impl AnalysisManager {
    /// Create a new analysis manager with an empty database.
    pub fn new() -> Self {
        Self {
            db: FactDatabase::new(),
            cache: AnalysisCache::new(),
            stats: AnalysisStats::default(),
        }
    }

    /// Get the fact database (read-only).
    pub fn database(&self) -> &FactDatabase {
        &self.db
    }

    /// Get analysis statistics.
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }

    /// Run all available analyses on a function and populate the database.
    pub fn run_all(&mut self, func: &Function) {
        self.run_use_def(func);
        self.run_dominance(func);
        self.run_constants(func);
        self.run_purity(func);
        self.run_ranges(func);
        self.run_alias(func);
        self.run_escape(func);
        self.run_loops(func);
        self.run_value_numbering(func);
    }

    /// Invalidate all cached results. Future calls will recompute.
    pub fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
        self.db = FactDatabase::new();
    }

    // ── Analysis-specific runners ───────────────────────────

    fn run_use_def(&mut self, func: &Function) {
        let id = TypeId::of::<UseDefAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = use_def::run_use_def(func);
        self.db.use_def = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_dominance(&mut self, func: &Function) {
        let id = TypeId::of::<DominanceAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = dominance::run_dominance(func);
        self.db.dominance = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_constants(&mut self, func: &Function) {
        let id = TypeId::of::<ConstantAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = constants::run_constants(func, Some(&self.db.use_def));
        self.db.constants = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_purity(&mut self, func: &Function) {
        let id = TypeId::of::<PurityAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = purity::run_purity(func);
        self.db.purity = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_ranges(&mut self, func: &Function) {
        let id = TypeId::of::<RangeAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = ranges::run_ranges(func);
        self.db.ranges = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_alias(&mut self, func: &Function) {
        let id = TypeId::of::<AliasAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = alias::run_alias(func);
        self.db.aliases = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_escape(&mut self, func: &Function) {
        let id = TypeId::of::<EscapeAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = escape::run_escape(func);
        self.db.escapes = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_loops(&mut self, func: &Function) {
        let id = TypeId::of::<LoopAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = loops::run_loops(func);
        self.db.loops = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }

    fn run_value_numbering(&mut self, func: &Function) {
        let id = TypeId::of::<ValueNumberingAnalysis>();
        if self.cache.is_valid(id, func) {
            self.stats.cache_hits += 1;
            return;
        }
        self.stats.cache_misses += 1;
        let start = Instant::now();
        let result = value_numbering::run_value_numbering(func);
        self.db.value_numbers = result;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
        self.cache.set_valid(id, func);
        self.stats.total_runs += 1;
    }
}

// ── Analysis type definitions ──────────────────────────────

/// Use-Definition analysis.
pub struct UseDefAnalysis;
impl Analysis for UseDefAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::UseDefFact>;
    fn name() -> &'static str {
        "UseDef"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = use_def::run_use_def(func);
        let runtime = start.elapsed();
        AnalysisResult::new(result, runtime, func.node_count())
    }
}

/// Dominance analysis.
pub struct DominanceAnalysis;
impl Analysis for DominanceAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::DominanceFact>;
    fn name() -> &'static str {
        "Dominance"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = dominance::run_dominance(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Constant propagation analysis.
pub struct ConstantAnalysis;
impl Analysis for ConstantAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::ConstantFact>;
    fn name() -> &'static str {
        "Constants"
    }
    fn analyze(func: &Function, facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let use_def = facts.map(|db| &db.use_def);
        let result = constants::run_constants(func, use_def);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Purity analysis.
pub struct PurityAnalysis;
impl Analysis for PurityAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::PurityFact>;
    fn name() -> &'static str {
        "Purity"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = purity::run_purity(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Range analysis.
pub struct RangeAnalysis;
impl Analysis for RangeAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::RangeFact>;
    fn name() -> &'static str {
        "Ranges"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = ranges::run_ranges(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Alias analysis.
pub struct AliasAnalysis;
impl Analysis for AliasAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::AliasFact>;
    fn name() -> &'static str {
        "Alias"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = alias::run_alias(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Escape analysis.
pub struct EscapeAnalysis;
impl Analysis for EscapeAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::EscapeFact>;
    fn name() -> &'static str {
        "Escape"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = escape::run_escape(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Loop analysis.
pub struct LoopAnalysis;
impl Analysis for LoopAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::LoopFact>;
    fn name() -> &'static str {
        "Loops"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = loops::run_loops(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

/// Value numbering analysis.
pub struct ValueNumberingAnalysis;
impl Analysis for ValueNumberingAnalysis {
    type Output = std::collections::HashMap<sir_types::NodeId, crate::facts::ValueNumberFact>;
    fn name() -> &'static str {
        "ValueNumbering"
    }
    fn analyze(func: &Function, _facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output> {
        let start = Instant::now();
        let result = value_numbering::run_value_numbering(func);
        AnalysisResult::new(result, start.elapsed(), func.node_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::{Span, Type};

    fn i32_type() -> Type {
        Type::i32()
    }
    fn unknown_span() -> Span {
        Span::unknown()
    }

    fn build_add() -> Function {
        let mut b = Builder::new("add", &[("a", i32_type()), ("b", i32_type())], i32_type());
        let a = b.parameter_index(0).unwrap();
        let b_param = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_param, unknown_span()).unwrap();
        b.return_value(sum, unknown_span()).unwrap();
        b.build()
    }

    #[test]
    fn manager_run_all_populates_database() {
        let func = build_add();
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&func);

        let db = mgr.database();
        assert!(!db.use_def.is_empty());
        assert!(!db.dominance.is_empty());
        assert!(!db.constants.is_empty());
        assert!(!db.purity.is_empty());
        assert!(!db.ranges.is_empty());
        assert!(!db.value_numbers.is_empty());
        assert!(db.total_facts() > 0);
    }

    #[test]
    fn manager_cache_hit_on_second_run() {
        let func = build_add();
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&func);
        let stats1 = mgr.stats().clone();

        // Run again — should be cache hits.
        mgr.run_all(&func);
        let stats2 = mgr.stats();

        assert!(stats2.cache_hits > stats1.cache_hits);
    }

    #[test]
    fn manager_invalidate_clears_everything() {
        let func = build_add();
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&func);
        mgr.invalidate_all();

        let db = mgr.database();
        assert!(db.is_empty());
    }

    #[test]
    fn individual_analysis_via_trait() {
        let func = build_add();
        let result = UseDefAnalysis::analyze(&func, None);
        assert!(result.nodes_processed > 0);
    }
}
