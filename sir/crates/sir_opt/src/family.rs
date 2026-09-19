//! Family registry: the extension point for optimization families.
//!
//! A family recognizes *one* eligible loop per call, proves it with the
//! shared law/backend machinery, and returns the patched function. The
//! driver runs the registry to a bounded fixed point. Two families must
//! use an abstraction before it becomes a framework.

use sir_analysis::facts::FactDatabase;
use sir_analysis::manager::AnalysisManager;
use sir_nodes::Function;

use crate::{all, any, count, parity, search, sum};

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

struct SearchFamily;
impl Family for SearchFamily {
    fn name(&self) -> &'static str {
        "search"
    }
    fn definitions(&self) -> &'static [u64] {
        &[200, 402]
    }
    fn apply_one(&self, function: &Function, facts: &FactDatabase) -> Option<Applied> {
        let (function, definition) = search::apply_one(function, facts)?;
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

/// Built-in family registry. Adding a family is one module plus one
/// entry here.
pub fn registry() -> Vec<Box<dyn Family>> {
    vec![
        Box::new(SearchFamily),
        Box::new(CountFamily),
        Box::new(AnyFamily),
        Box::new(AllFamily),
        Box::new(SumFamily),
        Box::new(ParityFamily),
    ]
}
