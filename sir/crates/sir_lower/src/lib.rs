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

use std::collections::HashMap;

use sir_builder::Builder;
use sir_nodes::{Function, NodeKind};
use sir_types::{ConstantData, Effects, NodeId, Span, Type, IntegerWidth};

/// A single LLVM IR instruction.
#[derive(Clone, Debug)]
struct Instruction {
    result: Option<String>,   // %N or %name
    opcode: String,            // add, load, icmp, phi, br, ret, etc.
    operands: Vec<String>,     // raw operand strings
    raw: String,               // full line for debugging
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
    params: Vec<(String, String)>,  // (name, type)
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
        "i128" => Some(Type::Integer { width: IntegerWidth::I128, signed: false, overflow: sir_types::OverflowBehavior::Wrapping }),
        "ptr" | "ptr noundef" | "ptr nocapture" => Some(Type::Pointer { pointee: Box::new(Type::u8()), mutable: false }),
        _ => None,
    }
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
        ret_type = before_at.split_whitespace().last().unwrap_or("void").to_string();
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
                    if param.is_empty() { continue; }
                    // e.g. "ptr nocapture noundef readonly %0" or "i64 noundef %1"
                    // The param name is the last %N token
                    let parts: Vec<&str> = param.split_whitespace().collect();
                    let name = parts.iter().rev().find(|p| p.starts_with('%'))
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
        if line.is_empty() || line.starts_with(';') || line.starts_with('!') || line.starts_with("attributes") {
            i += 1;
            continue;
        }

        // Block label (either "N:" or "entry:", possibly followed by a comment)
        // LLVM labels look like: "7:" or "7:   ; preds = %6, %2"
        if !line.starts_with('%') && !line.starts_with("define") {
            // Check if the line starts with a label: "<label>:" possibly followed by comments
            let before_comment = line.split(';').next().unwrap_or(line).trim();
            if before_comment.ends_with(':') && !before_comment.contains(' ') && !before_comment.is_empty() {
                let label = before_comment.trim_end_matches(':').to_string();
                if let Some(mut b) = current_block.take() {
                    blocks.push(b);
                }
                current_block = Some(Block { label: label.clone(), instructions: Vec::new(), preds: Vec::new() });
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
            if next.is_empty() || next.starts_with(';') { break; }
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
                        pred_map.entry(true_label.to_string()).or_default().push(String::new()); // will fix later
                        pred_map.entry(false_label.to_string()).or_default().push(String::new());
                    } else if inst.operands.len() == 1 {
                        // unconditional br: br label %target
                        let target = inst.operands[0].trim_start_matches("label ");
                        pred_map.entry(target.to_string()).or_default().push(String::new());
                    }
                }
                _ => {}
            }

            if let Some(ref mut b) = current_block {
                b.instructions.push(inst);
            } else {
                // No block yet — create an implicit entry block
                current_block = Some(Block { label: "entry".to_string(), instructions: vec![], preds: vec![] });
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
            Type::Integer { width: IntegerWidth::I8, signed: false, .. } => ConstantData::u8(val as u8),
            Type::Integer { width: IntegerWidth::I8, signed: true, .. } => ConstantData::i8(val as i8),
            Type::Integer { width: IntegerWidth::I16, signed: false, .. } => ConstantData::u16(val as u16),
            Type::Integer { width: IntegerWidth::I16, signed: true, .. } => ConstantData::i16(val as i16),
            Type::Integer { width: IntegerWidth::I32, signed: false, .. } => ConstantData::u32(val as u32),
            Type::Integer { width: IntegerWidth::I32, signed: true, .. } => ConstantData::i32(val as i32),
            Type::Integer { width: IntegerWidth::I64, signed: false, .. } => ConstantData::u64(val as u64),
            Type::Integer { width: IntegerWidth::I64, signed: true, .. } => ConstantData::i64(val),
            Type::Bool => ConstantData::boolean(val != 0),
            _ => ConstantData::u64(val as u64),
        };
        return Some(builder.constant(const_val, ty, span));
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
    lower(&func_text)
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
    let ir = parse_function(&text)?;;

    // Map return type
    let ret_ty = parse_type(&ir.ret_type).unwrap_or(Type::Unit);

    // Build parameter list for SIR
    let mut sir_params: Vec<(&str, Type)> = ir.params.iter().map(|(name, ty)| {
        let sir_ty = parse_type(ty).unwrap_or(Type::u64());
        // For pointer params, model as pointer to u8
        let sir_ty = if ty.starts_with("ptr") {
            Type::Pointer { pointee: Box::new(Type::u8()), mutable: false }
        } else {
            sir_ty
        };
        (name.as_str(), sir_ty)
    }).collect();

    // Scan for global array references (e.g., @popcount_table) in the function body.
    // These are added as extra implicit parameters so the SIR function can access them.
    let mut globals: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.contains("getelementptr") && line.contains(" ptr @") {
            if let Some(at) = line.find(" ptr @") {
                let after = &line[at + 5..];
                if let Some(comma) = after.find(',') {
                    let gname = after[..comma].trim().trim_start_matches('@').to_string();
                    if !gname.starts_with("llvm.") && !globals.contains(&gname) {
                        globals.push(gname);
                    }
                }
            }
        }
    }

    // Add globals as extra Array<u8, 256> parameters
    let mut global_param_names: Vec<String> = Vec::new();
    for gname in &globals {
        global_param_names.push(format!("__glob_{}", gname));
    }
    for pname in &global_param_names {
        sir_params.push((
            pname.as_str(),
            Type::Array { element: Box::new(Type::u8()), length: 256 },
        ));
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
    for (i, gname) in globals.iter().enumerate() {
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
        for (i, b) in ir.blocks.iter().enumerate() {
        }
        return Err("function has too few blocks for the lowerer".to_string());
    }

    // Find the loop block: the one with phi nodes AND a self-referencing back-edge.
    // In LLVM IR, the loop block's phi nodes have an incoming value from the same block.
    // The exit block also has phi nodes, but its incomings are from the entry and the loop block.
    for (i, b) in ir.blocks.iter().enumerate() {
        let has_phi = b.instructions.iter().any(|i| i.opcode == "phi");
    }
    let loop_block_idx = ir.blocks.iter().position(|b| {
        // The loop block has a phi that references itself as an incoming block
        b.instructions.iter().any(|i| {
            if i.opcode == "phi" {
                // Check if any incoming label matches this block's label
                let combined = i.operands.join(", ");
                combined.contains(&format!("%{}]", b.label)) ||
                combined.contains(&format!(", %{} ]", b.label)) ||
                combined.contains(&format!("%{}", b.label))
            } else {
                false
            }
        })
    });

    let result = match loop_block_idx {
        Some(lbi) => lower_loop_function(&ir, lbi, &mut builder, &mut value_map),
        None => lower_straight_line(&ir, &mut builder, &mut value_map),
    };

    result.map(|_| builder.build())
}

/// Lower a function with a detected loop.
fn lower_loop_function(
    ir: &IrFunction,
    loop_idx: usize,
    builder: &mut Builder,
    value_map: &mut HashMap<String, NodeId>,
) -> Result<(), String> {
    let span = Span::unknown();

    // Emit entry block instructions (non-branch) before the loop.
    // The entry block often contains an initial comparison (e.g., icmp sgt n, 0)
    // whose result is used by exit-block phis.
    if loop_idx > 0 {
        let entry_block = &ir.blocks[0];
        for inst in &entry_block.instructions {
            if inst.opcode == "br" || inst.opcode == "ret" {
                continue;
            }
            emit_instruction(inst, builder, value_map, &ir.params, span)?;
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
                    '[' => { in_bracket = true; current.clear(); pair_parts.clear(); }
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
                    _ if in_bracket => { current.push(c); }
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
    let mut carried_initials: Vec<(String, String)> = Vec::new();  // (phi_result, initial_value_str)
    let mut carried_nexts: Vec<(String, String)> = Vec::new();     // (phi_result, next_value_str)

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
        let phi_ty = phis.iter().find(|p| &p.result == phi_result).map(|p| p.ty.as_str()).unwrap_or("i64");
        let sir_ty = parse_type(phi_ty).unwrap_or(Type::u64());
        let node_id = if let Some(id) = get_node_id(&init_str, value_map, &ir.params, builder, Some(sir_ty.clone())) {
            id
        } else if let Some(val) = parse_int_constant(&init_str) {
            // Create a constant node with the PHI's declared type
            let const_val = match &sir_ty {
                Type::Integer { width: IntegerWidth::I8, signed: false, .. } => ConstantData::u8(val as u8),
                Type::Integer { width: IntegerWidth::I8, signed: true, .. } => ConstantData::i8(val as i8),
                Type::Integer { width: IntegerWidth::I16, signed: false, .. } => ConstantData::u16(val as u16),
                Type::Integer { width: IntegerWidth::I16, signed: true, .. } => ConstantData::i16(val as i16),
                Type::Integer { width: IntegerWidth::I32, signed: false, .. } => ConstantData::u32(val as u32),
                Type::Integer { width: IntegerWidth::I32, signed: true, .. } => ConstantData::i32(val as i32),
                Type::Integer { width: IntegerWidth::I64, signed: false, .. } => ConstantData::u64(val as u64),
                Type::Integer { width: IntegerWidth::I64, signed: true, .. } => ConstantData::i64(val),
                Type::Bool => ConstantData::boolean(val != 0),
                _ => ConstantData::u64(val as u64),
            };
            builder.constant(const_val, sir_ty, span)
        } else {
            // The initial value might be defined in the entry block (e.g., a load).
            // Try to emit entry block instructions to resolve it.
            return Err(format!("cannot resolve initial value '{}' for phi {}", init_str, phi_result));
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

    for inst in &loop_block.instructions {
    }

    // Find the termination condition: the conditional br at the end of the loop block
    // br i1 %cond, label %exit, label %loop
    // The condition is the first operand
    let mut termination_node: Option<NodeId> = None;
    let mut exit_label: Option<String> = None;

    for inst in &loop_block.instructions {
        if inst.opcode == "br" && inst.operands.len() >= 3 {
            // conditional branch
            let cond_str = strip_type(&inst.operands[0]);
            termination_node = get_node_id(&cond_str, value_map, &ir.params, builder, None);
            // The exit label is the one that's NOT the loop label
            let true_label = inst.operands[1].trim_start_matches("label ").trim_start_matches('%').to_string();
            let false_label = inst.operands[2].trim_start_matches("label ").trim_start_matches('%').to_string();
            exit_label = if true_label == *loop_label {
                Some(false_label)
            } else {
                Some(true_label)
            };
        }
    }

    let termination = termination_node.ok_or("no termination condition found in loop block")?;

    // Find the output nodes: the carried "next" values
    let mut output_nodes: Vec<NodeId> = Vec::new();
    for (_, next_str) in &carried_nexts {
        let next_str = strip_type(next_str);
        let node_id = get_node_id(&next_str, value_map, &ir.params, builder, None)
            .ok_or(format!("cannot resolve carried next value '{}'", next_str))?;
        output_nodes.push(node_id);
    }

    // Build the Loop node
    let output_types: Vec<Type> = output_nodes.iter().map(|id| {
        builder.function().get_node(*id).map(|n| n.ty.clone()).unwrap_or(Type::u64())
    }).collect();

    let loop_ty = Type::Tuple { elements: output_types };
    let loop_node = builder.r#loop(
        &body_nodes,
        termination,
        &output_nodes,
        &carried_init_nodes,
        loop_ty,
        span,
    ).map_err(|e| format!("loop build error: {:?}", e))?;

    // Map phi results to the loop outputs (via TupleExtract)
    let mut extract_for_output: Vec<NodeId> = Vec::new();
    for (i, phi_result) in carried_names.iter().enumerate() {
        let output_ty = builder.function().get_node(output_nodes[i])
            .map(|n| n.ty.clone())
            .unwrap_or(Type::u64());
        let extract = builder.tuple_extract(loop_node, i, output_ty, span).unwrap();
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
                            '[' => { in_bracket = true; current.clear(); pair_parts.clear(); }
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
                            _ if in_bracket => { current.push(c); }
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
                        } else if let Some(id) = get_node_id(&val, value_map, &ir.params, builder, None) {
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
                        builder.return_value(ret_node, span).map_err(|e| format!("return error: {:?}", e))?;
                    } else {
                        // ret void — emit a Unit return
                        let unit = builder.constant(ConstantData::Unit, Type::Unit, span);
                        builder.return_value(unit, span).map_err(|e| format!("return error: {:?}", e))?;
                    }
                } else if inst.opcode == "br" {
                    // Unconditional br: follow to the next block
                    if inst.operands.len() == 1 {
                        next_label = Some(inst.operands[0].trim_start_matches("label ").trim_start_matches('%').to_string());
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
                    builder.return_value(ret_node, span).map_err(|e| format!("return error: {:?}", e))?;
                } else {
                    // ret void
                    let unit = builder.constant(ConstantData::Unit, Type::Unit, span);
                    builder.return_value(unit, span).map_err(|e| format!("return error: {:?}", e))?;
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
        "add" | "sub" | "mul" | "and" | "or" | "xor" | "shl" | "lshr" | "ashr" | "udiv" | "sdiv" | "urem" | "srem" => {
            // Binary op: operands are like "i64 %7, i64 %1" or "i8 %10, 1"
            if inst.operands.len() < 2 {
                return Err(format!("{} needs 2 operands: {}", inst.opcode, inst.raw));
            }
            // Extract the type from the first operand (e.g., "i8 %10" → i8)
            let op_type = inst.operands[0].split_whitespace().next()
                .and_then(|t| parse_type(t));
            let lhs_str = strip_type(&inst.operands[0]);
            let rhs_str = strip_type(&inst.operands[1]);
            let lhs = get_node_id(&lhs_str, value_map, params, builder, op_type.clone())
                .ok_or(format!("cannot resolve lhs '{}' in {}", lhs_str, inst.raw))?;
            let rhs = get_node_id(&rhs_str, value_map, params, builder, op_type)
                .ok_or(format!("cannot resolve rhs '{}' in {}", rhs_str, inst.raw))?;

            let is_bool = inst.operands[0].split_whitespace().next()
                .map(|t| t == "i1")
                .unwrap_or(false);
            let node_id = match inst.opcode.as_str() {
                "add" => builder.add(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "sub" => builder.sub(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "mul" => builder.mul(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "and" if is_bool => builder.bool_and(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "and" => builder.bit_and(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "or" if is_bool => builder.bool_or(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "or" => builder.bit_or(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "xor" if is_bool => {
                    // SIR has no BoolXor. Convert bools to i8, bit_xor, return i8.
                    let lhs_i8 = builder.convert(lhs, Type::u8(), sir_nodes::ConvertKind::ZeroExtend, span)
                        .map_err(|e| format!("{:?}", e))?;
                    let rhs_i8 = builder.convert(rhs, Type::u8(), sir_nodes::ConvertKind::ZeroExtend, span)
                        .map_err(|e| format!("{:?}", e))?;
                    builder.bit_xor(lhs_i8, rhs_i8, span).map_err(|e| format!("{:?}", e))?
                }
                "xor" => builder.bit_xor(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "shl" => builder.shl(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "lshr" | "ashr" => builder.shr(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "udiv" | "sdiv" => builder.div(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "urem" | "srem" => builder.rem(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                _ => unreachable!(),
            };

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
                return Err(format!("icmp needs cmp_type + type + lhs + rhs: {}", inst.raw));
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

            let node_id = match cmp_type {
                "eq" => builder.eq(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "ne" => builder.ne(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "sgt" | "ugt" => builder.gt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "sge" | "uge" => builder.ge(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "slt" | "ult" => builder.lt(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
                "sle" | "ule" => builder.le(lhs, rhs, span).map_err(|e| format!("{:?}", e))?,
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
                (raw[..pos].trim().to_string(), raw[pos + 4..].trim().to_string())
            } else {
                (raw.clone(), String::new())
            };
            let src_str = strip_type(&src_part);
            let src = get_node_id(&src_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve {} source '{}'", inst.opcode, src_str))?;

            let to_type = parse_type(&to_part).unwrap_or(Type::u64());

            let kind = match inst.opcode.as_str() {
                "zext" => sir_nodes::ConvertKind::ZeroExtend,
                "sext" => sir_nodes::ConvertKind::SignExtend,
                "trunc" => sir_nodes::ConvertKind::Truncate,
                _ => unreachable!(),
            };

            let node_id = builder.convert(src, to_type, kind, span)
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
            let cond_str = strip_type(&inst.operands[0]);
            let true_str = strip_type(&inst.operands[1]);
            let false_str = strip_type(&inst.operands[2]);
            let cond = get_node_id(&cond_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve select cond '{}'", cond_str))?;
            let true_val = get_node_id(&true_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve select true '{}'", true_str))?;
            let false_val = get_node_id(&false_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve select false '{}'", false_str))?;

            let node_id = builder.select(cond, true_val, false_val, span)
                .map_err(|e| format!("{:?}", e))?;

            if let Some(name) = result_name {
                value_map.insert(name, node_id);
            }
            Ok(Some(node_id))
        }

        "load" => {
            // load i8, ptr %9, align 1
            // The first operand is the type, second is the pointer
            if inst.operands.len() < 2 {
                return Err(format!("load needs type + ptr: {}", inst.raw));
            }
            let loaded_ty = parse_type(&inst.operands[0]).unwrap_or(Type::u8());
            let ptr_str = strip_type(&inst.operands[1]);
            let ptr = get_node_id(&ptr_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve load ptr '{}'", ptr_str))?;

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
                return Err(format!("getelementptr needs type + ptr + index: {}", inst.raw));
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

            let node_id = builder.array_access(base, index, elem_ty, span)
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
            // operand 0: "i8 %result" (the value)
            // operand 1: "ptr %out_ptr" (the pointer)
            let val_str = strip_type(&inst.operands[0]);
            let ptr_str = strip_type(&inst.operands[1]);
            let val = get_node_id(&val_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve store value '{}'", val_str))?;
            let ptr = get_node_id(&ptr_str, value_map, params, builder, None)
                .ok_or(format!("cannot resolve store ptr '{}'", ptr_str))?;
            let _node_id = builder.store(ptr, val, span)
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
            let callee = inst.operands.first()
                .map(|s| s.trim())
                .unwrap_or("");
            if callee.contains("llvm.umax") || callee.contains("llvm.umin") {
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
                        let a = get_node_id(&a_str, value_map, params, builder, None)
                            .ok_or(format!("cannot resolve umax operand '{}'", a_str))?;
                        let b = get_node_id(&b_str, value_map, params, builder, None)
                            .ok_or(format!("cannot resolve umax operand '{}'", b_str))?;

                        let node_id = if callee.contains("llvm.umax") {
                            // umax(a, b) = select(a > b, a, b)
                            let cmp = builder.gt(a, b, span).map_err(|e| format!("{:?}", e))?;
                            builder.select(cmp, a, b, span).map_err(|e| format!("{:?}", e))?
                        } else {
                            // umin(a, b) = select(a < b, a, b)
                            let cmp = builder.lt(a, b, span).map_err(|e| format!("{:?}", e))?;
                            builder.select(cmp, a, b, span).map_err(|e| format!("{:?}", e))?
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

        _ => {
            Err(format!("unsupported instruction: {} (raw: {})", inst.opcode, inst.raw))
        }
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
            '[' => { in_bracket = true; current.clear(); pair_parts.clear(); }
            ']' if in_bracket => {
                in_bracket = false;
                if !current.trim().is_empty() { pair_parts.push(current.trim().to_string()); }
                incoming.push((
                    pair_parts.get(0).cloned().unwrap_or_default(),
                    pair_parts.get(1).cloned().unwrap_or_default(),
                ));
            }
            ',' if in_bracket => {
                if !current.trim().is_empty() { pair_parts.push(current.trim().to_string()); }
                current.clear();
            }
            _ if in_bracket => { current.push(c); }
            _ => {}
        }
    }
    incoming
}
