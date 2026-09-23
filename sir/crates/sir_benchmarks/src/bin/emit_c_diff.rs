//! emit_c_diff — native differential execution of emitted C vs the
//! original LLVM IR.
//!
//! For every function in a `.ll` corpus: lower to SIR, run the optimizer,
//! emit C (H3 findings F6–F8 fixed), compile the emitted C and a renamed
//! copy of the original IR with clang, run both on identical random
//! inputs, and compare the return value and every pointer buffer.
//!
//! Usage:
//!   emit_c_diff <corpus.ll> [--cases N] [--workdir DIR] [--function NAME]
//!
//! Exit code 0 iff every lowered function was differential-clean.

use sir_benchmarks::emit;
use sir_lower::{external_symbol, lower_function, parse_struct_layouts};
use sir_nodes::{Function, NodeKind};
use sir_types::Type;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static ALLOW_RUNTIME_SEARCH: AtomicBool = AtomicBool::new(false);
static ALLOW_DEFS: OnceLock<Vec<u64>> = OnceLock::new();
static PER_FUNCTION_ALLOW: OnceLock<BTreeMap<String, BTreeMap<u64, bool>>> = OnceLock::new();
static FFI_SUPPORT: OnceLock<Option<PathBuf>> = OnceLock::new();
static REF_OBJECT: OnceLock<Option<PathBuf>> = OnceLock::new();
static TIME_ABC: AtomicBool = AtomicBool::new(false);

/// Object file holding the reference side of a `--time` comparison, built
/// from the original source at the same optimization level as everything
/// else. The object must export `{name}__ref` and define every global the
/// function reads; the driver then injects nothing, so both sides read the
/// object's own tables.
fn ref_object() -> Option<PathBuf> {
    REF_OBJECT.get().and_then(|object| object.clone())
}

fn abc_mode() -> bool {
    TIME_ABC.load(Ordering::Relaxed)
}

fn ffi_support() -> Option<PathBuf> {
    FFI_SUPPORT.get().and_then(|support| support.clone())
}

/// Function names defined in the module (`define ... @name(`).
fn function_names(ll: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with("define ") {
            continue;
        }
        let Some(at) = line.find('@') else { continue };
        let rest = &line[at + 1..];
        let Some(paren) = rest.find('(') else {
            continue;
        };
        names.push(rest[..paren].to_string());
    }
    names
}

/// Direct call targets in one function. Calls to symbols not defined in the
/// module are refused: the harness has no implementation to link them with.
fn direct_callees(ll: &str, name: &str) -> Result<Vec<String>, String> {
    let defined: HashSet<String> = function_names(ll).into_iter().collect();
    let body = function_ir(ll, name).ok_or_else(|| format!("could not find definition @{name}"))?;
    let mut out = Vec::new();
    for line in body.lines() {
        if !line.contains("call ") {
            continue;
        }
        let Some(at) = line.find('@') else { continue };
        let rest = &line[at + 1..];
        let Some(open) = rest.find('(') else { continue };
        let target = rest[..open].trim();
        if target.is_empty() || target.starts_with("llvm.") || matches!(target, "bcmp" | "memcmp") {
            continue;
        }
        if !defined.contains(target) {
            if FFI_SUPPORT.get().is_some_and(|support| support.is_some()) {
                continue;
            }
            return Err(format!("unsupported external call: @{target}"));
        }
        if !out.iter().any(|existing| existing == target) {
            out.push(target.to_string());
        }
    }
    Ok(out)
}

/// Lower a root function together with the local functions it calls.
/// Entries are postordered: callees first, root last, so every callee body
/// is available before the caller that invokes it is tested.
fn lower_module(ll: &str, name: &str) -> Result<Vec<(String, Function)>, String> {
    fn visit(
        ll: &str,
        name: &str,
        root: &str,
        active: &mut HashSet<String>,
        done: &mut HashSet<String>,
        out: &mut Vec<(String, Function)>,
    ) -> Result<(), String> {
        if done.contains(name) {
            return Ok(());
        }
        if !active.insert(name.to_string()) {
            return Err(format!("recursive call cycle at @{name}"));
        }
        for callee in direct_callees(ll, name)? {
            visit(ll, &callee, root, active, done, out)?;
        }
        active.remove(name);
        let mut func = lower_function(ll, name)
            .map_err(|e| e.lines().last().unwrap_or(&e).trim().to_string())?;
        if name != root {
            func.name = external_symbol(name);
        }
        out.push((name.to_string(), func));
        done.insert(name.to_string());
        Ok(())
    }

    let mut out = Vec::new();
    visit(
        ll,
        name,
        name,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut out,
    )?;
    Ok(out)
}

/// Module-level global definitions. The C driver defines each one so
/// both the renamed reference IR and the emitted C resolve the symbol.
#[derive(Clone)]
struct GlobalDecl {
    name: String,
    ty: String,
    size: usize,
    pointer: bool,
    init: Option<String>,
}

/// Byte size of an LLVM integer/array/pointer type; `None` for shapes
/// (structs, vectors) whose layout this driver does not model.
fn llvm_type_size(ty: &str, layouts: &HashMap<String, (usize, usize)>) -> Option<usize> {
    let ty = ty.trim();
    if let Some(name) = ty.strip_prefix('%') {
        return layouts.get(&format!("%{name}")).map(|(size, _)| *size);
    }
    if ty.starts_with('[') {
        let inner = ty.strip_prefix('[')?.strip_suffix(']')?;
        let (length, element) = inner.split_once('x')?;
        Some(length.trim().parse::<usize>().ok()? * llvm_type_size(element, layouts)?)
    } else if let Some(inner) = ty.strip_prefix("<{").and_then(|t| t.strip_suffix("}>")) {
        let mut total = 0usize;
        let mut depth = 0usize;
        let mut start = 0usize;
        for (index, ch) in inner.char_indices() {
            match ch {
                '[' | '{' | '<' => depth += 1,
                ']' | '}' | '>' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    total += llvm_type_size(&inner[start..index], layouts)?;
                    start = index + 1;
                }
                _ => {}
            }
        }
        total += llvm_type_size(&inner[start..], layouts)?;
        Some(total)
    } else if let Some(bits) = ty.strip_prefix('i') {
        Some(bits.parse::<usize>().ok()?.div_ceil(8))
    } else if ty == "ptr" {
        Some(8)
    } else if ty == "float" {
        Some(4)
    } else if ty == "double" {
        Some(8)
    } else {
        None
    }
}

/// Split an LLVM type token from the rest of a global definition,
/// respecting nested `[...]` and `<{...}>` shapes.
fn split_llvm_type(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let (open, close) = if text.starts_with('[') {
        ('[', ']')
    } else if text.starts_with("<{") {
        ('<', '>')
    } else {
        return text.split_once(char::is_whitespace);
    };
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some((&text[..=index], text[index + 1..].trim()));
            }
        }
    }
    None
}

/// Cut trailing definition attributes (`align`, `section`, ...) off a global
/// initializer, leaving just the value.
fn strip_global_attrs(init: &str) -> &str {
    const ATTRS: [&str; 6] = ["align", "section", "comdat", "no_sanitize", "sanitize", "!dbg"];
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in init.char_indices() {
        // String payloads are bytes, not structure: a brace inside `c"..."`
        // must not hide the `, align 16` that follows the closing quote.
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' | '{' | '(' | '<' => depth += 1,
            ']' | '}' | ')' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let tail = init[index + 1..].trim_start();
                if ATTRS.iter().any(|attr| tail.starts_with(attr)) {
                    return init[..index].trim();
                }
            }
            _ => {}
        }
    }
    init.trim()
}

/// `(element count, element type)` for a flat LLVM array type.
fn llvm_array_type(ty: &str) -> Option<(usize, &str)> {
    let inner = ty.trim().strip_prefix('[')?.strip_suffix(']')?;
    let (length, element) = inner.split_once('x')?;
    Some((length.trim().parse().ok()?, element.trim()))
}

/// Decode an LLVM `c"..."` payload: printable bytes pass through, `\XX` is a
/// hex byte and `\\`/`\"` are the literal characters.
fn decode_llvm_string(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let escape = text.get(i + 1..).unwrap_or("");
        if escape.len() >= 2 {
            let hex = &escape[..2];
            if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                out.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
                i += 3;
                continue;
            }
        }
        if escape.starts_with('\\') || escape.starts_with('"') {
            out.push(bytes[i + 1]);
            i += 2;
            continue;
        }
        return Err(format!("unsupported escape in string initializer at {}", &text[i..]));
    }
    Ok(out)
}

/// Bytes of a global's module initializer, or `None` when it is all-zero
/// (the driver's default). Only shapes this harness can reproduce exactly —
/// string literals and flat integer arrays — are accepted; nested aggregates,
/// struct literals and pointer values are refused so no side ever reads a
/// table the other side does not have.
fn initializer_bytes(decl: &GlobalDecl) -> Result<Option<Vec<u8>>, String> {
    let Some(init) = decl.init.as_deref() else {
        return Ok(None);
    };
    if init == "zeroinitializer" || init == "null" {
        return Ok(None);
    }
    if let Some(quoted) = init.strip_prefix('c') {
        let Some(text) = quoted.trim().strip_prefix('"').and_then(|t| t.strip_suffix('"')) else {
            return Err(format!(
                "unsupported: global @{} has an unparsable string initializer",
                decl.name
            ));
        };
        let decoded = decode_llvm_string(text)?;
        if decoded.len() > decl.size {
            return Err(format!(
                "unsupported: global @{} initializes {} bytes into {} bytes",
                decl.name,
                decoded.len(),
                decl.size
            ));
        }
        let mut bytes = vec![0u8; decl.size];
        bytes[..decoded.len()].copy_from_slice(&decoded);
        return Ok(Some(bytes));
    }
    let (count, element) = llvm_array_type(&decl.ty).ok_or_else(|| {
        format!(
            "unsupported: global @{} has a non-array initializer this driver cannot reproduce",
            decl.name
        )
    })?;
    let element_size = llvm_type_size(element, &Default::default()).ok_or_else(|| {
        format!(
            "unsupported: global @{} has element type {element} this driver cannot reproduce",
            decl.name
        )
    })?;
    if !element.starts_with('i') || element_size > 8 {
        return Err(format!(
            "unsupported: global @{} has element type {element} this driver cannot reproduce",
            decl.name
        ));
    }
    let Some(body) = init.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else {
        return Err(format!(
            "unsupported: global @{} has an initializer this driver cannot reproduce",
            decl.name
        ));
    };
    let values: Vec<i64> = body
        .split(',')
        .map(|item| {
            let item = item.trim();
            let (ty, value) = item.split_once(char::is_whitespace).ok_or_else(|| {
                format!("unsupported: global @{} has a malformed initializer", decl.name)
            })?;
            if ty.trim() != element {
                return Err(format!(
                    "unsupported: global @{} initializes with {ty}, not the flat element {element}",
                    decl.name
                ));
            }
            let value = value.trim();
            value
                .parse::<i64>()
                .or_else(|_| i64::from_str_radix(value.trim_start_matches("0x"), 16))
                .or_else(|_| match value {
                    "true" => Ok(1),
                    "false" => Ok(0),
                    _ => Err(()),
                })
                .map_err(|_| {
                    format!(
                        "unsupported: global @{} has an initializer element {value}",
                        decl.name
                    )
                })
        })
        .collect::<Result<Vec<i64>, String>>()?;
    if values.len() != count {
        return Err(format!(
            "unsupported: global @{} initializes {} of {count} elements",
            decl.name,
            values.len()
        ));
    }
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes()[..element_size].to_vec())
        .collect();
    if bytes.len() != decl.size {
        return Err(format!(
            "unsupported: global @{} has initializer size {} against declared {}",
            decl.name,
            bytes.len(),
            decl.size
        ));
    }
    Ok(Some(bytes))
}

fn global_decls(ll: &str) -> Vec<GlobalDecl> {
    let mut out: Vec<GlobalDecl> = Vec::new();
    let layouts = parse_struct_layouts(ll);
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with('@') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let name = line[1..eq].trim();
        if name.is_empty() || name.contains('"') || out.iter().any(|g| g.name == name) {
            continue;
        }
        let rest = line[eq + 1..].trim();
        let Some((global_at, keyword_len)) = rest
            .find("global ")
            .map(|at| (at, "global ".len()))
            .or_else(|| rest.find("constant ").map(|at| (at, "constant ".len())))
        else {
            continue;
        };
        let after = &rest[global_at + keyword_len..];
        let Some((ty, init)) = split_llvm_type(after) else {
            continue;
        };
        let ty = ty.trim_end_matches(',');
        let size = llvm_type_size(ty, &layouts).unwrap_or(0);
        let init = strip_global_attrs(init);
        out.push(GlobalDecl {
            name: name.to_string(),
            ty: ty.to_string(),
            size,
            pointer: ty.trim() == "ptr",
            init: if init.is_empty() {
                None
            } else {
                Some(init.to_string())
            },
        });
    }
    out
}

/// Global names referenced by a function body.
fn referenced_globals(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
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

/// Rewrite the reference IR so the selected globals live in the driver's
/// namespaced buffers: definitions become `external` declarations under
/// the C-side name, and every reference is renamed to match.
fn shadow_globals(ll: &str, decls: &[GlobalDecl]) -> String {
    let mut out = String::with_capacity(ll.len() + 128);
    for line in ll.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            if let Some(eq) = trimmed.find('=') {
                let name = trimmed[1..eq].trim();
                if let Some(decl) = decls.iter().find(|decl| decl.name == name) {
                    out.push_str(&format!(
                        "@{} = external global {}\n",
                        emit::c_global_name(&decl.name),
                        decl.ty
                    ));
                    continue;
                }
            }
        }
        let mut pos = 0;
        while let Some(offset) = line[pos..].find('@') {
            let at = pos + offset;
            out.push_str(&line[pos..at]);
            let start = at + 1;
            let mut end = start;
            while end < line.len()
                && (line.as_bytes()[end].is_ascii_alphanumeric()
                    || b"_.$-".contains(&line.as_bytes()[end]))
            {
                end += 1;
            }
            let name = &line[start..end];
            if let Some(decl) = decls.iter().find(|decl| decl.name == name) {
                out.push('@');
                out.push_str(&emit::c_global_name(&decl.name));
            } else {
                out.push_str(&line[at..end]);
            }
            pos = end;
        }
        out.push_str(&line[pos..]);
        out.push('\n');
    }
    out
}

/// Globals this function cannot lower with the current driver model:
/// initialized (non-zero) internal data, or shapes whose layout is
/// unknown. Read-only zero-initialized globals are supported.
fn unsupported_global_use(ll: &str, name: &str) -> Option<String> {
    let body = function_ir(ll, name)?;
    let decls = global_decls(ll);
    for referenced in referenced_globals(&body) {
        let Some(decl) = decls.iter().find(|decl| decl.name == referenced) else {
            continue;
        };
        if decl.size == 0 || decl.size > (1 << 20) {
            return Some(format!(
                "unsupported: global @{} has unsupported size {}",
                referenced, decl.size
            ));
        }
    }
    None
}

/// Globals referenced by any function in the module, deduplicated by name.
fn used_globals_all(ll: &str, names: &[String]) -> Vec<GlobalDecl> {
    let decls = global_decls(ll);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in names {
        let Some(body) = function_ir(ll, name) else {
            continue;
        };
        for referenced in referenced_globals(&body) {
            if !seen.insert(referenced.clone()) {
                continue;
            }
            if let Some(decl) = decls.iter().find(|decl| decl.name == referenced) {
                out.push(decl.clone());
            }
        }
    }
    out
}

/// `(return type, parameter types)` for a definition, raw LLVM tokens.
fn ir_signature(ll: &str, name: &str) -> Option<(String, Vec<String>)> {
    let needle = format!("@{name}(");
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with("define ") || !line.contains(&needle) {
            continue;
        }
        let at = line.find(&needle)?;
        let ret = line[..at].split_whitespace().last()?.to_string();
        let after = &line[at + needle.len()..];
        let mut depth = 0usize;
        let mut end = after.len();
        for (i, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    end = i;
                    break;
                }
                ')' => depth -= 1,
                _ => {}
            }
        }
        let mut params = Vec::new();
        for part in after[..end].split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            params.push(part.split_whitespace().next()?.to_string());
        }
        return Some((ret, params));
    }
    None
}

/// C type for a raw LLVM scalar/pointer type token.
fn llvm_c_type(raw: &str) -> String {
    match raw {
        "void" => "void".to_string(),
        "i1" => "bool".to_string(),
        "i8" => "uint8_t".to_string(),
        "i16" => "uint16_t".to_string(),
        "i32" => "uint32_t".to_string(),
        "i64" => "uint64_t".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        t if t.starts_with("ptr") => "const uint8_t *".to_string(),
        _ => "uint64_t".to_string(),
    }
}

fn is_pointer_ty(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Pointer { .. } | Type::Array { .. } | Type::Slice { .. }
    )
}

/// C cast expression type for a pointer-like SIR parameter (arrays decay
/// to element pointers so the cast is valid C).
fn pointer_cast(ty: &Type) -> String {
    match ty {
        Type::Array { element, .. } | Type::Slice { element } => {
            format!("const {} *", emit::c_type(element))
        }
        other => emit::c_type(other),
    }
}

fn build_driver(
    name: &str,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    cases: usize,
    low_bits: bool,
    globals: &[GlobalDecl],
    avoid_zero: bool,
) -> String {
    let mut out = String::new();
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <string.h>\n\n",
    );
    out.push_str("extern void __sir_ffi_reset(void) __attribute__((weak));\n\n");

    // Reference prototype (renamed original IR).
    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    out.push_str(&format!(
        "extern {} {}__ref({});\n\n",
        llvm_c_type(ir_ret),
        name,
        ir_p.join(", ")
    ));
    // Emitted prototype (defined in the separately compiled emitted.c).
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
    let new_params = if new_p.is_empty() {
        "void".to_string()
    } else {
        new_p.join(", ")
    };
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));

    let lens = if avoid_zero {
        "{1,2,3,4,5,7,8,16,31,32,33}"
    } else {
        "{0,1,2,3,4,5,7,8,16,31,32}"
    };
    out.push_str(&format!(
        "static uint64_t val(int t, int k) {{\n\
         \x20   static const uint64_t lens[] = {lens};\n\
         \x20   static const uint64_t keys[] = {{1,2,3,5,7,11,13,16,31}};\n\
         \x20   if (k == 0) return lens[t % 11];\n\
         \x20   return keys[(t * 3 + k * 5) % 9];\n\
         }}\n\n",
    ));
    out.push_str("static _Alignas(64) uint8_t __sir_ptr_scratch[4096];\n\n");

    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("    int zero_state = 0, ref_only = 0;\n");
    out.push_str(
        "    for (int i = 1; i < argc; i++) {\n\
         \x20       if (!strcmp(argv[i], \"--zero\")) zero_state = 1;\n\
         \x20       if (!strcmp(argv[i], \"--ref-only\")) ref_only = 1;\n\
         \x20   }\n",
    );
    // 32 bytes of identically-filled guard slab precede every buffer, so
    // guarded negative indices read the same bytes on both sides instead
    // of whatever happens to precede each array.
    out.push_str("    static _Alignas(64) uint8_t bufA[8][2048 + 32];\n");
    out.push_str("    static _Alignas(64) uint8_t bufB[8][2048 + 32];\n");
    out.push_str("    uint64_t mismatches = 0;\n");
    out.push_str(&format!("    for (int t = 0; t < {cases}; t++) {{\n"));
    out.push_str("        uint64_t seed = 0x9E3779B97F4A7C15ULL ^ (uint64_t)t;\n");
    let fill = if low_bits {
        "bufA[b][i + 32] = (uint8_t)(seed & 1);"
    } else {
        "bufA[b][i + 32] = (uint8_t)seed;"
    };
    out.push_str(
        "        if (zero_state) {\n\
         \x20           memset(bufA, 0, sizeof bufA);\n\
         \x20           memset(bufB, 0, sizeof bufB);\n\
         \x20       } else {\n",
    );
    out.push_str(&format!(
        "            for (int b = 0; b < 8; b++) for (int i = 0; i < 2048; i++) {{\n\
         \x20               seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;\n\
         \x20               {fill} bufB[b][i + 32] = bufA[b][i + 32];\n\
         \x20           }}\n",
    ));
    out.push_str("        }\n");

    // Scalar declarations + argument lists.
    let mut ref_args: Vec<String> = Vec::new();
    let mut new_args: Vec<String> = Vec::new();
    let mut ptr_slot = 0usize;
    let mut global_slots: Vec<usize> = Vec::new();
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        if i >= ir_params.len() {
            // Extra SIR parameter (implicit global array); no IR counterpart.
            new_args.push(format!(
                "({})(bufB[{}] + 32)",
                pointer_cast(&p.ty),
                ptr_slot
            ));
            global_slots.push(ptr_slot);
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            let cast = pointer_cast(&p.ty);
            ref_args.push(format!(
                "({})(bufA[{}] + 32)",
                llvm_c_type(&ir_params[i]),
                ptr_slot
            ));
            new_args.push(format!("({})(bufB[{}] + 32)", cast, ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "        {cty} s{scalar_index} = ({cty})val(t, {scalar_index});\n"
            ));
            ref_args.push(format!("({})s{}", llvm_c_type(&ir_params[i]), scalar_index));
            new_args.push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }
    // Implicit global arrays: the reference reads its own zero-initialized
    // definition, so the SIR global buffer must be zeroed too.
    for &slot in &global_slots {
        out.push_str(&format!("        memset(bufA[{slot}] + 32, 0, 2048);\n"));
        out.push_str(&format!("        memset(bufB[{slot}] + 32, 0, 2048);\n"));
    }

    let mut global_reset = String::new();
    let mut global_snapshot = String::new();
    let mut global_compare = String::new();
    for (i, g) in globals.iter().enumerate() {
        if g.size == 0 || g.size > (1 << 20) {
            continue;
        }
        let cname = emit::c_global_name(&g.name);
        if g.pointer {
            global_reset.push_str(&format!(
                "        memset({cname}, 0, {});\n\
                 \x20       *(void **)(void *){cname} = (void *)__sir_ptr_scratch;\n",
                g.size
            ));
        } else {
            global_reset.push_str(&format!(
                "        memset({cname}, 0, {});\n\
                 \x20       if (!zero_state) {{ uint64_t gs = 0x9E3779B97F4A7C15ULL ^ (uint64_t)t ^ {i}ULL;\n\
                 \x20         for (int j = 0; j < {}; j++) {{ gs ^= gs << 13; gs ^= gs >> 7; gs ^= gs << 17; {cname}[j] = (uint8_t)(gs >> 32); }} }}\n",
                g.size, g.size
            ));
        }
        global_snapshot.push_str(&format!(
            "        static uint8_t __snap{i}[{}]; memcpy(__snap{i}, {cname}, {});\n",
            g.size, g.size
        ));
        global_compare.push_str(&format!(
            "        if (memcmp(__snap{i}, {cname}, {})) {{ if (mismatches < 5) printf(\"EMITGLOB {name} t=%d slot={i}\\n\", t); mismatches++; }}\n",
            g.size
        ));
    }

    let ret_ty = emit::c_type(&func.return_ty);
    if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        out.push_str(&global_reset);
        out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!(
            "        {}__ref({});\n",
            name,
            ref_args.join(", ")
        ));
        out.push_str("        if (ref_only) continue;\n");
        out.push_str(&global_snapshot);
        out.push_str(&global_reset);
        out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("        {}({});\n", name, new_args.join(", ")));
        out.push_str(&global_compare);
    } else {
        out.push_str(&global_reset);
        out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!(
            "        {} r_ref = {}__ref({});\n",
            llvm_c_type(ir_ret),
            name,
            ref_args.join(", ")
        ));
        out.push_str("        if (ref_only) continue;\n");
        out.push_str(&global_snapshot);
        out.push_str(&global_reset);
        out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!(
            "        {} r_new = {}({});\n",
            ret_ty,
            name,
            new_args.join(", ")
        ));
        out.push_str(
            "        if ((uint64_t)r_ref != (uint64_t)r_new) {\n\
             \x20           if (mismatches < 5) printf(\"EMITCASE ",
        );
        out.push_str(&format!(
            "{} t=%d ref=%llu new=%llu\\n\", t, (unsigned long long)r_ref, (unsigned long long)r_new);\n",
            name
        ));
        out.push_str("            mismatches++;\n        }\n");
        out.push_str(&global_compare);
    }
    out.push_str(&format!(
        "        for (int b = 0; b < {ptr_slot}; b++) if (memcmp(bufA[b] + 32, bufB[b] + 32, 2048)) {{\n\
         \x20           if (mismatches < 5) printf(\"EMITBUF {name} t=%d slot=%d\\n\", t, b);\n\
         \x20           mismatches++;\n\
         \x20       }}\n"
    ));
    out.push_str("    }\n");
    out.push_str(&format!(
        "    if (ref_only) {{ printf(\"EMITREF_OK cases={cases}\\n\"); return 0; }}\n"
    ));
    out.push_str(&format!(
        "    printf(\"EMITDIFF {name} cases={cases} mismatches=%llu\\n\", (unsigned long long)mismatches);\n"
    ));
    out.push_str("    return mismatches != 0;\n}\n");
    out
}

/// Same-address differential driver for functions with pointer-valued
/// memory or pointer returns. Both sides run sequentially on one buffer
/// (with a restart snapshot between calls), so stored pointer bytes are
/// directly comparable instead of differing by buffer placement.
fn build_same_address_driver(
    name: &str,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    cases: usize,
    low_bits: bool,
    globals: &[GlobalDecl],
    avoid_zero: bool,
) -> String {
    let mut out = String::new();
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <string.h>\n\n",
    );
    out.push_str("extern void __sir_ffi_reset(void) __attribute__((weak));\n\n");
    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    out.push_str(&format!(
        "extern {} {}__ref({});\n\n",
        llvm_c_type(ir_ret),
        name,
        ir_p.join(", ")
    ));
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        if new_p.is_empty() {
            "void".to_string()
        } else {
            new_p.join(", ")
        }
    ));
    let lens = if avoid_zero {
        "{1,2,3,4,5,7,8,16,31,32,33}"
    } else {
        "{0,1,2,3,4,5,7,8,16,31,32}"
    };
    out.push_str(&format!(
        "static uint64_t val(int t, int k) {{\n\
         \x20   static const uint64_t lens[] = {lens};\n\
         \x20   static const uint64_t keys[] = {{1,2,3,5,7,11,13,16,31}};\n\
         \x20   if (k == 0) return lens[t % 11];\n\
         \x20   return keys[(t * 3 + k * 5) % 9];\n\
         }}\n\n"
    ));
    out.push_str("static _Alignas(64) uint8_t __sir_ptr_scratch[4096];\n\n");
    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("    int zero_state = 0, ref_only = 0;\n");
    out.push_str(
        "    for (int i = 1; i < argc; i++) {\n\
         \x20       if (!strcmp(argv[i], \"--zero\")) zero_state = 1;\n\
         \x20       if (!strcmp(argv[i], \"--ref-only\")) ref_only = 1;\n\
         \x20   }\n",
    );
    out.push_str("    static _Alignas(64) uint8_t buf[8][2048 + 32];\n");
    out.push_str("    static _Alignas(64) uint8_t initbuf[8][2048 + 32];\n");
    out.push_str("    static _Alignas(64) uint8_t snap[8][2048 + 32];\n");
    out.push_str("    uint64_t mismatches = 0;\n");
    out.push_str(&format!("    for (int t = 0; t < {cases}; t++) {{\n"));
    // Pointer-carrying memory must start in a valid state: random bytes
    // would be interpreted as addresses. Same-address runs therefore use
    // zero-initialized buffers; scalar parameters still vary per case.
    let _ = low_bits;
    out.push_str("        memset(initbuf, 0, sizeof initbuf);\n");
    let mut ref_args: Vec<String> = Vec::new();
    let mut new_args: Vec<String> = Vec::new();
    let mut ptr_slot = 0usize;
    let mut global_slots: Vec<usize> = Vec::new();
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        if i >= ir_params.len() {
            new_args.push(format!("({})(buf[{}] + 32)", pointer_cast(&p.ty), ptr_slot));
            global_slots.push(ptr_slot);
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            ref_args.push(format!(
                "({})(buf[{}] + 32)",
                llvm_c_type(&ir_params[i]),
                ptr_slot
            ));
            new_args.push(format!("({})(buf[{}] + 32)", pointer_cast(&p.ty), ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "        {cty} s{scalar_index} = ({cty})val(t, {scalar_index});\n"
            ));
            ref_args.push(format!("({})s{}", llvm_c_type(&ir_params[i]), scalar_index));
            new_args.push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }
    let mut reset_buffers = String::new();
    for slot in 0..ptr_slot {
        reset_buffers.push_str(&format!(
            "        memcpy(buf[{slot}] + 32, initbuf[{slot}] + 32, 2048);\n"
        ));
    }
    for &slot in &global_slots {
        reset_buffers.push_str(&format!("        memset(buf[{slot}] + 32, 0, 2048);\n"));
    }
    let mut global_reset = String::new();
    let mut global_snapshot = String::new();
    let mut global_compare = String::new();
    for (i, g) in globals.iter().enumerate() {
        if g.size == 0 || g.size > (1 << 20) {
            continue;
        }
        let cname = emit::c_global_name(&g.name);
        if g.pointer {
            global_reset.push_str(&format!(
                "        memset({cname}, 0, {});\n\
                 \x20       *(void **)(void *){cname} = (void *)__sir_ptr_scratch;\n",
                g.size
            ));
        } else {
            global_reset.push_str(&format!(
                "        memset({cname}, 0, {});\n\
                 \x20       if (!zero_state) {{ uint64_t gs = 0x9E3779B97F4A7C15ULL ^ (uint64_t)t ^ {i}ULL;\n\
                 \x20         for (int j = 0; j < {}; j++) {{ gs ^= gs << 13; gs ^= gs >> 7; gs ^= gs << 17; {cname}[j] = (uint8_t)(gs >> 32); }} }}\n",
                g.size, g.size
            ));
        }
        global_snapshot.push_str(&format!(
            "        static uint8_t __snap{i}[{}]; memcpy(__snap{i}, {cname}, {});\n",
            g.size, g.size
        ));
        global_compare.push_str(&format!(
            "        if (memcmp(__snap{i}, {cname}, {})) {{ if (mismatches < 5) printf(\"EMITGLOB {name} t=%d slot={i}\\n\", t); mismatches++; }}\n",
            g.size
        ));
    }
    let ret_ty = emit::c_type(&func.return_ty);
    out.push_str(&reset_buffers);
    out.push_str(&global_reset);
    out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
    let void_ref = ret_ty == "void" || llvm_c_type(ir_ret) == "void";
    if void_ref {
        out.push_str(&format!(
            "        {}__ref({});\n",
            name,
            ref_args.join(", ")
        ));
        out.push_str("        if (ref_only) continue;\n");
    } else {
        out.push_str(&format!(
            "        {} r_ref = {}__ref({});\n",
            llvm_c_type(ir_ret),
            name,
            ref_args.join(", ")
        ));
        out.push_str("        if (ref_only) continue;\n");
    }
    out.push_str(&global_snapshot);
    for slot in 0..ptr_slot {
        out.push_str(&format!(
            "        memcpy(snap[{slot}] + 32, buf[{slot}] + 32, 2048);\n"
        ));
    }
    out.push_str(&reset_buffers);
    out.push_str(&global_reset);
    out.push_str("        if (__sir_ffi_reset) __sir_ffi_reset();\n");
    if void_ref {
        out.push_str(&format!("        {}({});\n", name, new_args.join(", ")));
    } else {
        out.push_str(&format!(
            "        {} r_new = {}({});\n",
            ret_ty,
            name,
            new_args.join(", ")
        ));
        out.push_str(&format!(
            "        if ((uint64_t)r_ref != (uint64_t)r_new) {{ if (mismatches < 5) printf(\"EMITCASE {name} t=%d ref=%llu new=%llu\\n\", t, (unsigned long long)r_ref, (unsigned long long)r_new); mismatches++; }}\n"
        ));
    }
    out.push_str(&global_compare);
    out.push_str(&format!(
        "        for (int b = 0; b < {ptr_slot}; b++) if (memcmp(snap[b] + 32, buf[b] + 32, 2048)) {{ if (mismatches < 5) printf(\"EMITBUF {name} t=%d slot=%d\\n\", t, b); mismatches++; }}\n"
    ));
    out.push_str("    }\n");
    out.push_str(&format!(
        "    if (ref_only) {{ printf(\"EMITREF_OK cases={cases}\\n\"); return 0; }}\n"
    ));
    out.push_str(&format!(
        "    printf(\"EMITDIFF {name} cases={cases} mismatches=%llu\\n\", (unsigned long long)mismatches);\n"
    ));
    out.push_str("    return mismatches != 0;\n}\n");
    out
}

/// True when a loop advances a carried value by a runtime operand. Such
/// loops terminate only for a nonzero step; keep the scalar driver
/// domain nonzero so the differential compares terminating executions.
fn runtime_step_loop(func: &Function) -> bool {
    for node in func.arena.iter() {
        let NodeKind::Loop {
            carried_inputs,
            outputs,
            ..
        } = &node.kind
        else {
            continue;
        };
        for &output in outputs {
            let Some(out_node) = func.get_node(output) else {
                continue;
            };
            let NodeKind::Add { lhs, rhs } = &out_node.kind else {
                continue;
            };
            let step = if carried_inputs.contains(lhs) {
                rhs
            } else if carried_inputs.contains(rhs) {
                lhs
            } else {
                continue;
            };
            if !matches!(
                func.get_node(*step).map(|node| &node.kind),
                Some(NodeKind::Constant(_))
            ) {
                return true;
            }
        }
    }
    false
}

/// Timing driver: same argument setup as the differential driver, but times a
/// fixed repetition count of `name__ref` and `name` in one process so both see
/// identical inputs, buffers, and clock conditions.
fn build_time_driver(
    name: &str,
    left: &str,
    label: &str,
    left_is_sir: bool,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    reps: usize,
    fill: Option<u8>,
) -> String {
    let mut out = String::new();
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n\
         #include <string.h>\n#include <time.h>\n\n",
    );
    out.push_str("extern void __sir_ffi_reset(void) __attribute__((weak));\n\n");

    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
    let new_params = if new_p.is_empty() {
        "void".to_string()
    } else {
        new_p.join(", ")
    };
    let (left_ret, left_p) = if left_is_sir {
        (emit::c_type(&func.return_ty), new_p.join(", "))
    } else {
        (llvm_c_type(ir_ret), ir_p.join(", "))
    };
    out.push_str(&format!("extern {} {}({});\n\n", left_ret, left, left_p));
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));

    out.push_str(
        "static uint64_t now_ns(void) {\n\
         \x20   // Invariant TSC ticks: unlike clock_gettime wall time, this is\n\
         \x20   // not distorted by CPU frequency scaling between phases. Only\n\
         \x20   // ratios are consumed, so the historical ref_ns/new_ns labels\n\
         \x20   // are kept even though the unit is ticks.\n\
         \x20   unsigned lo, hi;\n\
         \x20   __asm__ __volatile__(\"rdtsc\" : \"=a\"(lo), \"=d\"(hi));\n\
         \x20   return ((uint64_t)hi << 32) | (uint64_t)lo;\n\
         }\n\n",
    );

    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("    int zero_state = 0, ref_only = 0;\n");
    out.push_str(
        "    for (int i = 1; i < argc; i++) {\n\
         \x20       if (!strcmp(argv[i], \"--zero\")) zero_state = 1;\n\
         \x20       if (!strcmp(argv[i], \"--ref-only\")) ref_only = 1;\n\
         \x20   }\n",
    );
    out.push_str(
        "    static _Alignas(64) uint8_t bufA[8][2048 + 32];\n\
         \x20   static _Alignas(64) uint8_t bufB[8][2048 + 32];\n\
         \x20   static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};\n\
         \x20   static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};\n\
         \x20   uint64_t seed = 0x9E3779B97F4A7C15ull;\n\
         \x20   for (int i = 0; i < 8 * (2048 + 32); i++) {\n",
    );
    match fill {
        Some(value) => out.push_str(&format!(
            "        ((uint8_t *)bufA)[i] = zero_state ? 0u : (uint8_t){value}u;\n\
             \x20       ((uint8_t *)bufB)[i] = zero_state ? 0u : (uint8_t){value}u;\n"
        )),
        None => out.push_str(
            "        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;\n\
             \x20       ((uint8_t *)bufA)[i] = zero_state ? 0u : (uint8_t)seed;\n\
             \x20       ((uint8_t *)bufB)[i] = zero_state ? 0u : (uint8_t)seed;\n",
        ),
    }
    out.push_str("    }\n");

    let mut ref_args: Vec<String> = Vec::new();
    let mut new_args: Vec<String> = Vec::new();
    let mut ptr_slot = 0usize;
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        if i >= ir_params.len() {
            new_args.push(format!(
                "({})(bufB[{}] + 32)",
                pointer_cast(&p.ty),
                ptr_slot
            ));
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            let cast = pointer_cast(&p.ty);
            ref_args.push(format!(
                "({})(bufA[{}] + 32)",
                llvm_c_type(&ir_params[i]),
                ptr_slot
            ));
            new_args.push(format!("({})(bufB[{}] + 32)", cast, ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "    {cty} s{scalar_index} = ({cty})(lens[4] + keys[{scalar_index} % 9]);\n"
            ));
            ref_args.push(format!("({})s{}", llvm_c_type(&ir_params[i]), scalar_index));
            new_args.push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }

    let left_args: Vec<String> = if left_is_sir {
        new_args.clone()
    } else {
        ref_args.clone()
    };
    let ret_ty = emit::c_type(&func.return_ty);
    if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    {}({});\n", left, left_args.join(", ")));
        out.push_str("    if (ref_only) { printf(\"EMITTIME_REF_OK\\n\"); return 0; }\n");
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    {}({});\n", name, new_args.join(", ")));
    } else {
        out.push_str(&format!(
            "    if (__sir_ffi_reset) __sir_ffi_reset();\n\
             \x20   uint64_t left_ret = (uint64_t){}({});\n\
             \x20   if (ref_only) {{ printf(\"EMITTIME_REF_OK\\n\"); return 0; }}\n\
             \x20   if (__sir_ffi_reset) __sir_ffi_reset();\n\
             \x20   uint64_t right_ret = (uint64_t){}({});\n\
             \x20   if (left_ret != right_ret) {{\n\
             \x20       printf(\"{label}_MISMATCH {name}\\n\");\n\
             \x20       return 2;\n\
             \x20   }}\n",
            left,
            left_args.join(", "),
            name,
            new_args.join(", ")
        ));
    }
    out.push_str("    volatile uint64_t sink = 0;\n");
    out.push_str("    uint64_t t0, t1, ref_ns, new_ns, b0, b1, b2, b3, b4, o0, o1, o2, o3, o4;\n");
    let ref_loop_body = if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        format!("        {}({});\n", left, left_args.join(", "))
    } else {
        format!(
            "        sink += (uint64_t){}({});\n",
            left,
            left_args.join(", ")
        )
    };
    let new_loop_body = if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        format!("        {}({});\n", name, new_args.join(", "))
    } else {
        format!(
            "        sink += (uint64_t){}({});\n",
            name,
            new_args.join(", ")
        )
    };
    out.push_str(&format!(
        "    t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b0 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o0 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o1 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b1 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b2 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o2 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b3 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o3 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b4 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o4 = t1 - t0;\n\
         \x20   ref_ns = b0 < b1 ? b0 : b1; ref_ns = ref_ns < b2 ? ref_ns : b2;\n\
         \x20   ref_ns = ref_ns < b3 ? ref_ns : b3; ref_ns = ref_ns < b4 ? ref_ns : b4;\n\
         \x20   new_ns = o0 < o1 ? o0 : o1; new_ns = new_ns < o2 ? new_ns : o2;\n\
         \x20   new_ns = new_ns < o3 ? new_ns : o3; new_ns = new_ns < o4 ? new_ns : o4;\n\
         \x20   printf(\"{label} {name} reps={reps} state=%s ref_ns=%llu new_ns=%llu sink=%llu\\n\",\n\
         \x20       zero_state ? \"zero\" : \"random\",\n\
         \x20       (unsigned long long)ref_ns, (unsigned long long)new_ns, (unsigned long long)sink);\n\
         \x20   return 0;\n}}\n"
    ));
    out
}

/// Round order for the three-way driver. Each side occupies each position
/// exactly twice, so neither warm-up nor a mid-run counter/clock shift can
/// favour one side systematically.
const ABC_ORDER: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// Three-way timing driver: A (the reference object, built from the original
/// source at the selected optimization level), B (the scalar IR with rewrites
/// off) and C (the same IR with rewrites on) measured in one process, on
/// identically seeded buffers, with the rounds interleaved.
///
/// Two separate executables cannot be compared reliably here: the absolute
/// tick counts are bimodal (the CPU sits in one of two frequency states), and
/// a run that lands its two sides in different states reports a ratio that is
/// wrong by percent -- far more than the effects under measurement. In one
/// process all three sides see the same state, so the ratio survives it.
fn build_time_driver_abc(
    name: &str,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    reps: usize,
    fill: Option<u8>,
) -> String {
    let mut out = String::new();
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n\
         #include <string.h>\n#include <time.h>\n\n",
    );
    out.push_str("extern void __sir_ffi_reset(void) __attribute__((weak));\n\n");

    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
    let new_params = if new_p.is_empty() {
        "void".to_string()
    } else {
        new_p.join(", ")
    };
    // A is declared from the IR signature (the source compiled to the object
    // has it), B and C from the emitted C's own signature.
    out.push_str(&format!(
        "extern {} {}__ref({});\n\n",
        llvm_c_type(ir_ret),
        name,
        ir_p.join(", ")
    ));
    out.push_str(&format!(
        "extern {} {}__base({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));

    out.push_str(
        "static uint64_t now_ns(void) {\n\
         \x20   unsigned lo, hi;\n\
         \x20   __asm__ __volatile__(\"rdtsc\" : \"=a\"(lo), \"=d\"(hi));\n\
         \x20   return ((uint64_t)hi << 32) | (uint64_t)lo;\n\
         }\n\n",
    );

    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("    int zero_state = 0, ref_only = 0;\n");
    out.push_str(
        "    for (int i = 1; i < argc; i++) {\n\
         \x20       if (!strcmp(argv[i], \"--zero\")) zero_state = 1;\n\
         \x20       if (!strcmp(argv[i], \"--ref-only\")) ref_only = 1;\n\
         \x20   }\n",
    );
    out.push_str(
        "    static _Alignas(64) uint8_t bufA[8][2048 + 32];\n\
         \x20   static _Alignas(64) uint8_t bufB[8][2048 + 32];\n\
         \x20   static _Alignas(64) uint8_t bufC[8][2048 + 32];\n\
         \x20   static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};\n\
         \x20   static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};\n\
         \x20   uint64_t seed = 0x9E3779B97F4A7C15ull;\n\
         \x20   for (int i = 0; i < 8 * (2048 + 32); i++) {\n",
    );
    match fill {
        Some(value) => out.push_str(&format!(
            "        ((uint8_t *)bufA)[i] = zero_state ? 0u : (uint8_t){value}u;\n\
             \x20       ((uint8_t *)bufB)[i] = zero_state ? 0u : (uint8_t){value}u;\n\
             \x20       ((uint8_t *)bufC)[i] = zero_state ? 0u : (uint8_t){value}u;\n"
        )),
        None => out.push_str(
            "        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;\n\
             \x20       ((uint8_t *)bufA)[i] = zero_state ? 0u : (uint8_t)seed;\n\
             \x20       ((uint8_t *)bufB)[i] = zero_state ? 0u : (uint8_t)seed;\n\
             \x20       ((uint8_t *)bufC)[i] = zero_state ? 0u : (uint8_t)seed;\n",
        ),
    }
    out.push_str("    }\n");

    // Arguments are built once per side; every side gets the same scalar
    // values and identically seeded buffers, at distinct addresses so no
    // side can leave state for another to consume.
    let mut args: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut ptr_slot = 0usize;
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        let is_ir_param = i < ir_params.len();
        if !is_ir_param || is_pointer_ty(&p.ty) {
            args[0].push(format!(
                "({})(bufA[{}] + 32)",
                llvm_c_type(ir_params.get(i).map(String::as_str).unwrap_or("i8*")),
                ptr_slot
            ));
            args[1].push(format!("({})(bufB[{}] + 32)", pointer_cast(&p.ty), ptr_slot));
            args[2].push(format!("({})(bufC[{}] + 32)", pointer_cast(&p.ty), ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "    {cty} s{scalar_index} = ({cty})(lens[4] + keys[{scalar_index} % 9]);\n"
            ));
            args[0].push(format!(
                "({})s{}",
                llvm_c_type(&ir_params[i]),
                scalar_index
            ));
            args[1].push(format!("s{scalar_index}"));
            args[2].push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }

    let symbols = [format!("{name}__ref"), format!("{name}__base"), name.to_string()];
    let is_void = emit::c_type(&func.return_ty) == "void" || llvm_c_type(ir_ret) == "void";
    let calls: Vec<String> = (0..3)
        .map(|side| format!("{}({})", symbols[side], args[side].join(", ")))
        .collect();

    out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
    if is_void {
        out.push_str(&format!("    {};\n", calls[0]));
        out.push_str("    if (ref_only) { printf(\"EMITTIME_REF_OK\\n\"); return 0; }\n");
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    {};\n", calls[1]));
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    {};\n", calls[2]));
    } else {
        out.push_str(&format!("    uint64_t ra = (uint64_t){};\n", calls[0]));
        out.push_str("    if (ref_only) { printf(\"EMITTIME_REF_OK\\n\"); return 0; }\n");
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    uint64_t rb = (uint64_t){};\n", calls[1]));
        out.push_str("    if (__sir_ffi_reset) __sir_ffi_reset();\n");
        out.push_str(&format!("    uint64_t rc = (uint64_t){};\n", calls[2]));
        out.push_str(
            "    if (ra != rb || ra != rc) {\n\
             \x20       printf(\"EMITTIME_ABC_MISMATCH ",
        );
        out.push_str(name);
        out.push_str(
            " a=%llu b=%llu c=%llu\\n\", (unsigned long long)ra,\n\
             \x20           (unsigned long long)rb, (unsigned long long)rc);\n\
             \x20       return 2;\n\
             \x20   }\n",
        );
    }

    out.push_str("    volatile uint64_t sink = 0;\n");
    out.push_str("    uint64_t t0, t1;\n");
    out.push_str("    uint64_t v0[6], v1[6], v2[6];\n");
    let bodies: Vec<String> = (0..3)
        .map(|side| {
            if is_void {
                format!("        {};\n", calls[side])
            } else {
                format!("        sink += (uint64_t){};\n", calls[side])
            }
        })
        .collect();
    for (round, order) in ABC_ORDER.iter().enumerate() {
        for &side in order.iter() {
            out.push_str(&format!(
                "    t0 = now_ns();\n\
                 \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{body}    }}\n\
                 \x20   t1 = now_ns(); v{side}[{round}] = t1 - t0;\n",
                body = bodies[side]
            ));
        }
    }
    out.push_str(
        "    uint64_t a_ns = v0[0], b_ns = v1[0], c_ns = v2[0];\n\
         \x20   for (int r = 1; r < 6; r++) {\n\
         \x20       if (v0[r] < a_ns) a_ns = v0[r];\n\
         \x20       if (v1[r] < b_ns) b_ns = v1[r];\n\
         \x20       if (v2[r] < c_ns) c_ns = v2[r];\n\
         \x20   }\n\
         \x20   printf(\"EMITTIME_ABC ",
    );
    out.push_str(name);
    out.push_str(
        " reps=",
    );
    out.push_str(&format!("{reps}"));
    out.push_str(
        " state=%s a_ns=%llu b_ns=%llu c_ns=%llu sink=%llu\\n\",\n\
         \x20       zero_state ? \"zero\" : \"random\",\n\
         \x20       (unsigned long long)a_ns, (unsigned long long)b_ns,\n\
         \x20       (unsigned long long)c_ns, (unsigned long long)sink);\n",
    );
    for (round, order) in ABC_ORDER.iter().enumerate() {
        out.push_str(&format!(
            "    printf(\"EMITTIME_ABC_ROUND {name} {round} %llu %llu %llu\\n\",\n\
             \x20       (unsigned long long)v{}[{round}], (unsigned long long)v{}[{round}],\n\
             \x20       (unsigned long long)v{}[{round}]);\n",
            order[0], order[1], order[2]
        ));
    }
    out.push_str("    return 0;\n}\n");
    out
}

/// Guard band appended to every injected shadow. The driver only ever
/// addresses the declared size, so an out-of-range read lands in this
/// never-written tail instead of the driver's own scratch.
const GUARD_BYTES: usize = 256;

/// Define C-side buffers for the internal globals a function references.
/// The reference IR keeps its own definition; the emitted C gets a
/// namespaced buffer with identical initial contents. With `seed_init` the
/// buffer starts from the module's real initializer data -- the timing
/// driver never resets globals, so both sides must read the same table --
/// and shapes that cannot be reproduced faithfully are refused instead of
/// zero-seeded. The differential driver refills globals per case, so it
/// seeds nothing and cannot hit that refusal.
fn inject_globals(
    driver: &mut String,
    ll: &str,
    names: &[String],
    seed_init: bool,
) -> Result<(), String> {
    let globals = global_decls(ll);
    let mut seen = HashSet::new();
    let mut defs = String::new();
    for name in names {
        let Some(body) = function_ir(ll, name) else {
            continue;
        };
        for referenced in referenced_globals(&body) {
            if !seen.insert(referenced.clone()) {
                continue;
            }
            let Some(g) = globals.iter().find(|g| g.name == referenced) else {
                continue;
            };
            if g.size == 0 || g.size > (1 << 20) {
                continue;
            }
            let init = if seed_init {
                match initializer_bytes(g)? {
                    Some(bytes) => format!(
                        "{{{}}}",
                        bytes
                            .iter()
                            .map(|byte| byte.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    ),
                    None => "{0}".to_string(),
                }
            } else {
                "{0}".to_string()
            };
            defs.push_str(&format!(
                "_Alignas(64) uint8_t {}[{} + {GUARD_BYTES}] = {init};\n",
                emit::c_global_name(&g.name),
                g.size
            ));
        }
    }
    let pos = driver.find("int main").unwrap_or(driver.len());
    driver.insert_str(pos, &defs);
    Ok(())
}

/// Build and run the timing harness; returns the `EMITTIME` line on success.
fn time_function(
    ll: &str,
    name: &str,
    workdir: &Path,
    reps: usize,
    baseline: bool,
    fill: Option<u8>,
    cc: &str,
    target: &str,
) -> Result<String, String> {
    if let Some(reason) = unsupported_global_use(ll, name) {
        return Err(reason);
    }
    if !direct_callees(ll, name)?.is_empty() {
        return Err("timing: local calls are not supported yet".to_string());
    }
    let func =
        lower_function(ll, name).map_err(|e| e.lines().last().unwrap_or(&e).trim().to_string())?;
    let Some((ir_ret, ir_params)) = ir_signature(ll, name) else {
        return Err("could not parse IR signature".to_string());
    };
    let optimized = optimize(&func);
    let c_source = emit::emit_c(&optimized.function);
    let dir = workdir.join(name);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let (mut driver, extra_source, extra_is_ir, inject_ref_globals) = if abc_mode() {
        // A / B / C in one executable: the reference object built from the
        // original source, the unoptimized emission, and the optimized
        // emission, all linked by the same backend with the same flags. Only
        // the ABSAC stage differs between the three.
        let Some(object) = ref_object() else {
            return Err("--time-abc needs --ref-object PATH".to_string());
        };
        let base_c = emit::emit_c(&func);
        let renamed = base_c.replace(&format!("{name}("), &format!("{name}__base("));
        let base_path = dir.join("baseline.c");
        fs::write(&base_path, &renamed).map_err(|e| format!("write baseline.c: {e}"))?;
        (
            build_time_driver_abc(name, &optimized.function, &ir_ret, &ir_params, reps, fill),
            object,
            false,
            false,
        )
    } else if baseline {
        let base_c = emit::emit_c(&func);
        let renamed = base_c.replace(&format!("{name}("), &format!("{name}__base("));
        let base_path = dir.join("baseline.c");
        fs::write(&base_path, &renamed).map_err(|e| format!("write baseline.c: {e}"))?;
        (
            build_time_driver(
                name,
                &format!("{name}__base"),
                "EMITTIME_BASE",
                true,
                &optimized.function,
                &ir_ret,
                &ir_params,
                reps,
                fill,
            ),
            base_path,
            false,
            true,
        )
    } else if let Some(object) = ref_object() {
        // Reference built from the original source at the selected
        // optimization level, linked as an object. Both sides are then
        // compiled by the same backend with the same flags; the object
        // carries its own tables, so the driver injects nothing and both
        // sides read the same globals.
        (
            build_time_driver(
                name,
                &format!("{name}__ref"),
                "EMITTIME",
                false,
                &optimized.function,
                &ir_ret,
                &ir_params,
                reps,
                fill,
            ),
            object,
            false,
            false,
        )
    } else {
        // The differential path shadows the reference's globals so both
        // sides read the driver's injected buffer; the timing driver must
        // do the same, or the reference reads the module's real table while
        // the emitted C reads the injected one.
        let globals = used_globals_all(ll, &[name.to_string()]);
        let ref_ll = expose_definition(
            &rename_symbol(&shadow_globals(ll, &globals), name, "__ref"),
            &format!("{name}__ref"),
        );
        let ref_path = dir.join("ref.ll");
        fs::write(&ref_path, &ref_ll).map_err(|e| format!("write ref.ll: {e}"))?;
        (
            build_time_driver(
                name,
                &format!("{name}__ref"),
                "EMITTIME",
                false,
                &optimized.function,
                &ir_ret,
                &ir_params,
                reps,
                fill,
            ),
            ref_path,
            true,
            true,
        )
    };
    if inject_ref_globals {
        inject_globals(&mut driver, ll, &[name.to_string()], true)?;
    }

    let emitted_path = dir.join("emitted.c");
    let driver_path = dir.join("driver_time.c");
    let exe_path = dir.join("time_exe");
    fs::write(&emitted_path, &c_source).map_err(|e| format!("write emitted.c: {e}"))?;
    fs::write(&driver_path, &driver).map_err(|e| format!("write driver_time.c: {e}"))?;

    let mut extra_c = Vec::new();
    if let Some(support) = ffi_support() {
        extra_c.push(support);
    }
    if abc_mode() {
        // The B side travels as an ordinary translation unit, compiled by the
        // same backend invocation, at the same level, as the driver and the
        // optimized emission.
        extra_c.push(dir.join("baseline.c"));
    }
    compile_harness(
        cc,
        false,
        &exe_path,
        &driver_path,
        &emitted_path,
        &extra_c,
        &extra_source,
        extra_is_ir,
        target,
    )?;
    // The differential path probes the reference under `--ref-only` and
    // falls back to the zero state when the random one faults; synthetic
    // pointer arguments can put a data-dependent index out of the object it
    // points into. A crash is not an evidence state, so the timing path
    // uses the same probe and fallback.
    let ref_ok = |output: &std::process::Output| {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("EMITTIME_REF_OK")
    };
    let zero_mode = match run_binary(&exe_path, target, &["--ref-only"]) {
        Ok(output) if ref_ok(&output) => None,
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            return Err(format!("time run did not terminate: {e}"));
        }
        _ => match run_binary(&exe_path, target, &["--ref-only", "--zero"]) {
            Ok(output) if ref_ok(&output) => Some(true),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(format!("time run did not terminate: {e}"));
            }
            _ => {
                return Err(
                    "invalid-state input: reference faults under random and zero initialization"
                        .to_string(),
                );
            }
        },
    };
    let run_args: &[&str] = if zero_mode.is_some() { &["--zero"] } else { &[] };
    let run = run_binary(&exe_path, target, run_args).map_err(|e| format!("run: {e}"))?;
    let stdout = String::from_utf8_lossy(&run.stdout);
    if !run.status.success() {
        return Err(format!(
            "time harness exit {:?}: {}",
            run.status.code(),
            stdout.lines().next().unwrap_or("")
        ));
    }
    let line = stdout
        .lines()
        .find(|l| l.starts_with("EMITTIME"))
        .map(|s| s.to_string())
        .ok_or_else(|| "no EMITTIME line".to_string())?;
    if abc_mode() {
        // Keep the raw per-round ticks alongside the minima; a claim about a
        // ratio has to name the numbers it came from.
        let all: Vec<&str> = stdout
            .lines()
            .filter(|l| l.starts_with("EMITTIME_ABC"))
            .collect();
        return Ok(all.join("\n"));
    }
    Ok(line)
}
/// Compile the differential/timing harness with the selected C compiler.
///
/// `extra` is either the original IR (`extra_is_ir`) or an additional C
/// translation unit. GCC cannot read LLVM IR, so the IR is compiled to an
/// object with clang first and linked with the GCC-built driver + emitted C.
fn compile_harness(
    cc: &str,
    sanitize: bool,
    exe: &Path,
    driver: &Path,
    emitted: &Path,
    extra_c: &[PathBuf],
    extra: &Path,
    extra_is_ir: bool,
    target: &str,
) -> Result<(), String> {
    if target == "aarch64" && sanitize {
        return Err("aarch64 target does not support sanitizers".to_string());
    }
    let opt_level = std::env::var("SIR_OPT_LEVEL").unwrap_or_else(|_| "1".to_string());
    let opt_flag = format!("-O{opt_level}");
    let sanitizer_args: &[&str] = if sanitize {
        &[
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-g",
        ]
    } else {
        &[]
    };
    let cross = target == "aarch64";
    let output = if cross || cc == "gcc" {
        let linker = if cross {
            "aarch64-linux-gnu-gcc"
        } else {
            "gcc"
        };
        let mut objects = Vec::new();
        if extra_is_ir {
            let object = extra.with_extension("o");
            let mut clang = Command::new("clang");
            clang.args([
                opt_flag.as_str(),
                "-w",
                "-falign-functions=64",
                "-ffunction-sections",
                "-fdata-sections",
                "-c",
            ]);
            clang.args(sanitizer_args);
            if cross {
                clang.arg("--target=aarch64-linux-gnu");
            }
            clang.args([extra.to_str().unwrap(), "-o", object.to_str().unwrap()]);
            let compiled = clang
                .output()
                .map_err(|e| format!("clang IR object: {e}"))?;
            if !compiled.status.success() {
                return Err(format!(
                    "clang IR object: {}",
                    String::from_utf8_lossy(&compiled.stderr)
                        .lines()
                        .next()
                        .unwrap_or("")
                ));
            }
            objects.push(object);
        } else {
            objects.push(extra.to_path_buf());
        }
        let mut gcc = Command::new(linker);
        gcc.args([
            opt_flag.as_str(),
            "-w",
            "-falign-functions=64",
            "-Wl,--gc-sections",
        ]);
        if cross {
            gcc.arg("-static");
        }
        gcc.args(["-o", exe.to_str().unwrap()]);
        gcc.args(sanitizer_args);
        gcc.arg(driver.to_str().unwrap());
        gcc.arg(emitted.to_str().unwrap());
        for source in extra_c {
            gcc.arg(source.to_str().unwrap());
        }
        for object in &objects {
            gcc.arg(object.to_str().unwrap());
        }
        gcc.output().map_err(|e| format!("gcc: {e}"))?
    } else {
        let mut clang = Command::new("clang");
        clang.args([
            opt_flag.as_str(),
            "-w",
            "-falign-functions=64",
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
            "-o",
            exe.to_str().unwrap(),
        ]);
        clang.args(sanitizer_args);
        clang.args([driver.to_str().unwrap(), emitted.to_str().unwrap()]);
        for source in extra_c {
            clang.arg(source.to_str().unwrap());
        }
        clang.arg(extra.to_str().unwrap());
        clang.output().map_err(|e| format!("clang: {e}"))?
    };
    if !output.status.success() {
        return Err(format!(
            "compile: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or("")
        ));
    }
    Ok(())
}

/// Run the optimizer with stdout suppressed (it prints debug traces).
struct RunResult {
    function: Function,
    rewrites_applied: usize,
    definitions: Vec<u64>,
}

/// Run the single-pass optimizer.
fn optimize(func: &Function) -> RunResult {
    let fallback = |definition: u64| {
        if let Some(definitions) = ALLOW_DEFS.get() {
            definitions.contains(&definition)
        } else if ALLOW_RUNTIME_SEARCH.load(Ordering::Relaxed) {
            true
        } else {
            sir_opt::family::allow_profiled(definition)
        }
    };
    let result = if let Some(profile) = PER_FUNCTION_ALLOW.get() {
        sir_opt::optimize(func, &|definition| {
            profile
                .get(&func.name)
                .and_then(|definitions| definitions.get(&definition))
                .copied()
                .unwrap_or_else(|| fallback(definition))
        })
    } else if let Some(definitions) = ALLOW_DEFS.get() {
        sir_opt::optimize(func, &|definition| definitions.contains(&definition))
    } else if ALLOW_RUNTIME_SEARCH.load(Ordering::Relaxed) {
        sir_opt::optimize(func, &sir_opt::family::allow_all)
    } else {
        sir_opt::optimize(func, &sir_opt::family::allow_profiled)
    };
    if let Some((function, rewrites, definitions)) = result {
        if std::env::var_os("SIR_OPT_TRACE").is_some() {
            for definition in &definitions {
                eprintln!("SIR_OPT {} def={definition}", func.name);
            }
        }
        RunResult {
            function,
            rewrites_applied: rewrites,
            definitions,
        }
    } else {
        RunResult {
            function: func.clone(),
            rewrites_applied: 0,
            definitions: Vec::new(),
        }
    }
}

enum Outcome {
    Clean {
        cases: usize,
        rewrites: usize,
        /// The definitions the optimizer applied, in order. Carried here
        /// rather than only through the timing path so the firing record
        /// survives a timing failure.
        definitions: Vec<u64>,
    },
    Mismatch {
        cases: usize,
        mismatches: usize,
        rewrites: usize,
    },
    Sanitizer {
        reason: String,
    },
    Refused {
        reason: String,
    },
    HarnessError {
        reason: String,
    },
}

fn run_function(
    ll: &str,
    name: &str,
    cases: usize,
    workdir: &Path,
    sanitize: bool,
    cc: &str,
    target: &str,
) -> Outcome {
    let module = match lower_module(ll, name) {
        Ok(module) => module,
        Err(reason) => return Outcome::Refused { reason },
    };
    for (original, _) in &module {
        if let Some(reason) = unsupported_global_use(ll, original) {
            return Outcome::Refused { reason };
        }
    }
    let Some((ir_ret, ir_params)) = ir_signature(ll, name) else {
        return Outcome::HarnessError {
            reason: "could not parse IR signature".to_string(),
        };
    };
    let optimized_module: Vec<(String, RunResult)> = module
        .iter()
        .map(|(original, func)| (original.clone(), optimize(func)))
        .collect();
    let (_, optimized) = optimized_module
        .last()
        .expect("module always contains its root");
    let c_source = emit::emit_c(&optimized.function);
    let module_names: Vec<String> = module
        .iter()
        .map(|(original, _)| original.clone())
        .collect();

    let dir: PathBuf = workdir.join(name);
    if fs::create_dir_all(&dir).is_err() {
        return Outcome::HarnessError {
            reason: format!("cannot create {}", dir.display()),
        };
    }
    let globals = used_globals_all(ll, &module_names);
    let ref_ll = expose_definition(
        &rename_symbol(&shadow_globals(ll, &globals), name, "__ref"),
        &format!("{name}__ref"),
    );
    let low_bits = function_ir(ll, name)
        .map(|function| function.contains("!range"))
        .unwrap_or(false);
    let same_address = ir_ret.trim() == "ptr"
        || optimized_module.iter().any(|(original, _)| {
            function_ir(ll, original)
                .map(|body| body.contains("load ptr") || body.contains("store ptr"))
                .unwrap_or(false)
        });
    // Zero is a defined input unless *this* function's IR carries `undef`.
    // A module-wide match used to disable the zero case for every function in
    // the module, which hid the crc32pseudo zero-length divergence from the
    // referee: the timing driver always exercised zero while the differential
    // never did. The reference-only `--zero` probe already classifies genuinely
    // invalid zero states as refusals, so the differential may always compare
    // the zero case. Runtime-step loops still avoid zero: a zero stride can
    // hang the driver rather than fail it.
    let avoid_zero = function_ir(ll, name)
        .map(|body| body.contains(" undef"))
        .unwrap_or(true)
        || runtime_step_loop(&optimized.function);
    let driver = if same_address {
        build_same_address_driver(
            name,
            &optimized.function,
            &ir_ret,
            &ir_params,
            cases,
            low_bits,
            &globals,
            avoid_zero,
        )
    } else {
        build_driver(
            name,
            &optimized.function,
            &ir_ret,
            &ir_params,
            cases,
            low_bits,
            &globals,
            avoid_zero,
        )
    };

    let mut driver = driver;
    if let Err(reason) = inject_globals(&mut driver, ll, &module_names, false) {
        return Outcome::HarnessError { reason };
    }

    let ref_path = dir.join("ref.ll");
    let driver_path = dir.join("driver.c");
    let exe_path = dir.join("exe");
    let mut emitted_paths = Vec::new();
    let count = optimized_module.len();
    for (index, (_, result)) in optimized_module.iter().enumerate() {
        let path = if index + 1 == count {
            dir.join("emitted.c")
        } else {
            dir.join(format!("callee_{index}_{}.c", result.function.name))
        };
        let source = if index + 1 == count {
            c_source.clone()
        } else {
            emit::emit_c(&result.function)
        };
        if fs::write(&path, source).is_err() {
            return Outcome::HarnessError {
                reason: format!("cannot write {}", path.display()),
            };
        }
        emitted_paths.push(path);
    }
    if fs::write(&ref_path, &ref_ll).is_err() || fs::write(&driver_path, &driver).is_err() {
        return Outcome::HarnessError {
            reason: "cannot write harness sources".to_string(),
        };
    }
    let root_path = emitted_paths.pop().expect("module has a root");
    if let Some(support) = ffi_support() {
        emitted_paths.push(support);
    }

    // Native assurance hardening: under `sanitize` the ref/emitted pair runs
    // with ASan+UBSan and no recovery, so undefined behaviour aborts instead
    // of silently matching a wrong result.
    if let Err(reason) = compile_harness(
        cc,
        sanitize,
        &exe_path,
        &driver_path,
        &root_path,
        &emitted_paths,
        &ref_path,
        true,
        target,
    ) {
        return Outcome::HarnessError { reason };
    }

    let ref_ok = |output: &std::process::Output| {
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("EMITREF_OK")
    };
    let zero_mode = match run_binary(&exe_path, target, &["--ref-only"]) {
        Ok(output) if ref_ok(&output) => None,
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            return Outcome::Refused {
                reason: format!("run did not terminate: {e}"),
            };
        }
        _ => match run_binary(&exe_path, target, &["--ref-only", "--zero"]) {
            Ok(output) if ref_ok(&output) => Some(true),
            _ => {
                return Outcome::Refused {
                    reason:
                        "invalid-state input: reference faults under random and zero initialization"
                            .to_string(),
                };
            }
        },
    };
    let run_args: &[&str] = if zero_mode.is_some() {
        &["--zero"]
    } else {
        &[]
    };
    let run = match run_binary(&exe_path, target, run_args) {
        Ok(o) => o,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::TimedOut {
                return Outcome::Refused {
                    reason: format!("run did not terminate: {e}"),
                };
            }
            return Outcome::HarnessError {
                reason: format!("run: {e}"),
            };
        }
    };
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    if sanitize {
        let stderr = String::from_utf8_lossy(&run.stderr).to_string();
        if stderr.contains("runtime error:")
            || stderr.contains("AddressSanitizer")
            || stderr.contains("UndefinedBehaviorSanitizer")
        {
            let reason = stderr
                .lines()
                .find(|l| {
                    l.contains("runtime error:")
                        || l.contains("AddressSanitizer")
                        || l.contains("UndefinedBehaviorSanitizer")
                })
                .unwrap_or("sanitizer report")
                .to_string();
            return Outcome::Sanitizer { reason };
        }
    }
    let summary = stdout
        .lines()
        .find(|l| l.starts_with("EMITDIFF "))
        .map(|s| s.to_string());
    match summary {
        Some(line) => {
            let mismatches = line
                .split("mismatches=")
                .nth(1)
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(usize::MAX);
            if mismatches == 0 {
                Outcome::Clean {
                    cases,
                    rewrites: optimized.rewrites_applied,
                    definitions: optimized.definitions.clone(),
                }
            } else {
                Outcome::Mismatch {
                    cases,
                    mismatches,
                    rewrites: optimized.rewrites_applied,
                }
            }
        }
        None => Outcome::HarnessError {
            reason: format!(
                "no summary (status {:?}): {}",
                run.status.code(),
                stdout.lines().next().unwrap_or("")
            ),
        },
    }
}

fn ns_value(line: &str, key: &str) -> Option<u64> {
    let prefix = format!("{key}=");
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(&prefix)?.parse().ok())
}

/// Combine two retention measurements of the same definition.
///
/// `Some(retained)` is a completed comparison, `None` is a firing whose
/// timing did not complete. A measurement is kept over an unknown, and two
/// measurements keep the existing "retained everywhere it was measured"
/// conjunction.
fn merge_retention(previous: Option<bool>, measured: Option<bool>) -> Option<bool> {
    match (previous, measured) {
        (Some(previous), Some(measured)) => Some(previous && measured),
        (Some(previous), None) => Some(previous),
        (None, measured) => measured,
    }
}

fn run_binary(exe: &Path, target: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut command = if target == "aarch64" {
        let mut command = Command::new("qemu-aarch64");
        command.arg(exe);
        command.args(args);
        command
    } else {
        let mut command = Command::new(exe);
        command.args(args);
        command
    };
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let timeout = std::env::var("SIR_RUN_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10);
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }
        if start.elapsed() >= Duration::from_secs(timeout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("timeout after {timeout}s"),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Slice out one textual LLVM function body, used to detect input-domain
/// metadata such as `!range` before generating differential inputs.
fn function_ir<'a>(ll: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("@{name}(");
    let mut offset = 0usize;
    for line in ll.split_inclusive('\n') {
        if line.trim_start().starts_with("define ") && line.contains(&needle) {
            let rest = &ll[offset..];
            let end = rest
                .find("\n}\n")
                .map(|index| index + 3)
                .unwrap_or(rest.len());
            return Some(&rest[..end]);
        }
        offset += line.len();
    }
    None
}

/// Rename every exact `@name` symbol reference, including calls,
/// function-pointer arrays, and metadata such as `llvm.compiler.used`.
/// Longer identifiers sharing the prefix are left untouched.
fn rename_symbol(text: &str, name: &str, suffix: &str) -> String {
    let needle = format!("@{name}");
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(position) = rest.find(&needle) {
        out.push_str(&rest[..position]);
        out.push_str(&needle);
        let after = &rest[position + needle.len()..];
        let continues_identifier = after
            .chars()
            .next()
            .map(|c| c.is_ascii_alphanumeric() || c == '_')
            .unwrap_or(false);
        if !continues_identifier {
            out.push_str(suffix);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Drop `internal`/`private` from the selected definition so the renamed
/// reference symbol is visible to the C driver.
fn expose_definition(text: &str, name: &str) -> String {
    let needle = format!("@{name}(");
    text.lines()
        .map(|line| {
            if line.trim_start().starts_with("define ") && line.contains(&needle) {
                line.replace("internal ", "").replace("private ", "")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: emit_c_diff <corpus.ll> [--cases N] [--workdir DIR] [--function NAME] [--sanitize] [--cc clang|gcc] [--target native|aarch64] [--allow-runtime-search] [--allow-defs LIST] [--ffi-support PATH] [--ref-object PATH] [--profile-in PATH] [--profile-out PATH] [--time] [--time-reps N] [--time-baseline] [--time-abc] [--measured-retain] [--time-fill V]"
        );
        std::process::exit(2);
    }
    let corpus = &args[1];
    let mut cases = 24usize;
    let mut workdir = std::env::temp_dir().join("emit_c_diff");
    let mut only: Option<String> = None;
    let mut sanitize = false;
    let mut time_mode = false;
    let mut time_baseline = false;
    let mut measured_retain = false;
    let mut profile_out: Option<PathBuf> = None;
    let mut time_fill: Option<u8> = None;
    let mut time_reps = 200_000usize;
    let mut cc = "clang".to_string();
    let mut target = "native".to_string();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--cases" if i + 1 < args.len() => {
                cases = args[i + 1].parse().unwrap_or(24);
                i += 2;
            }
            "--workdir" if i + 1 < args.len() => {
                workdir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--function" if i + 1 < args.len() => {
                only = Some(args[i + 1].clone());
                i += 2;
            }
            "--sanitize" => {
                sanitize = true;
                i += 1;
            }
            "--time" => {
                time_mode = true;
                i += 1;
            }
            "--time-baseline" => {
                time_mode = true;
                time_baseline = true;
                i += 1;
            }
            "--time-abc" => {
                time_mode = true;
                TIME_ABC.store(true, Ordering::Relaxed);
                i += 1;
            }
            "--measured-retain" => {
                time_mode = true;
                measured_retain = true;
                i += 1;
            }
            "--allow-runtime-search" => {
                ALLOW_RUNTIME_SEARCH.store(true, Ordering::Relaxed);
                i += 1;
            }
            "--ffi-support" if i + 1 < args.len() => {
                let _ = FFI_SUPPORT.set(Some(PathBuf::from(&args[i + 1])));
                i += 2;
            }
            "--ref-object" if i + 1 < args.len() => {
                let _ = REF_OBJECT.set(Some(PathBuf::from(&args[i + 1])));
                i += 2;
            }
            "--allow-defs" if i + 1 < args.len() => {
                let definitions: Vec<u64> = args[i + 1]
                    .split(',')
                    .filter_map(|value| value.trim().parse().ok())
                    .collect();
                let _ = ALLOW_DEFS.set(definitions);
                i += 2;
            }
            "--profile-out" if i + 1 < args.len() => {
                profile_out = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--profile-in" if i + 1 < args.len() => {
                let path = &args[i + 1];
                let Ok(text) = fs::read_to_string(path) else {
                    eprintln!("cannot read profile {path}");
                    std::process::exit(2);
                };
                let mut global = Vec::new();
                let mut per_function: BTreeMap<String, BTreeMap<u64, bool>> = BTreeMap::new();
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    match parts.as_slice() {
                        [definition, allow] => {
                            if let Ok(definition) = definition.parse() {
                                if *allow != "0" {
                                    global.push(definition);
                                }
                            }
                        }
                        [function, definition, "unknown"] => {
                            // Fired without a completed comparison: recorded
                            // in the profile, but it carries no retention
                            // decision, so it leaves the policy alone.
                            let _ = (function, definition);
                        }
                        [function, definition, allow] => {
                            if let Ok(definition) = definition.parse() {
                                per_function
                                    .entry((*function).to_string())
                                    .or_default()
                                    .insert(definition, *allow != "0");
                            }
                        }
                        _ => {}
                    }
                }
                let _ = ALLOW_DEFS.set(global);
                let _ = PER_FUNCTION_ALLOW.set(per_function);
                i += 2;
            }
            "--time-reps" if i + 1 < args.len() => {
                time_reps = args[i + 1].parse().unwrap_or(200_000);
                i += 2;
            }
            "--time-fill" if i + 1 < args.len() => {
                let raw = args[i + 1].trim();
                time_fill = if let Some(hex) = raw.strip_prefix("0x") {
                    u8::from_str_radix(hex, 16).ok()
                } else {
                    raw.parse::<u8>().ok()
                };
                i += 2;
            }
            "--cc" if i + 1 < args.len() => {
                match args[i + 1].as_str() {
                    "clang" | "gcc" => cc = args[i + 1].clone(),
                    other => {
                        eprintln!("unsupported C compiler: {other} (expected clang or gcc)");
                        std::process::exit(2);
                    }
                }
                i += 2;
            }
            "--target" if i + 1 < args.len() => {
                match args[i + 1].as_str() {
                    "native" | "aarch64" => target = args[i + 1].clone(),
                    other => {
                        eprintln!("unsupported target: {other} (expected native or aarch64)");
                        std::process::exit(2);
                    }
                }
                i += 2;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let ll = match fs::read_to_string(corpus) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {corpus}: {e}");
            std::process::exit(2);
        }
    };

    let mut clean = 0usize;
    let mut mismatched = 0usize;
    let mut refused = 0usize;
    let mut errors = 0usize;
    let mut violations = 0usize;
    let mut retained_base = 0usize;
    let mut retention_profile: BTreeMap<String, BTreeMap<u64, Option<bool>>> = BTreeMap::new();
    for name in function_names(&ll) {
        if let Some(only) = &only {
            if &name != only {
                continue;
            }
        }
        match run_function(&ll, &name, cases, &workdir, sanitize, &cc, &target) {
            Outcome::Clean {
                cases,
                rewrites,
                definitions,
            } => {
                println!("NATIVE {name}: clean cases={cases} rewrites={rewrites}");
                clean += 1;
                if time_mode {
                    match time_function(
                        &ll,
                        &name,
                        &workdir,
                        time_reps,
                        time_baseline,
                        time_fill,
                        &cc,
                        &target,
                    ) {
                        Ok(line) => {
                            println!("{line}");
                            if measured_retain {
                                let ref_ns = ns_value(&line, "ref_ns");
                                let new_ns = ns_value(&line, "new_ns");
                                let retain = match (ref_ns, new_ns) {
                                    (Some(base), Some(new)) if new > base => {
                                        retained_base += 1;
                                        "base"
                                    }
                                    _ => "new",
                                };
                                println!(
                                    "EMITRETAIN {name} retain={retain} ref_ns={} new_ns={}",
                                    ref_ns.unwrap_or(0),
                                    new_ns.unwrap_or(0)
                                );
                                let retained = retain == "new";
                                for definition in &definitions {
                                    retention_profile
                                        .entry(name.clone())
                                        .or_default()
                                        .entry(*definition)
                                        .and_modify(|retention| {
                                            *retention =
                                                merge_retention(*retention, Some(retained));
                                        })
                                        .or_insert(Some(retained));
                                }
                            }
                        }
                        Err(reason) => {
                            println!("EMITTIME {name} error={reason}");
                            errors += 1;
                            if measured_retain {
                                // The rewrites fired even though the timing
                                // comparison did not complete: keep them in
                                // the profile as entries with no ratio, so
                                // the profile stays a firing record.
                                for definition in &definitions {
                                    retention_profile
                                        .entry(name.clone())
                                        .or_default()
                                        .entry(*definition)
                                        .or_insert(None);
                                }
                            }
                        }
                    }
                }
            }
            Outcome::Mismatch {
                cases,
                mismatches,
                rewrites,
            } => {
                println!(
                    "NATIVE {name}: MISMATCH cases={cases} mismatches={mismatches} rewrites={rewrites}"
                );
                mismatched += 1;
            }
            Outcome::Refused { reason } => {
                println!("NATIVE {name}: lower_refused ({reason})");
                refused += 1;
            }
            Outcome::Sanitizer { reason } => {
                println!("NATIVE {name}: SANITIZER ({reason})");
                violations += 1;
            }
            Outcome::HarnessError { reason } => {
                println!("NATIVE {name}: HARNESS_ERROR ({reason})");
                errors += 1;
            }
        }
    }
    if sanitize {
        println!("# sanitize=address,undefined");
        println!("EMITC_SANITIZE violations={violations}");
    }
    if measured_retain {
        let mut profile = String::new();
        for (function, definitions) in &retention_profile {
            for (definition, retention) in definitions {
                let allow = match retention {
                    Some(retained) => u8::from(*retained).to_string(),
                    // Fired, but the timing comparison did not complete, so
                    // there is no ratio to retain on. Recorded rather than
                    // dropped: the profile is a firing record.
                    None => "unknown".to_string(),
                };
                println!("EMITPROFILE function={function} def={definition} allow={allow}");
                profile.push_str(&format!("{function} {definition} {allow}\n"));
            }
        }
        if let Some(path) = &profile_out {
            if let Err(error) = fs::write(path, profile) {
                eprintln!("cannot write profile {}: {error}", path.display());
                errors += 1;
            }
        }
    }
    let retention = if measured_retain {
        format!(" retained_base={retained_base}")
    } else {
        String::new()
    };
    println!(
        "EMITC_SUMMARY clean={clean} mismatched={mismatched} lower_refused={refused} harness_errors={errors}{retention}"
    );
    std::process::exit(if mismatched == 0 && errors == 0 && violations == 0 {
        0
    } else {
        1
    });
}
