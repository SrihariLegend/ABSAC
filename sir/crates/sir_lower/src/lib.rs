//! sir_lower — LLVM IR → SIR lowerer (restricted subset).
//!
//! Parses LLVM IR text (.ll) and lowers a restricted subset to SIR:
//! - Integer arithmetic: add, sub, mul, div, rem
//! - Bitwise: and, or, xor, shl, lshr, ashr, not (via xor -1)
//! - Comparisons: icmp eq/ne/sgt/sge/slt/sle/ugt/uge/ult/ule
//! - Type widening: zext, sext (→ Convert node)
//! - Type narrowing: trunc (→ Convert node)
//! - Select (ternary)
//! - Memory: getelementptr + load → ArrayAccess
//! - Control flow: phi + br → Loop (with carried inputs/outputs)
//! - Return
//!
//! NOT supported (rejected explicitly):
//! - Function calls (except a few known intrinsics: llvm.umax, llvm.umin)
//! - Store, alloca, heap allocation
//! - Floating point
//! - Structs, vectors
//! - Early-exit loops (break with non-trivial value)

use std::collections::{HashMap, HashSet};

use sir_builder::Builder;
use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, Effects, IntegerWidth, NodeId, Span, Type};

/// A single LLVM IR instruction.
#[derive(Clone, Debug)]
struct Instruction {
    result: Option<String>, // %N or %name
    opcode: String,         // add, load, icmp, phi, br, ret, etc.
    operands: Vec<String>,  // raw operand strings
    raw: String,            // full line for debugging
}

/// A basic block.
#[derive(Clone, Debug)]
struct Block {
    label: String,
    instructions: Vec<Instruction>,
    preds: Vec<String>,
}

/// A parsed LLVM IR function.
struct IrFunction {
    name: String,
    ret_type: String,
    params: Vec<(String, String)>, // (name, type)
    blocks: Vec<Block>,
    block_map: HashMap<String, usize>,
}

/// Parse a type string like "i8", "i64", "ptr", "i1" to a SIR Type.
fn parse_type(s: &str) -> Option<Type> {
    let s = s.trim();
    match s {
        "i1" => Some(Type::Bool),
        "i8" => Some(Type::u8()),
        "i16" => Some(Type::u16()),
        "i32" => Some(Type::u32()),
        "i64" => Some(Type::u64()),
        "i128" => Some(Type::Integer {
            width: IntegerWidth::I128,
            signed: false,
            overflow: sir_types::OverflowBehavior::Wrapping,
        }),
        "ptr" | "ptr noundef" | "ptr nocapture" => Some(Type::Pointer {
            pointee: Box::new(Type::u8()),
            mutable: false,
        }),
        _ => None,
    }
}

/// Extract the source array type from a GEP instruction
/// (`getelementptr inbounds [64 x i8], ptr @g, ...` → `Array { u8, 64 }`).
///
/// The globals scan uses this to give a global-array parameter its real
/// extent. It historically hardcoded `Array<u8, 256>`, which made every
/// non-256-byte global unbindable at application binding ("counted loop
/// does not cover the complete collection extent") — H4b finding F10,
/// 2026-09-16.
fn gep_source_array_type(line: &str) -> Option<Type> {
    let start = line.find('[')?;
    let rest = &line[start + 1..];
    let end = rest.find(']')?;
    let (length_str, element_str) = rest[..end].split_once('x')?;
    let length = length_str.trim().parse::<usize>().ok()?;
    let element = parse_type(element_str.trim())?;
    Some(Type::Array {
        element: Box::new(element),
        length,
    })
}

/// Pointee type of a `getelementptr` source operand as used by a
/// parameter: `<elem>, ptr %p, ...` (or `[N x T], ptr %p, i64 0, i64 %i`
/// where the accessed element is `T`).
fn gep_pointee_type(inst: &Instruction) -> Option<Type> {
    if inst.operands.len() < 2 {
        return None;
    }
    let elem_str = inst.operands[0].replace("inbounds ", "");
    let elem_str = elem_str.trim();
    let multi_index = inst.operands.len() >= 4;
    let ty = if elem_str.starts_with('[') {
        gep_source_array_type(&inst.raw)?
    } else {
        parse_type(elem_str)?
    };
    Some(match ty {
        Type::Array { element, .. } if multi_index => *element,
        other => other,
    })
}

/// Infer the pointee type (and mutability) of opaque `ptr` parameters
/// from the element types of the GEPs/loads/stores that use them.
///
/// LLVM 15+ opaque pointers erase pointee types. The lowerer historically
/// modeled every `ptr` parameter as `*const u8` (H3 finding F7 — buffer
/// element-width typing): `ArrayAccess` nodes carried the real GEP source
/// element type (e.g. `i16`), but the SIR base stayed `*u8`, so the
/// interpreter read 8-bit elements and the C emitter declared
/// `uint8_t *`. Infer the pointee from direct uses; mixed-element uses
/// (aliasing through one pointer) keep the byte view — never guess.
fn infer_ptr_param_types(ir: &IrFunction) -> HashMap<String, (Type, bool)> {
    let ptr_param_names: Vec<&str> = ir
        .params
        .iter()
        .filter(|(_, ty)| ty.starts_with("ptr"))
        .map(|(name, _)| name.as_str())
        .collect();

    /// Record a candidate pointee; conflicting views collapse to `None`.
    fn note(
        candidates: &mut HashMap<String, Option<Type>>,
        name: &str,
        ty: Option<Type>,
    ) {
        let entry = candidates.entry(name.to_string()).or_insert_with(|| ty.clone());
        if entry.as_ref() != ty.as_ref() {
            *entry = None;
        }
    }

    let is_param = |name: &str| ptr_param_names.iter().any(|p| *p == name);
    let mut candidates: HashMap<String, Option<Type>> = HashMap::new();
    let mut mutable: HashMap<String, bool> = HashMap::new();
    // GEP result name → owning parameter, so stores through a GEP still
    // mark the parameter mutable.
    let mut gep_base: HashMap<String, String> = HashMap::new();

    for block in &ir.blocks {
        for inst in &block.instructions {
            match inst.opcode.as_str() {
                "getelementptr" if inst.operands.len() >= 2 => {
                    let base = strip_type(&inst.operands[1]);
                    if is_param(&base) {
                        note(&mut candidates, &base, gep_pointee_type(inst));
                    }
                    if let Some(result) = &inst.result {
                        gep_base.insert(result.clone(), base);
                    }
                }
                "load" if inst.operands.len() >= 2 => {
                    let ty = operand_type(&inst.operands[0]);
                    let base = strip_type(&inst.operands[1]);
                    if is_param(&base) {
                        note(&mut candidates, &base, ty);
                    } else if let Some(owner) = gep_base.get(&base).cloned() {
                        note(&mut candidates, &owner, ty);
                    }
                }
                "store" if inst.operands.len() >= 2 => {
                    let ty = operand_type(&inst.operands[0]);
                    let base = strip_type(&inst.operands[1]);
                    let owner = if is_param(&base) {
                        Some(base.clone())
                    } else {
                        gep_base.get(&base).cloned()
                    };
                    if let Some(owner) = owner {
                        note(&mut candidates, &owner, ty);
                        mutable.insert(owner, true);
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = HashMap::new();
    for name in ptr_param_names {
        let pointee = candidates
            .get(name)
            .cloned()
            .flatten()
            .unwrap_or_else(Type::u8);
        let mutable = mutable.get(name).copied().unwrap_or(false);
        out.insert(name.to_string(), (pointee, mutable));
    }
    out
}

/// Parse an integer constant from an operand string.
fn parse_int_constant(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.starts_with('-') {
        s.parse::<i64>().ok()
    } else {
        s.parse::<u64>().ok().map(|v| v as i64)
    }
}

/// Parse LLVM IR text into an IrFunction.
fn parse_function(text: &str) -> Result<IrFunction, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut func_name = String::new();
    let mut ret_type = String::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut block_map: HashMap<String, usize> = HashMap::new();
    let mut current_block: Option<Block> = None;

    // Find the function definition line
    let mut func_start = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("define ") && line.contains(" @") {
            func_start = Some(i);
            break;
        }
    }
    let func_start = func_start.ok_or("no function definition found")?;

    // Parse function signature
    let sig_line = lines[func_start];
    // e.g. "define dso_local i64 @name(ptr nocapture %0, i64 %1) #0 {"
    // Extract return type
    if let Some(at_pos) = sig_line.find('@') {
        let before_at = &sig_line[..at_pos];
        // The return type is the last token before @
        ret_type = before_at
            .split_whitespace()
            .last()
            .unwrap_or("void")
            .to_string();
        // Extract function name
        let after_at = &sig_line[at_pos + 1..];
        if let Some(open_paren) = after_at.find('(') {
            func_name = after_at[..open_paren].trim().to_string();
            // Extract params
            let params_str = &after_at[open_paren + 1..];
            if let Some(close_paren) = params_str.rfind(')') {
                let params_inner = &params_str[..close_paren];
                for param in params_inner.split(',') {
                    let param = param.trim();
                    if param.is_empty() {
                        continue;
                    }
                    // e.g. "ptr nocapture noundef readonly %0" or "i64 noundef %1"
                    // The param name is the last %N token
                    let parts: Vec<&str> = param.split_whitespace().collect();
                    let name = parts
                        .iter()
                        .rev()
                        .find(|p| p.starts_with('%'))
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    // The type is the first token
                    let ptype = parts.first().unwrap_or(&"void").to_string();
                    params.push((name, ptype));
                }
            }
        }
    }

    // Parse body — find blocks
    let mut i = func_start + 1;
    let mut pred_map: HashMap<String, Vec<String>> = HashMap::new();

    while i < lines.len() {
        let line = lines[i].trim();

        // End of function
        if line == "}" {
            break;
        }

        // Skip empty lines, metadata, attributes
        if line.is_empty()
            || line.starts_with(';')
            || line.starts_with('!')
            || line.starts_with("attributes")
        {
            i += 1;
            continue;
        }

        // Block label (either "N:" or "entry:", possibly followed by a comment)
        // LLVM labels look like: "7:" or "7:   ; preds = %6, %2"
        if !line.starts_with('%') && !line.starts_with("define") {
            // Check if the line starts with a label: "<label>:" possibly followed by comments
            let before_comment = line.split(';').next().unwrap_or(line).trim();
            if before_comment.ends_with(':')
                && !before_comment.contains(' ')
                && !before_comment.is_empty()
            {
                let label = before_comment.trim_end_matches(':').to_string();
                if let Some(b) = current_block.take() {
                    blocks.push(b);
                }
                current_block = Some(Block {
                    label: label.clone(),
                    instructions: Vec::new(),
                    preds: Vec::new(),
                });
                block_map.insert(label, blocks.len());
                i += 1;
                continue;
            }
        }

        // Instruction line (may span multiple lines if it has continuation)
        // Check if it's a labeled block with a pred comment
        if line.starts_with(';') {
            i += 1;
            continue;
        }

        // Collect the full instruction (handle multi-line via trailing commas)
        let mut inst_text = line.to_string();
        while inst_text.ends_with(',') && !inst_text.contains('=') && i + 1 < lines.len() {
            i += 1;
            let next = lines[i].trim();
            if next.is_empty() || next.starts_with(';') {
                break;
            }
            inst_text = format!("{} {}", inst_text, next);
        }

        // Parse the instruction
        if let Some(inst) = parse_instruction(&inst_text) {
            // Track predecessors from branches
            match inst.opcode.as_str() {
                "br" => {
                    if inst.operands.len() >= 3 {
                        // conditional br: br i1 %cond, label %true, label %false
                        let true_label = inst.operands[1].trim_start_matches("label ");
                        let false_label = inst.operands[2].trim_start_matches("label ");
                        pred_map
                            .entry(true_label.to_string())
                            .or_default()
                            .push(String::new()); // will fix later
                        pred_map
                            .entry(false_label.to_string())
                            .or_default()
                            .push(String::new());
                    } else if inst.operands.len() == 1 {
                        // unconditional br: br label %target
                        let target = inst.operands[0].trim_start_matches("label ");
                        pred_map
                            .entry(target.to_string())
                            .or_default()
                            .push(String::new());
                    }
                }
                _ => {}
            }

            if let Some(ref mut b) = current_block {
                b.instructions.push(inst);
            } else {
                // No block yet — create an implicit entry block
                current_block = Some(Block {
                    label: "entry".to_string(),
                    instructions: vec![],
                    preds: vec![],
                });
                block_map.insert("entry".to_string(), 0);
                if let Some(ref mut b) = current_block {
                    b.instructions.push(parse_instruction(&inst_text).unwrap());
                }
            }
        }

        i += 1;
    }

    if let Some(b) = current_block {
        blocks.push(b);
    }

    // Rebuild block_map with final indices
    block_map.clear();
    for (idx, b) in blocks.iter().enumerate() {
        block_map.insert(b.label.clone(), idx);
    }

    // Fix predecessors
    for b in &mut blocks {
        b.preds = pred_map.get(&b.label).cloned().unwrap_or_default();
    }

    Ok(IrFunction {
        name: func_name,
        ret_type,
        params,
        blocks,
        block_map,
    })
}

/// Parse a single instruction line.
fn parse_instruction(line: &str) -> Option<Instruction> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') || line.starts_with('!') {
        return None;
    }

    // Check if it has a result: "%N = opcode ..."
    if let Some(eq_pos) = line.find("= ") {
        // Check that this is an assignment (the = is not inside a comparison)
        let before_eq = line[..eq_pos].trim();
        if before_eq.starts_with('%') {
            let after_eq = line[eq_pos + 2..].trim();
            let (opcode, rest) = split_opcode(after_eq);
            let operands = split_operands(rest);
            return Some(Instruction {
                result: Some(before_eq.to_string()),
                opcode: opcode.to_string(),
                operands,
                raw: line.to_string(),
            });
        }
    }

    // No result — it's a void instruction (br, ret, store, etc.)
    let (opcode, rest) = split_opcode(line);
    if opcode.is_empty() {
        return None;
    }
    let operands = split_operands(rest);
    Some(Instruction {
        result: None,
        opcode: opcode.to_string(),
        operands,
        raw: line.to_string(),
    })
}

/// Split the opcode from the rest of the instruction.
fn split_opcode(s: &str) -> (&str, &str) {
    let s = s.trim();
    if let Some(space) = s.find(' ') {
        let opcode = &s[..space];
        // Handle special cases: "icmp eq", "br i1", "br label", "tail call"
        let rest = s[space + 1..].trim();
        if opcode == "icmp" || opcode == "br" {
            // For icmp, the next token is the comparison type (eq, ne, etc.)
            // We'll keep it as part of operands
            return (opcode, rest);
        }
        if opcode == "tail" || opcode == "musttail" {
            // "tail call ..." → opcode is "call", rest is the rest
            if rest.starts_with("call ") {
                return ("call", &rest[5..]);
            }
        }
        (opcode, rest)
    } else {
        (s, "")
    }
}

/// Split operands by commas, respecting parentheses and brackets.
fn split_operands(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }

    result
}

/// Strip metadata and type annotations from an operand to get the value reference.
/// e.g. "i64 %7" → "%7", "i8 %10" → "%10", "i64 0" → "0",
/// "nneg i8 %11" → "%11", "noundef i64 %1" → "%1"
fn strip_type(operand: &str) -> String {
    let operand = operand.trim();
    // Remove common LLVM qualifiers
    let operand = operand
        .replace("nneg ", "")
        .replace("noundef ", "")
        .replace("nocapture ", "")
        .replace("readonly ", "")
        .replace("zeroext ", "")
        .replace("signext ", "")
        .replace("inbounds ", "")
        .replace("nuw ", "")
        .replace("nsw ", "")
        .replace("tail ", "");
    let operand = operand.trim();
    // Remove leading type annotations
    let parts: Vec<&str> = operand.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let first = parts[0].trim();
        let second = parts[1].trim();
        if first.starts_with('i') && first[1..].chars().all(|c| c.is_ascii_digit()) {
            return second.to_string();
        }
        if first == "ptr" || first == "label" {
            return second.to_string();
        }
    }
    operand.to_string()
}

/// Extract the LLVM type from a typed operand ("i8 0", "i1 false",
/// "nuw nsw i64 %10", "nneg i8 %11"). Qualifiers may appear before the
/// type, so scan every token for the first representable type. Returns
/// None for bare references ("%7") — those resolve via the value map.
fn operand_type(operand: &str) -> Option<Type> {
    let cleaned = operand
        .replace("nneg ", "")
        .replace("noundef ", "")
        .replace("nocapture ", "")
        .replace("readonly ", "")
        .replace("zeroext ", "")
        .replace("signext ", "")
        .replace("inbounds ", "")
        .replace("nuw ", "")
        .replace("nsw ", "")
        .replace("tail ", "");
    cleaned.split_whitespace().find_map(parse_type)
}

/// Get the NodeId for a value reference (either an SSA value, a parameter, or a constant).
/// `type_hint` is used when creating constant nodes — it determines the constant's type.
fn get_node_id(
    ref_str: &str,
    value_map: &HashMap<String, NodeId>,
    params: &[(String, String)],
    builder: &mut Builder,
    type_hint: Option<Type>,
) -> Option<NodeId> {
    let ref_str = strip_type(ref_str);
    // Strip leading @ for global references
    let ref_str_owned = ref_str.trim_start_matches('@').to_string();
    if let Some(id) = value_map.get(&ref_str_owned) {
        return Some(*id);
    }
    // Check if it's a parameter
    for (i, (name, _)) in params.iter().enumerate() {
        if name == &ref_str {
            return Some(NodeId::new(i as u64));
        }
    }
    // Check if it's an integer constant
    if let Some(val) = parse_int_constant(&ref_str) {
        let span = Span::unknown();
        let ty = type_hint.unwrap_or(Type::u64());
        let const_val = match &ty {
            Type::Integer {
                width: IntegerWidth::I8,
                signed: false,
                ..
            } => ConstantData::u8(val as u8),
            Type::Integer {
                width: IntegerWidth::I8,
                signed: true,
                ..
            } => ConstantData::i8(val as i8),
            Type::Integer {
                width: IntegerWidth::I16,
                signed: false,
                ..
            } => ConstantData::u16(val as u16),
            Type::Integer {
                width: IntegerWidth::I16,
                signed: true,
                ..
            } => ConstantData::i16(val as i16),
            Type::Integer {
                width: IntegerWidth::I32,
                signed: false,
                ..
            } => ConstantData::u32(val as u32),
            Type::Integer {
                width: IntegerWidth::I32,
                signed: true,
                ..
            } => ConstantData::i32(val as i32),
            Type::Integer {
                width: IntegerWidth::I64,
                signed: false,
                ..
            } => ConstantData::u64(val as u64),
            Type::Integer {
                width: IntegerWidth::I64,
                signed: true,
                ..
            } => ConstantData::i64(val),
            Type::Bool => ConstantData::boolean(val != 0),
            _ => ConstantData::u64(val as u64),
        };
        return Some(builder.constant(const_val, ty, span));
    }
    // Boolean literals ("true"/"false"): LLVM spells i1 constants this way
    // (e.g. `select i1 %c, i1 %x, i1 false`). The integer parser above only
    // handles numeric spellings, so without this branch the literal could
    // never resolve (n14_saturating_count).
    if ref_str == "true" || ref_str == "false" {
        let span = Span::unknown();
        return Some(builder.constant(
            ConstantData::boolean(ref_str == "true"),
            Type::Bool,
            span,
        ));
    }
    None
}

/// Parse LLVM IR text and extract the first function definition.
fn extract_first_function(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut start = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("define ") && line.contains(" @") {
            start = Some(i);
            break;
        }
    }
    let start = match start {
        Some(s) => s,
        None => return text.to_string(),
    };
    // Find the closing brace at the start of a line (depth 0)
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim() == "}" {
            return lines[start..=i].join("\n");
        }
    }
    lines[start..].join("\n")
}

/// Lower a specific LLVM IR function (by name) to SIR.
pub fn lower_function(text: &str, func_name: &str) -> Result<Function, String> {
    let func_text = extract_function_by_name(text, func_name);
    if func_text.is_empty() {
        return Err(format!("function '{}' not found", func_name));
    }
    let mut func = lower(&func_text)?;

    // Sound dynamic-extent recall slice (constant extents): promote a
    // pointer parameter to a fixed array view when every access to it
    // is proven within a common constant extent. Conservative: any
    // uncovered use leaves the parameter a pointer (no fabricated
    // extent, no candidate).
    promote_constant_extent_buffers(&mut func);

    // ══════════════════════════════════════════════════════════
    // MANDATORY LOWERER VALIDATION (Gate 6A-v1, Priority 0A)
    //
    // No code path may obtain unverified SIR from the lowerer.
    // Finding: V07 lowered to structurally invalid SIR (two-loop CFG
    // bug) and returned Ok — recognition then ran on broken IR.
    // Verification is now part of lowering: lowering that produces
    // invalid SIR is an error, not silent success.
    //
    // This contains the more dangerous future failure mode only
    // partially: structurally valid SIR with wrong semantics still
    // requires translation validation (future work).
    // ══════════════════════════════════════════════════════════
    let mut verifier = sir_verify::Verifier::new(&func);
    if !verifier.verify() {
        let summary: Vec<String> = verifier
            .errors()
            .iter()
            .take(3)
            .map(|e| format!("{:?}", e))
            .collect();
        return Err(format!(
            "lowered SIR failed verification (soundness gate): {}",
            summary.join("; ")
        ));
    }
    Ok(func)
}

/// The integer value of a constant node (signed values normalized to
/// u64 when non-negative).
fn lower_constant_u64(func: &Function, id: NodeId) -> Option<u64> {
    match &func.get_node(id)?.kind {
        NodeKind::Constant(data) => data
            .as_u64()
            .or_else(|| data.as_i64().and_then(|v| u64::try_from(v).ok())),
        _ => None,
    }
}

/// The constant trip count of a strict counted loop whose carried
/// counter is `counter`: counter starts at 0, its successor is
/// `counter + 1` among the outputs, and the termination is EXACTLY the
/// bound test (`counter < K`, `counter != K`, or `counter <= K-1`).
/// A termination with an early-exit conjunct (found-flag scans) returns
/// None: those loops may read fewer than K elements, so promoting the
/// parameter to `[T; K]` would over-assert the caller's buffer.
fn strict_counted_bound(
    func: &Function,
    termination: NodeId,
    counter: NodeId,
    outputs: &[NodeId],
) -> Option<u64> {
    if lower_constant_u64(func, counter) != Some(0) {
        return None;
    }
    let successor_seen = outputs.iter().any(|out| {
        matches!(
            func.get_node(*out).map(|n| &n.kind),
            Some(NodeKind::Add { lhs, rhs })
                if *lhs == counter && lower_constant_u64(func, *rhs) == Some(1)
        )
    });
    if !successor_seen {
        return None;
    }
    // The counter comparison may be a CONJUNCT of the termination: the
    // synthesized early-exit search loop terminates on
    // `!found && i < BOUND`. Conjuncts only shrink the iteration domain,
    // so a proven counter bound still bounds every access.
    let bound = find_counter_bound_conjunct(func, termination, counter)?;
    if bound == 0 {
        return None;
    }
    Some(bound)
}

/// The constant bound of a `counter < K` / `counter != K` /
/// `counter <= K` comparison appearing anywhere in a conjunction.
fn find_counter_bound_conjunct(func: &Function, term: NodeId, counter: NodeId) -> Option<u64> {
    match func.get_node(term).map(|n| &n.kind) {
        Some(NodeKind::Lt { lhs, rhs }) | Some(NodeKind::Ne { lhs, rhs })
            if *lhs == counter =>
        {
            lower_constant_u64(func, *rhs)
        }
        Some(NodeKind::Le { lhs, rhs }) if *lhs == counter => {
            lower_constant_u64(func, *rhs)?.checked_add(1)
        }
        Some(NodeKind::BoolAnd { lhs, rhs }) => {
            find_counter_bound_conjunct(func, *lhs, counter)
                .or_else(|| find_counter_bound_conjunct(func, *rhs, counter))
        }
        _ => None,
    }
}

/// Promote `Pointer` parameters to `Array { element, length: K }` when
/// every use of the parameter is an `ArrayAccess` whose index is
/// provably within one common constant extent K:
///   - a constant index c requires K > c;
///   - a strict counted-loop counter requires K >= the loop's constant
///     trip count (see `strict_counted_bound`);
///   - any other use — an uncovered index, a store, a call, a return,
///     a direct load, or disagreeing element types — aborts promotion.
///
/// This asserts only what a defined execution of the source already
/// requires (all K elements are read), and it never fabricates an
/// extent: an unproven access leaves the pointer type in place.
fn promote_constant_extent_buffers(func: &mut Function) {
    let parameters: Vec<(usize, NodeId)> = func
        .arena
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::Parameter { index } => Some((*index, node.id)),
            _ => None,
        })
        .collect();

    for (index, param_id) in parameters {
        let Some(param) = func.get_node(param_id) else {
            continue;
        };
        if !matches!(param.ty, Type::Pointer { .. }) {
            continue;
        }

        // Every use must be an ArrayAccess on this pointer.
        let mut accesses: Vec<NodeId> = Vec::new();
        let mut unsupported = false;
        for node in func.arena.iter() {
            if node.kind.input_nodes().contains(&param_id) {
                if matches!(node.kind, NodeKind::ArrayAccess { .. }) {
                    accesses.push(node.id);
                } else {
                    unsupported = true;
                    break;
                }
            }
        }
        if unsupported || accesses.is_empty() {
            continue;
        }

        let mut extent: u64 = 0;
        let mut element_ty: Option<Type> = None;
        let mut saw_loop_bound = false;
        let mut ok = true;
        for access_id in &accesses {
            let Some(access) = func.get_node(*access_id) else {
                ok = false;
                break;
            };
            let NodeKind::ArrayAccess { index: idx, .. } = &access.kind else {
                ok = false;
                break;
            };
            let idx = *idx;
            match &element_ty {
                None => element_ty = Some(access.ty.clone()),
                Some(prev) if *prev != access.ty => {
                    ok = false;
                    break;
                }
                _ => {}
            }

            // The index may be the carried counter of an enclosing
            // strict counted loop. NOTE: in SIR the carried input IS a
            // constant node (the counter's initial value 0), so the
            // loop-domain proof must be tried BEFORE the constant-index
            // fast path — otherwise a 96-element scan would promote to
            // a 1-element view.
            let mut bound: Option<u64> = None;
            let mut enclosing_carried = false;
            for loop_node in func.arena.iter() {
                let NodeKind::Loop {
                    body,
                    termination,
                    outputs,
                    carried_inputs,
                } = &loop_node.kind
                else {
                    continue;
                };
                if !body.contains(access_id) || !carried_inputs.contains(&idx) {
                    continue;
                }
                enclosing_carried = true;
                if let Some(k) = strict_counted_bound(func, *termination, idx, outputs) {
                    bound = Some(bound.map_or(k, |prev| prev.min(k)));
                }
            }
            if let Some(k) = bound {
                extent = extent.max(k);
                saw_loop_bound = true;
                continue;
            }
            // A loop-varying index whose bound is not a constant has no
            // proven extent — never fall through to the constant path
            // (the carried input node is the constant initial value).
            if enclosing_carried {
                ok = false;
                break;
            }

            match lower_constant_u64(func, idx) {
                Some(c) => extent = extent.max(c.saturating_add(1)),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok || extent == 0 || !saw_loop_bound {
            continue;
        }

        let element = element_ty.unwrap_or_else(|| match &param.ty {
            Type::Pointer { pointee, .. } => (**pointee).clone(),
            other => other.clone(),
        });
        let array_ty = Type::Array {
            element: Box::new(element),
            length: extent as usize,
        };
        if let Some(node) = func.arena.get_mut(param_id) {
            node.ty = array_ty.clone();
        }
        if let Some(param) = func.params.get_mut(index) {
            param.ty = array_ty;
        }
    }
}

/// Extract a specific function by name from LLVM IR text.
fn extract_function_by_name(text: &str, name: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("define ") && line.contains(&format!("@{}", name)) {
            // Found the function — extract until closing brace
            for (j, end_line) in lines.iter().enumerate().skip(i + 1) {
                if end_line.trim() == "}" {
                    return lines[i..=j].join("\n");
                }
            }
        }
    }
    String::new()
}

/// A single-loop region outlined from a multi-loop function.
///
/// Gate 6B fusion support: every self-latch loop of a sequential-loop
/// function becomes its own single-loop LLVM function, so the existing
/// per-loop pipeline (lowering → analysis → recognition → plan
/// derivation) sees one reduction region at a time.
///
/// Provenance is preserved by construction: the original parameters keep
/// their positions in every region, so two regions that read the same
/// buffer and length carry the *same* parameter positions. Any value
/// produced by another loop becomes an explicit dependency parameter and
/// is recorded in `dep_sources`; the composition stage must refuse to
/// fuse across a dependency edge.
#[derive(Clone, Debug)]
pub struct LoopRegion {
    /// Function name of the outlined region (`<fn>_region<k>`).
    pub name: String,
    /// Self-contained LLVM IR module text defining the region.
    pub text: String,
    /// SIR parameter names in order: original params first, then deps.
    pub params: Vec<String>,
    /// Number of original function parameters (prefix of `params`).
    pub original_params: usize,
    /// For each dependency parameter, the index of the region that
    /// defines it (aligned with `params[original_params..]`); `None`
    /// means the definition site is outside the extracted chain.
    pub dep_sources: Vec<Option<usize>>,
}

/// True when `block` carries its own back edge (a self-latch loop header).
fn self_latch_block(block: &Block) -> bool {
    block.instructions.iter().any(|i| {
        if i.opcode != "phi" {
            return false;
        }
        let combined = i.operands.join(", ");
        combined.contains(&format!("%{}]", block.label))
            || combined.contains(&format!(", %{} ]", block.label))
            || combined.contains(&format!("%{}", block.label))
    })
}

/// Successor labels of a block (from its `br`), without the `%` prefix.
fn block_successors(block: &Block) -> Vec<String> {
    for inst in &block.instructions {
        match inst.opcode.as_str() {
            "br" => {
                return inst
                    .operands
                    .iter()
                    .filter_map(|op| {
                        op.trim()
                            .strip_prefix("label ")
                            .map(|l| l.trim().trim_start_matches('%').to_string())
                    })
                    .collect();
            }
            "ret" => return Vec::new(),
            _ => {}
        }
    }
    Vec::new()
}

/// The non-self target of a loop header's conditional branch.
fn loop_exit_label(block: &Block) -> Option<String> {
    for inst in &block.instructions {
        if inst.opcode == "br" && inst.operands.len() >= 3 {
            let true_label = inst.operands[1]
                .trim_start_matches("label ")
                .trim_start_matches('%')
                .to_string();
            let false_label = inst.operands[2]
                .trim_start_matches("label ")
                .trim_start_matches('%')
                .to_string();
            return Some(if true_label == block.label {
                false_label
            } else {
                true_label
            });
        }
    }
    None
}

/// First LLVM type token in an operand list (`i64`, `i8`, `ptr`, …).
fn first_type_token(operands: &[String]) -> Option<String> {
    const TYPES: [&str; 7] = ["i1", "i8", "i16", "i32", "i64", "i128", "ptr"];
    for op in operands {
        for token in op.split(|c: char| c.is_whitespace() || c == ',' || c == ']' || c == '[') {
            let token = token.trim();
            if TYPES.contains(&token) {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Extract the first `%name` value token from an operand string.
fn value_name(s: &str) -> Option<String> {
    let start = s.find('%')?;
    let rest = &s[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '%'))
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Value names *used* by an instruction (phi labels and `br` targets are
/// not values).
fn used_value_names(inst: &Instruction) -> Vec<String> {
    let mut out = Vec::new();
    match inst.opcode.as_str() {
        "br" | "ret" => {}
        "phi" => {
            for op in &inst.operands {
                let op = op.trim();
                let Some(inner) = op.strip_prefix('[') else { continue };
                let Some(inner) = inner.strip_suffix(']') else { continue };
                let Some((val, _label)) = inner.split_once(',') else { continue };
                if let Some(name) = value_name(val) {
                    out.push(name);
                }
            }
        }
        _ => {
            for op in &inst.operands {
                if let Some(name) = value_name(op) {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// True when the phi has an incoming edge from `label` (the loop back edge).
fn phi_fed_by(inst: &Instruction, label: &str) -> bool {
    for op in &inst.operands {
        let op = op.trim();
        let Some(inner) = op.strip_prefix('[') else { continue };
        let Some(inner) = inner.strip_suffix(']') else { continue };
        let Some((_val, lbl)) = inner.split_once(',') else { continue };
        if lbl.trim().trim_start_matches('%') == label {
            return true;
        }
    }
    false
}

/// Rewrite `%name` tokens through `map` (exact-token replacement).
fn rewrite_value_names(text: &str, map: &HashMap<String, String>) -> String {
    if map.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '.')
            {
                j += 1;
            }
            let token: String = chars[i..j].iter().collect();
            if let Some(replacement) = map.get(&token) {
                out.push_str(replacement);
            } else {
                out.push_str(&token);
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// LLVM result type of a named value defined in the function.
fn defined_value_type(ir: &IrFunction, name: &str) -> String {
    for block in &ir.blocks {
        for inst in &block.instructions {
            if inst.result.as_deref() == Some(name) {
                if inst.opcode == "icmp" || inst.opcode == "fcmp" {
                    return "i1".to_string();
                }
                return first_type_token(&inst.operands).unwrap_or_else(|| "i64".to_string());
            }
        }
    }
    "i64".to_string()
}

/// Order self-latch headers along the function's execution chain.
fn order_loop_chain(ir: &IrFunction, headers: &[usize]) -> Result<Vec<usize>, String> {
    let header_of = |label: &str| -> Option<usize> {
        headers
            .iter()
            .copied()
            .find(|&i| ir.blocks[i].label == label)
    };
    let mut order = Vec::new();
    let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut cursor_label = ir.blocks[0].label.clone();
    for _ in 0..4096 {
        if order.len() == headers.len() {
            break;
        }
        let Some(&idx) = ir.block_map.get(&cursor_label) else {
            return Err(format!(
                "cannot follow block '{}' while ordering loop headers",
                cursor_label
            ));
        };
        if !visited.insert(idx) {
            return Err(format!(
                "loop chain revisits block '{}' while ordering headers",
                cursor_label
            ));
        }
        if let Some(h) = header_of(&cursor_label) {
            order.push(h);
        }
        let succs = block_successors(&ir.blocks[idx]);
        let next = succs
            .iter()
            .find(|l| {
                ir.block_map
                    .get(*l)
                    .map(|i| !visited.contains(i))
                    .unwrap_or(false)
                    && header_of(l).is_some()
            })
            .or_else(|| {
                succs.iter().find(|l| {
                    ir.block_map
                        .get(*l)
                        .map(|i| !visited.contains(i))
                        .unwrap_or(false)
                })
            })
            .cloned();
        match next {
            Some(l) => cursor_label = l,
            None => break,
        }
    }
    if order.len() != headers.len() {
        return Err(format!(
            "could not order all loop headers ({} of {})",
            order.len(),
            headers.len()
        ));
    }
    Ok(order)
}

/// Outline every self-latch loop of `func_name` as its own single-loop
/// LLVM function. See [`LoopRegion`].
pub fn extract_loop_regions(text: &str, func_name: &str) -> Result<Vec<LoopRegion>, String> {
    let func_text = extract_function_by_name(text, func_name);
    if func_text.is_empty() {
        return Err(format!("function '{}' not found", func_name));
    }
    let ir = parse_function(&func_text)?;

    let headers: Vec<usize> = ir
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| self_latch_block(b))
        .map(|(i, _)| i)
        .collect();
    if headers.is_empty() {
        return Err(format!(
            "function '{}' has no self-latch loops to extract",
            func_name
        ));
    }
    let order = order_loop_chain(&ir, &headers)?;

    let mut regions: Vec<LoopRegion> = Vec::new();
    let mut produced_by: HashMap<String, usize> = HashMap::new();

    for (k, &h) in order.iter().enumerate() {
        let header = &ir.blocks[h];
        let exit_label = loop_exit_label(header)
            .ok_or_else(|| format!("loop block '{}' has no conditional exit", header.label))?;
        let exit_idx = *ir
            .block_map
            .get(&exit_label)
            .ok_or_else(|| format!("loop exit block '{}' not found", exit_label))?;
        let exit_block = &ir.blocks[exit_idx];

        // Result: the exit-block phi fed by this loop's back edge, or the
        // exit block's return operand when the loop exits to the tail.
        let mut result: Option<(String, String)> = None;
        for inst in &exit_block.instructions {
            if inst.opcode == "phi" && phi_fed_by(inst, &header.label) {
                let name = inst
                    .result
                    .clone()
                    .ok_or_else(|| "loop merge phi has no result".to_string())?;
                let ty = first_type_token(&inst.operands).unwrap_or_else(|| "i64".to_string());
                result = Some((name, ty));
                break;
            }
        }
        let (result_name, result_ty) = match result {
            Some(r) => r,
            None => {
                let ret = exit_block
                    .instructions
                    .iter()
                    .find(|i| i.opcode == "ret")
                    .ok_or_else(|| {
                        format!(
                            "cannot determine the result of loop block '{}'",
                            header.label
                        )
                    })?;
                let op = ret
                    .operands
                    .first()
                    .ok_or_else(|| "loop exit returns void".to_string())?;
                (
                    strip_type(op),
                    first_type_token(&ret.operands).unwrap_or_else(|| "i64".to_string()),
                )
            }
        };

        // Instruction stream of the region: pre-blocks (first region only),
        // the loop header, and the exit block's merge phis.
        let mut insts: Vec<Instruction> = Vec::new();
        let mut pre_blocks: Vec<(String, Vec<Instruction>)> = Vec::new();
        if k == 0 {
            for (bi, b) in ir.blocks.iter().enumerate() {
                if bi >= h {
                    break;
                }
                if b.instructions.iter().any(|i| i.opcode == "ret") {
                    continue;
                }
                // Do not duplicate the loop's own exit block (it is emitted
                // again below with the merge phis) and skip any other loop
                // header that precedes this one in block order.
                if b.label == exit_label || self_latch_block(b) {
                    continue;
                }
                let kept: Vec<Instruction> = b
                    .instructions
                    .iter()
                    .filter(|i| !matches!(i.opcode.as_str(), "br" | "ret" | "phi"))
                    .cloned()
                    .collect();
                insts.extend(kept.iter().cloned());
                pre_blocks.push((b.label.clone(), kept));
            }
        }
        insts.extend(header.instructions.iter().cloned());
        for inst in &exit_block.instructions {
            if inst.opcode == "phi" {
                insts.push(inst.clone());
            }
        }

        // Defined vs free names.
        let mut defined: std::collections::HashSet<String> =
            ir.params.iter().map(|(n, _)| n.clone()).collect();
        for inst in &insts {
            if let Some(r) = &inst.result {
                defined.insert(r.clone());
            }
        }
        let mut free: Vec<String> = Vec::new();
        for inst in &insts {
            for name in used_value_names(inst) {
                if !defined.contains(&name) && !free.contains(&name) {
                    free.push(name);
                }
            }
        }

        // Dependency parameters.
        let mut rewrite: HashMap<String, String> = HashMap::new();
        let mut dep_sources: Vec<Option<usize>> = Vec::new();
        for (d, old) in free.iter().enumerate() {
            let new_name = format!("%dep{}", d);
            rewrite.insert(old.clone(), new_name);
            dep_sources.push(produced_by.get(old).copied());
        }

        // Module text.
        let mut out = String::new();
        let mut param_decls: Vec<String> = ir
            .params
            .iter()
            .map(|(name, ty)| format!("{} {}", ty, name))
            .collect();
        for (d, old) in free.iter().enumerate() {
            param_decls.push(format!("{} %dep{}", defined_value_type(&ir, old), d));
        }
        out.push_str(&format!(
            "define {} @{}_region{}({}) {{\n",
            result_ty,
            func_name,
            k,
            param_decls.join(", ")
        ));

        for (label, kept) in &pre_blocks {
            out.push_str(&format!("{}:\n", label));
            for inst in kept {
                out.push_str(&format!(
                    "  {}\n",
                    rewrite_value_names(&inst.raw, &rewrite)
                ));
            }
        }
        out.push_str(&format!("{}:\n", header.label));
        for inst in &header.instructions {
            out.push_str(&format!(
                "  {}\n",
                rewrite_value_names(&inst.raw, &rewrite)
            ));
        }
        out.push_str(&format!("{}:\n", exit_block.label));
        for inst in &exit_block.instructions {
            if inst.opcode == "phi" {
                out.push_str(&format!(
                    "  {}\n",
                    rewrite_value_names(&inst.raw, &rewrite)
                ));
            }
        }
        out.push_str(&format!(
            "  ret {} {}\n}}\n",
            result_ty,
            rewrite_value_names(&result_name, &rewrite)
        ));

        // Record what this region defines for later regions.
        for inst in &insts {
            if let Some(r) = &inst.result {
                produced_by.entry(r.clone()).or_insert(k);
            }
        }

        let mut params: Vec<String> = ir.params.iter().map(|(n, _)| n.clone()).collect();
        for d in 0..free.len() {
            params.push(format!("%dep{}", d));
        }

        regions.push(LoopRegion {
            name: format!("{}_region{}", func_name, k),
            text: out,
            params,
            original_params: ir.params.len(),
            dep_sources,
        });
    }

    Ok(regions)
}

/// List all function names in the LLVM IR text.
pub fn list_functions(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        if line.contains("define ") && line.contains(" @") {
            // Extract function name between @ and (
            if let Some(at_pos) = line.find('@') {
                let after_at = &line[at_pos + 1..];
                if let Some(open_paren) = after_at.find('(') {
                    let name = after_at[..open_paren].trim().to_string();
                    // Skip intrinsic declarations
                    if !name.starts_with("llvm.") && !name.starts_with("_") {
                        names.push(name);
                    }
                }
            }
        }
    }
    names
}

/// Lower an LLVM IR function to SIR.
pub fn lower(text: &str) -> Result<Function, String> {
    let text = extract_first_function(text);
    let ir = parse_function(&text)?;

    // Map return type
    let ret_ty = parse_type(&ir.ret_type).unwrap_or(Type::Unit);

    // Opaque `ptr` parameters: recover the pointee element type (F7).
    let inferred_ptr_types = infer_ptr_param_types(&ir);

    // Build parameter list for SIR
    let mut sir_params: Vec<(&str, Type)> = ir
        .params
        .iter()
        .map(|(name, ty)| {
            let sir_ty = if ty.starts_with("ptr") {
                let (pointee, mutable) = inferred_ptr_types
                    .get(name)
                    .cloned()
                    .unwrap_or((Type::u8(), false));
                Type::Pointer {
                    pointee: Box::new(pointee),
                    mutable,
                }
            } else {
                parse_type(ty).unwrap_or(Type::u64())
            };
            (name.as_str(), sir_ty)
        })
        .collect();

    // Scan for global array references (e.g., @popcount_table) in the function body.
    // These are added as extra implicit parameters so the SIR function can access them.
    let mut globals: Vec<(String, Type)> = Vec::new();
    for line in text.lines() {
        if line.contains("getelementptr") && line.contains(" ptr @") {
            if let Some(at) = line.find(" ptr @") {
                let after = &line[at + 5..];
                if let Some(comma) = after.find(',') {
                    let gname = after[..comma].trim().trim_start_matches('@').to_string();
                    if !gname.starts_with("llvm.") && !globals.iter().any(|(n, _)| n == &gname) {
                        // The GEP source element type carries the global's
                        // declared array shape (`inbounds [N x T]`). Fall back
                        // to the historical 256-byte extent only for flat
                        // scalar-element pointer GEPs that carry no extent.
                        let ty = gep_source_array_type(line).unwrap_or(Type::Array {
                            element: Box::new(Type::u8()),
                            length: 256,
                        });
                        globals.push((gname, ty));
                    }
                }
            }
        }
    }

    // Add globals as extra array parameters with their declared extents.
    let mut global_param_names: Vec<String> = Vec::new();
    for (gname, _) in &globals {
        global_param_names.push(format!("__glob_{}", gname));
    }
    for (pname, (_, gty)) in global_param_names.iter().zip(globals.iter()) {
        sir_params.push((pname.as_str(), gty.clone()));
    }

    let mut builder = Builder::new(&ir.name, &sir_params, ret_ty);
    let n_orig_params = ir.params.len();

    // Value map: LLVM value name → SIR NodeId
    let mut value_map: HashMap<String, NodeId> = HashMap::new();

    // Map parameters
    for (i, (name, _)) in ir.params.iter().enumerate() {
        let node_id = builder.parameter_index(i).unwrap();
        value_map.insert(name.clone(), node_id);
    }

    // Map global names to their extra parameter NodeIds
    for (i, (gname, _)) in globals.iter().enumerate() {
        let param_idx = n_orig_params + i;
        let node_id = builder.parameter_index(param_idx).unwrap();
        value_map.insert(gname.clone(), node_id);
    }

    // Detect loop structure: find the back-edge block (the one that branches to itself or to a predecessor)
    // Simple pattern: entry → header (loop) → exit
    // header has a phi for each carried value, a conditional branch at the end

    // For now, handle the common pattern:
    // Block 0 (entry): initial check + br to loop or exit
    // Block 1 (loop header): phi nodes + body + br back or to exit
    // Block 2 (exit): phi for result + ret

    if ir.blocks.len() < 2 {
        return Err("function has too few blocks for the lowerer".to_string());
    }

    // Find the loop blocks: blocks with phi nodes AND a self-referencing
    // back-edge (an incoming value from the block itself). This recognizes
    // single-block loops where the header is its own latch — the only loop
    // shape this lowerer emits.
    let self_loop_blocks: Vec<usize> = ir
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            b.instructions.iter().any(|i| {
                if i.opcode == "phi" {
                    // Check if any incoming label matches this block's label
                    let combined = i.operands.join(", ");
                    combined.contains(&format!("%{}]", b.label))
                        || combined.contains(&format!(", %{} ]", b.label))
                        || combined.contains(&format!("%{}", b.label))
                } else {
                    false
                }
            })
        })
        .map(|(idx, _)| idx)
        .collect();

    // ── Fail-closed control-flow gates (Gate 6A corpus findings) ──
    // The emitted SIR model is exactly ONE loop whose header is its own
    // latch. Shapes outside that model are refused EXPLICITLY here instead
    // of half-lowered into invalid SIR. Before these gates:
    //   - two loops sharing an exit CFG (w08_two_reductions,
    //     v07_count_then_sum) were lowered as far as the first loop and the
    //     function came out without a Return — caught only downstream by
    //     the sir_verify soundness gate (MissingReturn).
    //   - a loop whose latch is a separate block (x08_early_exit_write,
    //     n03_early_terminate, n08_find_first_mismatch) went undetected;
    //     its header phis were never mapped and lowering died mid-
    //     instruction on "cannot resolve gep index '%N'".
    if self_loop_blocks.len() > 1 {
        // SEQUENTIAL MULTI-LOOP COMPOSITION (w08/p12 recall, re-landed
        // 2026-09-17 behind the multi-loop emitter): lower each
        // self-latching loop in block order. The first loop's exit walk
        // stops at the next loop's conditional guard, so its exit phis
        // feed the second loop's pre-header as straight-line values; the
        // last loop's exit walk emits the shared exit and the single
        // return. Any failure (nested/shared-exit shapes, unresolved
        // cross-loop values, a second return) falls back to the
        // historical explicit refusal — never a partial function. The
        // SIR→C emitter composes sequential loops (namespaced
        // carriers/outputs) and the native differential gates this
        // path.
        let mut sequential_ok = true;
        // Blocks consumed by an already-lowered loop must not be emitted
        // again as straight-line code: the duplicates are dead but they
        // break the constant-extent promotion's "every access is inside
        // a proven loop" proof (v6 p08/w08/p12 had an outside-loop
        // duplicate of the first loop's ArrayAccess, so the buffer was
        // never promoted and the whole-function regions produced no
        // candidates).
        let mut consumed_blocks: HashSet<String> = HashSet::new();
        for &lbi in &self_loop_blocks {
            if lower_loop_function(
                &ir,
                lbi,
                &mut builder,
                &mut value_map,
                &consumed_blocks,
            )
            .is_err()
            {
                sequential_ok = false;
                break;
            }
            if let Some(block) = ir.blocks.get(lbi) {
                consumed_blocks.insert(block.label.clone());
            }
        }
        if sequential_ok && builder.function().return_node.is_some() {
            return Ok(builder.build());
        }
        return Err(format!(
            "unsupported: multiple loops sharing an exit CFG (nested/sequential loops) not modeled: {}",
            ir.name
        ));
    }
    // A back-edge is any branch whose target block precedes its own block.
    let back_edge_exists = ir.blocks.iter().enumerate().any(|(idx, b)| {
        b.instructions.iter().any(|i| {
            if i.opcode != "br" {
                return false;
            }
            i.operands.iter().any(|op| {
                let Some(target) = op.trim().strip_prefix("label ") else {
                    return false; // the condition operand, not a target
                };
                ir.block_map
                    .get(target.trim().trim_start_matches('%'))
                    .map(|&target_idx| target_idx < idx)
                    .unwrap_or(false)
            })
        })
    });
    if self_loop_blocks.is_empty() && back_edge_exists {
        // EARLY-EXIT SEARCH LOWERING (v7 p09 recall): clang's
        // `for (i…) if (a[i]) return i;` is a header/latch/merge CFG
        // rather than a self-latching block. Synthesize the canonical
        // found-flag SIR loop so the scans recognizer and bitscan
        // recipes apply; any mismatch falls back to the historical
        // refusal (never a partial or guessed lowering).
        return match lower_early_exit_search(&ir, &mut builder, &mut value_map) {
            Ok(()) => Ok(builder.build()),
            Err(reason) => Err(format!(
                "unsupported: loop with a separate latch block (multi-block back-edge) not modeled: {} ({})",
                ir.name, reason
            )),
        };
    }

    let loop_block_idx = self_loop_blocks.first().copied();

    let result = match loop_block_idx {
        Some(lbi) => lower_loop_function(
            &ir,
            lbi,
            &mut builder,
            &mut value_map,
            &HashSet::new(),
        ),
        None => lower_straight_line(&ir, &mut builder, &mut value_map),
    };

    result.map(|_| builder.build())
}

/// D4/F9: recognize clang's counted loop rotation and reconstruct the
/// canonical pre-checked iteration domain.
///
/// The clang rotation inside one loop block is:
///
/// ```text
///   %next = add %c, 1            ; next = carry + 1
///   %t    = icmp eq %next, %K    ; exit test on the SUCCESSOR
///   br i1 %t, label %exit, label %loop   ; back-edge on false
/// ```
///
/// Under SIR continue-while-TRUE semantics this is faithfully expressed
/// as `Lt(c, K)`: `c` runs `0..K-1`, and the runtime entry guard
/// `K == 0 → skip` that clang emits for non-constant bounds folds into
/// `Lt(0, 0) == false`. Returns `(carry, bound)` when exactly one side
/// of the eq test is `Add(carry, 1)` with `carry` one of the loop's
/// carried inputs and the step exactly one. Anything else returns
/// `None` (the caller refuses rather than mis-lower).
fn rotated_counted_domain(
    builder: &Builder,
    termination: NodeId,
    carried_init_nodes: &[NodeId],
) -> Option<(NodeId, NodeId)> {
    use sir_nodes::NodeKind;
    let function = builder.function();
    let term = function.get_node(termination)?;
    let NodeKind::Eq { lhs, rhs } = &term.kind else {
        return None;
    };
    for (candidate, bound) in [(*lhs, *rhs), (*rhs, *lhs)] {
        let add_node = function.get_node(candidate)?;
        let NodeKind::Add {
            lhs: augend,
            rhs: addend,
        } = &add_node.kind
        else {
            continue;
        };
        // One side of the add must be the loop-carried counter, the
        // other the literal step +1.
        let (carry, step) = if carried_init_nodes.contains(augend) {
            (*augend, *addend)
        } else if carried_init_nodes.contains(addend) {
            (*addend, *augend)
        } else {
            continue;
        };
        let step_node = function.get_node(step)?;
        let NodeKind::Constant(data) = &step_node.kind else {
            continue;
        };
        let is_one = data.as_u64() == Some(1) || data.as_i64() == Some(1);
        if is_one {
            return Some((carry, bound));
        }
    }
    None
}

/// D4/F9 counterpart for clang's "back-edge on true" successor rotation.
///
/// `for (i = 0; i < n; i += k)` (steps > 1, and other successor-tested
/// shapes) lowers to:
///
/// ```text
///   %next = add %c, %k          ; k > 0
///   %t    = icmp ult %next, %K  ; continue test on the SUCCESSOR
///   br i1 %t, label %loop, label %exit
/// ```
///
/// With the pre-loop entry guard (`K == 0 → exit`) the faithful
/// pre-checked iteration domain is `c < K` (or `c <= K` for `ule`): the
/// guard folds into `Lt(0, 0) == false`. Returns `(inclusive, carry,
/// bound)`. Unguarded successors (a genuine source `do { } while (next
/// cmp K)`) are *not* returned here; the caller refuses them because a
/// pre-tested SIR loop cannot force the first iteration.
fn rotated_successor_domain(
    builder: &Builder,
    termination: NodeId,
    carried_init_nodes: &[NodeId],
    carried_next_nodes: &[NodeId],
) -> Option<(bool, NodeId, NodeId)> {
    use sir_nodes::NodeKind;
    let function = builder.function();
    let term = function.get_node(termination)?;
    let (lhs, rhs, inclusive) = match &term.kind {
        NodeKind::Lt { lhs, rhs } => (*lhs, *rhs, false),
        NodeKind::Le { lhs, rhs } => (*lhs, *rhs, true),
        _ => return None,
    };
    for (candidate, bound) in [(lhs, rhs), (rhs, lhs)] {
        let add_node = function.get_node(candidate)?;
        let NodeKind::Add {
            lhs: augend,
            rhs: addend,
        } = &add_node.kind
        else {
            continue;
        };
        let (carry_index, carry, step) = if let Some(i) = carried_init_nodes
            .iter()
            .position(|c| c == augend)
        {
            (i, *augend, *addend)
        } else if let Some(i) = carried_init_nodes.iter().position(|c| c == addend) {
            (i, *addend, *augend)
        } else {
            continue;
        };
        // The test must compare the carry's own successor value; an
        // unrelated `carry + k` in the condition would make the rebuilt
        // `carry < bound` domain wrong.
        if carried_next_nodes.get(carry_index) != Some(&candidate) {
            continue;
        }
        let step_node = function.get_node(step)?;
        match &step_node.kind {
            // Constant steps must advance (step 0 would be an infinite
            // source loop; refuse rather than emit it).
            NodeKind::Constant(data) => {
                if data.as_u64().map(|k| k > 0).unwrap_or(false) {
                    return Some((inclusive, carry, bound));
                }
            }
            // Runtime steps are fine: the source loop advances by the
            // same value the successor test used.
            _ => return Some((inclusive, carry, bound)),
        }
    }
    None
}

/// Integer constant data of the same SIR type as `ty` (for reconstructed
/// bounds).
fn int_constant_data(ty: &Type, val: u64) -> ConstantData {
    match ty {
        Type::Integer {
            width: IntegerWidth::I8,
            signed: false,
            ..
        } => ConstantData::u8(val as u8),
        Type::Integer {
            width: IntegerWidth::I8,
            signed: true,
            ..
        } => ConstantData::i8(val as i8),
        Type::Integer {
            width: IntegerWidth::I16,
            signed: false,
            ..
        } => ConstantData::u16(val as u16),
        Type::Integer {
            width: IntegerWidth::I16,
            signed: true,
            ..
        } => ConstantData::i16(val as i16),
        Type::Integer {
            width: IntegerWidth::I32,
            signed: false,
            ..
        } => ConstantData::u32(val as u32),
        Type::Integer {
            width: IntegerWidth::I32,
            signed: true,
            ..
        } => ConstantData::i32(val as i32),
        Type::Integer {
            width: IntegerWidth::I64,
            signed: true,
            ..
        } => ConstantData::i64(val as i64),
        _ => ConstantData::u64(val),
    }
}

/// Reconstruct the pre-tested SIR domain of clang's UNGUARDED post-tested
/// (do-while) carry-compare loop, or return None when it cannot be
/// proven.
///
/// Shape (self-loop, back-edge on TRUE, unconditional entry):
///
/// ```text
///   step = CONST > 0
///   next = carry + step            (or carry - step, descending)
///   term = carry < K / <= K        (or > / >=, descending)
/// ```
///
/// LLVM executes the body FIRST and continues while the test on the
/// pre-body carry holds, so the executed carries are exactly
/// `carry < K + step` (ascending) or `carry > K - step` (descending) —
/// provided the first iteration is forced (`term(c0)` true) and the
/// shifted bound does not overflow. That is the pre-tested SIR Loop that
/// models the do-while faithfully. Anything else returns None and the
/// caller refuses loudly (the 2026-09-17 v6 corpus caught the silent
/// version: a stride-4 `i < 60` do-while lowered as a pre-tested
/// `i < 60`, dropping the forced final iteration).
fn post_tested_carry_domain(
    builder: &mut Builder,
    termination: NodeId,
    carried_init_nodes: &[NodeId],
    carried_next_nodes: &[NodeId],
) -> Option<NodeId> {
    // Read phase: everything derived from the current graph.
    let (carry, shifted, carry_ty, ascending) = {
        let function = builder.function();
        let term = function.get_node(termination)?;
        let (carry, bound, ascending) = match &term.kind {
            NodeKind::Lt { lhs, rhs } | NodeKind::Le { lhs, rhs } => (*lhs, *rhs, true),
            NodeKind::Gt { lhs, rhs } | NodeKind::Ge { lhs, rhs } => (*lhs, *rhs, false),
            _ => return None,
        };
        let carry_index = carried_init_nodes.iter().position(|c| *c == carry)?;
        let bound_value = lower_constant_u64(function, bound)?;
        let next = *carried_next_nodes.get(carry_index)?;
        let step_value = match function.get_node(next).map(|n| &n.kind) {
            Some(NodeKind::Add { lhs, rhs }) if ascending && *lhs == carry => {
                lower_constant_u64(function, *rhs)?
            }
            Some(NodeKind::Sub { lhs, rhs }) if !ascending && *lhs == carry => {
                lower_constant_u64(function, *rhs)?
            }
            _ => return None,
        };
        if step_value == 0 {
            return None;
        }
        let c0 = lower_constant_u64(function, carry)?;
        let shifted = if ascending {
            bound_value.checked_add(step_value)?
        } else {
            bound_value.checked_sub(step_value)?
        };
        // The forced first iteration must satisfy the reconstructed pre-test.
        if (ascending && c0 >= shifted) || (!ascending && c0 <= shifted) {
            return None;
        }
        let carry_ty = function.get_node(carry)?.ty.clone();
        (carry, shifted, carry_ty, ascending)
    };
    let span = Span::unknown();
    let shifted_node = builder.constant(int_constant_data(&carry_ty, shifted), carry_ty, span);
    if ascending {
        builder.lt(carry, shifted_node, span).ok()
    } else {
        builder.gt(carry, shifted_node, span).ok()
    }
}

/// Does the function have a pre-loop entry guard (clang's rotation
/// pre-check: `icmp eq %n, 0` plus a conditional branch before the loop
/// block)? Required before rebuilding a successor-tested loop as the
/// pre-checked `c < n` domain.
fn has_entry_guard(ir: &IrFunction, loop_block_idx: usize) -> bool {
    let (Some(entry), Some(loop_block)) = (ir.blocks.first(), ir.blocks.get(loop_block_idx))
    else {
        return false;
    };
    if entry.label == loop_block.label {
        return false;
    }
    let Some(br) = entry
        .instructions
        .iter()
        .rev()
        .find(|inst| inst.opcode == "br")
    else {
        return false;
    };
    if br.operands.len() < 3 {
        return false;
    }
    let cond = strip_type(&br.operands[0]);
    entry.instructions.iter().any(|inst| {
        inst.opcode == "icmp"
            && inst.result.as_deref() == Some(cond.as_str())
            && inst
                .operands
                .iter()
                .any(|op| op.trim() == "0" || op.trim().ends_with(" 0"))
    })
}

/// Parse a phi instruction's `[ value, %label ]` pairs.
fn phi_incomings(inst: &Instruction) -> Vec<(String, String)> {
    let combined = inst.operands.join(", ");
    let mut out = Vec::new();
    for segment in combined.split('[').skip(1) {
        let body = segment.split(']').next().unwrap_or("");
        let mut parts = body.split(',').map(|s| s.trim().to_string());
        if let (Some(value), Some(label)) = (parts.next(), parts.next()) {
            if !value.is_empty() && !label.is_empty() {
                out.push((value, label.trim().trim_start_matches('%').to_string()));
            }
        }
    }
    out
}

/// Lower clang's early-return search loop into the canonical found-flag
/// SIR loop:
///
/// ```text
///   H: %i = phi [init, …], [%next, %L]
///      <element load / test>            ; cond true = no hit, continue
///      br %cond, %L, %M                 ; false = hit -> early exit
///   L: %next = add %i, 1
///      %done = icmp eq %next, BOUND
///      br %done, %M, %H
///   M: %r = phi [sentinel, %L], [%i, %H]
///      ret %r
/// ```
///
/// The synthesized loop carries `(found, index, i)` and exits when found
/// or the counter reaches BOUND; `index` starts at the sentinel and is
/// updated by the first hit, reproducing `%r` exactly. Any shape
/// mismatch is an error and the caller keeps the historical refusal.
fn lower_early_exit_search(
    ir: &IrFunction,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
) -> Result<(), String> {
    let span = Span::unknown();

    // 1. Header H with a latch L that branches back to H; H and L share
    //    the merge block M.
    let mut triple: Option<(usize, usize, usize)> = None;
    'outer: for (hi, h) in ir.blocks.iter().enumerate() {
        if !h.instructions.iter().any(|i| i.opcode == "phi") {
            continue;
        }
        let h_succs = block_successors(h);
        if h_succs.len() != 2 {
            continue;
        }
        for (li, l) in ir.blocks.iter().enumerate() {
            if li == hi {
                continue;
            }
            let l_succs = block_successors(l);
            if l_succs.len() != 2 || !l_succs.contains(&h.label) {
                continue;
            }
            let Some(m_label) = h_succs.iter().find(|s| **s != l.label) else {
                continue;
            };
            if !l_succs.contains(m_label) {
                continue;
            }
            let Some(&mi) = ir.block_map.get(m_label) else {
                continue;
            };
            triple = Some((hi, li, mi));
            break 'outer;
        }
    }
    let (hi, li, mi) = triple.ok_or("early-exit search: no header/latch/merge triple")?;
    let h = &ir.blocks[hi];
    let l = &ir.blocks[li];
    let m = &ir.blocks[mi];
    let h_br = h
        .instructions
        .iter()
        .rev()
        .find(|i| i.opcode == "br")
        .ok_or("early-exit search: header has no branch")?;
    let l_br = l
        .instructions
        .iter()
        .rev()
        .find(|i| i.opcode == "br")
        .ok_or("early-exit search: latch has no branch")?;
    if h_br.operands.len() < 3 || l_br.operands.len() < 3 {
        return Err("early-exit search: expected conditional branches".into());
    }

    // 2. Exactly one counter phi in H, carried by L.
    let h_phis: Vec<&Instruction> = h
        .instructions
        .iter()
        .filter(|i| i.opcode == "phi")
        .collect();
    if h_phis.len() != 1 {
        return Err("early-exit search: expected one header phi".into());
    }
    let h_phi = h_phis[0];
    let counter_name = h_phi
        .result
        .clone()
        .ok_or("early-exit search: header phi without result")?;
    let counter_ty_str = h_phi
        .operands
        .first()
        .and_then(|o| o.split_whitespace().next())
        .unwrap_or("i64");
    let counter_ty = parse_type(counter_ty_str).ok_or("early-exit search: bad counter type")?;
    let h_incomings = phi_incomings(h_phi);
    let init_val = h_incomings
        .iter()
        .find(|(_, label)| label != &l.label)
        .map(|(v, _)| v.clone());
    let next_val = h_incomings
        .iter()
        .find(|(_, label)| label == &l.label)
        .map(|(v, _)| v.clone());
    let (Some(init_val), Some(_next_val)) = (init_val, next_val) else {
        return Err("early-exit search: counter phi is not latch-carried".into());
    };

    // 3. Merge phi: incoming from H is the counter, incoming from L is
    //    the sentinel; the merge block just returns the phi.
    let m_phis: Vec<&Instruction> = m
        .instructions
        .iter()
        .filter(|i| i.opcode == "phi")
        .collect();
    if m_phis.len() != 1 {
        return Err("early-exit search: expected one merge phi".into());
    }
    let m_phi = m_phis[0];
    let merge_ty_str = m_phi
        .operands
        .first()
        .and_then(|o| o.split_whitespace().next())
        .unwrap_or("i64");
    let index_ty = parse_type(merge_ty_str).ok_or("early-exit search: bad merge type")?;
    let m_incomings = phi_incomings(m_phi);
    let from_h = m_incomings
        .iter()
        .find(|(_, label)| label == &h.label)
        .map(|(v, _)| strip_type(v));
    let sentinel = m_incomings
        .iter()
        .find(|(_, label)| label == &l.label)
        .map(|(v, _)| strip_type(v));
    let (Some(from_h), Some(sentinel_str)) = (from_h, sentinel) else {
        return Err("early-exit search: merge phi lacks the H/L incoming pair".into());
    };
    if from_h != counter_name {
        return Err("early-exit search: merge does not return the counter".into());
    }
    let merge_result = m_phi.result.clone().unwrap_or_default();
    // The merge may post-process the phi before returning (clang's
    // runtime-extent search clamps it with `llvm.umin(phi, n)` on the
    // `n == 0` guard path). Those instructions are emitted after the
    // synthesized loop with the phi mapped to the index output, so the
    // observable result is preserved without assuming the clamp is a
    // no-op.
    let ret_operand = m
        .instructions
        .iter()
        .find(|i| i.opcode == "ret")
        .and_then(|i| i.operands.first().cloned())
        .ok_or("early-exit search: merge does not return")?;

    // Extra merge predecessors are only tolerated as the zero-trip entry
    // guard (`sentinel == 0 -> merge`) whose incoming value is the zero
    // constant: the synthesized loop's zero-trip output is the sentinel,
    // which that guard pins to zero.
    for (value, label) in &m_incomings {
        if label == &h.label || label == &l.label {
            continue;
        }
        let pi = match ir.block_map.get(label).copied() {
            Some(pi) => pi,
            // The parser names an unlabeled entry block "entry", while phi
            // references use LLVM's implicit numeric label. The entry is
            // the only unlabeled block in textual IR, so resolve a numeric
            // unknown label to block 0; the branch/guard validation below
            // still has to confirm the shape.
            None if ir.blocks.first().map(|b| b.label.as_str()) == Some("entry")
                && label.chars().all(|c| c.is_ascii_digit()) =>
            {
                0
            }
            None => return Err("early-exit search: merge incoming from an unknown block".into()),
        };
        let p = &ir.blocks[pi];
        let p_br = p
            .instructions
            .iter()
            .rev()
            .find(|i| i.opcode == "br")
            .ok_or("early-exit search: merge predecessor has no branch")?;
        let p_succs = block_successors(p);
        let p_cond = strip_type(&p_br.operands[0]);
        // icmp operands keep the comparison token in the first operand
        // ("eq i64 %1"), so compare on each operand's value token.
        let value_token = |o: &str| {
            o.split_whitespace()
                .last()
                .unwrap_or(o)
                .trim_end_matches(')')
                .to_string()
        };
        let guard_matches = p.instructions.iter().any(|inst| {
            inst.opcode == "icmp"
                && inst.result.as_deref() == Some(p_cond.as_str())
                && inst
                    .operands
                    .iter()
                    .any(|o| value_token(o) == sentinel_str)
                && inst
                    .operands
                    .iter()
                    .any(|o| {
                        parse_int_constant(&value_token(o))
                            .map(|v| v == 0)
                            .unwrap_or(false)
                    })
        });
        let zero_incoming = parse_int_constant(&strip_type(value))
            .map(|v| v == 0)
            .unwrap_or(false);
        if !(p_succs.contains(&m.label) && guard_matches && zero_incoming) {
            return Err("early-exit search: unsupported extra merge predecessor".into());
        }
    }

    // 4. Pre-header blocks (everything before H, except the merge).
    for (bi, b) in ir.blocks.iter().enumerate() {
        if bi >= hi {
            break;
        }
        if bi == mi || b.instructions.iter().any(|i| i.opcode == "ret") {
            continue;
        }
        for inst in &b.instructions {
            if inst.opcode == "phi" || inst.opcode == "br" || inst.opcode == "ret" {
                continue;
            }
            emit_instruction(inst, builder, value_map, &ir.params, span)?;
        }
    }

    // 5. Carried initial values.
    let counter_init = get_node_id(
        &init_val,
        value_map,
        &ir.params,
        builder,
        Some(counter_ty.clone()),
    )
    .ok_or("early-exit search: cannot resolve the counter initial value")?;
    let sentinel_node = get_node_id(
        &sentinel_str,
        value_map,
        &ir.params,
        builder,
        Some(index_ty.clone()),
    )
    .ok_or("early-exit search: cannot resolve the merge sentinel")?;
    value_map.insert(counter_name.clone(), counter_init);

    // 6. Emit H's and L's body instructions; the counter phi resolves to
    //    the carried index.
    let mut body_nodes: Vec<NodeId> = Vec::new();
    for inst in h.instructions.iter().chain(l.instructions.iter()) {
        if inst.opcode == "phi" || inst.opcode == "br" || inst.opcode == "ret" {
            continue;
        }
        if let Some(id) = emit_instruction(inst, builder, value_map, &ir.params, span)? {
            body_nodes.push(id);
        }
    }

    // 7. Branch polarities.
    let h_succs = block_successors(h);
    let hit_on_true = h_succs.first().map(|s| s == &m.label).unwrap_or(false);
    if hit_on_true && h_succs.get(1).map(|s| s != &l.label).unwrap_or(true) {
        return Err("early-exit search: header branch is not hit-exit".into());
    }
    if !hit_on_true && h_succs.first().map(|s| s != &l.label).unwrap_or(true) {
        return Err("early-exit search: header branch is not hit-exit".into());
    }
    let h_cond = get_node_id(
        &strip_type(&h_br.operands[0]),
        value_map,
        &ir.params,
        builder,
        None,
    )
    .ok_or("early-exit search: cannot resolve the header condition")?;
    let hit = if hit_on_true {
        h_cond
    } else {
        let not = builder
            .bool_not(h_cond, span)
            .map_err(|e| format!("early-exit search: {:?}", e))?;
        body_nodes.push(not);
        not
    };

    let l_succs = block_successors(l);
    if l_succs.first().map(|s| s != &m.label).unwrap_or(true)
        || l_succs.get(1).map(|s| s != &h.label).unwrap_or(true)
    {
        return Err("early-exit search: latch is not the counted exit-on-true form".into());
    }
    let l_cond = get_node_id(
        &strip_type(&l_br.operands[0]),
        value_map,
        &ir.params,
        builder,
        None,
    )
    .ok_or("early-exit search: cannot resolve the latch condition")?;
    let (carry, bound) = rotated_counted_domain(builder, l_cond, &[counter_init])
        .ok_or("early-exit search: latch is not a counted successor test")?;

    // 8. Synthesize the found-flag recurrence.
    let found_init = builder.constant(ConstantData::boolean(false), Type::Bool, span);
    let not_found = builder
        .bool_not(found_init, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let found_next = builder
        .bool_or(found_init, hit, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let update = builder
        .bool_and(hit, not_found, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let index_next = builder
        .select(update, counter_init, sentinel_node, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let successor = l
        .instructions
        .iter()
        .find_map(|inst| (inst.opcode == "add").then(|| inst.result.clone()).flatten())
        .ok_or("early-exit search: no latch successor")?;
    let successor = get_node_id(&successor, value_map, &ir.params, builder, None)
        .ok_or("early-exit search: cannot resolve the latch successor")?;
    let domain = builder
        .lt(carry, bound, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let termination = builder
        .bool_and(not_found, domain, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    body_nodes.extend([found_next, update, index_next, domain, termination]);

    // 9. Build the loop and return the index output.
    let outputs = vec![found_next, index_next, successor];
    let carried = vec![found_init, sentinel_node, counter_init];
    let loop_ty = Type::Tuple {
        elements: vec![Type::Bool, index_ty.clone(), counter_ty.clone()],
    };
    let loop_node = builder
        .r#loop(&body_nodes, termination, &outputs, &carried, loop_ty, span)
        .map_err(|e| format!("early-exit search loop build: {:?}", e))?;
    let index_extract = builder
        .tuple_extract(loop_node, 1, index_ty, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    let counter_extract = builder
        .tuple_extract(loop_node, 2, counter_ty, span)
        .map_err(|e| format!("early-exit search: {:?}", e))?;
    value_map.insert(counter_name, counter_extract);
    value_map.insert(merge_result, index_extract);

    // Emit the merge's post-processing and return its result; a bare
    // `ret phi` resolves straight back to the index extract.
    for inst in &m.instructions {
        if inst.opcode == "phi" || inst.opcode == "ret" {
            continue;
        }
        emit_instruction(inst, builder, value_map, &ir.params, span)?;
    }
    let ret_node = get_node_id(
        &strip_type(&ret_operand),
        value_map,
        &ir.params,
        builder,
        None,
    )
    .ok_or("early-exit search: cannot resolve the merge return value")?;
    builder
        .return_value(ret_node, span)
        .map_err(|e| format!("early-exit search return: {:?}", e))?;
    Ok(())
}

/// Lower a function with a detected loop.
fn lower_loop_function(
    ir: &IrFunction,
    loop_idx: usize,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
    skip_blocks: &HashSet<String>,
) -> Result<(), String> {
    let span = Span::unknown();

    // Emit pre-loop block instructions (non-branch) before the loop.
    //
    // The entry block often contains an initial comparison (e.g., icmp sgt n, 0)
    // whose result is used by exit-block phis. Pre-header blocks between the
    // entry and the loop often contain value conversions (e.g., zext i32 %1 to
    // i64 for a mixed-width loop bound) that the loop body references.
    //
    // Gate 6A-v1 finding: only the entry block was emitted, so a zext in an
    // intermediate pre-header block was never lowered and the loop body's
    // icmp failed with "cannot resolve rhs" (V02, N16, H01, H02).
    //
    // We emit ALL blocks that appear before the loop, EXCEPT:
    //   - the loop's exit block (it consumes loop outputs — must run after),
    //   - blocks containing ret (they belong to an alternate exit path).
    //
    // The loop's exit label is determined from the loop block's conditional
    // branch (the target that is not the loop itself).
    if loop_idx > 0 {
        // Find the loop's exit label from its conditional branch
        let loop_exit_label: Option<String> =
            ir.blocks[loop_idx].instructions.iter().find_map(|inst| {
                if inst.opcode == "br" && inst.operands.len() >= 3 {
                    let true_label = inst.operands[1]
                        .trim_start_matches("label ")
                        .trim_start_matches('%')
                        .to_string();
                    let false_label = inst.operands[2]
                        .trim_start_matches("label ")
                        .trim_start_matches('%')
                        .to_string();
                    let loop_label = &ir.blocks[loop_idx].label;
                    Some(if true_label == *loop_label {
                        false_label
                    } else {
                        true_label
                    })
                } else {
                    None
                }
            });

        for (bi, b) in ir.blocks.iter().enumerate() {
            if bi >= loop_idx {
                break;
            }
            if skip_blocks.contains(&b.label) {
                continue;
            }
            if Some(b.label.clone()) == loop_exit_label {
                continue;
            }
            if b.instructions.iter().any(|i| i.opcode == "ret") {
                continue;
            }
            for inst in &b.instructions {
                if inst.opcode == "br" || inst.opcode == "ret" || inst.opcode == "phi" {
                    continue;
                }
                emit_instruction(inst, builder, value_map, &ir.params, span)?;
            }
        }
    }

    // The loop block has phi nodes. Each phi has two incoming values:
    // one from the entry (initial value) and one from the loop body (next value).
    // We need to:
    // 1. Parse phi nodes to get carried inputs (initial) and outputs (next)
    // 2. Emit body instructions (non-phi) in the loop block
    // 3. Build the Loop node

    let loop_block = &ir.blocks[loop_idx];

    // Parse phi nodes
    // phi format: "%N = phi i64 [ %next, %loop_label ], [ init, %entry_label ]"
    // or: "%N = phi i64 [ 0, %entry ], [ %13, %loop ]"
    #[derive(Debug)]
    struct PhiInfo {
        result: String,
        ty: String,
        // (value, block_label) pairs
        incoming: Vec<(String, String)>,
    }

    let mut phis: Vec<PhiInfo> = Vec::new();
    for inst in &loop_block.instructions {
        if inst.opcode == "phi" {
            // Parse: "i64 [ %next, %loop ], [ init, %entry ]"
            let combined = inst.operands.join(", ");
            // Extract type (first token)
            let parts: Vec<&str> = combined.splitn(2, ' ').collect();
            let ty = parts[0].to_string();
            let rest = parts.get(1).unwrap_or(&"");

            // Parse incoming pairs: [ value, label ]
            let mut incoming = Vec::new();
            // Find all [ ... ] pairs
            let mut chars = rest.chars().peekable();
            let mut in_bracket = false;
            let mut current = String::new();
            let mut pair_parts: Vec<String> = Vec::new();

            while let Some(c) = chars.next() {
                match c {
                    '[' => {
                        in_bracket = true;
                        current.clear();
                        pair_parts.clear();
                    }
                    ']' if in_bracket => {
                        in_bracket = false;
                        if !current.trim().is_empty() {
                            pair_parts.push(current.trim().to_string());
                        }
                        // pair_parts should have [value, label]
                        // But we may have split on commas inside
                        // Actually the format is [ value, label ]
                        // Let's just split current on comma
                        incoming.push((
                            pair_parts.get(0).cloned().unwrap_or_default(),
                            pair_parts.get(1).cloned().unwrap_or_default(),
                        ));
                    }
                    ',' if in_bracket => {
                        if !current.trim().is_empty() {
                            pair_parts.push(current.trim().to_string());
                        }
                        current.clear();
                    }
                    _ if in_bracket => {
                        current.push(c);
                    }
                    _ => {}
                }
            }

            phis.push(PhiInfo {
                result: inst.result.clone().unwrap_or_default(),
                ty,
                incoming,
            });
        }
    }

    // Identify which incoming is the initial value (from entry) and which is the
    // loop-carried value (from the loop block itself or a block that branches back).
    // The entry block is typically the one that's not the loop block.
    let loop_label = &loop_block.label;

    // For each phi, find the initial value and the carried (next) value
    let mut carried_initials: Vec<(String, String)> = Vec::new(); // (phi_result, initial_value_str)
    let mut carried_nexts: Vec<(String, String)> = Vec::new(); // (phi_result, next_value_str)

    for phi in &phis {
        let mut initial = None;
        let mut next = None;
        for (val, label) in &phi.incoming {
            let label = label.trim().trim_start_matches('%');
            if label == loop_label.as_str() {
                // This is the back-edge (next value)
                next = Some(val.clone());
            } else {
                // This is the initial value
                initial = Some(val.clone());
            }
        }
        let initial = initial.ok_or(format!("phi {} has no initial value", phi.result))?;
        let next = next.ok_or(format!("phi {} has no back-edge value", phi.result))?;
        carried_initials.push((phi.result.clone(), initial));
        carried_nexts.push((phi.result.clone(), next));
    }

    // Create constant nodes for initial values
    let mut carried_init_nodes: Vec<NodeId> = Vec::new();
    let mut carried_names: Vec<String> = Vec::new();

    for (phi_result, init_str) in &carried_initials {
        let init_str = strip_type(init_str);
        let phi_ty = phis
            .iter()
            .find(|p| &p.result == phi_result)
            .map(|p| p.ty.as_str())
            .unwrap_or("i64");
        let sir_ty = parse_type(phi_ty).unwrap_or(Type::u64());
        let node_id = if let Some(id) = get_node_id(
            &init_str,
            value_map,
            &ir.params,
            builder,
            Some(sir_ty.clone()),
        ) {
            id
        } else if let Some(val) = parse_int_constant(&init_str) {
            // Create a constant node with the PHI's declared type
            let const_val = match &sir_ty {
                Type::Integer {
                    width: IntegerWidth::I8,
                    signed: false,
                    ..
                } => ConstantData::u8(val as u8),
                Type::Integer {
                    width: IntegerWidth::I8,
                    signed: true,
                    ..
                } => ConstantData::i8(val as i8),
                Type::Integer {
                    width: IntegerWidth::I16,
                    signed: false,
                    ..
                } => ConstantData::u16(val as u16),
                Type::Integer {
                    width: IntegerWidth::I16,
                    signed: true,
                    ..
                } => ConstantData::i16(val as i16),
                Type::Integer {
                    width: IntegerWidth::I32,
                    signed: false,
                    ..
                } => ConstantData::u32(val as u32),
                Type::Integer {
                    width: IntegerWidth::I32,
                    signed: true,
                    ..
                } => ConstantData::i32(val as i32),
                Type::Integer {
                    width: IntegerWidth::I64,
                    signed: false,
                    ..
                } => ConstantData::u64(val as u64),
                Type::Integer {
                    width: IntegerWidth::I64,
                    signed: true,
                    ..
                } => ConstantData::i64(val),
                Type::Bool => ConstantData::boolean(val != 0),
                _ => ConstantData::u64(val as u64),
            };
            builder.constant(const_val, sir_ty, span)
        } else {
            // The initial value might be defined in the entry block (e.g., a load).
            // Try to emit entry block instructions to resolve it.
            return Err(format!(
                "cannot resolve initial value '{}' for phi {}",
                init_str, phi_result
            ));
        };
        carried_init_nodes.push(node_id);
        carried_names.push(phi_result.clone());
    }

    // Map phi results to their carried-input NodeIds for body emission
    // The phi result is used inside the loop body as a reference to the carried value.
    // We map it to the corresponding carried_init node — when body instructions
    // reference this phi, they'll get the carried input (which the Loop node will
    // replace with the correct iteration value).
    for (i, (phi_result, _)) in carried_initials.iter().enumerate() {
        value_map.insert(phi_result.clone(), carried_init_nodes[i]);
    }

    // Emit body instructions (non-phi) from the loop block
    let mut body_nodes: Vec<NodeId> = Vec::new();

    for inst in &loop_block.instructions {
        if inst.opcode == "phi" {
            continue;
        }

        if let Some(node_id) = emit_instruction(inst, builder, value_map, &ir.params, span)? {
            body_nodes.push(node_id);
        }
    }

    // Find the termination condition: the conditional br at the end of the loop block
    // br i1 %cond, label %exit, label %loop
    // The condition is the first operand
    let mut termination_node: Option<NodeId> = None;
    let mut exit_label: Option<String> = None;
    // True when the back-edge (loop continuation) is taken on `cond == true`.
    let mut continue_on_true: bool = true;

    for inst in &loop_block.instructions {
        if inst.opcode == "br" && inst.operands.len() >= 3 {
            // conditional branch
            let cond_str = strip_type(&inst.operands[0]);
            termination_node = get_node_id(&cond_str, value_map, &ir.params, builder, None);
            // The exit label is the one that's NOT the loop label
            let true_label = inst.operands[1]
                .trim_start_matches("label ")
                .trim_start_matches('%')
                .to_string();
            let false_label = inst.operands[2]
                .trim_start_matches("label ")
                .trim_start_matches('%')
                .to_string();
            continue_on_true = true_label == *loop_label;
            exit_label = if continue_on_true {
                Some(false_label)
            } else {
                Some(true_label)
            };
        }
    }

    let termination = termination_node.ok_or("no termination condition found in loop block")?;

    // Find the output nodes: the carried "next" values. Resolved before
    // the termination normalization so a successor-tested loop can check
    // that its condition really compares the carry's own next value.
    let mut output_nodes: Vec<NodeId> = Vec::new();
    for (_, next_str) in &carried_nexts {
        let next_str = strip_type(next_str);
        let node_id = get_node_id(&next_str, value_map, &ir.params, builder, None)
            .ok_or(format!("cannot resolve carried next value '{}'", next_str))?;
        output_nodes.push(node_id);
    }

    // ── D4/F9: rotated-exit polarity normalization ──────────────
    // SIR semantics: a Loop continues while its termination evaluates
    // to TRUE. clang `-O1` counts loops by ROTATION: the header tests
    // the SUCCESSOR (`c' = c + 1`) and backs onto the header when the
    // exit test is FALSE (`br i1 icmp eq %next, %bound, exit, loop`).
    // Feeding that raw eq test as the SIR termination INVERTS the loop
    // (continue-while-true on an exit condition ⇒ the loop body runs
    // zero times — silently dead SIR, never a valid program).
    //
    // For the counted rotation `do { body(c); c' = c + 1 } while
    // (c' != K)` (clang emits `icmp eq c+1, K` + back-edge-on-false),
    // the faithful pre-checked iteration domain is `Lt(c, K)`: body runs
    // for c = 0..K-1 and the entry guard (`K == 0 → skip`) folds into
    // `Lt(0, 0) == false`. This normalizes compiler loop syntax onto
    // the SAME `Lt(carry, bound)` domain the binder already derives
    // trip counts from. Any rotated shape that is not this counted
    // pattern is refused loudly — never silently inverted.
    let termination = if continue_on_true {
        match rotated_successor_domain(
            builder,
            termination,
            &carried_init_nodes,
            &output_nodes,
        ) {
            Some((inclusive, carry, bound)) => {
                if !has_entry_guard(ir, loop_idx) {
                    return Err(format!(
                        "unsupported: unguarded successor-tested loop (`next` compared to \
                         the bound with a true back-edge and no pre-loop `n == 0` exit); \
                         SIR loops are pre-tested, so a do-while's first forced iteration \
                         cannot be represented: {}",
                        ir.name
                    ));
                }
                let rebuilt = if inclusive {
                    builder.le(carry, bound, span)
                } else {
                    builder.lt(carry, bound, span)
                };
                rebuilt.map_err(|e| {
                    format!("loop build error (F8 carry-domain reconstruction): {:?}", e)
                })?
            }
            None => {
                // The termination compares the CARRY itself (not its
                // successor). With a pre-loop guard the source is
                // pre-tested and the comparison is already the SIR
                // domain. Without a guard, clang emitted a post-tested
                // do-while: reconstruct the shifted pre-tested domain or
                // refuse loudly — lowering it as-is would drop the
                // forced first/last iteration (v6 finding).
                if has_entry_guard(ir, loop_idx) {
                    termination
                } else {
                    match post_tested_carry_domain(
                        builder,
                        termination,
                        &carried_init_nodes,
                        &output_nodes,
                    ) {
                        Some(rebuilt) => rebuilt,
                        None => {
                            return Err(format!(
                                "unsupported: unguarded post-tested loop (do-while carry test) \
                                 that is not a constant-step counted form; SIR loops are \
                                 pre-tested, so the forced iterations cannot be represented: {}",
                                ir.name
                            ));
                        }
                    }
                }
            }
        }
    } else {
        match rotated_counted_domain(builder, termination, &carried_init_nodes) {
            Some((counter, bound)) => builder.lt(counter, bound, span).map_err(|e| {
                format!("loop build error (F9 Lt reconstruction): {:?}", e)
            })?,
            None => {
                return Err(format!(
                    "unsupported: rotated loop exit (back-edge on false) that is not the \
                     counted eq-next form (`icmp eq next, bound` with `next = carry + 1`); \
                     lowering it as SIR would invert or shift the iteration domain"
                ));
            }
        }
    };

    // Build the Loop node
    let output_types: Vec<Type> = output_nodes
        .iter()
        .map(|id| {
            builder
                .function()
                .get_node(*id)
                .map(|n| n.ty.clone())
                .unwrap_or(Type::u64())
        })
        .collect();

    let loop_ty = Type::Tuple {
        elements: output_types,
    };
    let loop_node = builder
        .r#loop(
            &body_nodes,
            termination,
            &output_nodes,
            &carried_init_nodes,
            loop_ty,
            span,
        )
        .map_err(|e| format!("loop build error: {:?}", e))?;

    // Map phi results to the loop outputs (via TupleExtract)
    let mut extract_for_output: Vec<NodeId> = Vec::new();
    for (i, phi_result) in carried_names.iter().enumerate() {
        let output_ty = builder
            .function()
            .get_node(output_nodes[i])
            .map(|n| n.ty.clone())
            .unwrap_or(Type::u64());
        let extract = builder
            .tuple_extract(loop_node, i, output_ty, span)
            .unwrap();
        value_map.insert(phi_result.clone(), extract);
        extract_for_output.push(extract);
    }

    // Also map the "next" values (output_nodes) to their TupleExtracts.
    // This ensures that exit-block phis referencing output values
    // (e.g., %5 = phi [0, %entry], [%13, %loop]) resolve to the
    // TupleExtract, not the loop body node.
    // We FORCE remap here (overwriting the carried-init mapping from line 766)
    // because the carried-init mapping was only for body emission, and
    // the exit block needs the TupleExtract.
    for (i, (_, next_str)) in carried_nexts.iter().enumerate() {
        let next_str = strip_type(next_str);
        value_map.insert(next_str, extract_for_output[i]);
    }

    // Now emit the exit block instructions
    if let Some(exit_lbl) = exit_label {
        // Follow the block chain from the loop exit to the return.
        // There may be intermediate blocks (e.g., post-loop and+zext before ret).
        let mut current_label = exit_lbl;
        let mut visited = std::collections::HashSet::new();
        while let Some(&exit_idx) = ir.block_map.get(&current_label) {
            if !visited.insert(exit_idx) {
                break; // avoid cycles
            }
            let exit_block = &ir.blocks[exit_idx];
            let mut next_label = None;
            for inst in &exit_block.instructions {
                if inst.opcode == "phi" {
                    // Exit block phi: resolve from loop output
                    // phi i64 [ 0, %entry ], [ %13, %loop ]
                    // The value from %loop is the one we want
                    let combined = inst.operands.join(", ");
                    let parts: Vec<&str> = combined.splitn(2, ' ').collect();
                    let _ty = parts[0];
                    let rest = parts.get(1).unwrap_or(&"");

                    // Find the incoming value — try the loop block first,
                    // then any other block that we've already processed.
                    let mut found_val = None;
                    let mut chars = rest.chars().peekable();
                    let mut in_bracket = false;
                    let mut current = String::new();
                    let mut pair_parts: Vec<String> = Vec::new();

                    while let Some(c) = chars.next() {
                        match c {
                            '[' => {
                                in_bracket = true;
                                current.clear();
                                pair_parts.clear();
                            }
                            ']' if in_bracket => {
                                in_bracket = false;
                                if !current.trim().is_empty() {
                                    pair_parts.push(current.trim().to_string());
                                }
                                let val = pair_parts.get(0).cloned().unwrap_or_default();
                                let label = pair_parts.get(1).cloned().unwrap_or_default();
                                let label_trimmed = label.trim().trim_start_matches('%');
                                // Prefer the loop-block incoming, but accept any
                                // incoming whose value is in the value_map.
                                if label_trimmed == loop_label.as_str() {
                                    found_val = Some(val);
                                } else if found_val.is_none() {
                                    // Try to resolve from value_map (might be from
                                    // an intermediate block we already processed).
                                    let val_stripped = strip_type(&val);
                                    if value_map.contains_key(&val_stripped) {
                                        found_val = Some(val);
                                    }
                                }
                            }
                            ',' if in_bracket => {
                                if !current.trim().is_empty() {
                                    pair_parts.push(current.trim().to_string());
                                }
                                current.clear();
                            }
                            _ if in_bracket => {
                                current.push(c);
                            }
                            _ => {}
                        }
                    }

                    if let Some(val) = found_val {
                        let val = strip_type(&val);
                        // Check if this value was already remapped to a TupleExtract
                        // (loop phi results are remapped earlier). If so, use that.
                        if let Some(id) = value_map.get(&val) {
                            // Already remapped — use the existing mapping (TupleExtract)
                            if let Some(result) = &inst.result {
                                value_map.insert(result.clone(), id.clone());
                            }
                        } else if let Some(id) =
                            get_node_id(&val, value_map, &ir.params, builder, None)
                        {
                            if let Some(result) = &inst.result {
                                value_map.insert(result.clone(), id);
                            }
                        }
                    } else {
                    }
                } else if inst.opcode == "ret" {
                    if !inst.operands.is_empty() {
                        let ret_str = strip_type(&inst.operands[0]);
                        let ret_node = get_node_id(&ret_str, value_map, &ir.params, builder, None)
                            .ok_or(format!("cannot resolve return value '{}'", ret_str))?;
                        builder
                            .return_value(ret_node, span)
                            .map_err(|e| format!("return error: {:?}", e))?;
                    } else {
                        // ret void — emit a Unit return
                        let unit = builder.constant(ConstantData::Unit, Type::Unit, span);
                        builder
                            .return_value(unit, span)
                            .map_err(|e| format!("return error: {:?}", e))?;
                    }
                } else if inst.opcode == "br" {
                    // Unconditional br: follow to the next block
                    if inst.operands.len() == 1 {
                        next_label = Some(
                            inst.operands[0]
                                .trim_start_matches("label ")
                                .trim_start_matches('%')
                                .to_string(),
                        );
                    }
                } else {
                    // Other instructions in exit block
                    emit_instruction(inst, builder, value_map, &ir.params, span)?;
                }
            }
            // Follow to the next block if there was an unconditional br
            match next_label {
                Some(lbl) => current_label = lbl,
                None => break, // ret or conditional br — stop
            }
        }
    }

    Ok(())
}

/// Lower a function without loops (straight-line or multi-block without back-edges).
fn lower_straight_line(
    ir: &IrFunction,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
) -> Result<(), String> {
    let span = Span::unknown();

    for block in &ir.blocks {
        for inst in &block.instructions {
            if inst.opcode == "ret" {
                if !inst.operands.is_empty() {
                    let ret_str = strip_type(&inst.operands[0]);
                    let ret_node = get_node_id(&ret_str, value_map, &ir.params, builder, None)
                        .ok_or(format!("cannot resolve return value '{}'", ret_str))?;
                    builder
                        .return_value(ret_node, span)
                        .map_err(|e| format!("return error: {:?}", e))?;
                } else {
                    // ret void
                    let unit = builder.constant(ConstantData::Unit, Type::Unit, span);
                    builder
                        .return_value(unit, span)
                        .map_err(|e| format!("return error: {:?}", e))?;
                }
            } else {
                emit_instruction(inst, builder, value_map, &ir.params, span)?;
            }
        }
    }

    Ok(())
}

/// Emit a single instruction as a SIR node. Returns the NodeId if the
/// instruction produces a value.
fn emit_instruction(
    inst: &Instruction,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
    params: &[(String, String)],
    span: Span,
) -> Result<Option<NodeId>, String> {
    let result_name = inst.result.clone();

    match inst.opcode.as_str() {
        "add" | "sub" | "mul" | "and" | "or" | "xor" | "shl" | "lshr" | "ashr" | "udiv"
        | "sdiv" | "urem" | "srem" => {
            if inst.operands.len() < 2 {
                return Err(format!("{} needs 2 operands: {}", inst.opcode, inst.raw));
            }
            // ── Fail-closed semantic gates (advisor signedness audit) ──
            // SIR has a single Shr/Div/Rem node whose emitted-C semantics
            // follow the operand type, and the lowerer types every LLVM
            // `iN` as UNSIGNED. Signed opcodes therefore mistranslate
            // (negative operands get unsigned semantics; sdiv INT_MIN/-1
            // trap is unmodeled). Refuse loudly instead of silently
            // producing a differently-defined program.
            if matches!(inst.opcode.as_str(), "ashr" | "sdiv" | "srem") {
                return Err(format!(
                    "unsupported: signed opcode '{}' (SIR Shr/Div/Rem are unsigned-model; arithmetic-shift and signed-division semantics are not representable): {}",
                    inst.opcode, inst.raw
                ));
            }
            // `exact` means poison if the division has a remainder —
            // poison semantics are unmodeled; fail closed (advisor
            // arithmetic-flags directive).
            if inst.raw.contains(" exact ")
                && matches!(
                    inst.opcode.as_str(),
                    "udiv" | "sdiv" | "lshr" | "ashr" | "shl"
                )
            {
                return Err(format!(
                    "unsupported: 'exact' division/shift flag (poison-on-inexact semantics not modeled): {}",
                    inst.raw
                ));
            }
            // Extract the type from the first operand (e.g., "i8 %10" → i8)
            let op_type = inst.operands[0]
                .split_whitespace()
                .next()
                .and_then(|t| parse_type(t));
            let lhs_str = strip_type(&inst.operands[0]);
            let rhs_str = strip_type(&inst.operands[1]);
            let lhs = get_node_id(&lhs_str, value_map, params, builder, op_type.clone())
                .ok_or(format!("cannot resolve lhs '{}' in {}", lhs_str, inst.raw))?;
            let rhs = get_node_id(&rhs_str, value_map, params, builder, op_type)
                .ok_or(format!("cannot resolve rhs '{}' in {}", rhs_str, inst.raw))?;

            let is_bool = inst.operands[0]
                .split_whitespace()
                .next()
                .map(|t| t == "i1")
                .unwrap_or(false);
            let node_id = match inst.opcode.as_str() {
                "add" => builder
                    .add(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "sub" => builder
                    .sub(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "mul" => builder
                    .mul(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "and" if is_bool => builder
                    .bool_and(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "and" => builder
                    .bit_and(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "or" if is_bool => builder
                    .bool_or(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "or" => builder
                    .bit_or(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "xor" if is_bool => {
                    // SIR has no BoolXor. Convert bools to i8, bit_xor, return i8.
                    let lhs_i8 = builder
                        .convert(lhs, Type::u8(), sir_nodes::ConvertKind::ZeroExtend, span)
                        .map_err(|e| format!("{:?}", e))?;
                    let rhs_i8 = builder
                        .convert(rhs, Type::u8(), sir_nodes::ConvertKind::ZeroExtend, span)
                        .map_err(|e| format!("{:?}", e))?;
                    builder
                        .bit_xor(lhs_i8, rhs_i8, span)
                        .map_err(|e| format!("{:?}", e))?
                }
                "xor" => builder
                    .bit_xor(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "shl" => builder
                    .shl(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "lshr" | "ashr" => builder
                    .shr(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "udiv" | "sdiv" => builder
                    .div(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "urem" | "srem" => builder
                    .rem(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                _ => unreachable!(),
            };

            // ── Preserve LLVM operation flags (X02, Gate 6A-v2) ──
            // `nsw`/`nuw` mean overflow produces POISON, not wrapping.
            // Previously dropped, which authorized vector reassociation of
            // signed-overflow-sensitive sums. The metadata is consumed by
            // the reduction certificate's integer-semantics check.
            let flag = if inst.raw.contains(" nsw ") && inst.raw.contains(" nuw ") {
                Some("nsw+nuw")
            } else if inst.raw.contains(" nsw ") {
                Some("nsw")
            } else if inst.raw.contains(" nuw ") {
                Some("nuw")
            } else {
                None
            };
            if let Some(flag) = flag {
                builder.set_overflow_flag(node_id, flag);
            }

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "icmp" => {
            // icmp sgt i64 %10, 0
            // The raw operands are split by comma, but the first part contains
            // the comparison type, element type, and lhs: "sgt i64 %10"
            // We need to parse it differently.
            // Rejoin and parse manually.
            let raw_operands = inst.operands.join(", ");
            let parts: Vec<&str> = raw_operands.splitn(3, ' ').collect();
            if parts.len() < 3 {
                return Err(format!(
                    "icmp needs cmp_type + type + lhs + rhs: {}",
                    inst.raw
                ));
            }
            let cmp_type = parts[0].trim();
            // parts[1] is the type (e.g., "i8", "i64", "i1")
            let cmp_ty = parse_type(parts[1].trim());
            // parts[2] is "lhs, rhs"
            let remaining = parts[2];
            let comma_parts: Vec<&str> = remaining.splitn(2, ',').collect();
            if comma_parts.len() < 2 {
                return Err(format!("icmp needs lhs and rhs: {}", inst.raw));
            }
            let lhs_str = strip_type(comma_parts[0].trim());
            let rhs_str = strip_type(comma_parts[1].trim());
            let lhs = get_node_id(&lhs_str, value_map, params, builder, cmp_ty.clone())
                .ok_or(format!("cannot resolve lhs '{}' in {}", lhs_str, inst.raw))?;
            let rhs = get_node_id(&rhs_str, value_map, params, builder, cmp_ty)
                .ok_or(format!("cannot resolve rhs '{}' in {}", rhs_str, inst.raw))?;

            // ── Fail-closed signedness gate (advisor signedness audit) ──
            // SIR comparison semantics on lowered code are UNSIGNED: the
            // lowerer types every LLVM `iN` as unsigned and the emitted C
            // compares unsigned operands unsigned-ly. Collapsing signed
            // predicates (sgt/sge/slt/sle) onto the same nodes
            // mistranslates any potentially-negative operand — e.g.
            // `icmp sgt i8 %x, -1` becomes `%x > 255` under unsigned
            // semantics. Refuse loudly; only eq/ne (signedness-agnostic)
            // and unsigned predicates lower.
            if matches!(cmp_type, "sgt" | "sge" | "slt" | "sle") {
                return Err(format!(
                    "unsupported: signed icmp '{}' (SIR comparison semantics are unsigned-model; signed predicate would mistranslate negative operands): {}",
                    cmp_type, inst.raw
                ));
            }
            let node_id = match cmp_type {
                "eq" => builder.eq(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ne" => builder.ne(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ugt" => builder.gt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "uge" => builder.ge(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ult" => builder.lt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ule" => builder.le(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                _ => return Err(format!("unsupported icmp type: {}", cmp_type)),
            };

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "zext" | "sext" | "trunc" => {
            // zext i8 %11 to i64
            // The raw line was split by commas, but zext has no commas.
            // Rejoin and parse manually.
            let raw = inst.operands.join(", ");
            // Format: "i8 %11 to i64"
            // Split on " to " to separate source and target type
            let (src_part, to_part) = if let Some(pos) = raw.find(" to ") {
                (
                    raw[..pos].trim().to_string(),
                    raw[pos + 4..].trim().to_string(),
                )
            } else {
                (raw.clone(), String::new())
            };
            let src_str = strip_type(&src_part);
            let src = get_node_id(&src_str, value_map, params, builder, None).ok_or(format!(
                "cannot resolve {} source '{}'",
                inst.opcode, src_str
            ))?;

            let to_type = parse_type(&to_part).unwrap_or(Type::u64());

            let kind = match inst.opcode.as_str() {
                "zext" => sir_nodes::ConvertKind::ZeroExtend,
                "sext" => sir_nodes::ConvertKind::SignExtend,
                "trunc" => sir_nodes::ConvertKind::Truncate,
                _ => unreachable!(),
            };

            let node_id = builder
                .convert(src, to_type, kind, span)
                .map_err(|e| format!("{:?}", e))?;

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "select" => {
            // select i1 %cond, i8 %true, i8 %false
            if inst.operands.len() < 3 {
                return Err(format!("select needs 3 operands: {}", inst.raw));
            }
            // ── Select arm typing (Gate 6A corpus finding) ──
            // Each operand declares its own type ("i8 0", "i1 false",
            // "i64 %x"). Literal arms MUST be built at that width, not
            // the u64 default: `select i1 %c, i8 %acc, i8 0` previously
            // typed the `0` as u64 and the builder refused the width
            // mismatch (w07_all_min, h02_all_match, n14_saturating_count).
            let cond_ty = operand_type(&inst.operands[0]);
            let true_ty = operand_type(&inst.operands[1]);
            let false_ty = operand_type(&inst.operands[2]);
            let cond_str = strip_type(&inst.operands[0]);
            let true_str = strip_type(&inst.operands[1]);
            let false_str = strip_type(&inst.operands[2]);
            let cond = get_node_id(&cond_str, value_map, params, builder, cond_ty)
                .ok_or(format!("cannot resolve select cond '{}'", cond_str))?;
            let true_val = get_node_id(&true_str, value_map, params, builder, true_ty)
                .ok_or(format!("cannot resolve select true '{}'", true_str))?;
            let false_val = get_node_id(&false_str, value_map, params, builder, false_ty)
                .ok_or(format!("cannot resolve select false '{}'", false_str))?;

            let node_id = builder
                .select(cond, true_val, false_val, span)
                .map_err(|e| format!("{:?}", e))?;

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "load" => {
            // load i8, ptr %9, align 1
            // load volatile i8, ptr %9, align 1
            // The first operand is the type, second is the pointer
            if inst.operands.len() < 2 {
                return Err(format!("load needs type + ptr: {}", inst.raw));
            }
            // ── Fail-closed memory semantics (advisor D3 queue item 1) ──
            // Atomic accesses carry ordering semantics our single-thread
            // functional model cannot represent. Refuse loudly as an
            // explicit UNSUPPORTED class — never as an ordinary read.
            if inst.raw.contains("atomic") {
                return Err(format!(
                    "unsupported: atomic load (ordering semantics not modeled): {}",
                    inst.raw
                ));
            }
            // Detect volatile keyword
            let is_volatile = inst.operands[0].contains("volatile");
            let type_str = inst.operands[0].replace("volatile", "").trim().to_string();
            let loaded_ty = parse_type(&type_str).unwrap_or(Type::u8());
            let ptr_str = strip_type(&inst.operands[1]);
            let ptr = get_node_id(&ptr_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve load ptr '{}'", ptr_str))?;

            // If the load is volatile, mark the node with VOLATILE effect
            if is_volatile {
                builder.add_effects(ptr, Effects::VOLATILE);
            }

            // If the pointer came from a getelementptr, we already have it
            // as a node. We model load-from-GEP as ArrayAccess.
            // But if the GEP produced a node, the load just uses that node's
            // result type. We need to model this as the GEP result being
            // the loaded value.
            // Actually, in our model, getelementptr+load is collapsed into
            // ArrayAccess. The GEP node IS the array access, and the load
            // just passes through its result.
            if let Some(name) = result_name {
                value_map.insert(name, ptr);
            }
            Ok(Some(ptr))
        }

        "getelementptr" => {
            // getelementptr inbounds i8, ptr %0, i64 %7
            //   → buf[i] — model as ArrayAccess(base, index)
            // getelementptr inbounds [256 x i8], ptr @table, i64 0, i64 %11
            //   → table[%11] — multi-index GEP, skip index 0
            if inst.operands.len() < 3 {
                return Err(format!(
                    "getelementptr needs type + ptr + index: {}",
                    inst.raw
                ));
            }
            // operand 0: "inbounds i8" or "inbounds [256 x i8]" — the element type
            let elem_type_str = inst.operands[0].trim_start_matches("inbounds").trim();
            let elem_ty = parse_type(elem_type_str).unwrap_or(Type::u8());

            // operand 1: "ptr %0" or "ptr @table" — the base pointer
            let base_str = strip_type(&inst.operands[1]);
            let base = get_node_id(&base_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve gep base '{}'", base_str))?;

            // Determine the index: for 3-operand GEP it's operand[2].
            // For 4+ operand GEP (multi-index), skip the first index (0) and use the last.
            let index_str = if inst.operands.len() >= 4 {
                // Multi-index: use the last operand as the index, skip the first (0)
                strip_type(inst.operands.last().unwrap())
            } else {
                strip_type(&inst.operands[2])
            };
            let index = get_node_id(&index_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve gep index '{}'", index_str))?;

            let node_id = builder
                .array_access(base, index, elem_ty, span)
                .map_err(|e| format!("{:?}", e))?;

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "store" => {
            // store i8 %result, ptr %out_ptr
            // SIR Store { ptr, value } → returns Unit
            if inst.operands.len() < 2 {
                return Err(format!("store needs value + ptr: {}", inst.raw));
            }
            // ── Fail-closed memory semantics (advisor D3 queue item 1) ──
            // A volatile store has observable occurrence + ordering
            // semantics; dropping or merging it changes program behavior.
            // Refuse loudly as an explicit UNSUPPORTED class — never
            // silently lower it as an ordinary write.
            if inst.raw.contains("volatile") {
                return Err(format!(
                    "unsupported: volatile store (observable occurrence/ordering semantics): {}",
                    inst.raw
                ));
            }
            if inst.raw.contains("atomic") {
                return Err(format!(
                    "unsupported: atomic store (ordering semantics not modeled): {}",
                    inst.raw
                ));
            }
            // operand 0: "i8 %result" (the value)
            // operand 1: "ptr %out_ptr" (the pointer)
            let val_str = strip_type(&inst.operands[0]);
            let ptr_str = strip_type(&inst.operands[1]);
            let val = get_node_id(&val_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve store value '{}'", val_str))?;
            let ptr = get_node_id(&ptr_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve store ptr '{}'", ptr_str))?;
            let _node_id = builder
                .store(ptr, val, span)
                .map_err(|e| format!("{:?}", e))?;
            // Store produces no value (Unit) — don't insert into value_map
            Ok(None)
        }

        "br" | "ret" | "phi" => {
            // Handled by the loop/exit block logic
            Ok(None)
        }

        "call" => {
            // Handle a few known intrinsics
            // tail call i8 @llvm.umax.i8(i8 %10, i8 %8)
            let callee = inst.operands.first().map(|s| s.trim()).unwrap_or("");
            if callee.contains("llvm.umax")
                || callee.contains("llvm.umin")
                || callee.contains("llvm.smax")
                || callee.contains("llvm.smin")
            {
                // Model as select(gt(a,b), a, b) for umax, or select(lt(a,b), a, b) for umin
                // operands: "@llvm.umax.i8(i8 %10, i8 %8"  (with closing paren possibly)
                // Actually the operands were split by commas, so:
                // operand 0: "@llvm.umax.i8(i8 %10"
                // operand 1: "i8 %8)"
                // We need to extract the two value operands
                let raw = inst.operands.join(", ");
                // Find the part inside parentheses
                if let Some(open) = raw.find('(') {
                    let inside = &raw[open + 1..];
                    let inside = inside.trim_end_matches(')');
                    let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        let a_str = strip_type(parts[0]);
                        let b_str = strip_type(parts[1]);
                        // Extract declared operand type (e.g., "i32 %1" → i32) so
                        // constant operands get the correct width
                        let op_ty = parts[0].split_whitespace().next().and_then(parse_type);
                        let a = get_node_id(&a_str, value_map, params, builder, op_ty.clone())
                            .ok_or(format!("cannot resolve umax operand '{}'", a_str))?;
                        let b = get_node_id(&b_str, value_map, params, builder, op_ty)
                            .ok_or(format!("cannot resolve umax operand '{}'", b_str))?;

                        let node_id =
                            if callee.contains("llvm.umax") || callee.contains("llvm.smax") {
                                // umax/smax(a, b) = select(a > b, a, b)
                                let cmp = builder.gt(a, b, span).map_err(|e| format!("{:?}", e))?;
                                builder
                                    .select(cmp, a, b, span)
                                    .map_err(|e| format!("{:?}", e))?
                            } else {
                                // umin/smin(a, b) = select(a < b, a, b)
                                let cmp = builder.lt(a, b, span).map_err(|e| format!("{:?}", e))?;
                                builder
                                    .select(cmp, a, b, span)
                                    .map_err(|e| format!("{:?}", e))?
                            };

                        if let Some(name) = result_name {
                            value_map.insert(name, node_id);
                        }
                        return Ok(Some(node_id));
                    }
                }
            }
            Err(format!("unsupported call: {}", inst.raw))
        }

        _ => Err(format!(
            "unsupported instruction: {} (raw: {})",
            inst.opcode, inst.raw
        )),
    }
}

fn phi_info_debug(inst: &Instruction) -> Vec<(String, String)> {
    let combined = inst.operands.join(", ");
    let parts: Vec<&str> = combined.splitn(2, ' ').collect();
    let rest = parts.get(1).unwrap_or(&"");
    let mut incoming = Vec::new();
    let mut chars = rest.chars().peekable();
    let mut in_bracket = false;
    let mut current = String::new();
    let mut pair_parts: Vec<String> = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '[' => {
                in_bracket = true;
                current.clear();
                pair_parts.clear();
            }
            ']' if in_bracket => {
                in_bracket = false;
                if !current.trim().is_empty() {
                    pair_parts.push(current.trim().to_string());
                }
                incoming.push((
                    pair_parts.get(0).cloned().unwrap_or_default(),
                    pair_parts.get(1).cloned().unwrap_or_default(),
                ));
            }
            ',' if in_bracket => {
                if !current.trim().is_empty() {
                    pair_parts.push(current.trim().to_string());
                }
                current.clear();
            }
            _ if in_bracket => {
                current.push(c);
            }
            _ => {}
        }
    }
    incoming
}
