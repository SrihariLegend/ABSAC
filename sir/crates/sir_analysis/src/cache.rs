//! Analysis result cache.
//!
//! Tracks which analyses have been computed for which functions
//! using a function fingerprint that includes node content.

use std::any::TypeId;
use std::collections::HashMap;
use sir_nodes::Function;

/// State of a cached analysis result.
#[derive(Clone, Debug)]
pub(crate) struct CacheEntry {
    /// Hash of the function this was computed for.
    pub function_hash: u64,
    /// Whether the result is currently valid.
    pub valid: bool,
}

/// Cache for analysis results.
#[derive(Clone, Debug, Default)]
pub struct AnalysisCache {
    entries: HashMap<TypeId, CacheEntry>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a cached result is valid for the given function.
    pub fn is_valid(&self, analysis_id: TypeId, func: &Function) -> bool {
        match self.entries.get(&analysis_id) {
            Some(entry) => entry.valid && entry.function_hash == hash_function(func),
            None => false,
        }
    }

    /// Mark an analysis as cached for the given function.
    pub fn set_valid(&mut self, analysis_id: TypeId, func: &Function) {
        self.entries.insert(
            analysis_id,
            CacheEntry {
                function_hash: hash_function(func),
                valid: true,
            },
        );
    }

    /// Invalidate a single analysis.
    pub fn invalidate(&mut self, analysis_id: TypeId) {
        if let Some(entry) = self.entries.get_mut(&analysis_id) {
            entry.valid = false;
        }
    }

    /// Invalidate all cached analyses.
    #[allow(dead_code)]
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    /// Return the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.values().filter(|e| e.valid).count()
    }

    /// Return true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Deterministic hash of a function for cache validation.
///
/// Uses FNV-1a, which is deterministic across Rust versions and process
/// runs (unlike `DefaultHasher`, which is seeded from entropy).
/// Hashes name, param count/types, return type, and every node's content
/// (kind discriminant, type, effects) — not just NodeId counts.
fn hash_function(func: &Function) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;

    fn mix(h: &mut u64, v: u64) {
        *h ^= v;
        *h = h.wrapping_mul(0x100000001b3);
    }

    fn mix_bytes(h: &mut u64, bytes: &[u8]) {
        for &b in bytes {
            mix(h, b as u64);
        }
    }

    mix_bytes(&mut h, func.name.as_bytes());
    mix(&mut h, func.params.len() as u64);

    for p in &func.params {
        mix_bytes(&mut h, p.name.as_bytes());
        mix_bytes(&mut h, p.ty.type_name().as_bytes());
    }

    mix_bytes(&mut h, func.return_ty.type_name().as_bytes());
    mix(&mut h, func.arena.len() as u64);

    // Hash every node's content — kind, type, effects.
    // This catches in-place mutations that don't change the NodeId set.
    for (id, node) in func.arena.nodes() {
        mix(&mut h, id.as_u64());
        // NodeKind discriminant — a u32 tag.
        mix(&mut h, crate::value_numbering::kind_variant_tag(&node.kind) as u64);
        mix_bytes(&mut h, node.ty.type_name().as_bytes());
        mix(&mut h, node.effects.bits() as u64);
    }

    h
}
