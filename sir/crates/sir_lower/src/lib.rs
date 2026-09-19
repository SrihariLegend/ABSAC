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

/// Names referenced as `@name` in function text. Intrinsic and call
/// targets are skipped (a call target is followed by `(`); everything
/// else is treated as global storage.
fn collect_global_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'@' {
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || b"_.$-".contains(&bytes[end]))
            {
                end += 1;
            }
            let name = &line[start..end];
            let call_target = bytes.get(end) == Some(&b'(');
            if !name.is_empty()
                && !name.starts_with("llvm.")
                && !name.starts_with("sir.")
                && !call_target
                && !out.iter().any(|n| n == name)
            {
                out.push(name.to_string());
            }
            i = end.max(i + 1);
        }
    }
    out
}

/// GEP results that are used as pointer values (phi incoming, another
/// GEP's base, select/compare/call/return operand) rather than only being
/// consumed by a load/store. Only these need a pointer-typed node; the
/// rest keep the recognizer-friendly `ArrayAccess` form.
fn escaping_gep_results(ir: &IrFunction) -> HashSet<String> {
    let gep_names: HashSet<String> = ir
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|inst| inst.opcode == "getelementptr")
        .filter_map(|inst| inst.result.clone())
        .collect();
    let mut escaping = HashSet::new();
    for block in &ir.blocks {
        for inst in &block.instructions {
            match inst.opcode.as_str() {
                "phi" => {
                    for (value, _) in phi_incomings(inst) {
                        let value = strip_type(&value);
                        if gep_names.contains(&value) {
                            escaping.insert(value);
                        }
                    }
                }
                "getelementptr" if inst.operands.len() >= 2 => {
                    let base = strip_type(&inst.operands[1]);
                    if gep_names.contains(&base) {
                        escaping.insert(base);
                    }
                }
                "select" | "icmp" | "call" | "ret" => {
                    for operand in &inst.operands {
                        let value = strip_type(&operand_value_token(operand));
                        if gep_names.contains(&value) {
                            escaping.insert(value);
                        }
                    }
                }
                "store" if !inst.operands.is_empty() => {
                    let value = strip_type(&inst.operands[0]);
                    if gep_names.contains(&value) {
                        escaping.insert(value);
                    }
                }
                _ => {}
            }
        }
    }
    escaping
}

/// Fold a constant `getelementptr (SRC, ptr @name, ..., i64 idx)`
/// operand into `(global, element type, index)`.
fn inline_global_gep(operand: &str) -> Option<(String, Type, u64)> {
    let start = operand.find("getelementptr")?;
    let rest = &operand[start..];
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    let inside = &rest[open + 1..close];
    let parts: Vec<&str> = inside.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let source = parts[0].trim_start_matches("inbounds").trim();
    let at = parts[1].find('@')?;
    let name: String = parts[1][at + 1..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || "_.$-".contains(*c))
        .collect();
    if name.is_empty() {
        return None;
    }
    let index: u64 = parts.last()?.split_whitespace().last()?.parse().ok()?;
    let element = if source.starts_with('[') {
        match gep_source_array_type(source)? {
            Type::Array { element, .. } => *element,
            other => other,
        }
    } else {
        parse_type(source)?
    };
    Some((name, element, index))
}

/// Resolve a global name to its byte-addressed base node, creating it on
/// first use.
fn resolve_global(
    name: &str,
    value_map: &mut HashMap<String, NodeId>,
    builder: &mut Builder,
    span: Span,
) -> NodeId {
    if let Some(id) = value_map.get(name) {
        return *id;
    }
    let id = builder.global(name.to_string(), span);
    value_map.insert(name.to_string(), id);
    id
}

/// Resolve a pointer-valued call operand (direct global/param, or an
/// inline constant GEP) to a pointer-typed SIR node.
fn resolve_pointer_operand(
    operand: &str,
    value_map: &mut HashMap<String, NodeId>,
    params: &[(String, String)],
    builder: &mut Builder,
    span: Span,
) -> Option<NodeId> {
    if let Some((name, element, index)) = inline_global_gep(operand) {
        let base = resolve_global(&name, value_map, builder, span);
        let index = builder.constant(ConstantData::u64(index), Type::u64(), span);
        return builder.pointer_offset(base, index, element, span).ok();
    }
    let token = operand
        .split_whitespace()
        .rev()
        .find(|token| token.starts_with('@') || token.starts_with('%'))?
        .trim_end_matches(|ch| ch == ')' || ch == ',');
    get_node_id(token, value_map, params, builder, None)
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
        // `switch` case lists are printed across lines; fold them into one
        // instruction so the CFG walker sees the complete terminator.
        if inst_text.contains("switch ") && inst_text.contains('[') && !inst_text.contains(']') {
            while i + 1 < lines.len() && !inst_text.contains(']') {
                i += 1;
                inst_text.push(' ');
                inst_text.push_str(lines[i].trim());
            }
        }
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
/// Lower an LLVM IR function to SIR.
pub fn lower(text: &str) -> Result<Function, String> {
    let text = extract_first_function(text);
    let ir = parse_function(&text)?;

    // Signed comparisons control loop domains in ways the reconstruction
    // paths do not model yet. Refuse signed control flow wholesale for
    // now; signed value operations (ashr, smax/smin in straight-line
    // code) are handled below.
    let has_back_edge = ir.blocks.iter().enumerate().any(|(idx, block)| {
        block.instructions.iter().any(|inst| {
            inst.opcode == "br"
                && inst.operands.iter().any(|operand| {
                    operand
                        .trim()
                        .strip_prefix("label %")
                        .and_then(|label| ir.block_map.get(label).copied())
                        .is_some_and(|target| target <= idx)
                })
        })
    });
    let signed_control_flow = ir.blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            let raw = inst.raw.as_str();
            raw.contains("icmp sgt")
                || raw.contains("icmp sge")
                || raw.contains("icmp slt")
                || raw.contains("icmp sle")
                || raw.contains("llvm.smax")
                || raw.contains("llvm.smin")
        })
    });
    if has_back_edge && signed_control_flow {
        return Err(
            "unsupported: signed comparison/min/max inside a loop (loop-domain signedness is not modeled)"
                .to_string(),
        );
    }

    // Map return type
    let ret_ty = parse_type(&ir.ret_type).unwrap_or(Type::Unit);

    // Opaque `ptr` parameters: recover the pointee element type (F7).
    let inferred_ptr_types = infer_ptr_param_types(&ir);

    // Build parameter list for SIR
    let sir_params: Vec<(&str, Type)> = ir
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

    let mut builder = Builder::new(&ir.name, &sir_params, ret_ty);

    // Value map: LLVM value name → SIR NodeId
    let mut value_map: HashMap<String, NodeId> = HashMap::new();

    // Map parameters
    for (i, (name, _)) in ir.params.iter().enumerate() {
        let node_id = builder.parameter_index(i).unwrap();
        value_map.insert(name.clone(), node_id);
    }

    // Globals are byte-addressed base nodes. Pre-scan the function body so
    // every `@name` resolves through `get_node_id`; driver-provided storage
    // supplies the bytes. Inline constant GEPs are folded in the memory arms.
    for gname in collect_global_refs(&text) {
        let node_id = builder.global(gname.clone(), Span::unknown());
        value_map.insert(gname, node_id);
    }
    builder.set_escaping_geps(escaping_gep_results(&ir));

    // Acyclic switch CFGs are lowered by path enumeration: branch guards
    // become Select conditions, and constant global accesses become
    // predicated memory states. Loop functions and non-constant memory
    // operands stay with the existing (fail-closed) machinery.
    if !has_back_edge && ir.blocks.len() > 1 && acyclic_global_memory_cfg(&ir) {
        let return_ty = builder.function().return_ty.clone();
        return lower_switch_cfg(&ir, &mut builder, value_map, return_ty)
            .map(|_| builder.build());
    }

    // Detect loop structure: find the back-edge block (the one that branches to itself or to a predecessor)
    // Simple pattern: entry → header (loop) → exit
    // header has a phi for each carried value, a conditional branch at the end

    // For now, handle the common pattern:
    // Block 0 (entry): initial check + br to loop or exit
    // Block 1 (loop header): phi nodes + body + br back or to exit
    // Block 2 (exit): phi for result + ret

    if ir.blocks.is_empty() {
        return Err("function has no blocks".to_string());
    }
    if ir.blocks.len() == 1 {
        // Straight-line single-block functions (scalar rotates, byte swaps,
        // bit reversals) have no CFG to synthesize. They were previously
        // refused by the generic "too few blocks" gate, which kept the
        // permutation recipes from ever seeing clang's scalar intrinsics.
        // Only a phi-free, branch-free block is admitted; anything else still
        // fails closed with the historical message.
        let block = &ir.blocks[0];
        let has_phi = block.instructions.iter().any(|i| i.opcode == "phi");
        let has_branch = block.instructions.iter().any(|i| i.opcode == "br");
        if has_phi || has_branch {
            return Err("function has too few blocks for the lowerer".to_string());
        }
        return lower_straight_line(&ir, &mut builder, &mut value_map).map(|_| builder.build());
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

/// Rotated zero-test normalization for runtime-bounded loops.
///
/// Clang's `while (x != 0) { x = f(x); }` with an entry guard lowers to
/// a self-latch block whose exit test compares the **next** carried
/// value against zero with a back-edge on false:
///
/// ```text
///   %next = f(%x)
///   %exit = icmp eq %next, 0
///   br i1 %exit, label %out, label %loop
/// ```
///
/// With the pre-loop guard (`x0 == 0 → out`) this is the pre-tested
/// domain `x != 0`, evaluated on the current carried value. The `ne`
/// case is the symmetric `while (x == 0)` form. Anything else returns
/// `None`.
fn rotated_zero_test_domain(
    builder: &mut Builder,
    termination: NodeId,
    carried_init_nodes: &[NodeId],
    output_nodes: &[NodeId],
) -> Option<NodeId> {
    let (lhs, rhs, exit_on_eq) = {
        let function = builder.function();
        let term = function.get_node(termination)?;
        match &term.kind {
            sir_nodes::NodeKind::Eq { lhs, rhs } => (*lhs, *rhs, true),
            sir_nodes::NodeKind::Ne { lhs, rhs } => (*lhs, *rhs, false),
            _ => return None,
        }
    };
    for (candidate, zero_side) in [(lhs, rhs), (rhs, lhs)] {
        if lower_constant_u64(builder.function(), zero_side) != Some(0) {
            continue;
        }
        let index = output_nodes.iter().position(|output| *output == candidate)?;
        let carry = *carried_init_nodes.get(index)?;
        let ty = builder.function().get_node(carry)?.ty.clone();
        let span = Span::unknown();
        let zero = builder.constant(int_constant_data(&ty, 0), ty, span);
        return if exit_on_eq {
            builder.ne(carry, zero, span).ok()
        } else {
            builder.eq(carry, zero, span).ok()
        };
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

/// Value token of an LLVM operand ("i64 %13" -> "%13",
/// "inbounds i8" -> "i8", "i64 0" -> "0").
fn operand_value_token(o: &str) -> String {
    o.split_whitespace()
        .last()
        .unwrap_or(o)
        .trim_end_matches(')')
        .trim_end_matches(',')
        .to_string()
}

/// A peeled search: clang lifts the first element check out of the loop
/// and threads a found-flag + index pair through the merge, then selects
/// against a computed no-hit sentinel.
struct PeeledSearch {
    header: usize,
    body: usize,
    merge: usize,
    counter_name: String,
    successor_name: String,
    bound_name: String,
    body_cmp_name: String,
    body_found_on_true: bool,
    sentinel_name: String,
    select_result: String,
    ret_operand: String,
}

fn block_by_label<'a>(ir: &'a IrFunction, label: &str) -> Option<&'a Block> {
    ir.block_map.get(label).and_then(|i| ir.blocks.get(*i))
}

/// Find `<access> = gep base, index; load access; icmp op load, scalar`
/// plus the branch polarity towards `hit_label`.
fn hit_test_in(
    block: &Block,
    index_token: &str,
    hit_label: &str,
) -> Option<(String, String, String, String, bool)> {
    let access = block.instructions.iter().find(|i| {
        i.opcode == "getelementptr"
            && i.operands
                .last()
                .map(|o| operand_value_token(o) == index_token)
                .unwrap_or(false)
    });
    // Index 0 is often folded into the base pointer (no GEP): then the
    // block loads the base directly.
    let (base, load) = if let Some(access) = access {
        let access_name = access.result.clone()?;
        let base = access
            .operands
            .iter()
            .find(|o| o.trim().starts_with("ptr "))
            .map(|o| operand_value_token(o))
            .or_else(|| access.operands.get(1).map(|o| operand_value_token(o)))?;
        let load = block.instructions.iter().find(|i| {
            i.opcode == "load"
                && i.operands
                    .iter()
                    .any(|o| operand_value_token(o) == access_name)
        })?;
        (base, load)
    } else {
        if index_token != "0" {
            return None;
        }
        let load = block.instructions.iter().find(|i| i.opcode == "load")?;
        let base = load
            .operands
            .iter()
            .find(|o| o.trim().starts_with("ptr "))
            .map(|o| operand_value_token(o))?;
        (base, load)
    };
    let load_name = load.result.clone()?;
    let cmp = block.instructions.iter().find(|i| {
        i.opcode == "icmp"
            && i.operands
                .iter()
                .any(|o| operand_value_token(o) == load_name)
    })?;
    let cmp_name = cmp.result.clone()?;
    let op = cmp.operands.first()?.split_whitespace().next()?.to_string();
    let scalar = cmp
        .operands
        .iter()
        .map(|o| operand_value_token(o))
        .find(|t| *t != load_name)?;
    let br = block.instructions.iter().rev().find(|i| i.opcode == "br")?;
    if br.operands.len() < 3 {
        return None;
    }
    let true_label = br.operands[1]
        .trim_start_matches("label ")
        .trim_start_matches('%')
        .to_string();
    Some((base, op, scalar, cmp_name, true_label == hit_label))
}

/// Pure detection of the peeled search shape; no emission.
fn detect_peeled_search(ir: &IrFunction) -> Option<PeeledSearch> {
    for e in ir.blocks.iter() {
        let e_succs = block_successors(e);
        if e_succs.len() != 2 {
            continue;
        }
        let e_br = e.instructions.iter().rev().find(|i| i.opcode == "br")?;
        if e_br.operands.len() < 3 {
            continue;
        }
        let e_cond = strip_type(&e_br.operands[0]);
        let e_cmp = e
            .instructions
            .iter()
            .find(|i| i.opcode == "icmp" && i.result.as_deref() == Some(e_cond.as_str()))?;
        let e_op = e_cmp.operands.first()?.split_whitespace().next()?;
        if e_op != "ne" && e_op != "eq" {
            continue;
        }
        let mut e_tokens: Vec<String> = e_cmp.operands.iter().map(|o| operand_value_token(o)).collect();
        if e_tokens.len() != 2 {
            continue;
        }
        let guard_value = if parse_int_constant(&e_tokens[0]) == Some(0) {
            e_tokens.remove(1);
            e_tokens.remove(0);
            e_tokens.first().cloned()?
        } else if parse_int_constant(&e_tokens[1]) == Some(0) {
            e_tokens.remove(1);
            e_tokens.first().cloned()?
        } else {
            continue;
        };
        // Prefer the successor that reaches a head block with a phi-carrying
        // loop; try both orderings.
        for (p_label, m_label) in [(&e_succs[0], &e_succs[1]), (&e_succs[1], &e_succs[0])] {
            let Some(p) = block_by_label(ir, p_label) else {
                continue;
            };
            let Some(m) = block_by_label(ir, m_label) else {
                continue;
            };
            if p.instructions.iter().any(|i| i.opcode == "phi") {
                continue;
            }
            let p_succs = block_successors(p);
            if p_succs.len() != 2 || !p_succs.contains(m_label) {
                continue;
            }
            let Some(h_label) = p_succs.iter().find(|s| *s != m_label) else {
                continue;
            };
            let Some(h) = block_by_label(ir, h_label) else {
                continue;
            };
            let h_phis: Vec<&Instruction> = h
                .instructions
                .iter()
                .filter(|i| i.opcode == "phi")
                .collect();
            if h_phis.len() != 1 {
                continue;
            }
            let counter_phi = h_phis[0];
            let Some(counter_name) = counter_phi.result.clone() else {
                continue;
            };
            let h_inc = phi_incomings(counter_phi);
            let Some(init) = h_inc.iter().find(|(_, l)| l == p_label) else {
                continue;
            };
            if parse_int_constant(&strip_type(&init.0)) != Some(0) {
                continue;
            }
            let Some(next_name) = h_inc
                .iter()
                .find(|(_, l)| l != p_label)
                .map(|(v, _)| strip_type(v))
            else {
                continue;
            };
            let Some(succ_inst) = h.instructions.iter().find(|i| {
                i.result.as_deref() == Some(next_name.as_str())
                    && (i.opcode == "add" || i.opcode == "sub")
            }) else {
                continue;
            };
            if strip_type(succ_inst.operands.first()?) != counter_name {
                continue;
            }
            let Some(step) = succ_inst
                .operands
                .get(1)
                .and_then(|o| parse_int_constant(&operand_value_token(o)))
            else {
                continue;
            };
            if !((succ_inst.opcode == "add" && step == 1) || (succ_inst.opcode == "sub" && step == 1))
            {
                continue;
            }
            let h_succs = block_successors(h);
            if h_succs.len() != 2 {
                continue;
            }
            // Exit-on-true to the exit block X.
            let x_label = h_succs[0].clone();
            let l_label = h_succs[1].clone();
            let Some(x) = block_by_label(ir, &x_label) else {
                continue;
            };
            let Some(l) = block_by_label(ir, &l_label) else {
                continue;
            };
            if block_successors(x) != [m_label.clone()] {
                continue;
            }
            let l_succs = block_successors(l);
            if l_succs.len() != 2
                || !l_succs.contains(h_label)
                || !l_succs.contains(&x_label)
            {
                continue;
            }
            let Some(h_br) = h.instructions.iter().rev().find(|i| i.opcode == "br") else {
                continue;
            };
            let h_cond = strip_type(&h_br.operands[0]);
            let Some(h_cmp) = h.instructions.iter().find(|i| {
                i.opcode == "icmp" && i.result.as_deref() == Some(h_cond.as_str())
            }) else {
                continue;
            };
            if h_cmp.operands.first()?.split_whitespace().next()? != "eq" {
                continue;
            }
            let h_tokens: Vec<String> = h_cmp.operands.iter().map(|o| operand_value_token(o)).collect();
            let bound_name = if h_tokens.first() == Some(&next_name) {
                h_tokens.get(1).cloned()
            } else if h_tokens.get(1) == Some(&next_name) {
                h_tokens.first().cloned()
            } else {
                None
            };
            let Some(bound_name) = bound_name else {
                continue;
            };
            // Predicate equality between the peeled head (index 0) and the
            // loop body (index = successor).
            let Some((head_base, head_op, head_scalar, _, head_found)) =
                hit_test_in(p, "0", m_label)
            else {
                continue;
            };
            let Some((body_base, body_op, body_scalar, body_cmp_name, body_found)) =
                hit_test_in(l, &next_name, &x_label)
            else {
                continue;
            };
            if head_base != body_base
                || head_op != body_op
                || head_scalar != body_scalar
                || head_found != body_found
            {
                continue;
            }
            // X carries the exit index (bound on the header path,
            // successor on the body path) and the found comparison.
            let Some(index_phi_x) = x.instructions.iter().find(|phi| {
                if phi.opcode != "phi" {
                    return false;
                }
                let inc = phi_incomings(phi);
                inc.iter().any(|(v, l)| {
                    l == h_label && strip_type(v) == bound_name
                }) && inc.iter().any(|(v, l)| {
                    l == &l_label && strip_type(v) == next_name
                })
            }) else {
                continue;
            };
            let Some(index_phi_x_name) = index_phi_x.result.clone() else {
                continue;
            };
            let Some(x_cmp) = x.instructions.iter().find(|i| {
                i.opcode == "icmp"
                    && i.operands
                        .iter()
                        .any(|o| operand_value_token(o) == next_name)
                    && i.operands
                        .iter()
                        .any(|o| operand_value_token(o) == bound_name)
            }) else {
                continue;
            };
            let Some(x_cmp_name) = x_cmp.result.clone() else {
                continue;
            };
            // M has exactly the index and found phis and the sentinel select.
            let m_phis: Vec<&Instruction> = m
                .instructions
                .iter()
                .filter(|i| i.opcode == "phi")
                .collect();
            if m_phis.len() != 2 {
                continue;
            }
            let Some(index_phi_m) = m_phis.iter().find(|phi| {
                phi_incomings(phi)
                    .iter()
                    .any(|(v, l)| l == &x_label && strip_type(v) == index_phi_x_name)
            }) else {
                continue;
            };
            let Some(found_phi_m) = m_phis.iter().find(|phi| {
                phi_incomings(phi)
                    .iter()
                    .any(|(v, l)| l == &x_label && strip_type(v) == x_cmp_name)
            }) else {
                continue;
            };
            if std::ptr::eq(*index_phi_m, *found_phi_m) {
                continue;
            }
            let (Some(index_phi_name), Some(found_phi_name)) =
                (index_phi_m.result.clone(), found_phi_m.result.clone())
            else {
                continue;
            };
            let Some(sel) = m.instructions.iter().find(|i| i.opcode == "select") else {
                continue;
            };
            if sel.operands.len() < 3 {
                continue;
            }
            let sel_cond = strip_type(sel.operands.first()?);
            let sel_true = strip_type(sel.operands.get(1)?);
            let sentinel_name = strip_type(sel.operands.get(2)?);
            if sel_cond != found_phi_name || sel_true != index_phi_name {
                continue;
            }
            let Some(select_result) = sel.result.clone() else {
                continue;
            };
            let Some(ret_operand) = m
                .instructions
                .iter()
                .find(|i| i.opcode == "ret")
                .and_then(|i| i.operands.first().cloned())
            else {
                continue;
            };
            if strip_type(&guard_value) != bound_name {
                continue;
            }
            let Some(hi) = ir.block_map.get(h_label).copied() else {
                continue;
            };
            let Some(li) = ir.block_map.get(&l_label).copied() else {
                continue;
            };
            let Some(mi) = ir.block_map.get(m_label).copied() else {
                continue;
            };
            return Some(PeeledSearch {
                header: hi,
                body: li,
                merge: mi,
                counter_name,
                successor_name: next_name,
                bound_name,
                body_cmp_name,
                body_found_on_true: body_found,
                sentinel_name,
                select_result,
                ret_operand,
            });
        }
    }
    None
}

/// De-peel the recognized search into one canonical found-flag loop over
/// the full index range 0 .. bound-1, then select the sentinel on no hit.
fn build_peeled_search(
    ir: &IrFunction,
    shape: &PeeledSearch,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
) -> Result<(), String> {
    let span = Span::unknown();
    let h = &ir.blocks[shape.header];
    let l = &ir.blocks[shape.body];
    let m = &ir.blocks[shape.merge];

    let counter_phi = h
        .instructions
        .iter()
        .find(|i| i.opcode == "phi")
        .ok_or("peeled search: header phi missing")?;
    let counter_ty_str = counter_phi
        .operands
        .first()
        .and_then(|o| o.split_whitespace().next())
        .unwrap_or("i64");
    let counter_ty = parse_type(counter_ty_str).ok_or("peeled search: bad counter type")?;
    let counter_init = builder.constant(
        int_constant_data(&counter_ty, 0),
        counter_ty.clone(),
        span,
    );
    let bound_node = get_node_id(
        &shape.bound_name,
        value_map,
        &ir.params,
        builder,
        Some(counter_ty.clone()),
    )
    .ok_or("peeled search: cannot resolve the bound")?;

    value_map.insert(shape.counter_name.clone(), counter_init);
    let mut body_nodes: Vec<NodeId> = Vec::new();
    for inst in &h.instructions {
        if inst.opcode == "phi" || inst.opcode == "br" || inst.opcode == "ret" {
            continue;
        }
        if let Some(id) = emit_instruction(inst, builder, value_map, &ir.params, span)? {
            body_nodes.push(id);
        }
    }
    let successor = value_map
        .get(&shape.successor_name)
        .copied()
        .ok_or("peeled search: successor did not emit")?;
    // The body accesses the element at the SUCCESSOR; the de-peeled loop
    // covers index 0 as well, so remap the successor name to the carried
    // counter for the body's access/test emission.
    value_map.insert(shape.successor_name.clone(), counter_init);
    for inst in &l.instructions {
        if inst.opcode == "phi" || inst.opcode == "br" || inst.opcode == "ret" {
            continue;
        }
        if let Some(id) = emit_instruction(inst, builder, value_map, &ir.params, span)? {
            body_nodes.push(id);
        }
    }
    let body_cmp = value_map
        .get(&shape.body_cmp_name)
        .copied()
        .ok_or("peeled search: body comparison did not emit")?;
    let hit = if shape.body_found_on_true {
        body_cmp
    } else {
        let not = builder
            .bool_not(body_cmp, span)
            .map_err(|e| format!("peeled search: {:?}", e))?;
        body_nodes.push(not);
        not
    };
    let found_init = builder.constant(ConstantData::boolean(false), Type::Bool, span);
    let index_init = builder.constant(
        int_constant_data(&counter_ty, 0),
        counter_ty.clone(),
        span,
    );
    let not_found = builder
        .bool_not(found_init, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let found_next = builder
        .bool_or(found_init, hit, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let update = builder
        .bool_and(hit, not_found, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let index_next = builder
        .select(update, counter_init, index_init, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let domain = builder
        .lt(counter_init, bound_node, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let termination = builder
        .bool_and(not_found, domain, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    body_nodes.extend([found_next, update, index_next, domain, termination]);
    let outputs = vec![found_next, index_next, successor];
    let carried = vec![found_init, index_init, counter_init];
    let loop_ty = Type::Tuple {
        elements: vec![Type::Bool, counter_ty.clone(), counter_ty.clone()],
    };
    let loop_node = builder
        .r#loop(&body_nodes, termination, &outputs, &carried, loop_ty, span)
        .map_err(|e| format!("peeled search loop build: {:?}", e))?;
    let found_extract = builder
        .tuple_extract(loop_node, 0, Type::Bool, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let index_extract = builder
        .tuple_extract(loop_node, 1, counter_ty.clone(), span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    let counter_extract = builder
        .tuple_extract(loop_node, 2, counter_ty.clone(), span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    value_map.insert(shape.counter_name.clone(), counter_extract);
    value_map.insert(shape.successor_name.clone(), counter_extract);

    // Emit the merge instructions that produce the sentinel (everything
    // except the phis, the substituted select and the return), then the
    // sentinel select and any post-select instructions.
    let mut after_select = false;
    for inst in &m.instructions {
        match inst.opcode.as_str() {
            "phi" | "ret" => continue,
            "select" => {
                after_select = true;
                continue;
            }
            _ => {}
        }
        if !after_select {
            emit_instruction(inst, builder, value_map, &ir.params, span)?;
        }
    }
    let sentinel = get_node_id(
        &shape.sentinel_name,
        value_map,
        &ir.params,
        builder,
        Some(counter_ty.clone()),
    )
    .ok_or("peeled search: cannot resolve the sentinel")?;
    let result = builder
        .select(found_extract, index_extract, sentinel, span)
        .map_err(|e| format!("peeled search: {:?}", e))?;
    value_map.insert(shape.select_result.clone(), result);
    let mut post_select = false;
    for inst in &m.instructions {
        match inst.opcode.as_str() {
            "phi" | "ret" | "select" => {
                if inst.opcode == "select" {
                    post_select = true;
                }
                continue;
            }
            _ => {}
        }
        if post_select {
            emit_instruction(inst, builder, value_map, &ir.params, span)?;
        }
    }
    let ret_node = get_node_id(
        &strip_type(&shape.ret_operand),
        value_map,
        &ir.params,
        builder,
        None,
    )
    .ok_or("peeled search: cannot resolve the return value")?;
    builder
        .return_value(ret_node, span)
        .map_err(|e| format!("peeled search return: {:?}", e))?;
    Ok(())
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
    // Peeled form first: clang lifts the first element check out of the
    // loop and threads a found-flag + index pair through the merge, then
    // selects against a computed no-hit sentinel. Detection is pure, so a
    // non-match falls through to the ordinary ascent/descent synthesis.
    if let Some(shape) = detect_peeled_search(ir) {
        return build_peeled_search(ir, &shape, builder, value_map);
    }
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
            // The entry guard also branches back to the header, but it is
            // not the latch: a latch must be a non-entry block that
            // contributes a BACK-EDGE incoming to a header phi.
            if li == 0 {
                continue;
            }
            let contributes_back_edge = h
                .instructions
                .iter()
                .filter(|inst| inst.opcode == "phi")
                .any(|phi| {
                    phi_incomings(phi)
                        .iter()
                        .any(|(_, label)| label == &l.label)
                });
            if !contributes_back_edge {
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
    let merge_result = m_phi.result.clone().unwrap_or_default();
    // The merge may post-process the phi before returning (clang's
    // runtime-extent search clamps it with `llvm.umin(phi, n)` on the
    // `n == 0` guard path). Those instructions are emitted after the
    // synthesized loop with the phi mapped to the index output, so the
    // observable result is preserved without assuming the clamp is a
    // no-op.
    // Either the merge returns directly (the common shape) or it branches to
    // a tail phi that selects the entry-guard value against the merge result
    // (clang's explicit `n == 0` guard). The tail form is resolved below.
    let ret_operand: Option<String> = m
        .instructions
        .iter()
        .find(|i| i.opcode == "ret")
        .and_then(|i| i.operands.first().cloned());

    // icmp operands keep the comparison token in the first operand
    // ("eq i64 %1"), so compare on each operand's value token.
    let value_token = |o: &str| {
        o.split_whitespace()
            .last()
            .unwrap_or(o)
            .trim_end_matches(')')
            .to_string()
    };

    // ── Descending (pre-decrement) variant ─────────────────────────
    // clang lowers `for (i = n; i-- > 0;) if (buf[i]) return i;` as a
    // header that exits at `i == 0` and a latch/body computing
    // `next = i - 1` (the accessed index). The merge's header incoming
    // is the sentinel (the initial counter); its latch incoming is the
    // successor. Synthesize the loop with `index` updated by the
    // SUCCESSOR on the first hit and terminate on `!found && i != 0`:
    // the pre-tested domain covers indices init-1 .. 0 exactly.
    {
        let h_succs = block_successors(h);
        let l_succs = block_successors(l);
        let successor_name = l.instructions.iter().find_map(|inst| {
            let (lhs, rhs) = match inst.opcode.as_str() {
                "sub" => (
                    strip_type(inst.operands.first()?),
                    value_token(inst.operands.get(1)?),
                ),
                "add" => (
                    strip_type(inst.operands.first()?),
                    value_token(inst.operands.get(1)?),
                ),
                _ => return None,
            };
            if lhs != counter_name {
                return None;
            }
            let step = parse_int_constant(&rhs)?;
            let is_decrement = (inst.opcode == "sub" && step == 1)
                || (inst.opcode == "add" && step == -1);
            if !is_decrement {
                return None;
            }
            inst.result.clone()
        });
        let descending = successor_name
            .as_deref()
            .map(|succ| strip_type(&sentinel_str) == succ)
            .unwrap_or(false)
            && strip_type(&from_h) == strip_type(&init_val)
            && h_succs.contains(&m.label)
            && h_succs.contains(&l.label)
            && l_succs.contains(&m.label)
            && l_succs.contains(&h.label)
            && !m_incomings
                .iter()
                .any(|(_, label)| label != &h.label && label != &l.label);
        if descending {
            let successor_name = successor_name.unwrap();
            // Pre-header blocks (same rule as the ascending path).
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
            let counter_init = get_node_id(
                &init_val,
                value_map,
                &ir.params,
                builder,
                Some(counter_ty.clone()),
            )
            .ok_or("early-exit search: cannot resolve the descending counter init")?;
            let sentinel_node = get_node_id(
                &from_h,
                value_map,
                &ir.params,
                builder,
                Some(index_ty.clone()),
            )
            .ok_or("early-exit search: cannot resolve the descending sentinel")?;
            value_map.insert(counter_name.clone(), counter_init);
            let mut body_nodes: Vec<NodeId> = Vec::new();
            for inst in h.instructions.iter().chain(l.instructions.iter()) {
                if inst.opcode == "phi" || inst.opcode == "br" || inst.opcode == "ret" {
                    continue;
                }
                // Normalize clang's `add counter, -1` successor to a real
                // `Sub(counter, 1)` node: semantically identical, and the
                // word-level recognizers use the Sub form to classify a
                // scan as reverse (an Add with -1 would be mistaken for a
                // forward scan by the position-search heuristic).
                if inst.result.as_deref() == Some(successor_name.as_str())
                    && inst.opcode == "add"
                {
                    let one = builder.constant(
                        int_constant_data(&counter_ty, 1),
                        counter_ty.clone(),
                        span,
                    );
                    let sub = builder
                        .sub(counter_init, one, span)
                        .map_err(|e| format!("early-exit search: {:?}", e))?;
                    body_nodes.push(sub);
                    value_map.insert(successor_name.clone(), sub);
                    continue;
                }
                if let Some(id) = emit_instruction(inst, builder, value_map, &ir.params, span)? {
                    body_nodes.push(id);
                }
            }
            let h_cond = get_node_id(
                &strip_type(&h_br.operands[0]),
                value_map,
                &ir.params,
                builder,
                None,
            )
            .ok_or("early-exit search: cannot resolve the descending exit test")?;
            let l_cond = get_node_id(
                &strip_type(&l_br.operands[0]),
                value_map,
                &ir.params,
                builder,
                None,
            )
            .ok_or("early-exit search: cannot resolve the descending hit test")?;
            // Continue while the header's counter-zero exit test is false.
            // When the test is `counter == 0`, encode the continuation as
            // `Gt(counter, 0)` rather than `BoolNot(...)`: they are
            // equivalent for unsigned counters, and the explicit `Gt` form
            // keeps the descending scan from matching the FORWARD
            // found-flag recognizer (whose inclusive-counter model would
            // otherwise derive a wrong FirstOccurrence truth).
            let h_exit_on_true = h_succs.first().map(|s| s == &m.label).unwrap_or(false);
            let continuation = if h_exit_on_true {
                let counter_zero = {
                    let function = builder.function();
                    let is_zero =
                        |id: NodeId| lower_constant_u64(function, id) == Some(0);
                    match function.get_node(h_cond).map(|n| &n.kind) {
                        Some(NodeKind::Eq { lhs, rhs })
                        | Some(NodeKind::Ne { lhs, rhs }) => {
                            (*lhs == counter_init && is_zero(*rhs))
                                || (*rhs == counter_init && is_zero(*lhs))
                        }
                        _ => false,
                    }
                };
                if counter_zero {
                    let zero = builder.constant(
                        int_constant_data(&counter_ty, 0),
                        counter_ty.clone(),
                        span,
                    );
                    builder
                        .gt(counter_init, zero, span)
                        .map_err(|e| format!("early-exit search: {:?}", e))?
                } else {
                    let not = builder
                        .bool_not(h_cond, span)
                        .map_err(|e| format!("early-exit search: {:?}", e))?;
                    body_nodes.push(not);
                    not
                }
            } else if h_succs.get(1).map(|s| s == &m.label).unwrap_or(false) {
                h_cond
            } else {
                return Err("early-exit search: descending header branch is not the exit test".into());
            };
            // Hit from the latch branch: continuing on true means hit = !cond.
            let l_continue_on_true = l_succs.first().map(|s| s == &h.label).unwrap_or(false);
            let hit = if l_continue_on_true {
                let not = builder
                    .bool_not(l_cond, span)
                    .map_err(|e| format!("early-exit search: {:?}", e))?;
                body_nodes.push(not);
                not
            } else if l_succs.first().map(|s| s == &m.label).unwrap_or(false) {
                l_cond
            } else {
                return Err("early-exit search: descending latch branch is not the hit test".into());
            };
            let successor = get_node_id(
                &successor_name,
                value_map,
                &ir.params,
                builder,
                None,
            )
            .ok_or("early-exit search: cannot resolve the descending successor")?;
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
            // The descending index seed is the sentinel, which is often the
            // same SIR node as the counter seed (both are the entry counter).
            // If that node is used directly as a loop-carried input and also
            // referenced inside the body, the emitter's carrier renaming can
            // alias the reference to the MUTABLE counter carrier instead of
            // the frozen sentinel. Materialize a distinct pre-loop snapshot so
            // the no-hit path keeps returning the original sentinel.
            let zero = builder.constant(int_constant_data(&index_ty, 0), index_ty.clone(), span);
            let sentinel_snapshot = builder
                .add(sentinel_node, zero, span)
                .map_err(|e| format!("early-exit search: {:?}", e))?;
            let index_next = builder
                .select(update, successor, sentinel_snapshot, span)
                .map_err(|e| format!("early-exit search: {:?}", e))?;
            let termination = builder
                .bool_and(not_found, continuation, span)
                .map_err(|e| format!("early-exit search: {:?}", e))?;
            body_nodes.extend([found_next, update, index_next, termination]);
            let outputs = vec![found_next, index_next, successor];
            let carried = vec![found_init, sentinel_snapshot, counter_init];
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
            value_map.insert(counter_name.clone(), counter_extract);
            value_map.insert(merge_result.clone(), index_extract);
            for inst in &m.instructions {
                if inst.opcode == "phi" || inst.opcode == "ret" {
                    continue;
                }
                emit_instruction(inst, builder, value_map, &ir.params, span)?;
            }
            let Some(descending_ret) = ret_operand.as_ref() else {
                return Err("early-exit search: descending merge does not return".into());
            };
            let ret_node = get_node_id(
                &strip_type(descending_ret),
                value_map,
                &ir.params,
                builder,
                None,
            )
            .ok_or("early-exit search: cannot resolve the descending return value")?;
            builder
                .return_value(ret_node, span)
                .map_err(|e| format!("early-exit search return: {:?}", e))?;
            return Ok(());
        }
    }

    // The ascending synthesis carries the counter itself as the found
    // index; the merge's header incoming must be that counter.
    if strip_type(&from_h) != counter_name {
        return Err("early-exit search: merge does not return the counter".into());
    }

    // How the merge turns the phi into the returned value:
    //   Identity — `ret phi` (the zero-trip observable is the extra
    //              incoming, so it must equal the sentinel);
    //   Clamp    — `ret umin(phi, bound)` (the zero-trip observable is
    //              `umin(x, 0) == 0` for any unsigned x, so the guard
    //              only has to pin the trip bound to zero).
    #[derive(Debug)]
    enum PostForm {
        Identity,
        Clamp(String),
    }
    struct TailExit {
        entry_label: String,
        entry_value: String,
    }
    let (post_value, tail_exit) = match ret_operand.as_ref() {
        Some(ret) => (strip_type(ret), None),
        None => {
            let tail_label = block_successors(m)
                .into_iter()
                .next()
                .ok_or("early-exit search: merge neither returns nor branches to a tail")?;
            let tail = ir
                .block_map
                .get(&tail_label)
                .and_then(|index| ir.blocks.get(*index))
                .ok_or("early-exit search: tail block not found")?;
            let tail_phi = tail
                .instructions
                .iter()
                .find(|inst| inst.opcode == "phi")
                .ok_or("early-exit search: tail has no phi")?;
            if !tail.instructions.iter().any(|inst| inst.opcode == "ret") {
                return Err("early-exit search: tail does not return".into());
            }
            let incomings = phi_incomings(tail_phi);
            let merge_value = incomings
                .iter()
                .find(|(_, label)| label == &m.label)
                .map(|(value, _)| strip_type(value))
                .ok_or("early-exit search: tail phi lacks the merge incoming")?;
            let (entry_value, entry_label) = incomings
                .iter()
                .find(|(_, label)| label != &m.label)
                .map(|(value, label)| (strip_type(value), label.clone()))
                .ok_or("early-exit search: tail phi lacks the entry incoming")?;
            (
                merge_value,
                Some(TailExit {
                    entry_label,
                    entry_value,
                }),
            )
        }
    };
    let post_form = if post_value == merge_result {
        PostForm::Identity
    } else {
        let call = m
            .instructions
            .iter()
            .find(|i| {
                i.opcode == "call"
                    && i.result
                        .as_deref()
                        .map(|r| strip_type(r) == post_value)
                        .unwrap_or(false)
            })
            .ok_or("early-exit search: merge return is not the phi or a call")?;
        let raw = call.operands.join(", ");
        if !raw.contains("llvm.umin") {
            return Err("early-exit search: merge post-processing is not a umin clamp".into());
        }
        let inside = raw
            .split('(')
            .nth(1)
            .unwrap_or("")
            .trim_end_matches(')');
        let parts: Vec<String> = inside
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let phi_seen = parts.iter().any(|p| {
            p.split_whitespace()
                .last()
                .map(|t| t.trim_end_matches(')') == merge_result)
                .unwrap_or(false)
        });
        if !phi_seen {
            return Err("early-exit search: umin clamp does not consume the merge phi".into());
        }
        let bound = parts
            .iter()
            .filter_map(|p| p.split_whitespace().last())
            .map(|t| t.trim_end_matches(')').to_string())
            .find(|t| *t != merge_result)
            .ok_or("early-exit search: umin clamp has no bound operand")?;
        PostForm::Clamp(bound)
    };

    // Extra merge predecessors are only tolerated as the zero-trip entry
    // guard: the predecessor must branch to the merge when the loop's
    // TRIP BOUND is zero (not some unrelated value), and the observable
    // on that path must agree with the synthesized loop's zero-trip
    // output (identity: the incoming equals the sentinel; clamp: the
    // umin bound is the trip bound, which forces zero either way).
    // icmp operands keep the comparison token in the first operand
    // ("eq i64 %1"), so compare on each operand's value token.
    let value_token = |o: &str| {
        o.split_whitespace()
            .last()
            .unwrap_or(o)
            .trim_end_matches(')')
            .to_string()
    };
    // The latch's bound operand: the non-successor side of its
    // comparison. The zero-trip guard must compare THIS value, not merely
    // the merge's latch incoming (otherwise a guard on one value with a
    // different trip bound would let the synthesized loop iterate while
    // the source skipped).
    let latch_successor = l
        .instructions
        .iter()
        .find_map(|inst| (inst.opcode == "add").then(|| inst.result.clone()).flatten());
    let latch_bound_str = latch_successor.as_ref().and_then(|succ| {
        l.instructions
            .iter()
            .find(|inst| inst.opcode == "icmp")
            .and_then(|inst| {
                inst.operands
                    .iter()
                    .map(|o| value_token(o))
                    .find(|t| t != succ)
            })
    });
    let Some(latch_bound_str) = latch_bound_str else {
        return Err("early-exit search: latch bound not identifiable".into());
    };
    if let PostForm::Clamp(bound) = &post_form {
        if bound != &latch_bound_str {
            return Err("early-exit search: umin clamp bound is not the trip bound".into());
        }
    }
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
        let guard_matches = p.instructions.iter().any(|inst| {
            inst.opcode == "icmp"
                && inst.result.as_deref() == Some(p_cond.as_str())
                && inst
                    .operands
                    .iter()
                    .any(|o| value_token(o) == latch_bound_str)
                && inst
                    .operands
                    .iter()
                    .any(|o| {
                        parse_int_constant(&value_token(o))
                            .map(|v| v == 0)
                            .unwrap_or(false)
                    })
        });
        let observable_matches = match &post_form {
            PostForm::Identity => strip_type(value) == sentinel_str,
            PostForm::Clamp(_) => true,
        };
        if !(p_succs.contains(&m.label) && guard_matches && observable_matches) {
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
        .tuple_extract(loop_node, 1, index_ty.clone(), span)
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
    let post_node = get_node_id(&post_value, value_map, &ir.params, builder, None)
        .ok_or("early-exit search: cannot resolve the merge return value")?;
    let ret_node = match &tail_exit {
        None => post_node,
        Some(tail) => {
            // The tail phi selects the entry-guard value (for example 0 on
            // `n == 0`) against the merge result. Re-emit that select with the
            // already-lowered guard condition.
            let entry_index = ir.block_map.get(&tail.entry_label).copied().or_else(|| {
                (ir.blocks.first().map(|block| block.label.as_str()) == Some("entry")
                    && tail.entry_label.chars().all(|c| c.is_ascii_digit()))
                .then_some(0usize)
            });
            let entry_block = entry_index
                .and_then(|index| ir.blocks.get(index))
                .ok_or("early-exit search: tail entry block not found")?;
            let entry_br = entry_block
                .instructions
                .iter()
                .rev()
                .find(|inst| inst.opcode == "br")
                .ok_or("early-exit search: tail entry block has no branch")?;
            let guard_cond = get_node_id(
                &strip_type(&entry_br.operands[0]),
                value_map,
                &ir.params,
                builder,
                None,
            )
            .ok_or("early-exit search: tail guard condition not resolved")?;
            let entry_node = get_node_id(
                &tail.entry_value,
                value_map,
                &ir.params,
                builder,
                Some(index_ty.clone()),
            )
            .ok_or("early-exit search: tail entry value not resolved")?;
            builder
                .select(guard_cond, entry_node, post_node, span)
                .map_err(|e| format!("early-exit search: {:?}", e))?
        }
    };
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
            None => match rotated_zero_test_domain(
                builder,
                termination,
                &carried_init_nodes,
                &output_nodes,
            ) {
                Some(rebuilt) if has_entry_guard(ir, loop_idx) => rebuilt,
                Some(_) => {
                    return Err(format!(
                        "unsupported: unguarded rotated zero-test loop (back-edge on false \
                         with an exit test on the next carried value); SIR loops are \
                         pre-tested, so the first forced iteration cannot be represented: {}",
                        ir.name
                    ));
                }
                None => {
                    return Err(format!(
                        "unsupported: rotated loop exit (back-edge on false) that is not the \
                         counted eq-next form (`icmp eq next, bound` with `next = carry + 1`) \
                         or a guarded zero-test (`icmp eq next, 0`); lowering it as SIR would \
                         invert or shift the iteration domain"
                    ));
                }
            },
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
                    let is_void = inst
                        .operands
                        .first()
                        .map(|operand| strip_type(operand) == "void")
                        .unwrap_or(true);
                    if !is_void {
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

/// A constant global memory location: symbol + element type + element index.
#[derive(Clone, PartialEq, Eq, Hash)]
struct MemLoc {
    global: String,
    ty: Type,
    index: u64,
}

#[derive(Clone)]
struct SwitchPath {
    cond: NodeId,
    ret: Option<NodeId>,
    writes: Vec<(MemLoc, NodeId)>,
}

struct SwitchState {
    block: usize,
    incoming: Option<usize>,
    values: HashMap<String, NodeId>,
    memory: HashMap<MemLoc, NodeId>,
    written: Vec<MemLoc>,
    cond: NodeId,
}

#[derive(Clone)]
struct BlockState {
    values: HashMap<String, NodeId>,
    memory: HashMap<MemLoc, NodeId>,
}

struct ExitState {
    reach: NodeId,
    ret: Option<NodeId>,
    memory: HashMap<MemLoc, NodeId>,
}

struct SwitchLowerer<'a> {
    ir: &'a IrFunction,
    builder: &'a mut Builder,
    span: Span,
    true_cond: NodeId,
    false_cond: NodeId,
    gep_locs: HashMap<String, MemLoc>,
    global_elem: HashMap<String, Type>,
    initial_loads: HashMap<MemLoc, NodeId>,
    paths: Vec<SwitchPath>,
    exits: Vec<ExitState>,
}

impl<'a> SwitchLowerer<'a> {
    fn cond_and(&mut self, a: NodeId, b: NodeId) -> Result<NodeId, String> {
        if a == self.false_cond || b == self.false_cond {
            return Ok(self.false_cond);
        }
        if a == self.true_cond {
            return Ok(b);
        }
        if b == self.true_cond {
            return Ok(a);
        }
        self.builder
            .bool_and(a, b, self.span)
            .map_err(|e| format!("switch CFG: {:?}", e))
    }

    fn cond_not(&mut self, a: NodeId) -> Result<NodeId, String> {
        if a == self.true_cond {
            return Ok(self.false_cond);
        }
        if a == self.false_cond {
            return Ok(self.true_cond);
        }
        self.builder
            .bool_not(a, self.span)
            .map_err(|e| format!("switch CFG: {:?}", e))
    }

    fn cond_or(&mut self, a: NodeId, b: NodeId) -> Result<NodeId, String> {
        if a == self.true_cond || b == self.true_cond {
            return Ok(self.true_cond);
        }
        if a == self.false_cond {
            return Ok(b);
        }
        if b == self.false_cond {
            return Ok(a);
        }
        self.builder
            .bool_or(a, b, self.span)
            .map_err(|e| format!("switch CFG: {:?}", e))
    }

    fn note_loc(&mut self, name: String, ty: Type, index: u64) -> Result<MemLoc, String> {
        if let Some(previous) = self.global_elem.get(&name) {
            if previous != &ty {
                return Err(format!(
                    "switch CFG: global @{} accessed with mixed element types",
                    name
                ));
            }
        } else {
            self.global_elem.insert(name.clone(), ty.clone());
        }
        Ok(MemLoc { global: name, ty, index })
    }

    fn resolve_loc(&mut self, operand: &str, access_ty: &Type) -> Result<MemLoc, String> {
        if let Some((name, ty, index)) = inline_global_gep(operand) {
            return self.note_loc(name, ty, index);
        }
        let base = strip_type(operand);
        if let Some(loc) = self.gep_locs.get(&base) {
            return Ok(loc.clone());
        }
        if let Some(name) = base.strip_prefix('@') {
            return self.note_loc(name.to_string(), access_ty.clone(), 0);
        }
        Err(format!(
            "switch CFG: non-global memory operand '{}'",
            operand
        ))
    }

    fn record_gep(&mut self, inst: &Instruction) -> Result<(), String> {
        if inst.operands.len() < 3 {
            return Err(format!("switch CFG: malformed GEP: {}", inst.raw));
        }
        let source = inst.operands[0]
            .trim_start_matches("inbounds")
            .trim();
        let element = if source.starts_with('[') {
            match gep_source_array_type(&inst.raw) {
                Some(Type::Array { element, .. }) => *element,
                _ => return Err(format!("switch CFG: malformed GEP type: {}", inst.raw)),
            }
        } else {
            parse_type(source).ok_or_else(|| format!("unsupported GEP type: {}", inst.raw))?
        };
        let base = strip_type(&inst.operands[1]);
        let index_str = if inst.operands.len() >= 4 {
            strip_type(inst.operands.last().unwrap())
        } else {
            strip_type(&inst.operands[2])
        };
        let index = parse_int_constant(&index_str)
            .filter(|index| *index >= 0)
            .ok_or_else(|| format!("switch CFG: non-constant GEP index: {}", inst.raw))?
            as u64;
        let name = if let Some(name) = base.strip_prefix('@') {
            name.to_string()
        } else if let Some(loc) = self.gep_locs.get(&base) {
            loc.global.clone()
        } else {
            return Ok(());
        };
        let loc = self.note_loc(name, element, index)?;
        if let Some(result) = &inst.result {
            self.gep_locs.insert(result.clone(), loc);
        }
        Ok(())
    }

    fn initial_load(
        &mut self,
        loc: &MemLoc,
        seed: &HashMap<String, NodeId>,
    ) -> Result<NodeId, String> {
        if let Some(id) = self.initial_loads.get(loc) {
            return Ok(*id);
        }
        let global = seed
            .get(&loc.global)
            .copied()
            .or_else(|| {
                get_node_id(
                    &format!("@{}", loc.global),
                    seed,
                    &self.ir.params,
                    self.builder,
                    None,
                )
            })
            .ok_or(format!("switch CFG: unknown global @{}", loc.global))?;
        let index = self
            .builder
            .constant(ConstantData::u64(loc.index), Type::u64(), self.span);
        let access = self
            .builder
            .array_access(global, index, loc.ty.clone(), self.span)
            .map_err(|e| format!("switch CFG: {:?}", e))?;
        // `ArrayAccess` is both the lvalue and the current-value read in
        // the existing lowering; the initial memory state is the location
        // itself (stores are emitted after all path selections).
        self.initial_loads.insert(loc.clone(), access);
        Ok(access)
    }

    fn push_successor(
        &mut self,
        stack: &mut Vec<SwitchState>,
        state: &SwitchState,
        label: &str,
        cond: NodeId,
    ) -> Result<(), String> {
        if cond == self.false_cond {
            return Ok(());
        }
        let Some(block) = self.ir.block_map.get(label).copied() else {
            return Err(format!("switch CFG: unknown block %{}", label));
        };
        stack.push(SwitchState {
            block,
            incoming: Some(state.block),
            values: state.values.clone(),
            memory: state.memory.clone(),
            written: state.written.clone(),
            cond,
        });
        Ok(())
    }

    fn run(&mut self, seed: HashMap<String, NodeId>) -> Result<(), String> {
        let mut stack = vec![SwitchState {
            block: 0,
            incoming: None,
            values: seed,
            memory: HashMap::new(),
            written: Vec::new(),
            cond: self.true_cond,
        }];
        let mut processed = 0usize;
        while let Some(mut state) = stack.pop() {
            processed += 1;
            if processed > 65536 {
                return Err("switch CFG: path exploration limit exceeded".to_string());
            }
            let instructions = self.ir.blocks[state.block].instructions.clone();
            let incoming_label = state
                .incoming
                .map(|index| self.ir.blocks[index].label.clone());
            for inst in instructions.iter().filter(|i| i.opcode == "phi") {
                let phi_ty = inst
                    .operands
                    .first()
                    .and_then(|operand| operand_type(operand));
                let wanted = incoming_label.as_ref().ok_or(
                    "switch CFG: phi in entry block".to_string(),
                )?;
                let incomings = phi_incomings(inst);
                let Some((value, _)) = incomings.iter().find(|(_, label)| label == wanted) else {
                    return Err(format!("switch CFG: phi missing edge from %{}", wanted));
                };
                let value = get_node_id(
                    &strip_type(value),
                    &state.values,
                    &self.ir.params,
                    self.builder,
                    phi_ty,
                )
                .ok_or(format!("switch CFG: unresolved phi value '{}'", value))?;
                if let Some(result) = &inst.result {
                    state.values.insert(result.clone(), value);
                }
            }
            let mut terminator: Option<Instruction> = None;
            for inst in instructions.iter().filter(|i| i.opcode != "phi") {
                match inst.opcode.as_str() {
                    "load" => {
                        if inst.operands.len() < 2 {
                            return Err(format!("switch CFG: malformed load: {}", inst.raw));
                        }
                        if inst.raw.contains("volatile") || inst.raw.contains("atomic") {
                            return Err(format!(
                                "switch CFG: volatile/atomic memory: {}",
                                inst.raw
                            ));
                        }
                        let access_ty = operand_type(&inst.operands[0])
                            .ok_or(format!("switch CFG: load type: {}", inst.raw))?;
                        let loc = self.resolve_loc(&inst.operands[1], &access_ty)?;
                        let value = if let Some(value) = state.memory.get(&loc) {
                            *value
                        } else {
                            self.initial_load(&loc, &state.values)?
                        };
                        if let Some(result) = &inst.result {
                            state.values.insert(result.clone(), value);
                        }
                    }
                    "store" => {
                        if inst.operands.len() < 2 {
                            return Err(format!("switch CFG: malformed store: {}", inst.raw));
                        }
                        if inst.raw.contains("volatile") || inst.raw.contains("atomic") {
                            return Err(format!(
                                "switch CFG: volatile/atomic memory: {}",
                                inst.raw
                            ));
                        }
                        let access_ty = operand_type(&inst.operands[0])
                            .ok_or(format!("switch CFG: store type: {}", inst.raw))?;
                        let value = get_node_id(
                            &strip_type(&inst.operands[0]),
                            &state.values,
                            &self.ir.params,
                            self.builder,
                            Some(access_ty.clone()),
                        )
                        .ok_or(format!("switch CFG: unresolved store value: {}", inst.raw))?;
                        if self
                            .builder
                            .function()
                            .get_node(value)
                            .is_some_and(|node| node.ty != access_ty)
                        {
                            return Err(format!(
                                "switch CFG: store type mismatch: {}",
                                inst.raw
                            ));
                        }
                        let loc = self.resolve_loc(&inst.operands[1], &access_ty)?;
                        state.memory.insert(loc.clone(), value);
                        state.written.push(loc);
                    }
                    "getelementptr" => {
                        self.record_gep(inst)?;
                    }
                    "br" | "switch" | "ret" | "unreachable" => {
                        terminator = Some(inst.clone());
                        break;
                    }
                    _ => {
                        emit_instruction(
                            inst,
                            self.builder,
                            &mut state.values,
                            &self.ir.params,
                            self.span,
                        )?;
                    }
                }
            }
            let terminator =
                terminator.ok_or_else(|| "switch CFG: block without terminator".to_string())?;
            match terminator.opcode.as_str() {
                "ret" => {
                    let is_void = terminator
                        .operands
                        .first()
                        .map(|operand| strip_type(operand) == "void")
                        .unwrap_or(true);
                    let ret = if is_void {
                        None
                    } else {
                        Some(
                            get_node_id(
                                &strip_type(&terminator.operands[0]),
                                &state.values,
                                &self.ir.params,
                                self.builder,
                                None,
                            )
                            .ok_or_else(|| {
                                format!("switch CFG: unresolved return: {}", terminator.raw)
                            })?,
                        )
                    };
                    let mut writes: Vec<(MemLoc, NodeId)> = Vec::new();
                    for loc in &state.written {
                        if let Some(value) = state.memory.get(loc) {
                            writes.retain(|(existing, _)| existing != loc);
                            writes.push((loc.clone(), *value));
                        }
                    }
                    self.paths.push(SwitchPath {
                        cond: state.cond,
                        ret,
                        writes,
                    });
                    if self.paths.len() > 1024 {
                        return Err("switch CFG: too many paths".to_string());
                    }
                }
                "unreachable" => {
                    return Err("switch CFG: unreachable terminator".to_string());
                }
                "br" => {
                    if terminator.operands.len() == 1 {
                        let label = terminator.operands[0]
                            .trim_start_matches("label ")
                            .trim_start_matches('%')
                            .to_string();
                        let cond = state.cond;
                        self.push_successor(&mut stack, &state, &label, cond)?;
                    } else if terminator.operands.len() >= 3 {
                        let condition = get_node_id(
                            &strip_type(&terminator.operands[0]),
                            &state.values,
                            &self.ir.params,
                            self.builder,
                            None,
                        )
                        .ok_or_else(|| {
                            format!("switch CFG: unresolved branch: {}", terminator.raw)
                        })?;
                        let true_label = terminator.operands[1]
                            .trim_start_matches("label ")
                            .trim_start_matches('%')
                            .to_string();
                        let false_label = terminator.operands[2]
                            .trim_start_matches("label ")
                            .trim_start_matches('%')
                            .to_string();
                        let true_cond = self.cond_and(state.cond, condition)?;
                        let negated = self.cond_not(condition)?;
                        let false_cond = self.cond_and(state.cond, negated)?;
                        self.push_successor(&mut stack, &state, &true_label, true_cond)?;
                        self.push_successor(&mut stack, &state, &false_label, false_cond)?;
                    } else {
                        return Err(format!("switch CFG: malformed br: {}", terminator.raw));
                    }
                }
                "switch" => {
                    let raw = &terminator.raw;
                    let open = raw
                        .find('[')
                        .ok_or_else(|| format!("switch CFG: malformed switch: {}", raw))?;
                    let close = raw
                        .rfind(']')
                        .ok_or_else(|| format!("switch CFG: malformed switch: {}", raw))?;
                    let head = &raw[..open];
                    let body = &raw[open + 1..close];
                    let mut head_parts = head.split(',').map(str::trim);
                    let value_part = head_parts.next().unwrap_or("");
                    let value_part = value_part.trim_start_matches("switch").trim();
                    let value_ty = operand_type(value_part)
                        .ok_or_else(|| format!("switch CFG: switch type: {}", raw))?;
                    let value = get_node_id(
                        &strip_type(value_part),
                        &state.values,
                        &self.ir.params,
                        self.builder,
                        Some(value_ty.clone()),
                    )
                    .ok_or_else(|| format!("switch CFG: unresolved switch value: {}", raw))?;
                    let default = head_parts
                        .next()
                        .ok_or_else(|| format!("switch CFG: missing default: {}", raw))?
                        .trim_start_matches("label ")
                        .trim_start_matches('%')
                        .to_string();
                    let mut cases: Vec<(i64, String)> = Vec::new();
                    let tokens: Vec<&str> = body.split_whitespace().collect();
                    let mut i = 0;
                    while i + 3 < tokens.len() {
                        if tokens[i + 2] == "label" {
                            let constant = tokens[i + 1]
                                .trim_end_matches(',')
                                .parse::<i64>()
                                .map_err(|_| {
                                    format!("switch CFG: non-constant case: {}", raw)
                                })?;
                            let label = tokens[i + 3].trim_start_matches('%').to_string();
                            cases.push((constant, label));
                            i += 4;
                        } else {
                            i += 1;
                        }
                    }
                    let mut default_guard = self.true_cond;
                    for (constant, label) in &cases {
                        let constant_node = self.builder.constant(
                            int_constant_data(&value_ty, *constant as u64),
                            value_ty.clone(),
                            self.span,
                        );
                        let equal = self
                            .builder
                            .eq(value, constant_node, self.span)
                            .map_err(|e| format!("switch CFG: {:?}", e))?;
                        let case_cond = self.cond_and(state.cond, equal)?;
                        self.push_successor(&mut stack, &state, label, case_cond)?;
                        let not_equal = self
                            .builder
                            .ne(value, constant_node, self.span)
                            .map_err(|e| format!("switch CFG: {:?}", e))?;
                        default_guard = self.cond_and(default_guard, not_equal)?;
                    }
                    let default_cond = self.cond_and(state.cond, default_guard)?;
                    self.push_successor(&mut stack, &state, &default, default_cond)?;
                }
                _ => return Err(format!("switch CFG: unsupported terminator: {}", terminator.raw)),
            }
        }
        Ok(())
    }

    fn finish(mut self, ret_ty: &Type, seed: &HashMap<String, NodeId>) -> Result<(), String> {
        let mut finals: HashMap<MemLoc, NodeId> = HashMap::new();
        let mut result: Option<NodeId> = None;
        let paths = self.paths.clone();
        for path in &paths {
            let cond = path.cond;
            let ret = path.ret;
            let writes = path.writes.clone();
            let mut last: HashMap<MemLoc, NodeId> = HashMap::new();
            for (loc, value) in writes {
                last.insert(loc, value);
            }
            for (loc, value) in last {
                let initial = self.initial_load(&loc, seed)?;
                let current = finals.entry(loc).or_insert(initial);
                *current = self
                    .builder
                    .select(cond, value, *current, self.span)
                    .map_err(|e| format!("switch CFG: {:?}", e))?;
            }
            if let Some(value) = ret {
                result = Some(match result {
                    Some(previous) => self
                        .builder
                        .select(cond, value, previous, self.span)
                        .map_err(|e| format!("switch CFG: {:?}", e))?,
                    None => value,
                });
            }
        }
        for (loc, value) in finals {
            let global = seed
                .get(&loc.global)
                .copied()
                .ok_or(format!("switch CFG: unknown global @{}", loc.global))?;
            let index = self
                .builder
                .constant(ConstantData::u64(loc.index), Type::u64(), self.span);
            let access = self
                .builder
                .array_access(global, index, loc.ty.clone(), self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
            self.builder
                .store(access, value, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        }
        if ret_ty == &Type::Unit {
            let unit = self
                .builder
                .constant(ConstantData::Unit, Type::Unit, self.span);
            self.builder
                .return_value(unit, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        } else {
            let result = result
                .ok_or_else(|| "switch CFG: no returning path".to_string())?;
            self.builder
                .return_value(result, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        }
        Ok(())
    }

    /// Block-level join engine: one state per CFG block, with incoming
    /// conditions folded through `Select`. Scales where path enumeration
    /// explodes (state-machine switches).
    fn run_blocks(&mut self, seed: HashMap<String, NodeId>) -> Result<(), String> {
        let order = dag_order(self.ir);
        let block_count = self.ir.blocks.len();
        let mut preds: Vec<Option<BlockState>> = vec![None; block_count];
        let mut incoming: Vec<Vec<(usize, NodeId)>> = vec![Vec::new(); block_count];

        for idx in order {
            let instructions = self.ir.blocks[idx].instructions.clone();
            let used = block_value_names(&instructions);
            let entries = incoming[idx].clone();
            let (reach, mut values, mut memory) = if idx == 0 {
                (self.true_cond, seed.clone(), HashMap::new())
            } else if entries.is_empty() {
                continue;
            } else {
                let mut reach = self.false_cond;
                for (_, cond) in &entries {
                    reach = self.cond_or(reach, *cond)?;
                }
                let mut values = seed.clone();
                let mut keys: HashSet<String> = HashSet::new();
                for (pred, _) in &entries {
                    if let Some(state) = &preds[*pred] {
                        for key in state.values.keys() {
                            if !seed.contains_key(key) {
                                keys.insert(key.clone());
                            }
                        }
                    }
                }
                for key in keys {
                    let mut joined = None;
                    let mut present = 0usize;
                    for (pred, cond) in &entries {
                        if let Some(value) =
                            preds[*pred].as_ref().and_then(|state| state.values.get(&key))
                        {
                            present += 1;
                            joined = Some(match joined {
                                None => *value,
                                Some(previous) => self
                                    .builder
                                    .select(*cond, *value, previous, self.span)
                                    .map_err(|e| format!("switch CFG: {:?}", e))?,
                            });
                        }
                    }
                    if present != entries.len() && used.contains(&key) {
                        return Err(format!(
                            "switch CFG: value {} is not defined on every incoming path",
                            key
                        ));
                    }
                    if present == entries.len() {
                        if let Some(value) = joined {
                            values.insert(key, value);
                        }
                    }
                }
                let mut keys: HashSet<MemLoc> = HashSet::new();
                for (pred, _) in &entries {
                    if let Some(state) = &preds[*pred] {
                        keys.extend(state.memory.keys().cloned());
                    }
                }
                let mut memory = HashMap::new();
                for loc in keys {
                    let mut joined = None;
                    for (pred, cond) in &entries {
                        let value = match preds[*pred]
                            .as_ref()
                            .and_then(|state| state.memory.get(&loc))
                        {
                            Some(value) => *value,
                            None => self.initial_load(&loc, &seed)?,
                        };
                        joined = Some(match joined {
                            None => value,
                            Some(previous) => self
                                .builder
                                .select(*cond, value, previous, self.span)
                                .map_err(|e| format!("switch CFG: {:?}", e))?,
                        });
                    }
                    if let Some(value) = joined {
                        memory.insert(loc, value);
                    }
                }
                (reach, values, memory)
            };

            for inst in instructions.iter().filter(|inst| inst.opcode == "phi") {
                let phi_ty = inst
                    .operands
                    .first()
                    .and_then(|operand| operand_type(operand));
                let mut joined = None;
                for (value, label) in phi_incomings(inst) {
                    let Some(pred) = self.ir.block_map.get(&label).copied() else {
                        continue;
                    };
                    let Some((_, cond)) = entries.iter().find(|(p, _)| *p == pred) else {
                        continue;
                    };
                    let mut lookup = values.clone();
                    if let Some(state) = &preds[pred] {
                        lookup.extend(state.values.clone());
                    }
                    let value = get_node_id(
                        &strip_type(&value),
                        &lookup,
                        &self.ir.params,
                        self.builder,
                        phi_ty.clone(),
                    )
                    .ok_or(format!("switch CFG: unresolved phi value '{}'", value))?;
                    joined = Some(match joined {
                        None => value,
                        Some(previous) => self
                            .builder
                            .select(*cond, value, previous, self.span)
                            .map_err(|e| format!("switch CFG: {:?}", e))?,
                    });
                }
                if let (Some(value), Some(result)) = (joined, &inst.result) {
                    values.insert(result.clone(), value);
                }
            }

            let mut terminator: Option<Instruction> = None;
            for inst in instructions.iter().filter(|inst| inst.opcode != "phi") {
                match inst.opcode.as_str() {
                    "load" => {
                        if inst.raw.contains("volatile") || inst.raw.contains("atomic") {
                            return Err(format!(
                                "switch CFG: volatile/atomic memory: {}",
                                inst.raw
                            ));
                        }
                        let access_ty = operand_type(&inst.operands[0])
                            .ok_or(format!("switch CFG: load type: {}", inst.raw))?;
                        let loc = self.resolve_loc(&inst.operands[1], &access_ty)?;
                        let value = match memory.get(&loc) {
                            Some(value) => *value,
                            None => self.initial_load(&loc, &seed)?,
                        };
                        if let Some(result) = &inst.result {
                            values.insert(result.clone(), value);
                        }
                    }
                    "store" => {
                        let access_ty = operand_type(&inst.operands[0])
                            .ok_or(format!("switch CFG: store type: {}", inst.raw))?;
                        let value = get_node_id(
                            &strip_type(&inst.operands[0]),
                            &values,
                            &self.ir.params,
                            self.builder,
                            Some(access_ty.clone()),
                        )
                        .ok_or(format!("switch CFG: unresolved store value: {}", inst.raw))?;
                        let loc = self.resolve_loc(&inst.operands[1], &access_ty)?;
                        memory.insert(loc, value);
                    }
                    "getelementptr" => {
                        self.record_gep(inst)?;
                    }
                    "br" | "switch" | "ret" | "unreachable" => {
                        terminator = Some(inst.clone());
                        break;
                    }
                    _ => {
                        emit_instruction(
                            inst,
                            self.builder,
                            &mut values,
                            &self.ir.params,
                            self.span,
                        )?;
                    }
                }
            }
            let terminator =
                terminator.ok_or_else(|| "switch CFG: block without terminator".to_string())?;
            match terminator.opcode.as_str() {
                "ret" => {
                    let is_void = terminator
                        .operands
                        .first()
                        .map(|operand| strip_type(operand) == "void")
                        .unwrap_or(true);
                    let ret = if is_void {
                        None
                    } else {
                        Some(
                            get_node_id(
                                &strip_type(&terminator.operands[0]),
                                &values,
                                &self.ir.params,
                                self.builder,
                                None,
                            )
                            .ok_or_else(|| {
                                format!("switch CFG: unresolved return: {}", terminator.raw)
                            })?,
                        )
                    };
                    self.exits.push(ExitState {
                        reach,
                        ret,
                        memory: memory.clone(),
                    });
                }
                "unreachable" => {}
                "br" => {
                    let (true_label, false_cond) = if terminator.operands.len() == 1 {
                        (
                            terminator.operands[0]
                                .trim_start_matches("label ")
                                .trim_start_matches('%')
                                .to_string(),
                            None,
                        )
                    } else if terminator.operands.len() >= 3 {
                        let condition = get_node_id(
                            &strip_type(&terminator.operands[0]),
                            &values,
                            &self.ir.params,
                            self.builder,
                            None,
                        )
                        .ok_or_else(|| {
                            format!("switch CFG: unresolved branch: {}", terminator.raw)
                        })?;
                        let true_label = terminator.operands[1]
                            .trim_start_matches("label ")
                            .trim_start_matches('%')
                            .to_string();
                        let false_label = terminator.operands[2]
                            .trim_start_matches("label ")
                            .trim_start_matches('%')
                            .to_string();
                        let negated = self.cond_not(condition)?;
                        let false_cond = self.cond_and(reach, negated)?;
                        if let Some(succ) = self.ir.block_map.get(&false_label).copied() {
                            if false_cond != self.false_cond {
                                incoming[succ].push((idx, false_cond));
                            }
                        }
                        (true_label, Some(condition))
                    } else {
                        return Err(format!("switch CFG: malformed br: {}", terminator.raw));
                    };
                    let true_cond = match false_cond {
                        None => reach,
                        Some(condition) => self.cond_and(reach, condition)?,
                    };
                    if true_cond != self.false_cond {
                        if let Some(succ) = self.ir.block_map.get(&true_label).copied() {
                            incoming[succ].push((idx, true_cond));
                        }
                    }
                }
                "switch" => {
                    let raw = &terminator.raw;
                    let open = raw
                        .find('[')
                        .ok_or_else(|| format!("switch CFG: malformed switch: {}", raw))?;
                    let close = raw
                        .rfind(']')
                        .ok_or_else(|| format!("switch CFG: malformed switch: {}", raw))?;
                    let head = &raw[..open];
                    let body = &raw[open + 1..close];
                    let mut head_parts = head.split(',').map(str::trim);
                    let value_part = head_parts.next().unwrap_or("");
                    let value_part = value_part.trim_start_matches("switch").trim();
                    let value_ty = operand_type(value_part)
                        .ok_or_else(|| format!("switch CFG: switch type: {}", raw))?;
                    let value = get_node_id(
                        &strip_type(value_part),
                        &values,
                        &self.ir.params,
                        self.builder,
                        Some(value_ty.clone()),
                    )
                    .ok_or_else(|| format!("switch CFG: unresolved switch value: {}", raw))?;
                    let default = head_parts
                        .next()
                        .ok_or_else(|| format!("switch CFG: missing default: {}", raw))?
                        .trim_start_matches("label ")
                        .trim_start_matches('%')
                        .to_string();
                    let tokens: Vec<&str> = body.split_whitespace().collect();
                    let mut cases: Vec<(i64, String)> = Vec::new();
                    let mut i = 0;
                    while i + 3 < tokens.len() {
                        if tokens[i + 2] == "label" {
                            let constant = tokens[i + 1]
                                .trim_end_matches(',')
                                .parse::<i64>()
                                .map_err(|_| {
                                    format!("switch CFG: non-constant case: {}", raw)
                                })?;
                            let label = tokens[i + 3].trim_start_matches('%').to_string();
                            cases.push((constant, label));
                            i += 4;
                        } else {
                            i += 1;
                        }
                    }
                    let mut default_guard = self.true_cond;
                    for (constant, label) in &cases {
                        let constant_node = self.builder.constant(
                            int_constant_data(&value_ty, *constant as u64),
                            value_ty.clone(),
                            self.span,
                        );
                        let equal = self
                            .builder
                            .eq(value, constant_node, self.span)
                            .map_err(|e| format!("switch CFG: {:?}", e))?;
                        let case_cond = self.cond_and(reach, equal)?;
                        if case_cond != self.false_cond {
                            if let Some(succ) = self.ir.block_map.get(label).copied() {
                                incoming[succ].push((idx, case_cond));
                            }
                        }
                        let not_equal = self
                            .builder
                            .ne(value, constant_node, self.span)
                            .map_err(|e| format!("switch CFG: {:?}", e))?;
                        default_guard = self.cond_and(default_guard, not_equal)?;
                    }
                    let default_cond = self.cond_and(reach, default_guard)?;
                    if default_cond != self.false_cond {
                        if let Some(succ) = self.ir.block_map.get(&default).copied() {
                            incoming[succ].push((idx, default_cond));
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "switch CFG: unsupported terminator: {}",
                        terminator.raw
                    ))
                }
            }
            preds[idx] = Some(BlockState {
                values,
                memory,
            });
        }
        Ok(())
    }

    fn finish_blocks(mut self, ret_ty: &Type, seed: &HashMap<String, NodeId>) -> Result<(), String> {
        let mut result: Option<NodeId> = None;
        let mut locations: HashSet<MemLoc> = HashSet::new();
        for exit in &self.exits {
            locations.extend(exit.memory.keys().cloned());
        }
        let mut finals: HashMap<MemLoc, NodeId> = HashMap::new();
        for loc in locations {
            let initial = self.initial_load(&loc, seed)?;
            let mut value = initial;
            for exit in &self.exits {
                let branch = exit
                    .memory
                    .get(&loc)
                    .copied()
                    .unwrap_or(initial);
                value = self
                    .builder
                    .select(exit.reach, branch, value, self.span)
                    .map_err(|e| format!("switch CFG: {:?}", e))?;
            }
            finals.insert(loc, value);
        }
        for exit in &self.exits {
            if let Some(ret) = exit.ret {
                result = Some(match result {
                    None => ret,
                    Some(previous) => self
                        .builder
                        .select(exit.reach, ret, previous, self.span)
                        .map_err(|e| format!("switch CFG: {:?}", e))?,
                });
            }
        }
        for (loc, value) in finals {
            let global = seed
                .get(&loc.global)
                .copied()
                .ok_or(format!("switch CFG: unknown global @{}", loc.global))?;
            let index = self
                .builder
                .constant(ConstantData::u64(loc.index), Type::u64(), self.span);
            let access = self
                .builder
                .array_access(global, index, loc.ty.clone(), self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
            self.builder
                .store(access, value, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        }
        if ret_ty == &Type::Unit {
            let unit = self
                .builder
                .constant(ConstantData::Unit, Type::Unit, self.span);
            self.builder
                .return_value(unit, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        } else {
            let result =
                result.ok_or_else(|| "switch CFG: no returning path".to_string())?;
            self.builder
                .return_value(result, self.span)
                .map_err(|e| format!("switch CFG: {:?}", e))?;
        }
        Ok(())
    }
}

fn terminator_targets(inst: &Instruction) -> Vec<String> {
    match inst.opcode.as_str() {
        "br" => inst
            .operands
            .iter()
            .filter_map(|operand| operand.trim().strip_prefix("label "))
            .map(|label| label.trim_start_matches('%').to_string())
            .collect(),
        "switch" => {
            let raw = &inst.raw;
            let Some(open) = raw.find('[') else {
                return Vec::new();
            };
            let Some(close) = raw.rfind(']') else {
                return Vec::new();
            };
            let head = &raw[..open];
            let body = &raw[open + 1..close];
            let mut labels = Vec::new();
            if let Some(default) = head.split(',').nth(1) {
                labels.push(
                    default
                        .trim()
                        .trim_start_matches("label ")
                        .trim_start_matches('%')
                        .to_string(),
                );
            }
            let tokens: Vec<&str> = body.split_whitespace().collect();
            let mut i = 0;
            while i + 3 < tokens.len() {
                if tokens[i + 2] == "label" {
                    labels.push(tokens[i + 3].trim_start_matches('%').to_string());
                    i += 4;
                } else {
                    i += 1;
                }
            }
            labels
        }
        _ => Vec::new(),
    }
}

fn successor_indices(ir: &IrFunction, idx: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for inst in &ir.blocks[idx].instructions {
        if matches!(inst.opcode.as_str(), "br" | "switch") {
            for label in terminator_targets(inst) {
                if let Some(succ) = ir.block_map.get(&label).copied() {
                    out.push(succ);
                }
            }
            break;
        }
    }
    out
}

fn dag_order(ir: &IrFunction) -> Vec<usize> {
    let count = ir.blocks.len();
    let mut indegree = vec![0usize; count];
    for idx in 0..count {
        for succ in successor_indices(ir, idx) {
            indegree[succ] += 1;
        }
    }
    let mut order = Vec::with_capacity(count);
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for succ in successor_indices(ir, idx) {
            indegree[succ] = indegree[succ].saturating_sub(1);
            if indegree[succ] == 0 {
                queue.push_back(succ);
            }
        }
    }
    order
}

fn acyclic_path_count(ir: &IrFunction) -> usize {
    let mut counts = vec![0usize; ir.blocks.len()];
    counts[0] = 1;
    for idx in dag_order(ir) {
        if idx >= counts.len() {
            continue;
        }
        let incoming = counts[idx];
        for succ in successor_indices(ir, idx) {
            counts[succ] = (counts[succ] + incoming).min(4096);
        }
    }
    counts.into_iter().max().unwrap_or(0)
}

fn block_value_names(instructions: &[Instruction]) -> HashSet<String> {
    let mut out = HashSet::new();
    for inst in instructions {
        if inst.opcode == "phi" {
            for (value, _) in phi_incomings(inst) {
                let value = strip_type(&value);
                if value.starts_with('%') {
                    out.insert(value);
                }
            }
        }
        for operand in &inst.operands {
            let value = strip_type(&operand_value_token(operand));
            if value.starts_with('%') {
                out.insert(value);
            }
        }
    }
    out
}

fn lower_switch_cfg(
    ir: &IrFunction,
    builder: &mut Builder,
    entry_values: HashMap<String, NodeId>,
    ret_ty: Type,
) -> Result<(), String> {
    let span = Span::unknown();
    let true_cond = builder.constant(ConstantData::Bool(true), Type::Bool, span);
    let false_cond = builder.constant(ConstantData::Bool(false), Type::Bool, span);
    let mut lowerer = SwitchLowerer {
        ir,
        builder,
        span,
        true_cond,
        false_cond,
        gep_locs: HashMap::new(),
        global_elem: HashMap::new(),
        initial_loads: HashMap::new(),
        paths: Vec::new(),
        exits: Vec::new(),
    };
    if acyclic_path_count(ir) > 512 {
        lowerer.run_blocks(entry_values.clone())?;
        lowerer.finish_blocks(&ret_ty, &entry_values)
    } else {
        lowerer.run(entry_values.clone())?;
        lowerer.finish(&ret_ty, &entry_values)
    }
}

/// True when an acyclic CFG has branch-guarded global memory: the
/// straight-line block walker would execute guarded stores
/// unconditionally, so these functions use the predicated engine.
fn acyclic_global_memory_cfg(ir: &IrFunction) -> bool {
    let mut global_geps: HashSet<String> = HashSet::new();
    let mut has_memory = false;
    let mut has_control = false;
    for block in &ir.blocks {
        for inst in &block.instructions {
            match inst.opcode.as_str() {
                "getelementptr" if inst.operands.len() >= 2 => {
                    let base = strip_type(&inst.operands[1]);
                    if base.starts_with('@') || global_geps.contains(&base) {
                        if let Some(result) = &inst.result {
                            global_geps.insert(result.clone());
                        }
                    }
                }
                "load" | "store" => {
                    has_memory = true;
                    let Some(ptr) = inst.operands.get(1) else {
                        return false;
                    };
                    if inline_global_gep(ptr).is_none() {
                        let base = strip_type(ptr);
                        if !(base.starts_with('@') || global_geps.contains(&base)) {
                            return false;
                        }
                    }
                }
                "switch" => has_control = true,
                "br" if inst.operands.len() >= 3 => has_control = true,
                _ => {}
            }
        }
    }
    has_memory && has_control
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
                let is_void = inst
                    .operands
                    .first()
                    .map(|operand| strip_type(operand) == "void")
                    .unwrap_or(true);
                if !is_void {
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

/// Extract an intrinsic call's argument list from the parser's operand join.
///
/// Clang prefixes calls with attributes such as `range(i32 0, 33)`, so the
/// first `(` in the joined text is not necessarily the callee's. Locate the
/// `llvm.` callee first, then its argument list.
fn intrinsic_arguments(raw: &str) -> Option<&str> {
    let callee_at = raw.find("llvm.")?;
    let open = raw[callee_at..].find('(')? + callee_at;
    Some(raw[open + 1..].trim_end_matches(')'))
}

/// Top-level call arguments with their declared types/attributes.
fn call_arguments(inst: &Instruction) -> Option<String> {
    let raw = inst.operands.join(", ");
    let open = raw.find('(')?;
    let close = raw.rfind(')')?;
    Some(raw[open + 1..close].to_string())
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
            let op_type = operand_type(&inst.operands[0]);
            let lhs_str = strip_type(&inst.operands[0]);
            let rhs_str = strip_type(&inst.operands[1]);
            let lhs = get_node_id(&lhs_str, value_map, params, builder, op_type.clone())
                .ok_or(format!("cannot resolve lhs '{}' in {}", lhs_str, inst.raw))?;
            let rhs = get_node_id(&rhs_str, value_map, params, builder, op_type.clone())
                .ok_or(format!("cannot resolve rhs '{}' in {}", rhs_str, inst.raw))?;

            let is_bool = matches!(op_type, Some(Type::Bool));
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
                "sdiv" | "srem" => {
                    let ty = builder
                        .function()
                        .get_node(lhs)
                        .map(|node| node.ty.clone())
                        .ok_or("signed division operand missing")?;
                    let name = if inst.opcode == "sdiv" {
                        "sir_sdiv"
                    } else {
                        "sir_srem"
                    };
                    builder
                        .intrinsic(name, &[lhs, rhs], ty, Effects::empty(), span)
                        .map_err(|e| format!("{:?}", e))?
                }
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
                "lshr" => builder
                    .shr(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "ashr" => builder
                    .intrinsic(
                        "sir_ashr",
                        &[lhs, rhs],
                        op_type.unwrap_or(Type::u64()),
                        Effects::empty(),
                        span,
                    )
                    .map_err(|e| format!("{:?}", e))?,
                "udiv" => builder
                    .div(lhs, rhs, span)
                    .map_err(|e| format!("{:?}", e))?,
                "urem" => builder
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
            let node_id = if matches!(cmp_type, "sgt" | "sge" | "slt" | "sle") {
                builder
                    .intrinsic(
                        format!("sir_{cmp_type}"),
                        &[lhs, rhs],
                        Type::Bool,
                        Effects::empty(),
                        span,
                    )
                    .map_err(|e| format!("{:?}", e))?
            } else {
                match cmp_type {
                "eq" => builder.eq(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ne" => builder.ne(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ugt" => builder.gt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "uge" => builder.ge(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ult" => builder.lt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ule" => builder.le(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                _ => return Err(format!("unsupported icmp type: {}", cmp_type)),
                }
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
            let ptr = if let Some((name, element, index)) =
                inline_global_gep(&inst.operands[1])
            {
                let base = resolve_global(&name, value_map, builder, span);
                let index_node =
                    builder.constant(ConstantData::u64(index), Type::u64(), span);
                builder
                    .array_access(base, index_node, element, span)
                    .map_err(|e| format!("{:?}", e))?
            } else {
                let ptr_str = strip_type(&inst.operands[1]);
                get_node_id(&ptr_str, value_map, params, builder, None)
                    .ok_or(format!("cannot resolve load ptr '{}'", ptr_str))?
            };
            let loaded_ty = operand_type(&inst.operands[0])
                .unwrap_or_else(|| builder.function().get_node(ptr).map(|n| n.ty.clone()).unwrap_or(Type::u8()));
            let ptr_kind = builder
                .function()
                .get_node(ptr)
                .map(|node| node.kind.clone())
                .ok_or("load pointer node missing")?;
            // Create a distinct read node. A GEP result may also be a
            // store destination; reusing it as the loaded value would tie
            // the two roles together (and break lvalue emission).
            let read = match ptr_kind {
                NodeKind::ArrayAccess { base, index } => builder
                    .array_access(base, index, loaded_ty, span)
                    .map_err(|e| format!("{:?}", e))?,
                _ if builder
                    .function()
                    .get_node(ptr)
                    .is_some_and(|node| node.ty.is_pointer_like()) =>
                {
                    let index_ty = Type::u64();
                    let zero = builder.constant(ConstantData::u64(0), index_ty, span);
                    builder
                        .array_access(ptr, zero, loaded_ty, span)
                        .map_err(|e| format!("{:?}", e))?
                }
                _ => ptr,
            };
            if is_volatile {
                builder.add_effects(read, Effects::VOLATILE);
            }
            if let Some(name) = result_name {
                value_map.insert(name, read);
            }
            Ok(Some(read))
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
            let elem_ty = if elem_type_str.starts_with('[') {
                match gep_source_array_type(&inst.raw) {
                    Some(Type::Array { element, .. }) => *element,
                    other => other.unwrap_or_else(Type::u8),
                }
            } else {
                parse_type(elem_type_str).unwrap_or(Type::u8())
            };

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

            let base_is_pointer = builder
                .function()
                .get_node(base)
                .is_some_and(|node| node.ty.is_pointer_like());
            let escaping = inst
                .result
                .as_ref()
                .is_some_and(|name| builder.gep_escapes(name));
            let node_id = if inst.operands.len() == 3 && base_is_pointer && escaping {
                builder
                    .pointer_offset(base, index, elem_ty, span)
                    .map_err(|e| format!("{:?}", e))?
            } else {
                builder
                    .array_access(base, index, elem_ty, span)
                    .map_err(|e| format!("{:?}", e))?
            };

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "store" => {
            if inst.operands.len() < 2 {
                return Err(format!("store needs value + ptr: {}", inst.raw));
            }
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
            let val_str = strip_type(&inst.operands[0]);
            let val = get_node_id(&val_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve store value '{}'", val_str))?;
            let ptr = if let Some((name, element, index)) =
                inline_global_gep(&inst.operands[1])
            {
                let base = resolve_global(&name, value_map, builder, span);
                let index_node =
                    builder.constant(ConstantData::u64(index), Type::u64(), span);
                builder
                    .array_access(base, index_node, element, span)
                    .map_err(|e| format!("{:?}", e))?
            } else {
                let ptr_str = strip_type(&inst.operands[1]);
                get_node_id(&ptr_str, value_map, params, builder, None)
                    .ok_or(format!("cannot resolve store ptr '{}'", ptr_str))?
            };
            let pointer_like = builder
                .function()
                .get_node(ptr)
                .is_some_and(|node| node.ty.is_pointer_like());
            let array_lvalue = builder
                .function()
                .get_node(ptr)
                .is_some_and(|node| matches!(node.kind, NodeKind::ArrayAccess { .. }));
            if !(pointer_like || array_lvalue) {
                return Err(format!(
                    "unsupported: store destination is neither a pointer nor an array element: {}",
                    inst.raw
                ));
            }
            let (target, stored) = if array_lvalue {
                let value_ty = builder
                    .function()
                    .get_node(val)
                    .map(|node| node.ty.clone())
                    .ok_or("store value node missing")?;
                let element_ty = builder
                    .function()
                    .get_node(ptr)
                    .map(|node| node.ty.clone())
                    .ok_or("store destination node missing")?;
                if element_ty != value_ty {
                    // A GEP in source-element units (often i8) feeding a
                    // wider store: reinterpret at the same address.
                    let NodeKind::ArrayAccess { base, index } = builder
                        .function()
                        .get_node(ptr)
                        .map(|node| node.kind.clone())
                        .ok_or("store destination node missing")?
                    else {
                        return Err(format!("store destination missing: {}", inst.raw));
                    };
                    let access = builder
                        .array_access(base, index, value_ty, span)
                        .map_err(|e| format!("{:?}", e))?;
                    (access, val)
                } else {
                    (ptr, val)
                }
            } else {
                let value_ty = builder
                    .function()
                    .get_node(val)
                    .map(|node| node.ty.clone())
                    .ok_or("store value node missing")?;
                let zero = builder.constant(ConstantData::u64(0), Type::u64(), span);
                let access = builder
                    .array_access(ptr, zero, value_ty, span)
                    .map_err(|e| format!("{:?}", e))?;
                (access, val)
            };
            let node_id = builder
                .store(target, stored, span)
                .map_err(|e| format!("{:?}", e))?;
            // Return the Unit-typed Store so loop builders include it in
            // the body; it has no SSA result, so callers must not map it.
            Ok(Some(node_id))
        }

        "br" | "ret" | "phi" => {
            // Handled by the loop/exit block logic
            Ok(None)
        }

        "call" => {
            // Handle a few known intrinsics
            // tail call i8 @llvm.umax.i8(i8 %10, i8 %8)
            let callee = inst.operands.first().map(|s| s.trim()).unwrap_or("");
            // Compile-time hints with no runtime semantics. Dropping them
            // is behaviour-preserving; keeping them would block otherwise
            // ordinary functions.
            if callee.contains("llvm.lifetime.")
                || callee.contains("llvm.dbg.")
                || callee.contains("llvm.assume")
                || callee.contains("llvm.experimental.noalias.scope.decl")
            {
                return Ok(None);
            }
            if callee.contains("llvm.fshl") || callee.contains("llvm.fshr") {
                // Funnel shifts with a constant amount are clang's lowering of
                // scalar rotates: fshl(a,b,k) = (a << k) | (b >> (w-k)) and
                // fshr(a,b,k) = (a >> k) | (b << (w-k)). Lower them to the
                // shift/or pattern the bit-permutation recognizer expects.
                let raw = inst.operands.join(", ");
                let callee_name = raw
                    .split('@')
                    .nth(1)
                    .and_then(|rest| rest.split('(').next())
                    .unwrap_or(callee);
                let width = callee_name
                    .rsplit('.')
                    .next()
                    .and_then(|suffix| suffix.strip_prefix('i'))
                    .and_then(|digits| digits.parse::<u32>().ok())
                    .ok_or_else(|| format!("unsupported funnel width: {}", inst.raw))?;
                let inside = intrinsic_arguments(&raw)
                    .ok_or_else(|| format!("malformed funnel call: {}", inst.raw))?;
                let parts: Vec<&str> = inside.split(',').map(str::trim).collect();
                if parts.len() < 3 {
                    return Err(format!("malformed funnel operands: {}", inst.raw));
                }
                let ty = parts[0]
                    .split_whitespace()
                    .next()
                    .and_then(parse_type)
                    .ok_or_else(|| format!("unsupported funnel type: {}", inst.raw))?;
                let a_str = strip_type(parts[0]);
                let b_str = strip_type(parts[1]);
                let amount_str = strip_type(parts[2]);
                let a = get_node_id(&a_str, value_map, params, builder, Some(ty.clone()))
                    .ok_or(format!("cannot resolve funnel operand '{}'", a_str))?;
                let b = get_node_id(&b_str, value_map, params, builder, Some(ty.clone()))
                    .ok_or(format!("cannot resolve funnel operand '{}'", b_str))?;
                let amount = parse_int_constant(&amount_str)
                    .ok_or_else(|| format!("funnel amount must be constant: {}", inst.raw))?;
                if amount <= 0 || amount >= width as i64 {
                    return Err(format!("funnel amount out of range: {}", inst.raw));
                }
                let k = builder.constant(int_constant_data(&ty, amount as u64), ty.clone(), span);
                // `fshl(x, x, k)` / `fshr(x, x, k)` are scalar rotates;
                // keep the direct SIR op instead of a shift/or chain.
                if a == b {
                    let node_id = if callee.contains("llvm.fshl") {
                        builder.rol(a, k, span)
                    } else {
                        builder.ror(a, k, span)
                    }
                    .map_err(|e| format!("{:?}", e))?;
                    if let Some(name) = result_name {
                        value_map.insert(name, node_id);
                    }
                    return Ok(Some(node_id));
                }
                // The bit-permutation recognizer matches clang's canonical
                // `(x << k) | (x >> (w - k))` shape, where the second amount
                // is a `Sub(width_constant, k)` node rather than a folded
                // constant. Materialize the subtraction.
                let width_node =
                    builder.constant(int_constant_data(&ty, width as u64), ty.clone(), span);
                let w_minus_k = builder
                    .sub(width_node, k, span)
                    .map_err(|e| format!("{:?}", e))?;
                let (first, second) = if callee.contains("llvm.fshl") {
                    (
                        builder.shl(a, k, span).map_err(|e| format!("{:?}", e))?,
                        builder
                            .shr(b, w_minus_k, span)
                            .map_err(|e| format!("{:?}", e))?,
                    )
                } else {
                    (
                        builder.shr(a, k, span).map_err(|e| format!("{:?}", e))?,
                        builder
                            .shl(b, w_minus_k, span)
                            .map_err(|e| format!("{:?}", e))?,
                    )
                };
                let node_id = builder
                    .bit_or(first, second, span)
                    .map_err(|e| format!("{:?}", e))?;
                if let Some(name) = result_name {
                    value_map.insert(name, node_id);
                }
                return Ok(Some(node_id));
            }
            if callee.contains("llvm.bswap") || callee.contains("llvm.bitreverse") {
                // Clang emits a single intrinsic for byte swap / bit
                // reverse. Keep it as a direct SIR intrinsic: the emitter
                // selects `__builtin_bswap*` / the rbit helper, which is
                // strictly cheaper than expanding to a shift/mask chain.
                let raw = inst.operands.join(", ");
                let inside = intrinsic_arguments(&raw)
                    .ok_or_else(|| format!("malformed permutation call: {}", inst.raw))?;
                let operand = inside.split(',').next().map(str::trim).unwrap_or("");
                let ty = operand
                    .split_whitespace()
                    .next()
                    .and_then(parse_type)
                    .ok_or_else(|| format!("unsupported permutation type: {}", inst.raw))?;
                let x_str = strip_type(operand);
                let x = get_node_id(&x_str, value_map, params, builder, Some(ty.clone()))
                    .ok_or(format!("cannot resolve permutation operand '{}'", x_str))?;
                let name = if callee.contains("llvm.bswap") {
                    "bswap"
                } else {
                    "rbit"
                };
                let current = builder
                    .intrinsic(name, &[x], ty, Effects::empty(), span)
                    .map_err(|e| format!("{:?}", e))?;
                if let Some(name) = result_name {
                    value_map.insert(name, current);
                }
                return Ok(Some(current));
            }
            if callee.contains("llvm.cttz")
                || callee.contains("llvm.ctlz")
                || callee.contains("llvm.ctpop")
            {
                // Scalar population count / bit scans. clang's cttz/ctlz
                // carry a zero-undef flag; only the well-defined `false` form
                // (zero returns the width) maps onto the SIR conventions, so
                // the poison form is refused.
                let raw = inst.operands.join(", ");
                let callee_name = raw
                    .split('@')
                    .nth(1)
                    .and_then(|rest| rest.split('(').next())
                    .unwrap_or(callee);
                let _width = callee_name
                    .rsplit('.')
                    .next()
                    .and_then(|suffix| suffix.strip_prefix('i'))
                    .and_then(|digits| digits.parse::<u32>().ok())
                    .ok_or_else(|| format!("unsupported bit-count width: {}", inst.raw))?;
                let inside = intrinsic_arguments(&raw)
                    .ok_or_else(|| format!("malformed bit-count call: {}", inst.raw))?;
                let parts: Vec<&str> = inside.split(',').map(str::trim).collect();
                if parts.is_empty() {
                    return Err(format!("malformed bit-count operands: {}", inst.raw));
                }
                if (callee.contains("llvm.cttz") || callee.contains("llvm.ctlz"))
                    && !parts
                        .get(1)
                        .map(|flag| flag.trim().ends_with("false"))
                        .unwrap_or(false)
                {
                    return Err(format!(
                        "zero-undef bit-count form is poison and not modeled: {}",
                        inst.raw
                    ));
                }
                let ty = parts[0]
                    .split_whitespace()
                    .next()
                    .and_then(parse_type)
                    .ok_or_else(|| format!("unsupported bit-count type: {}", inst.raw))?;
                let operand_str = strip_type(parts[0]);
                let operand =
                    get_node_id(&operand_str, value_map, params, builder, Some(ty.clone()))
                        .ok_or(format!("cannot resolve bit-count operand '{operand_str}'"))?;
                let counted = if callee.contains("llvm.cttz") {
                    builder.trailing_zeros(operand, span)
                } else if callee.contains("llvm.ctlz") {
                    builder.leading_zeros(operand, span)
                } else {
                    builder.popcount(operand, span)
                }
                .map_err(|e| format!("{:?}", e))?;
                // The builder produces i32 counts; widen to the IR result
                // width so downstream recognizers see the expected type.
                let needs_widen = builder
                    .function()
                    .get_node(counted)
                    .map(|node| node.ty != ty)
                    .unwrap_or(true);
                let counted = if needs_widen {
                    builder
                        .convert(
                            counted,
                            ty.clone(),
                            sir_nodes::ConvertKind::ZeroExtend,
                            span,
                        )
                        .map_err(|e| format!("{:?}", e))?
                } else {
                    counted
                };
                if let Some(name) = result_name {
                    value_map.insert(name, counted);
                }
                return Ok(Some(counted));
            }
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

                        let signed =
                            callee.contains("llvm.smax") || callee.contains("llvm.smin");
                        let max = callee.contains("llvm.umax") || callee.contains("llvm.smax");
                        let cmp = if signed {
                            let op = if max { "sir_sgt" } else { "sir_slt" };
                            builder
                                .intrinsic(op, &[a, b], Type::Bool, Effects::empty(), span)
                                .map_err(|e| format!("{:?}", e))?
                        } else if max {
                            builder.gt(a, b, span).map_err(|e| format!("{:?}", e))?
                        } else {
                            builder.lt(a, b, span).map_err(|e| format!("{:?}", e))?
                        };
                        let node_id = builder
                            .select(cmp, a, b, span)
                            .map_err(|e| format!("{:?}", e))?;

                        if let Some(name) = result_name {
                            value_map.insert(name, node_id);
                        }
                        return Ok(Some(node_id));
                    }
                }
            }
            if callee.contains("llvm.memset") || callee.contains("llvm.memcpy") {
                let inside = call_arguments(inst)
                    .ok_or_else(|| format!("malformed memset/memcpy call: {}", inst.raw))?;
                let parts: Vec<String> = split_operands(&inside);
                if parts.len() < 3 {
                    return Err(format!("memset/memcpy needs 3 operands: {}", inst.raw));
                }
                let is_memset = callee.contains("llvm.memset");
                let volatile = parts
                    .get(3)
                    .map(|flag| strip_type(flag) == "true")
                    .unwrap_or(false);
                if volatile {
                    return Err(format!(
                        "unsupported: volatile memset/memcpy (ordering semantics not modeled): {}",
                        inst.raw
                    ));
                }
                let dst = resolve_pointer_operand(&parts[0], value_map, params, builder, span)
                    .ok_or_else(|| format!("cannot resolve memset/memcpy dst: {}", inst.raw))?;
                let length_str = strip_type(&parts[2]);
                let length = get_node_id(
                    &length_str,
                    value_map,
                    params,
                    builder,
                    Some(Type::u64()),
                )
                .ok_or(format!("cannot resolve memset/memcpy length '{}'", length_str))?;
                let node_id = if is_memset {
                    let value_str = strip_type(&parts[1]);
                    let value = get_node_id(
                        &value_str,
                        value_map,
                        params,
                        builder,
                        Some(Type::u8()),
                    )
                    .ok_or(format!("cannot resolve memset value '{}'", value_str))?;
                    builder
                        .intrinsic(
                            "sir_memset",
                            &[dst, value, length],
                            Type::Unit,
                            Effects::WRITE_MEMORY,
                            span,
                        )
                        .map_err(|e| format!("{:?}", e))?
                } else {
                    let src = resolve_pointer_operand(&parts[1], value_map, params, builder, span)
                        .ok_or_else(|| {
                            format!("cannot resolve memcpy src: {}", inst.raw)
                        })?;
                    builder
                        .intrinsic(
                            "sir_memcpy",
                            &[dst, src, length],
                            Type::Unit,
                            Effects::READ_MEMORY | Effects::WRITE_MEMORY,
                            span,
                        )
                        .map_err(|e| format!("{:?}", e))?
                };
                return Ok(Some(node_id));
            }
            if callee.contains("@bcmp") || callee.contains("@memcmp") {
                let inside = call_arguments(inst)
                    .ok_or_else(|| format!("malformed bcmp/memcmp call: {}", inst.raw))?;
                let parts: Vec<String> = split_operands(&inside);
                if parts.len() < 3 {
                    return Err(format!("bcmp/memcmp needs 3 operands: {}", inst.raw));
                }
                let a = resolve_pointer_operand(&parts[0], value_map, params, builder, span)
                    .ok_or_else(|| format!("cannot resolve bcmp lhs: {}", inst.raw))?;
                let b = resolve_pointer_operand(&parts[1], value_map, params, builder, span)
                    .ok_or_else(|| format!("cannot resolve bcmp rhs: {}", inst.raw))?;
                let length_str = strip_type(&parts[2]);
                let length = get_node_id(
                    &length_str,
                    value_map,
                    params,
                    builder,
                    Some(Type::u64()),
                )
                .ok_or(format!("cannot resolve bcmp length '{}'", length_str))?;
                let node_id = builder
                    .intrinsic(
                        "sir_bcmp",
                        &[a, b, length],
                        Type::u32(),
                        Effects::READ_MEMORY,
                        span,
                    )
                    .map_err(|e| format!("{:?}", e))?;
                if let Some(name) = result_name {
                    value_map.insert(name, node_id);
                }
                return Ok(Some(node_id));
            }
            Err(format!("unsupported call: {}", inst.raw))
        }

        _ => Err(format!(
            "unsupported instruction: {} (raw: {})",
            inst.opcode, inst.raw
        )),
    }
}
