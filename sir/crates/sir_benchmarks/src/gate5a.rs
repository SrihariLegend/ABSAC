//! Gate 5A — Target-Plan Search Landscape
//!
//! Enumerates all valid target plans for the three semantic reductions,
//! benchmarks them, and records the full search landscape.
//!
//! Candidate dimensions:
//!   ISA:          scalar, SSE2, AVX2
//!   Vector width: 16, 32 bytes (matched to ISA)
//!   Reduction:    lane-accumulation, movemask+popcount, psadbw, vector-reduce
//!   Unrolling:    1×, 2×, 4×
//!   Tail:         scalar, narrower-vector
//!   Control:      early-exit, branchless-full-reduction
//!
//! For each kernel, we enumerate all valid combinations, compile, benchmark,
//! and record: plan id, parameters, correctness, median ns, speedup vs orig.

use crate::vector_plan::{VectorOp, VectorPredicate, TailPolicy, VectorPlan};

/// A candidate target plan with all dimensions specified.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CandidatePlan {
    pub id: u32,
    pub kernel: String,
    pub isa: ISA,
    pub reduction: ReductionStrategy,
    pub unroll: UnrollFactor,
    pub tail: TailStrategy,
    pub control: ControlFlow,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ISA {
    Scalar,
    SSE2,
    AVX2,
}

impl ISA {
    pub fn width(&self) -> usize {
        match self {
            ISA::Scalar => 1,
            ISA::SSE2 => 16,
            ISA::AVX2 => 32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReductionStrategy {
    /// Scalar: process one byte at a time
    ScalarLoop,
    /// Lane-wise accumulation (what clang does): compare → widen → vpaddq
    LaneAccumulation,
    /// movemask + popcount (best for Cardinality/All)
    MovemaskPopcount,
    /// psadbw (best for Sum)
    Psadbw,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnrollFactor {
    U1,
    U2,
    U4,
}

impl UnrollFactor {
    pub fn count(&self) -> usize {
        match self {
            UnrollFactor::U1 => 1,
            UnrollFactor::U2 => 2,
            UnrollFactor::U4 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TailStrategy {
    Scalar,
    NarrowerVector,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ControlFlow {
    /// Full reduction (no early exit)
    FullReduction,
    /// Early exit on first mismatch (only valid for All)
    EarlyExit,
}

/// A benchmark result for a candidate plan.
#[derive(Clone, Debug)]
pub struct PlanResult {
    pub plan: CandidatePlan,
    pub correct: bool,
    pub median_ns: f64,
    pub min_ns: f64,
    pub speedup_vs_orig: f64,
}

/// Enumerate all valid candidate plans for a given kernel.
pub fn enumerate_plans(kernel: &str) -> Vec<CandidatePlan> {
    let mut plans = Vec::new();
    let mut id = 0u32;

    let isas = match kernel {
        "k18_all_equal" | "k43_sum_ascii" | "k50_count_masked" => {
            vec![ISA::Scalar, ISA::SSE2, ISA::AVX2]
        }
        _ => vec![ISA::Scalar],
    };

    // Valid reduction strategies per kernel
    let reductions = match kernel {
        "k18_all_equal" => vec![ReductionStrategy::ScalarLoop, ReductionStrategy::LaneAccumulation, ReductionStrategy::MovemaskPopcount],
        "k43_sum_ascii" => vec![ReductionStrategy::ScalarLoop, ReductionStrategy::LaneAccumulation, ReductionStrategy::Psadbw],
        "k50_count_masked" => vec![ReductionStrategy::ScalarLoop, ReductionStrategy::LaneAccumulation, ReductionStrategy::MovemaskPopcount],
        _ => vec![ReductionStrategy::ScalarLoop],
    };

    let unrolls = vec![UnrollFactor::U1, UnrollFactor::U2, UnrollFactor::U4];
    let tails = vec![TailStrategy::Scalar, TailStrategy::NarrowerVector];
    let controls = match kernel {
        "k18_all_equal" => vec![ControlFlow::FullReduction, ControlFlow::EarlyExit],
        _ => vec![ControlFlow::FullReduction],
    };

    for isa in &isas {
        for reduction in &reductions {
            // Skip invalid combinations
            // - Scalar ISA only has ScalarLoop
            if *isa == ISA::Scalar && *reduction != ReductionStrategy::ScalarLoop {
                continue;
            }
            // - SSE2/AVX2 don't use ScalarLoop
            if *isa != ISA::Scalar && *reduction == ReductionStrategy::ScalarLoop {
                continue;
            }
            // - LaneAccumulation only valid for AVX2 (matches clang's approach)
            if *reduction == ReductionStrategy::LaneAccumulation && *isa != ISA::AVX2 {
                continue;
            }
            // - Psadbw only valid for Sum
            if *reduction == ReductionStrategy::Psadbw && kernel != "k43_sum_ascii" {
                continue;
            }
            // - MovemaskPopcount only valid for Cardinality/All
            if *reduction == ReductionStrategy::MovemaskPopcount && kernel == "k43_sum_ascii" {
                continue;
            }
            // - NarrowerVector tail only valid for AVX2 (SSE2 has no narrower vector)
            for unroll in &unrolls {
                for tail in &tails {
                    if *tail == TailStrategy::NarrowerVector && *isa != ISA::AVX2 {
                        continue;
                    }
                    for control in &controls {
                        // Early exit only makes sense for All
                        if *control == ControlFlow::EarlyExit && kernel != "k18_all_equal" {
                            continue;
                        }
                        // Scalar doesn't have early exit (it's inherent in the scalar loop)
                        if *control == ControlFlow::EarlyExit && *isa == ISA::Scalar {
                            continue;
                        }

                        plans.push(CandidatePlan {
                            id,
                            kernel: kernel.to_string(),
                            isa: isa.clone(),
                            reduction: reduction.clone(),
                            unroll: unroll.clone(),
                            tail: tail.clone(),
                            control: control.clone(),
                        });
                        id += 1;
                    }
                }
            }
        }
    }

    plans
}

/// Generate C code for a candidate plan.
pub fn emit_plan(plan: &CandidatePlan, params: &str, buf: &str, len: &str, extra_params: &str) -> String {
    let mut out = String::new();

    let vw = plan.isa.width();

    // Function signature
    out.push_str(&format!("uint64_t {}_c{}({}) {{\n", plan.kernel, plan.id, params));

    match plan.kernel.as_str() {
        "k18_all_equal" => emit_k18(&mut out, plan, buf, len),
        "k43_sum_ascii" => emit_k43(&mut out, plan, buf, len),
        "k50_count_masked" => emit_k50(&mut out, plan, buf, len, extra_params),
        _ => {}
    }

    out.push_str("}\n");
    out
}

fn emit_k18(out: &mut String, plan: &CandidatePlan, buf: &str, len: &str) {
    let vw = plan.isa.width();
    let val = "val";
    let unroll = plan.unroll.count();

    if plan.isa == ISA::Scalar {
        // Scalar loop
        if plan.control == ControlFlow::EarlyExit || plan.control == ControlFlow::FullReduction {
            // Scalar always has early exit capability (the loop just breaks)
            out.push_str("    for (uint64_t i = 0; i < n; i++) {\n");
            out.push_str(&format!("        if ({}[i] != {}) return 0;\n", buf, val));
            out.push_str("    }\n");
            out.push_str("    return 1;\n");
        }
        return;
    }

    // Vector plans
    let (load_fn, cmp_fn, mask_fn, type_suffix, set1_fn, full_mask) = match vw {
        32 => ("_mm256_loadu_si256", "_mm256_cmpeq_epi8", "_mm256_movemask_epi8", "256", "_mm256_set1_epi8", "-1"),
        16 => ("_mm_loadu_si128", "_mm_cmpeq_epi8", "_mm_movemask_epi8", "128", "_mm_set1_epi8", "0xFFFF"),
        _ => return,
    };

    out.push_str(&format!("    __m{}i val_v = {}((char){});\n", type_suffix, set1_fn, val));
    out.push_str("    uint64_t i = 0;\n");

    let step = vw * unroll;
    out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));

    if plan.control == ControlFlow::EarlyExit {
        // Early exit: check each chunk immediately
        for u in 0..unroll {
            out.push_str(&format!("        __m{}i chunk{} = {}((__m{}i *)({} + i + {}));\n",
                type_suffix, u, load_fn, type_suffix, buf, u * vw));
            out.push_str(&format!("        __m{}i cmp{} = {}(chunk{}, val_v);\n", type_suffix, u, cmp_fn, u));
            out.push_str(&format!("        int mask{} = {}(cmp{});\n", u, mask_fn, u));
            out.push_str(&format!("        if (mask{} != {}) return 0;\n", u, full_mask));
        }
    } else {
        // Full reduction: accumulate all chunks, check at end
        out.push_str("        int all_match = 1;\n");
        for u in 0..unroll {
            out.push_str(&format!("        __m{}i chunk{} = {}((__m{}i *)({} + i + {}));\n",
                type_suffix, u, load_fn, type_suffix, buf, u * vw));
            out.push_str(&format!("        __m{}i cmp{} = {}(chunk{}, val_v);\n", type_suffix, u, cmp_fn, u));
            out.push_str(&format!("        int mask{} = {}(cmp{});\n", u, mask_fn, u));
            out.push_str(&format!("        if (mask{} != {}) all_match = 0;\n", u, full_mask));
        }
        out.push_str("        if (!all_match) return 0;\n");
    }

    out.push_str(&format!("        i += {};\n", step));
    out.push_str("    }\n");

    // Tail
    if plan.tail == TailStrategy::NarrowerVector && vw == 32 {
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        out.push_str(&format!("        __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char){}));\n", val));
        out.push_str("        int mask = _mm_movemask_epi8(cmp);\n");
        out.push_str("        if (mask != 0xFFFF) return 0;\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    out.push_str(&format!("        if ({}[i] != {}) return 0;\n", buf, val));
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return 1;\n");
}

fn emit_k43(out: &mut String, plan: &CandidatePlan, buf: &str, len: &str) {
    let vw = plan.isa.width();
    let unroll = plan.unroll.count();

    if plan.isa == ISA::Scalar {
        out.push_str("    uint64_t sum = 0;\n");
        out.push_str("    for (uint64_t i = 0; i < n; i++) {\n");
        out.push_str(&format!("        sum += {}[i];\n", buf));
        out.push_str("    }\n");
        out.push_str("    return sum;\n");
        return;
    }

    if plan.reduction == ReductionStrategy::Psadbw {
        if vw == 32 {
            out.push_str("    __m256i zero = _mm256_setzero_si256();\n");
            out.push_str("    __m256i acc = zero;\n");
            out.push_str("    uint64_t i = 0;\n");
            let step = 32 * unroll;
            out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));
            for u in 0..unroll {
                out.push_str(&format!("        __m256i chunk{} = _mm256_loadu_si256((__m256i *)({} + i + {}));\n", u, buf, u * 32));
                out.push_str(&format!("        __m256i sums{} = _mm256_sad_epu8(chunk{}, zero);\n", u, u));
                out.push_str(&format!("        acc = _mm256_add_epi64(acc, sums{});\n", u));
            }
            out.push_str(&format!("        i += {};\n", step));
            out.push_str("    }\n");
            out.push_str("    __m128i lo = _mm256_castsi256_si128(acc);\n");
            out.push_str("    __m128i hi = _mm256_extracti128_si256(acc, 1);\n");
            out.push_str("    uint64_t sum = (uint64_t)_mm_cvtsi128_si64(lo)\n");
            out.push_str("                 + (uint64_t)_mm_extract_epi64(lo, 1)\n");
            out.push_str("                 + (uint64_t)_mm_cvtsi128_si64(hi)\n");
            out.push_str("                 + (uint64_t)_mm_extract_epi64(hi, 1);\n");
        } else {
            out.push_str("    __m128i zero = _mm_setzero_si128();\n");
            out.push_str("    __m128i acc = zero;\n");
            out.push_str("    uint64_t i = 0;\n");
            let step = 16 * unroll;
            out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));
            for u in 0..unroll {
                out.push_str(&format!("        __m128i chunk{} = _mm_loadu_si128((__m128i *)({} + i + {}));\n", u, buf, u * 16));
                out.push_str(&format!("        __m128i sums{} = _mm_sad_epu8(chunk{}, zero);\n", u, u));
                out.push_str(&format!("        acc = _mm_add_epi64(acc, sums{});\n", u));
            }
            out.push_str(&format!("        i += {};\n", step));
            out.push_str("    }\n");
            out.push_str("    uint64_t sum = (uint64_t)_mm_cvtsi128_si64(acc) + (uint64_t)_mm_extract_epi64(acc, 1);\n");
        }
    } else {
        // LaneAccumulation: widen 4 bytes to 64-bit and accumulate
        // This is deliberately less efficient (4 bytes/iter vs psadbw's 32)
        out.push_str("    uint64_t sum = 0;\n");
        out.push_str("    uint64_t i = 0;\n");
        let step = 4 * unroll;
        out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));
        for u in 0..unroll {
            out.push_str(&format!("        __m256i chunk{} = _mm256_cvtepu8_epi64(_mm_cvtsi32_si128(*(const uint32_t *)({} + i + {})));
", u, buf, u * 4));
            out.push_str(&format!("        sum += (uint64_t)_mm256_extract_epi64(chunk{}, 0) + (uint64_t)_mm256_extract_epi64(chunk{}, 1) + (uint64_t)_mm256_extract_epi64(chunk{}, 2) + (uint64_t)_mm256_extract_epi64(chunk{}, 3);\n", u, u, u, u));
        }
        out.push_str(&format!("        i += {};\n", step));
        out.push_str("    }\n");
    }

    // Tail
    if plan.tail == TailStrategy::NarrowerVector && vw == 32 {
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        out.push_str("        __m128i sums = _mm_sad_epu8(chunk, _mm_setzero_si128());\n");
        out.push_str("        sum += (uint64_t)_mm_cvtsi128_si64(sums) + (uint64_t)_mm_extract_epi64(sums, 1);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    out.push_str(&format!("        sum += {}[i];\n", buf));
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return sum;\n");
}

fn emit_k50(out: &mut String, plan: &CandidatePlan, buf: &str, len: &str, extra_params: &str) {
    let vw = plan.isa.width();
    let unroll = plan.unroll.count();

    if plan.isa == ISA::Scalar {
        out.push_str("    uint64_t count = 0;\n");
        out.push_str("    for (uint64_t i = 0; i < n; i++) {\n");
        out.push_str("        count += ((buf[i] & mask) == target);\n");
        out.push_str("    }\n");
        out.push_str("    return count;\n");
        return;
    }

    let (load_fn, cmp_fn, mask_fn, and_fn, type_suffix, set1_fn) = match vw {
        32 => ("_mm256_loadu_si256", "_mm256_cmpeq_epi8", "_mm256_movemask_epi8", "_mm256_and_si256", "256", "_mm256_set1_epi8"),
        16 => ("_mm_loadu_si128", "_mm_cmpeq_epi8", "_mm_movemask_epi8", "_mm_and_si128", "128", "_mm_set1_epi8"),
        _ => return,
    };

    out.push_str(&format!("    __m{}i mask_v = {}((char)mask);\n", type_suffix, set1_fn));
    out.push_str(&format!("    __m{}i target_v = {}((char)target);\n", type_suffix, set1_fn));

    if plan.reduction == ReductionStrategy::MovemaskPopcount {
        out.push_str("    uint64_t count = 0;\n");
        out.push_str("    uint64_t i = 0;\n");
        let step = vw * unroll;
        out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));
        for u in 0..unroll {
            out.push_str(&format!("        __m{}i chunk{} = {}((__m{}i *)({} + i + {}));\n",
                type_suffix, u, load_fn, type_suffix, buf, u * vw));
            out.push_str(&format!("        __m{}i masked{} = {}(chunk{}, mask_v);\n", type_suffix, u, and_fn, u));
            out.push_str(&format!("        __m{}i cmp{} = {}(masked{}, target_v);\n", type_suffix, u, cmp_fn, u));
            out.push_str(&format!("        int bits{} = {}(cmp{});\n", u, mask_fn, u));
            out.push_str(&format!("        count += __builtin_popcount(bits{});\n", u));
        }
        out.push_str(&format!("        i += {};\n", step));
        out.push_str("    }\n");
    } else {
        // LaneAccumulation: compare → AND with 1s → psadbw to count (more instructions than movemask+popcount)
        out.push_str("    __m256i zero = _mm256_setzero_si256();\n");
        out.push_str("    __m256i ones = _mm256_set1_epi8(1);\n");
        out.push_str("    __m256i acc = zero;\n");
        out.push_str("    uint64_t i = 0;\n");
        let step = vw * unroll;
        out.push_str(&format!("    while (i + {} <= {}) {{\n", step, len));
        for u in 0..unroll {
            out.push_str(&format!("        __m{}i chunk{} = {}((__m{}i *)({} + i + {}));\n",
                type_suffix, u, load_fn, type_suffix, buf, u * vw));
            out.push_str(&format!("        __m{}i masked{} = {}(chunk{}, mask_v);\n", type_suffix, u, and_fn, u));
            out.push_str(&format!("        __m{}i cmp{} = {}(masked{}, target_v);\n", type_suffix, u, cmp_fn, u));
            // AND with 1s to get 0x01 per match (pcmpeqb gives 0xFF)
            out.push_str(&format!("        __m{}i bits{} = {}(cmp{}, ones);\n", type_suffix, u, and_fn, u));
            // Use psadbw to sum the 0/1 bytes
            out.push_str(&format!("        __m{}i sums{} = _mm{}_sad_epu8(bits{}, zero);\n", type_suffix, u, type_suffix, u));
            out.push_str(&format!("        acc = _mm{}_add_epi64(acc, sums{});\n", type_suffix, u));
        }
        out.push_str(&format!("        i += {};\n", step));
        out.push_str("    }\n");
        // Extract sum from accumulator
        if vw == 32 {
            out.push_str("    __m128i lo = _mm256_castsi256_si128(acc);\n");
            out.push_str("    __m128i hi = _mm256_extracti128_si256(acc, 1);\n");
            out.push_str("    uint64_t count = (uint64_t)_mm_cvtsi128_si64(lo)\n");
            out.push_str("                     + (uint64_t)_mm_extract_epi64(lo, 1)\n");
            out.push_str("                     + (uint64_t)_mm_cvtsi128_si64(hi)\n");
            out.push_str("                     + (uint64_t)_mm_extract_epi64(hi, 1);\n");
        } else {
            out.push_str("    uint64_t count = (uint64_t)_mm_cvtsi128_si64(acc) + (uint64_t)_mm_extract_epi64(acc, 1);\n");
        }
    }

    // Tail
    if plan.tail == TailStrategy::NarrowerVector && vw == 32 {
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        out.push_str("        __m128i masked = _mm_and_si128(chunk, _mm_set1_epi8((char)mask));\n");
        out.push_str("        __m128i cmp = _mm_cmpeq_epi8(masked, _mm_set1_epi8((char)target));\n");
        out.push_str("        int bits = _mm_movemask_epi8(cmp);\n");
        out.push_str("        count += __builtin_popcount(bits);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    out.push_str("        count += ((buf[i] & mask) == target);\n");
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return count;\n");
}

/// The deterministic selection (Engine 0): what the current ABSAC pipeline picks.
pub fn engine0_selection(kernel: &str) -> CandidatePlan {
    match kernel {
        "k18_all_equal" => CandidatePlan {
            id: 0, kernel: kernel.to_string(),
            isa: ISA::AVX2,
            reduction: ReductionStrategy::MovemaskPopcount,
            unroll: UnrollFactor::U1,
            tail: TailStrategy::NarrowerVector,
            control: ControlFlow::EarlyExit,
        },
        "k43_sum_ascii" => CandidatePlan {
            id: 0, kernel: kernel.to_string(),
            isa: ISA::AVX2,
            reduction: ReductionStrategy::Psadbw,
            unroll: UnrollFactor::U1,
            tail: TailStrategy::NarrowerVector,
            control: ControlFlow::FullReduction,
        },
        "k50_count_masked" => CandidatePlan {
            id: 0, kernel: kernel.to_string(),
            isa: ISA::AVX2,
            reduction: ReductionStrategy::MovemaskPopcount,
            unroll: UnrollFactor::U1,
            tail: TailStrategy::NarrowerVector,
            control: ControlFlow::FullReduction,
        },
        _ => panic!("unknown kernel: {}", kernel),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_k18_plans() {
        let plans = enumerate_plans("k18_all_equal");
        // Should have multiple plans
        assert!(plans.len() > 5);
        // All should be k18
        assert!(plans.iter().all(|p| p.kernel == "k18_all_equal"));
        // No duplicates
        let unique: std::collections::HashSet<_> = plans.iter().collect();
        assert_eq!(unique.len(), plans.len());
    }

    #[test]
    fn enumerate_k43_plans() {
        let plans = enumerate_plans("k43_sum_ascii");
        assert!(plans.len() > 3);
        // No MovemaskPopcount for Sum
        assert!(plans.iter().all(|p| p.reduction != ReductionStrategy::MovemaskPopcount));
        // No EarlyExit for Sum
        assert!(plans.iter().all(|p| p.control != ControlFlow::EarlyExit));
    }

    #[test]
    fn enumerate_k50_plans() {
        let plans = enumerate_plans("k50_count_masked");
        assert!(plans.len() > 3);
        // No Psadbw for Cardinality
        assert!(plans.iter().all(|p| p.reduction != ReductionStrategy::Psadbw));
        // No EarlyExit for Cardinality
        assert!(plans.iter().all(|p| p.control != ControlFlow::EarlyExit));
    }

    #[test]
    fn engine0_picks_best_known() {
        let k18 = engine0_selection("k18_all_equal");
        assert_eq!(k18.isa, ISA::AVX2);
        assert_eq!(k18.reduction, ReductionStrategy::MovemaskPopcount);
        assert_eq!(k18.control, ControlFlow::EarlyExit);

        let k43 = engine0_selection("k43_sum_ascii");
        assert_eq!(k43.isa, ISA::AVX2);
        assert_eq!(k43.reduction, ReductionStrategy::Psadbw);

        let k50 = engine0_selection("k50_count_masked");
        assert_eq!(k50.isa, ISA::AVX2);
        assert_eq!(k50.reduction, ReductionStrategy::MovemaskPopcount);
    }

    #[test]
    fn no_invalid_combinations() {
        for kernel in &["k18_all_equal", "k43_sum_ascii", "k50_count_masked"] {
            let plans = enumerate_plans(kernel);
            for p in &plans {
                // Scalar only with ScalarLoop
                if p.isa == ISA::Scalar {
                    assert_eq!(p.reduction, ReductionStrategy::ScalarLoop);
                }
                // Non-scalar never with ScalarLoop
                if p.isa != ISA::Scalar {
                    assert_ne!(p.reduction, ReductionStrategy::ScalarLoop);
                }
                // NarrowerVector tail only with AVX2
                if p.tail == TailStrategy::NarrowerVector {
                    assert_eq!(p.isa, ISA::AVX2);
                }
                // EarlyExit only for k18
                if p.control == ControlFlow::EarlyExit {
                    assert_eq!(*kernel, "k18_all_equal");
                }
            }
        }
    }
}
