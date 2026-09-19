//! Family registry: the extension point for optimization families.
//!
//! A family recognizes *one* eligible loop per call, proves it with the
//! shared law/backend machinery, and returns the patched function. The
//! driver runs the registry to a bounded fixed point. Two families must
//! use an abstraction before it becomes a framework.

use sir_analysis::facts::FactDatabase;
use sir_analysis::manager::AnalysisManager;
use sir_nodes::Function;

use crate::{
    all, any, arithmetic, count, parity, popcount, runtime_count, runtime_sum, search, sum,
};

/// One successful family rewrite.
pub struct Applied {
    pub function: Function,
    pub definition: u64,
}

/// An optimization family.
pub trait Family: Send + Sync {
    fn name(&self) -> &'static str;
    /// Definitions this family can produce.
    fn definitions(&self) -> &'static [u64];
    /// Recognize the first eligible loop, prove it, and patch it.
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied>;
}

/// Run `f` with a fresh analysis of `function`.
pub fn with_facts<T>(function: &Function, f: impl FnOnce(&FactDatabase) -> T) -> T {
    let mut analysis = AnalysisManager::new();
    analysis.run_all(function);
    f(analysis.database())
}

/// Policy hook: rewrite every recognized definition.
pub fn allow_all(_definition: u64) -> bool {
    true
}

/// Default policy when no predictive profile exists: runtime-dependent
/// rewrites stay off. Definition 403 (runtime `memchr`) measured 1.25×
/// on no-hit inputs but 0.82× on early-hit inputs, so it is opt-in until
/// the cost model can choose per call.
pub fn allow_profiled(definition: u64) -> bool {
    definition != 403
}

struct SearchFamily;
impl Family for SearchFamily {
    fn name(&self) -> &'static str {
        "search"
    }
    fn definitions(&self) -> &'static [u64] {
        &[200, 402, 403]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = search::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

struct ArithmeticFamily;
impl Family for ArithmeticFamily {
    fn name(&self) -> &'static str {
        "arithmetic"
    }
    fn definitions(&self) -> &'static [u64] {
        &[100, 101, 102, 103]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = arithmetic::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

struct CountFamily;
impl Family for CountFamily {
    fn name(&self) -> &'static str {
        "count"
    }
    fn definitions(&self) -> &'static [u64] {
        &[0]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        Some(Applied {
            function: count::apply_one(function, facts)?,
            definition: 0,
        })
    }
}

struct AnyFamily;
impl Family for AnyFamily {
    fn name(&self) -> &'static str {
        "any"
    }
    fn definitions(&self) -> &'static [u64] {
        &[4]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        Some(Applied {
            function: any::apply_one(function, facts)?,
            definition: 4,
        })
    }
}

struct AllFamily;
impl Family for AllFamily {
    fn name(&self) -> &'static str {
        "all"
    }
    fn definitions(&self) -> &'static [u64] {
        &[5]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        Some(Applied {
            function: all::apply_one(function, facts)?,
            definition: 5,
        })
    }
}

struct SumFamily;
impl Family for SumFamily {
    fn name(&self) -> &'static str {
        "sum"
    }
    fn definitions(&self) -> &'static [u64] {
        &[400, 401]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = sum::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

struct ParityFamily;
impl Family for ParityFamily {
    fn name(&self) -> &'static str {
        "parity"
    }
    fn definitions(&self) -> &'static [u64] {
        &[6]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        Some(Applied {
            function: parity::apply_one(function, facts)?,
            definition: 6,
        })
    }
}

struct PopcountFamily;
impl Family for PopcountFamily {
    fn name(&self) -> &'static str {
        "popcount"
    }
    fn definitions(&self) -> &'static [u64] {
        &[7]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = popcount::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

struct RuntimeSumFamily;
impl Family for RuntimeSumFamily {
    fn name(&self) -> &'static str {
        "runtime-sum"
    }
    fn definitions(&self) -> &'static [u64] {
        &[404]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = runtime_sum::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

struct RuntimeCountFamily;
impl Family for RuntimeCountFamily {
    fn name(&self) -> &'static str {
        "runtime-count"
    }
    fn definitions(&self) -> &'static [u64] {
        &[405]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = runtime_count::apply_one(function, facts)?;
        Some(Applied {
            function,
            definition,
        })
    }
}

/// Built-in family registry. Adding a family is one module plus one
/// entry here.
pub fn registry() -> Vec<Box<dyn Family>> {
    vec![
        Box::new(ArithmeticFamily),
        Box::new(SearchFamily),
        Box::new(CountFamily),
        Box::new(AnyFamily),
        Box::new(AllFamily),
        Box::new(SumFamily),
        Box::new(ParityFamily),
        Box::new(PopcountFamily),
        Box::new(RuntimeSumFamily),
        Box::new(RuntimeCountFamily),
    ]
}
