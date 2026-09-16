//! Gate 6B — multi-reduction fusion automation.
//!
//! Pipeline per multi-loop kernel:
//!
//! ```text
//! .ll ──extract_loop_regions──► one single-loop region per reduction
//!         │
//!         ├── lower each region (existing single-loop lowerer)
//!         ├── analysis + semantic recognition per region
//!         └── derive_vector_plan per region            (primitive actions)
//!                       │
//!         composition ──┴── group plans sharing buffer/length with no
//!                           dependency edge, emit ONE fused pass
//! ```
//!
//! The composition never looks at hand-written fused source: the fused C
//! is emitted from the recognized per-region plans, and the search stage
//! (see `bin/gate6b_run.rs`) selects the fusion over the deterministic
//! per-region baseline.

use sir_analysis::manager::AnalysisManager;
use sir_lower::{extract_loop_regions, lower_function, LoopRegion};
use sir_nodes::Function;
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::truth::SemanticTruth;

use crate::vector_derive::derive_vector_plan_for_region;
use crate::vector_plan::{VectorOp, VectorPlan, VectorPredicate};

/// One outlined reduction region with its recognized primitive plan.
pub struct RegionPlan {
    pub region: LoopRegion,
    pub function: Function,
    pub plan: Option<VectorPlan>,
    pub concepts: Vec<String>,
    pub deps: Vec<usize>,
}

impl RegionPlan {
    /// Parameter position of the region's buffer, when derivable from the
    /// plan's buffer name (original params keep their positions).
    pub fn buffer_param(&self) -> Option<usize> {
        self.param_index(&self.plan.as_ref()?.buffer_name)
    }

    /// Parameter position of the region's iteration length.
    pub fn length_param(&self) -> Option<usize> {
        self.param_index(&self.plan.as_ref()?.length_name)
    }

    pub fn param_index(&self, name: &str) -> Option<usize> {
        for (i, p) in self.function.params.iter().enumerate() {
            // `vector_derive::param_name` renames `%N` params to `pN`.
            let by_name = p.name == name || p.name.trim_start_matches('%') == name;
            let by_emitted_name = p.name.starts_with('%') && name == format!("p{}", i);
            if by_name || by_emitted_name {
                return Some(i);
            }
        }
        None
    }

    pub fn operation(&self) -> Option<VectorOp> {
        self.plan.as_ref().map(|p| p.operation.clone())
    }
}

/// Outline every loop of `func_name`, lower it, recognize it, and derive
/// its primitive vector plan.
pub fn analyze_kernel(ll_text: &str, func_name: &str) -> Result<Vec<RegionPlan>, String> {
    let regions = extract_loop_regions(ll_text, func_name)?;
    let mut out = Vec::new();
    for region in regions {
        let function = lower_function(&region.text, &region.name)
            .map_err(|e| format!("region {} did not lower: {}", region.name, e))?;
        let mut mgr = AnalysisManager::new();
        mgr.run_all(&function);
        let mut engine = SemanticEngine::new();
        engine.derive(&function, mgr.database());
        let truths: Vec<SemanticTruth> = engine.database().truths().cloned().collect();
        let plan = derive_vector_plan_for_region(&function, &truths);
        let concepts = truths
            .iter()
            .map(|t| format!("{:?}", t.concept))
            .collect::<Vec<_>>();
        let deps = region
            .dep_sources
            .iter()
            .filter_map(|d| *d)
            .collect::<Vec<_>>();
        out.push(RegionPlan {
            region,
            function,
            plan,
            concepts,
            deps,
        });
    }
    Ok(out)
}

/// A compatible set of regions the composition can fuse into one pass.
#[derive(Clone, Debug)]
pub struct FusionCandidate {
    pub members: Vec<usize>,
}

/// All maximal fusion groups: regions that (a) have a derived plan, (b)
/// share the same buffer and length parameter positions, (c) have no
/// dependency edge between them, and (d) use the same vector width.
///
/// The grouping is purely structural — it is discovery, not a declaration:
/// two loops are put in one group exactly when the semantic layer proved
/// they traverse the same buffer over the same domain.
pub fn fusion_candidates(regions: &[RegionPlan]) -> Vec<FusionCandidate> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        // A region with dependency parameters needs inputs that are not the
        // shared traversal parameters; it is vectorized independently but is
        // not a fusion member (conservative and sound).
        if !r.region.dep_sources.is_empty() {
            continue;
        }
        let (Some(buffer), Some(length)) = (r.buffer_param(), r.length_param()) else {
            continue;
        };
        let Some(plan) = &r.plan else { continue };
        let mut placed = false;
        for group in groups.iter_mut() {
            let Some(&first) = group.first() else { continue };
            let f = &regions[first];
            let compatible = f.buffer_param() == Some(buffer)
                && f.length_param() == Some(length)
                && f.plan.as_ref().map(|p| p.vector_width) == Some(plan.vector_width);
            let independent = group
                .iter()
                .all(|&m| !r.deps.contains(&m) && !regions[m].deps.contains(&i));
            if compatible && independent {
                group.push(i);
                placed = true;
                break;
            }
        }
        if !placed {
            groups.push(vec![i]);
        }
    }
    groups
        .into_iter()
        .filter(|g| g.len() >= 2)
        .map(|members| FusionCandidate { members })
        .collect()
}

/// Canonical order of fusion members (accumulator results are reported in
/// this order).
pub fn member_order(regions: &[RegionPlan], members: &[usize]) -> Vec<usize> {
    let mut m = members.to_vec();
    m.sort_by_key(|&i| {
        let op = regions[i].operation();
        match op {
            Some(VectorOp::Cardinality) => 0,
            Some(VectorOp::Sum) => 1,
            Some(VectorOp::All) => 2,
            None => 3,
        }
    });
    m
}

/// Every fusion-eligible member of a kernel in canonical order (derived
/// plan, no dependency parameters).
pub fn eligible_members(regions: &[RegionPlan]) -> Vec<usize> {
    let ids: Vec<usize> = regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.plan.is_some() && r.region.dep_sources.is_empty())
        .map(|(i, _)| i)
        .collect();
    member_order(regions, &ids)
}

/// Human-readable description of one member's per-chunk work (used by the
/// cost model in the search stage).
pub fn member_cost(plan: &VectorPlan) -> (u32, u32) {
    // (loads per chunk, extra op weight per chunk)
    match plan.operation {
        VectorOp::Cardinality => (1, 2),
        VectorOp::Sum => (1, 1),
        VectorOp::All => (1, 1),
    }
}

/// One candidate in the composition action space.
#[derive(Clone, Debug)]
pub struct PlanOption {
    pub name: String,
    /// Members executed in a single fused pass (empty = all independent).
    pub fused_members: Vec<usize>,
    /// Modelled cost in "per-chunk load units" (one full traversal = 1 load
    /// unit plus the members' operation weights).
    pub modelled_cost: f64,
}

/// Search over the composition action space for a kernel.
///
/// Engine 0 (the deterministic pipeline) has only the per-region action:
/// vectorize every loop independently. The enriched action space adds the
/// fusion primitive for any compatible member set; the cost model prices
/// memory traffic (one shared load per chunk) and applies an early-exit
/// discount to an independent `All` loop, whose traversal can stop at the
/// first mismatch (the Gate 5B early-exit interaction).
pub fn search_plans(
    regions: &[RegionPlan],
    members: &[usize],
    early_exit_prob: f64,
) -> Vec<PlanOption> {
    let op_weight = |m: usize| -> f64 {
        match regions[m].operation() {
            Some(VectorOp::Cardinality) => 0.60,
            Some(VectorOp::Sum) => 0.35,
            Some(VectorOp::All) => 0.25,
            None => 0.10,
        }
    };
    let independent_cost = |m: usize| -> f64 {
        let mut c = 1.0 + op_weight(m);
        if matches!(regions[m].operation(), Some(VectorOp::All)) {
            c *= early_exit_prob.max(1e-3);
        }
        c
    };

    // Engine 0: every member vectorized independently.
    let mut options = vec![PlanOption {
        name: "engine0_3vec".to_string(),
        fused_members: Vec::new(),
        modelled_cost: members.iter().map(|&m| independent_cost(m)).sum(),
    }];

    // Fusion primitive: fuse every subset of size >= 2 among compatible
    // members (the members list is already a compatibility group).
    let n = members.len();
    for mask in 1u32..(1u32 << n) {
        let subset: Vec<usize> = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| members[i])
            .collect();
        if subset.len() < 2 {
            continue;
        }
        let rest: Vec<usize> = members
            .iter()
            .copied()
            .filter(|m| !subset.contains(m))
            .collect();
        let fused_cost = 1.0 + subset.iter().map(|&m| op_weight(m)).sum::<f64>();
        let rest_cost: f64 = rest.iter().map(|&m| independent_cost(m)).sum();
        options.push(PlanOption {
            name: format!("fuse_{}", subset.len()),
            fused_members: subset,
            modelled_cost: fused_cost + rest_cost,
        });
    }
    options.sort_by(|a, b| {
        a.modelled_cost
            .partial_cmp(&b.modelled_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    options
}

/// Predicate description for diagnostics.
pub fn describe_predicate(p: &Option<VectorPredicate>) -> String {
    match p {
        Some(VectorPredicate::MaskedEqual { mask, target }) => {
            format!("masked_eq(mask={},target={})", mask, target)
        }
        Some(VectorPredicate::Equal(t)) => format!("eq({})", t),
        Some(VectorPredicate::Identity) => "identity".to_string(),
        None => "none".to_string(),
    }
}

/// C parameter name for parameter `i` (matches the emitter's naming).
pub fn c_param_name(func: &Function, i: usize) -> String {
    crate::vector_derive::param_name(func, sir_types::NodeId::new(i as u64))
}

/// One fused implementation emitted from per-region primitive plans.
pub struct FusedEmission {
    /// The fused C function.
    pub code: String,
    /// Member indices in accumulator order (`out[k]` matches element k).
    pub ordered_members: Vec<usize>,
}

fn plan_of<'a>(regions: &'a [RegionPlan], m: usize) -> Result<&'a VectorPlan, String> {
    regions[m]
        .plan
        .as_ref()
        .ok_or_else(|| format!("region {} has no derived plan", m))
}

fn predicate_exprs(plan: &VectorPlan) -> (Option<String>, Option<String>) {
    match &plan.predicate {
        Some(VectorPredicate::MaskedEqual { mask, target }) => (Some(mask.clone()), Some(target.clone())),
        Some(VectorPredicate::Equal(t)) => (None, Some(t.clone())),
        _ => (None, None),
    }
}

/// Emit the fused single-pass implementation for a compatible member set.
pub fn emit_fused(
    kernel: &str,
    regions: &[RegionPlan],
    members: &[usize],
    all_members: &[usize],
) -> Result<FusedEmission, String> {
    let ordered = member_order(regions, members);
    if ordered.len() < 2 {
        return Err("fusion needs at least two members".to_string());
    }
    let first = &regions[ordered[0]];
    let func = &first.function;
    let plan0 = plan_of(regions, ordered[0])?;
    let buf = plan0.buffer_name.clone();
    let len = plan0.length_name.clone();

    let mut out = String::new();
    out.push_str("/* Gate 6B fused pass — generated by ABSAC from per-region plans */\n");
    out.push_str("#include <stdint.h>\n#include <immintrin.h>\n\n");
    out.push_str(&format!("static inline void absac_fused_{}(", kernel));
    for (i, p) in func.params.iter().enumerate() {
        out.push_str(&format!(
            "{} {}",
            crate::vector_emit::c_type(&p.ty),
            c_param_name(func, i)
        ));
        out.push_str(", ");
    }
    out.push_str("uint64_t *out) {\n");

    // Per-member setup.
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        let (mask, target) = predicate_exprs(plan);
        match plan.operation {
            VectorOp::Cardinality => out.push_str(&format!("    uint64_t acc{k} = 0;\n")),
            VectorOp::Sum => {
                out.push_str(&format!(
                    "    __m256i acc{k}_v = _mm256_setzero_si256();\n    uint64_t acc{k} = 0;\n"
                ));
            }
            VectorOp::All => out.push_str(&format!("    uint64_t acc{k} = 1;\n")),
        }
        if let Some(m) = mask {
            out.push_str(&format!(
                "    __m256i mask{k}_v = _mm256_set1_epi8((char)({m}));\n"
            ));
        }
        if let Some(t) = target {
            out.push_str(&format!(
                "    __m256i tgt{k}_v = _mm256_set1_epi8((char)({t}));\n"
            ));
        }
    }
    out.push_str("    __m256i zero = _mm256_setzero_si256();\n    uint64_t i = 0;\n");

    // 32-byte main loop.
    out.push_str(&format!("    while (i + 32 <= {len}) {{\n"));
    out.push_str(&format!(
        "        __m256i chunk = _mm256_loadu_si256((const __m256i *)({buf} + i));\n"
    ));
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        let (mask, _) = predicate_exprs(plan);
        match plan.operation {
            VectorOp::Cardinality => {
                let input = match &mask {
                    Some(_) => format!("_mm256_and_si256(chunk, mask{k}_v)"),
                    None => "chunk".to_string(),
                };
                out.push_str(&format!(
                    "        {{ __m256i cmp = _mm256_cmpeq_epi8({input}, tgt{k}_v);\n          \
                     acc{k} += (uint64_t)__builtin_popcount((unsigned)_mm256_movemask_epi8(cmp)); }}\n"
                ));
            }
            VectorOp::Sum => {
                out.push_str(&format!(
                    "        acc{k}_v = _mm256_add_epi64(acc{k}_v, _mm256_sad_epu8(chunk, zero));\n"
                ));
            }
            VectorOp::All => {
                out.push_str(&format!(
                    "        {{ __m256i cmp = _mm256_cmpeq_epi8(chunk, tgt{k}_v);\n          \
                     if (_mm256_movemask_epi8(cmp) != -1) acc{k} = 0; }}\n"
                ));
            }
        }
    }
    out.push_str("        i += 32;\n    }\n");

    // Merge 256-bit sum accumulators.
    for (k, &mi) in ordered.iter().enumerate() {
        if matches!(plan_of(regions, mi)?.operation, VectorOp::Sum) {
            out.push_str(&format!(
                "    {{ __m128i lo = _mm256_castsi256_si128(acc{k}_v);\n      \
                 __m128i hi = _mm256_extracti128_si256(acc{k}_v, 1);\n      \
                 acc{k} = (uint64_t)_mm_cvtsi128_si64(lo) + (uint64_t)_mm_extract_epi64(lo, 1)\n             \
                 + (uint64_t)_mm_cvtsi128_si64(hi) + (uint64_t)_mm_extract_epi64(hi, 1); }}\n"
            ));
        }
    }

    // 16-byte tail.
    out.push_str(&format!("    while (i + 16 <= {len}) {{\n"));
    out.push_str(&format!(
        "        __m128i chunk = _mm_loadu_si128((const __m128i *)({buf} + i));\n"
    ));
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        let (mask, _) = predicate_exprs(plan);
        match plan.operation {
            VectorOp::Cardinality => {
                let setup_mask = match &mask {
                    Some(_) => format!(
                        "        __m128i mask{k}_v = _mm_set1_epi8((char)({}));\n",
                        predicate_exprs(plan).0.unwrap()
                    ),
                    None => String::new(),
                };
                let input = match &mask {
                    Some(_) => format!("_mm_and_si128(chunk, mask{k}_v)"),
                    None => "chunk".to_string(),
                };
                out.push_str(&setup_mask);
                out.push_str(&format!(
                    "        {{ __m128i cmp = _mm_cmpeq_epi8({input}, _mm_set1_epi8((char)({})));\n          \
                     acc{k} += (uint64_t)__builtin_popcount((unsigned)_mm_movemask_epi8(cmp)); }}\n",
                    predicate_exprs(plan).1.unwrap()
                ));
            }
            VectorOp::Sum => {
                out.push_str(&format!(
                    "        {{ __m128i s = _mm_sad_epu8(chunk, _mm_setzero_si128());\n          \
                     acc{k} += (uint64_t)_mm_cvtsi128_si64(s) + (uint64_t)_mm_extract_epi64(s, 1); }}\n"
                ));
            }
            VectorOp::All => {
                out.push_str(&format!(
                    "        {{ __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char)({})));\n          \
                     if (_mm_movemask_epi8(cmp) != 0xFFFF) acc{k} = 0; }}\n",
                    predicate_exprs(plan).1.unwrap()
                ));
            }
        }
    }
    out.push_str("        i += 16;\n    }\n");

    // Scalar tail.
    out.push_str(&format!("    while (i < {len}) {{\n"));
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        let (mask, target) = predicate_exprs(plan);
        match plan.operation {
            VectorOp::Cardinality => match mask {
                Some(m) => out.push_str(&format!(
                    "        acc{k} += (({buf}[i] & ({m})) == ({}));\n",
                    target.unwrap()
                )),
                None => out.push_str(&format!(
                    "        acc{k} += ({buf}[i] == ({}));\n",
                    target.unwrap()
                )),
            },
            VectorOp::Sum => out.push_str(&format!("        acc{k} += {buf}[i];\n")),
            VectorOp::All => out.push_str(&format!(
                "        if ({buf}[i] != ({})) acc{k} = 0;\n",
                target.unwrap()
            )),
        }
    }
    out.push_str("        i++;\n    }\n");

    for (k, &mi) in ordered.iter().enumerate() {
        let slot = all_members.iter().position(|m| *m == mi).unwrap_or(k);
        out.push_str(&format!("    out[{slot}] = acc{k};\n"));
    }
    out.push_str("}\n");

    Ok(FusedEmission {
        code: out,
        ordered_members: ordered,
    })
}

/// Emit one independent vectorized loop for member `k` (the deterministic
/// per-region plan, in place).
fn emit_independent_loop(
    out: &mut String,
    k: usize,
    plan: &VectorPlan,
    buf: &str,
    len: &str,
) -> Result<(), String> {
    let (mask, target) = predicate_exprs(plan);
    match plan.operation {
        VectorOp::Cardinality => out.push_str(&format!("    uint64_t acc{k} = 0;\n")),
        VectorOp::Sum => out.push_str(&format!(
            "    __m256i acc{k}_v = _mm256_setzero_si256();\n    uint64_t acc{k} = 0;\n"
        )),
        VectorOp::All => out.push_str(&format!("    uint64_t acc{k} = 1;\n")),
    }
    if let Some(m) = &mask {
        out.push_str(&format!(
            "    __m256i mask{k}_v = _mm256_set1_epi8((char)({m}));\n"
        ));
    }
    if let Some(t) = &target {
        out.push_str(&format!(
            "    __m256i tgt{k}_v = _mm256_set1_epi8((char)({t}));\n"
        ));
    }
    out.push_str("    {\n        uint64_t i = 0;\n");
    out.push_str(&format!("        while (i + 32 <= {len}) {{\n"));
    out.push_str(&format!(
        "            __m256i chunk = _mm256_loadu_si256((const __m256i *)({buf} + i));\n"
    ));
    match plan.operation {
        VectorOp::Cardinality => {
            let input = match &mask {
                Some(_) => format!("_mm256_and_si256(chunk, mask{k}_v)"),
                None => "chunk".to_string(),
            };
            out.push_str(&format!(
                "            {{ __m256i cmp = _mm256_cmpeq_epi8({input}, tgt{k}_v);\n              \
                 acc{k} += (uint64_t)__builtin_popcount((unsigned)_mm256_movemask_epi8(cmp)); }}\n"
            ));
        }
        VectorOp::Sum => out.push_str(&format!(
            "            acc{k}_v = _mm256_add_epi64(acc{k}_v, _mm256_sad_epu8(chunk, _mm256_setzero_si256()));\n"
        )),
        VectorOp::All => out.push_str(&format!(
            "            {{ __m256i cmp = _mm256_cmpeq_epi8(chunk, tgt{k}_v);\n              \
             if (_mm256_movemask_epi8(cmp) != -1) acc{k} = 0; }}\n"
        )),
    }
    out.push_str("            i += 32;\n        }\n");
    if matches!(plan.operation, VectorOp::Sum) {
        out.push_str(&format!(
            "        {{ __m128i lo = _mm256_castsi256_si128(acc{k}_v);\n          \
             __m128i hi = _mm256_extracti128_si256(acc{k}_v, 1);\n          \
             acc{k} = (uint64_t)_mm_cvtsi128_si64(lo) + (uint64_t)_mm_extract_epi64(lo, 1)\n                 \
             + (uint64_t)_mm_cvtsi128_si64(hi) + (uint64_t)_mm_extract_epi64(hi, 1); }}\n"
        ));
    }
    out.push_str(&format!("        while (i + 16 <= {len}) {{\n"));
    out.push_str(&format!(
        "            __m128i chunk = _mm_loadu_si128((const __m128i *)({buf} + i));\n"
    ));
    match plan.operation {
        VectorOp::Cardinality => {
            let input = match &mask {
                Some(m) => format!(
                    "_mm_and_si128(chunk, _mm_set1_epi8((char)({m})))"
                ),
                None => "chunk".to_string(),
            };
            out.push_str(&format!(
                "            {{ __m128i cmp = _mm_cmpeq_epi8({input}, _mm_set1_epi8((char)({})));\n              \
                 acc{k} += (uint64_t)__builtin_popcount((unsigned)_mm_movemask_epi8(cmp)); }}\n",
                target.clone().unwrap_or_default()
            ));
        }
        VectorOp::Sum => out.push_str(&format!(
            "            {{ __m128i s = _mm_sad_epu8(chunk, _mm_setzero_si128());\n              \
             acc{k} += (uint64_t)_mm_cvtsi128_si64(s) + (uint64_t)_mm_extract_epi64(s, 1); }}\n"
        )),
        VectorOp::All => out.push_str(&format!(
            "            {{ __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char)({})));\n              \
             if (_mm_movemask_epi8(cmp) != 0xFFFF) acc{k} = 0; }}\n",
            target.clone().unwrap_or_default()
        )),
    }
    out.push_str("            i += 16;\n        }\n");
    out.push_str(&format!("        while (i < {len}) {{\n"));
    match plan.operation {
        VectorOp::Cardinality => match &mask {
            Some(m) => out.push_str(&format!(
                "            acc{k} += (({buf}[i] & ({m})) == ({}));\n",
                target.clone().unwrap_or_default()
            )),
            None => out.push_str(&format!(
                "            acc{k} += ({buf}[i] == ({}));\n",
                target.clone().unwrap_or_default()
            )),
        },
        VectorOp::Sum => out.push_str(&format!("            acc{k} += {buf}[i];\n")),
        VectorOp::All => out.push_str(&format!(
            "            if ({buf}[i] != ({})) acc{k} = 0;\n",
            target.clone().unwrap_or_default()
        )),
    }
    out.push_str("            i++;\n        }\n    }\n");
    Ok(())
}

/// Emit the deterministic baseline: ONE function containing each region's
/// independently vectorized loop, exactly as the pipeline would emit them
/// in place for a multi-loop kernel.
pub fn emit_independent_set(
    prefix: &str,
    kernel: &str,
    regions: &[RegionPlan],
    members: &[usize],
    all_members: &[usize],
) -> Result<String, String> {
    let ordered = member_order(regions, members);
    let signature_member = if ordered.is_empty() {
        all_members[0]
    } else {
        ordered[0]
    };
    let func = &regions[signature_member].function;
    let plan0 = plan_of(regions, signature_member)?;
    let buf = plan0.buffer_name.clone();
    let len = plan0.length_name.clone();

    let mut out = String::new();
    out.push_str("/* Gate 6B independent per-region loops */\n");
    out.push_str("#include <stdint.h>\n#include <immintrin.h>\n\n");
    out.push_str(&format!("static void absac_{}_", prefix));
    out.push_str(&format!("{}(", kernel));
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{} {}",
            crate::vector_emit::c_type(&p.ty),
            c_param_name(func, i)
        ));
    }
    out.push_str(", uint64_t *out) {\n");
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        emit_independent_loop(&mut out, k, plan, &buf, &len)?;
        let slot = all_members.iter().position(|m| *m == mi).unwrap_or(k);
        out.push_str(&format!("    out[{slot}] = acc{k};\n"));
    }
    out.push_str("}\n");
    Ok(out)
}

/// Emit a scalar reference oracle for each member (independent of the
/// fused and 3vec code paths).
pub fn emit_reference(
    kernel: &str,
    regions: &[RegionPlan],
    members: &[usize],
) -> Result<String, String> {
    let ordered = member_order(regions, members);
    let func = &regions[ordered[0]].function;
    let plan0 = plan_of(regions, ordered[0])?;
    let buf = plan0.buffer_name.clone();
    let len = plan0.length_name.clone();

    let mut out = String::new();
    out.push_str("/* Gate 6B scalar reference oracle */\n");
    out.push_str("#include <stdint.h>\n\n");
    out.push_str(&format!("void {}_ref(", kernel));
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{} {}",
            crate::vector_emit::c_type(&p.ty),
            c_param_name(func, i)
        ));
    }
    out.push_str(", uint64_t *out) {\n");
    for (k, &mi) in ordered.iter().enumerate() {
        let plan = plan_of(regions, mi)?;
        let (mask, target) = predicate_exprs(plan);
        match plan.operation {
            VectorOp::Cardinality => {
                let cond = match mask {
                    Some(m) => format!("(({buf}[j] & ({m})) == ({}))", target.unwrap()),
                    None => format!("({buf}[j] == ({}))", target.unwrap()),
                };
                out.push_str(&format!(
                    "    uint64_t acc{k} = 0;\n    for (uint64_t j = 0; j < {len}; j++) acc{k} += {cond};\n"
                ));
            }
            VectorOp::Sum => {
                out.push_str(&format!(
                    "    uint64_t acc{k} = 0;\n    for (uint64_t j = 0; j < {len}; j++) acc{k} += {buf}[j];\n"
                ));
            }
            VectorOp::All => {
                out.push_str(&format!(
                    "    uint64_t acc{k} = 1;\n    for (uint64_t j = 0; j < {len}; j++) if ({buf}[j] != ({})) acc{k} = 0;\n",
                    target.unwrap()
                ));
            }
        }
    }
    for (k, _) in ordered.iter().enumerate() {
        out.push_str(&format!("    out[{k}] = acc{k};\n"));
    }
    out.push_str("}\n");
    Ok(out)
}

/// Scalar argument expressions for a kernel's non-buffer parameters.
/// `buffer_expr` is the expression passed for pointer/array parameters.
pub fn scalar_args(
    regions: &[RegionPlan],
    members: &[usize],
    buffer_expr: &str,
    length_expr: &str,
) -> Vec<String> {
    let func = &regions[members[0]].function;
    let length_param = regions[members[0]].length_param();
    const VALUES: [u64; 5] = [0x0F, 0x05, 0x42, 0x33, 0x77];
    let mut out = Vec::new();
    let mut scalar_i = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        match &p.ty {
            sir_types::Type::Pointer { .. } | sir_types::Type::Array { .. } => {
                out.push(buffer_expr.to_string())
            }
            _ => {
                if Some(i) == length_param {
                    out.push(length_expr.to_string());
                    continue;
                }
                let v = VALUES[scalar_i.min(VALUES.len() - 1)];
                scalar_i += 1;
                out.push(format!("{}", v));
            }
        }
    }
    out
}

/// Build the standalone differential + benchmark driver for one kernel.
pub fn build_driver(
    kernel: &str,
    regions: &[RegionPlan],
    fused_members: &[usize],
    independent_members: &[usize],
    all_members: &[usize],
    fused: Option<&FusedEmission>,
) -> Result<String, String> {
    let n = all_members.len();
    let args = scalar_args(regions, all_members, "buf", "n");
    let bench_args = scalar_args(regions, all_members, "buf + offset", "n");

    let mut out = String::new();
    out.push_str("#define _GNU_SOURCE\n#include <stdint.h>\n#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n#include <string.h>\n#include <time.h>\n#include <sched.h>\n\n");
    if let Some(f) = fused {
        out.push_str(&f.code);
        out.push('\n');
    }
    out.push_str(&emit_independent_set(
        "threevec",
        kernel,
        regions,
        independent_members,
        all_members,
    )?);
    out.push('\n');
    out.push_str(&emit_independent_set(
        "engine0",
        kernel,
        regions,
        all_members,
        all_members,
    )?);
    out.push('\n');
    out.push_str(&emit_reference(kernel, regions, all_members)?);
    out.push('\n');

    out.push_str("static volatile uint64_t g_sink = 0;\n");
    out.push_str("static double now_sec(void){struct timespec ts;clock_gettime(CLOCK_MONOTONIC,&ts);return (double)ts.tv_sec+(double)ts.tv_nsec*1e-9;}\n\n");

    // Reference / 3vec / fused call sites.
    let arg_list = args.join(", ");
    let bench_arg_list = bench_args.join(", ");
    let ref_call = format!("{}_ref({}, out_ref)", kernel, arg_list);
    let plan_calls = format!(
        "{}{}",
        if fused_members.is_empty() {
            String::new()
        } else {
            format!("absac_fused_{}({}, out_impl);\n            ", kernel, arg_list)
        },
        if independent_members.is_empty() {
            String::new()
        } else {
            format!(
                "absac_threevec_{}({}, out_impl);\n            ",
                kernel, arg_list
            )
        }
    );
    let plan_calls_bench = format!(
        "{}{}",
        if fused_members.is_empty() {
            String::new()
        } else {
            format!(
                "absac_fused_{}({}, out_impl);\n                ",
                kernel, bench_arg_list
            )
        },
        if independent_members.is_empty() {
            String::new()
        } else {
            format!(
                "absac_threevec_{}({}, out_impl);\n                ",
                kernel, bench_arg_list
            )
        }
    );
    let engine0_call_bench = format!("absac_engine0_{}({}, out_3v);", kernel, bench_arg_list);
    // Value the "equal" distribution is filled with: the All member's
    // predicate value resolved through the driver's scalar arguments.
    let equal_fill = {
        let mut fill = "0x42".to_string();
        for &m in all_members {
            if let Some(plan) = &regions[m].plan {
                if matches!(plan.operation, VectorOp::All) {
                    if let Some((_, Some(t))) = {
                        let (mask, target) = predicate_exprs(plan);
                        Some((mask, target))
                    } {
                        if let Some(idx) = t
                            .trim_start_matches('p')
                            .parse::<usize>()
                            .ok()
                            .filter(|_| t.starts_with('p'))
                        {
                            if let Some(v) = args.get(idx) {
                                fill = v.clone();
                            }
                        } else {
                            fill = t.clone();
                        }
                    }
                }
            }
        }
        fill
    };

    out.push_str(&format!(
        r#"
int main(void) {{
    {{ cpu_set_t cs; CPU_ZERO(&cs); CPU_SET(0, &cs); sched_setaffinity(0, sizeof(cs), &cs); }}
    const uint64_t NMAX = 1u << 20;
    uint8_t *buf = (uint8_t *)aligned_alloc(64, NMAX);
    uint64_t out_ref[{n}], out_3v[{n}], out_impl[{n}];
    uint64_t checks = 0, failures = 0;
    uint64_t sizes[] = {{0,1,15,16,17,31,32,33,63,64,65,127,128,129,255,256,257,4096,65536}};
    int patterns = 7;
    for (int p = 0; p < patterns; p++) {{
        for (int s = 0; s < (int)(sizeof(sizes)/sizeof(sizes[0])); s++) {{
            uint64_t n = sizes[s];
            for (uint64_t j = 0; j < NMAX; j++) {{
                switch (p) {{
                    case 0: buf[j] = (uint8_t)((j * 2654435761u) >> 13); break;   /* pseudo-random */
                    case 1: buf[j] = 0; break;                                     /* zeros */
                    case 2: buf[j] = 0x42; break;                                  /* all = val */
                    case 3: buf[j] = 0x05; break;                                  /* all = target */
                    case 4: buf[j] = (j & 1) ? 0xFF : 0x00; break;                 /* alternating */
                    case 5: buf[j] = (j == 0) ? 0x42 : 0x00; break;                /* single hit head */
                    default: buf[j] = (j + 1 == NMAX) ? 0x42 : 0x00; break;        /* single hit tail */
                }}
            }}
            {ref_call};
            {plan_calls}
            for (int k = 0; k < {n}; k++) {{
                checks++;
                if (out_impl[k] != out_ref[k]) {{
                    if (failures < 8) printf("CORRECTNESS_FAIL kernel={kernel} path=plan pattern=%d n=%llu member=%d ref=%llu got=%llu\n", p, (unsigned long long)n, k, (unsigned long long)out_ref[k], (unsigned long long)out_impl[k]);
                    failures++;
                }}
            }}
        }}
    }}
    printf("CORRECTNESS %s checks=%llu failures=%llu\n", failures ? "FAIL" : "OK", (unsigned long long)checks, (unsigned long long)failures);

    uint64_t bench_sizes[] = {{4096, 65536}};
    for (int b = 0; b < 2; b++) {{
        uint64_t n = bench_sizes[b];
        for (int dist = 0; dist < 2; dist++) {{
            for (uint64_t j = 0; j < NMAX; j++) {{
                buf[j] = dist ? (uint8_t)({equal_fill}) : (uint8_t)((j * 2654435761u) >> 13);
            }}
            double t3[25], tf[25];
            uint64_t iters = 5000;
            volatile uint64_t offset = 0;
            for (int t = 0; t < 25; t++) {{
                double t0 = now_sec();
                for (uint64_t it = 0; it < iters; it++) {{
                    offset = (offset + 1) & 0xFFF;
                    {engine0_call_bench}
                    g_sink ^= out_3v[0] ^ out_3v[{last}];
                }}
                t3[t] = (now_sec()-t0)/(double)iters*1e9;
                double t1 = now_sec();
                for (uint64_t it = 0; it < iters; it++) {{
                    offset = (offset + 1) & 0xFFF;
                    {plan_calls_bench}g_sink ^= out_impl[0] ^ out_impl[{last}];
                }}
                tf[t] = (now_sec()-t1)/(double)iters*1e9;
            }}
            for (int a = 0; a < 24; a++) for (int c = a+1; c < 25; c++) {{
                if (t3[c] < t3[a]) {{ double tmp=t3[a]; t3[a]=t3[c]; t3[c]=tmp; }}
                if (tf[c] < tf[a]) {{ double tmp=tf[a]; tf[a]=tf[c]; tf[c]=tmp; }}
            }}
            printf("BENCH kernel={kernel} n=%llu dist=%s engine0_ns=%.2f plan_ns=%.2f ratio=%.4f\n",
                (unsigned long long)n, dist ? "equal" : "random", t3[12], tf[12], tf[12]/t3[12]);
        }}
    }}
    free(buf);
    return failures ? 1 : 0;
}}
"#,
        n = n,
        last = n - 1,
        kernel = kernel,
        ref_call = ref_call,
        plan_calls = plan_calls,
        plan_calls_bench = plan_calls_bench,
        engine0_call_bench = engine0_call_bench,
        equal_fill = equal_fill,
    ));
    Ok(out)
}
