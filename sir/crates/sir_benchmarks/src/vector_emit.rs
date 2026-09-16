//! Vector emit — target-aware lowering of vector plans to C with intrinsics.

use crate::vector_plan::{VectorOp, VectorPredicate, TailPolicy, VectorPlan};
use sir_nodes::Function;

pub fn emit_vectorized(func: &Function, plan: &VectorPlan) -> String {
    let mut out = String::new();

    // Function signature — match the original param names
    let ret_type = c_type(&func.return_ty);
    out.push_str(&format!("{} {}(", ret_type, func.name));
    for (i, param) in func.params.iter().enumerate() {
        if i > 0 { out.push_str(", "); }
        let pname = sanitize_name(&param.name, i);
        out.push_str(&format!("{} {}", c_type(&param.ty), pname));
    }
    out.push_str(") {\n");

    let buf = &plan.buffer_name;
    let len = &plan.length_name;

    match plan.operation {
        VectorOp::Cardinality => emit_cardinality(&mut out, plan, buf, len),
        VectorOp::All => emit_all(&mut out, plan, buf, len),
        VectorOp::Sum => emit_sum(&mut out, plan, buf, len),
    }

    out.push_str("}\n");
    out
}

fn emit_cardinality(out: &mut String, plan: &VectorPlan, buf: &str, len: &str) {
    let vw = plan.vector_width;

    // Resolve predicate values
    let (mask_c, target_c, has_mask) = match &plan.predicate {
        Some(VectorPredicate::MaskedEqual { mask, target }) => {
            (mask.clone(), target.clone(), true)
        }
        Some(VectorPredicate::Equal(target)) => {
            (String::new(), target.clone(), false)
        }
        _ => (String::new(), String::new(), false),
    };

    if vw == 32 {
        // AVX2
        if has_mask {
            out.push_str(&format!("    __m256i mask_v = _mm256_set1_epi8((char){});\n", mask_c));
        }
        if !target_c.is_empty() {
            out.push_str(&format!("    __m256i target_v = _mm256_set1_epi8((char){});\n", target_c));
        }
        out.push_str("    uint64_t count = 0;\n");
        out.push_str("    uint64_t i = 0;\n");
        out.push_str(&format!("    while (i + 32 <= {}) {{\n", len));
        out.push_str(&format!("        __m256i chunk = _mm256_loadu_si256((__m256i *)({} + i));\n", buf));
        if has_mask {
            out.push_str("        __m256i masked = _mm256_and_si256(chunk, mask_v);\n");
            out.push_str("        __m256i cmp = _mm256_cmpeq_epi8(masked, target_v);\n");
        } else if !target_c.is_empty() {
            out.push_str("        __m256i cmp = _mm256_cmpeq_epi8(chunk, target_v);\n");
        }
        out.push_str("        int bits = _mm256_movemask_epi8(cmp);\n");
        out.push_str("        count += __builtin_popcount(bits);\n");
        out.push_str("        i += 32;\n");
        out.push_str("    }\n");

        // 16-byte tail
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        if has_mask {
            out.push_str(&format!("        __m128i masked = _mm_and_si128(chunk, _mm_set1_epi8((char){}));\n", mask_c));
            out.push_str(&format!("        __m128i cmp = _mm_cmpeq_epi8(masked, _mm_set1_epi8((char){}));\n", target_c));
        } else if !target_c.is_empty() {
            out.push_str(&format!("        __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char){}));\n", target_c));
        }
        out.push_str("        int bits = _mm_movemask_epi8(cmp);\n");
        out.push_str("        count += __builtin_popcount(bits);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    } else {
        // SSE2 (vw=16)
        if has_mask {
            out.push_str(&format!("    __m128i mask_v = _mm_set1_epi8((char){});\n", mask_c));
        }
        if !target_c.is_empty() {
            out.push_str(&format!("    __m128i target_v = _mm_set1_epi8((char){});\n", target_c));
        }
        out.push_str("    uint64_t count = 0;\n");
        out.push_str("    uint64_t i = 0;\n");
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        if has_mask {
            out.push_str("        __m128i masked = _mm_and_si128(chunk, mask_v);\n");
            out.push_str("        __m128i cmp = _mm_cmpeq_epi8(masked, target_v);\n");
        } else if !target_c.is_empty() {
            out.push_str("        __m128i cmp = _mm_cmpeq_epi8(chunk, target_v);\n");
        }
        out.push_str("        int bits = _mm_movemask_epi8(cmp);\n");
        out.push_str("        count += __builtin_popcount(bits);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    match &plan.predicate {
        Some(VectorPredicate::MaskedEqual { mask, target }) => {
            out.push_str(&format!("        count += (({}[i] & {}) == {});\n", buf, mask, target));
        }
        Some(VectorPredicate::Equal(target)) => {
            out.push_str(&format!("        count += ({}[i] == {});\n", buf, target));
        }
        _ => {
            out.push_str(&format!("        count += {}[i];\n", buf));
        }
    }
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return count;\n");
}

fn emit_all(out: &mut String, plan: &VectorPlan, buf: &str, len: &str) {
    let vw = plan.vector_width;
    let target_c = match &plan.predicate {
        Some(VectorPredicate::Equal(t)) => t.clone(),
        _ => String::new(),
    };

    if vw == 32 {
        if !target_c.is_empty() {
            out.push_str(&format!("    __m256i target_v = _mm256_set1_epi8((char){});\n", target_c));
        }
        out.push_str("    uint64_t i = 0;\n");
        out.push_str(&format!("    while (i + 32 <= {}) {{\n", len));
        out.push_str(&format!("        __m256i chunk = _mm256_loadu_si256((__m256i *)({} + i));\n", buf));
        if !target_c.is_empty() {
            out.push_str("        __m256i cmp = _mm256_cmpeq_epi8(chunk, target_v);\n");
        }
        out.push_str("        int mask = _mm256_movemask_epi8(cmp);\n");
        out.push_str("        if (mask != -1) return 0;\n");
        out.push_str("        i += 32;\n");
        out.push_str("    }\n");

        // 16-byte tail
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        if !target_c.is_empty() {
            out.push_str(&format!("        __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char){}));\n", target_c));
        }
        out.push_str("        int mask = _mm_movemask_epi8(cmp);\n");
        out.push_str("        if (mask != 0xFFFF) return 0;\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    if !target_c.is_empty() {
        out.push_str(&format!("        if ({}[i] != {}) return 0;\n", buf, target_c));
    }
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return 1;\n");
}

fn emit_sum(out: &mut String, plan: &VectorPlan, buf: &str, len: &str) {
    let vw = plan.vector_width;

    if vw == 32 {
        out.push_str("    __m256i zero = _mm256_setzero_si256();\n");
        out.push_str("    __m256i acc = zero;\n");
        out.push_str("    uint64_t i = 0;\n");
        out.push_str(&format!("    while (i + 32 <= {}) {{\n", len));
        out.push_str(&format!("        __m256i chunk = _mm256_loadu_si256((__m256i *)({} + i));\n", buf));
        out.push_str("        __m256i sums = _mm256_sad_epu8(chunk, zero);\n");
        out.push_str("        acc = _mm256_add_epi64(acc, sums);\n");
        out.push_str("        i += 32;\n");
        out.push_str("    }\n");
        out.push_str("    __m128i lo = _mm256_castsi256_si128(acc);\n");
        out.push_str("    __m128i hi = _mm256_extracti128_si256(acc, 1);\n");
        out.push_str("    uint64_t sum = (uint64_t)_mm_cvtsi128_si64(lo)\n");
        out.push_str("                 + (uint64_t)_mm_extract_epi64(lo, 1)\n");
        out.push_str("                 + (uint64_t)_mm_cvtsi128_si64(hi)\n");
        out.push_str("                 + (uint64_t)_mm_extract_epi64(hi, 1);\n");

        // 16-byte tail
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        out.push_str("        __m128i sums = _mm_sad_epu8(chunk, _mm_setzero_si128());\n");
        out.push_str("        sum += (uint64_t)_mm_cvtsi128_si64(sums) + (uint64_t)_mm_extract_epi64(sums, 1);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
    } else {
        out.push_str("    __m128i zero = _mm_setzero_si128();\n");
        out.push_str("    __m128i acc = zero;\n");
        out.push_str("    uint64_t i = 0;\n");
        out.push_str(&format!("    while (i + 16 <= {}) {{\n", len));
        out.push_str(&format!("        __m128i chunk = _mm_loadu_si128((__m128i *)({} + i));\n", buf));
        out.push_str("        __m128i sums = _mm_sad_epu8(chunk, zero);\n");
        out.push_str("        acc = _mm_add_epi64(acc, sums);\n");
        out.push_str("        i += 16;\n");
        out.push_str("    }\n");
        out.push_str("    uint64_t sum = (uint64_t)_mm_cvtsi128_si64(acc) + (uint64_t)_mm_extract_epi64(acc, 1);\n");
    }

    // Scalar tail
    out.push_str(&format!("    while (i < {}) {{\n", len));
    out.push_str(&format!("        sum += {}[i];\n", buf));
    out.push_str("        i++;\n");
    out.push_str("    }\n");
    out.push_str("    return sum;\n");
}

pub fn sanitize_name(name: &str, idx: usize) -> String {
    if name.starts_with('%') {
        format!("p{}", idx)
    } else {
        name.to_string()
    }
}

pub fn c_type(ty: &sir_types::Type) -> String {
    use sir_types::Type;
    match ty {
        Type::Unit => "void".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Integer { width, .. } => {
            use sir_types::IntegerWidth;
            match width {
                IntegerWidth::I8 => "uint8_t".to_string(),
                IntegerWidth::I16 => "uint16_t".to_string(),
                IntegerWidth::I32 => "uint32_t".to_string(),
                IntegerWidth::I64 => "uint64_t".to_string(),
                IntegerWidth::I128 => "uint64_t".to_string(),
            }
        }
        Type::Array { element, .. } => format!("const {} *", c_type(element)),
        Type::Pointer { .. } => "const uint8_t *".to_string(),
        _ => "uint64_t".to_string(),
    }
}
