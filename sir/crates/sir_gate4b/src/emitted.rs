//! Strict recognizer for the committed generated C artifacts.
//!
//! The proof must be about the *actual* emitted code. This module
//! reads `gate4/*.c` from disk and matches it against fail-closed
//! templates: every recognized line is recorded with its source span,
//! and anything that does not match (an extra statement, a reordered
//! intrinsic, a different constant) is a parse error. The file digest
//! and the matched spans travel into the proof artifact so a replay can
//! prove the proof was about these bytes.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// Parameter kind in an emitted kernel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParamType {
    BufPtr,
    U64,
    U8,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::BufPtr => write!(f, "const uint8_t*"),
            ParamType::U64 => write!(f, "uint64_t"),
            ParamType::U8 => write!(f, "uint8_t"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CParam {
    pub name: String,
    pub ty: ParamType,
}

/// Which verified kernel family the file is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelKind {
    /// `All`: every byte equals `val`.
    AllEquality { buf: String, n: String, val: String },
    /// `Cardinality`: count bytes with `(b & mask) == target`.
    CardinalityMasked {
        buf: String,
        n: String,
        mask: String,
        target: String,
    },
    /// `Sum`: sum of all bytes.
    SumAscii { buf: String, n: String },
}

impl KernelKind {
    pub fn name(&self) -> &'static str {
        match self {
            KernelKind::AllEquality { .. } => "AllEquality",
            KernelKind::CardinalityMasked { .. } => "CardinalityMasked",
            KernelKind::SumAscii { .. } => "SumAscii",
        }
    }

    /// The roles the recognizer bound, in the order the theorem uses
    /// them: (role, C identifier).
    pub fn roles(&self) -> Vec<(&'static str, String)> {
        match self {
            KernelKind::AllEquality { buf, n, val } => vec![
                ("buffer", buf.clone()),
                ("length", n.clone()),
                ("value", val.clone()),
            ],
            KernelKind::CardinalityMasked {
                buf,
                n,
                mask,
                target,
            } => vec![
                ("buffer", buf.clone()),
                ("length", n.clone()),
                ("mask", mask.clone()),
                ("target", target.clone()),
            ],
            KernelKind::SumAscii { buf, n } => {
                vec![("buffer", buf.clone()), ("length", n.clone())]
            }
        }
    }
}

/// One recognized source line (normalized whitespace) with its span in
/// the original file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub first_line: usize,
    pub last_line: usize,
    pub text: String,
}

/// A parsed, recognized generated kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedKernel {
    pub path: String,
    /// FNV-1a 64 of the raw file bytes.
    pub digest: u64,
    /// SHA-256-style hex would need a dependency; FNV plus the exact
    /// span texts is what the replay checks.
    pub function: String,
    pub params: Vec<CParam>,
    pub kind: KernelKind,
    pub spans: Vec<Span>,
    pub total_lines: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Io(String),
    Signature { line: usize, text: String },
    Mismatch {
        line: usize,
        expected: String,
        actual: String,
    },
    Trailing { line: usize, text: String },
    UnknownShape,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(m) => write!(f, "io: {m}"),
            ParseError::Signature { line, text } => {
                write!(f, "unrecognized signature at line {line}: {text}")
            }
            ParseError::Mismatch {
                line,
                expected,
                actual,
            } => write!(
                f,
                "generated artifact differs at line {line}: expected `{expected}`, found `{actual}`"
            ),
            ParseError::Trailing { line, text } => {
                write!(f, "unrecognized trailing statement at line {line}: {text}")
            }
            ParseError::UnknownShape => write!(f, "no kernel template matched"),
        }
    }
}

impl std::error::Error for ParseError {}

/// FNV-1a 64-bit digest (same construction as the repository's
/// `sir_verification` diagnostic digests).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Parse a generated kernel from raw text.
pub fn parse(path: &str, text: &str) -> Result<EmittedKernel, ParseError> {
    let raw_lines: Vec<&str> = text.lines().collect();
    let lines = normalize_lines(&raw_lines);
    let digest = fnv1a64(text.as_bytes());

    let mut best_error: Option<ParseError> = None;
    for shape in [Shape::All, Shape::Cardinality, Shape::Sum] {
        match match_shape(shape, &lines) {
            Ok(kernel) => {
                return Ok(EmittedKernel {
                    path: path.to_string(),
                    digest,
                    function: kernel.function,
                    params: kernel.params,
                    kind: kernel.kind,
                    spans: kernel.spans,
                    total_lines: raw_lines.len(),
                })
            }
            Err(e) => {
                let better = match (&best_error, &e) {
                    (None, _) => true,
                    (Some(old), new) => error_line(new) > error_line(old),
                };
                if better {
                    best_error = Some(e);
                }
            }
        }
    }
    Err(best_error.unwrap_or(ParseError::UnknownShape))
}

fn error_line(e: &ParseError) -> usize {
    match e {
        ParseError::Signature { line, .. }
        | ParseError::Mismatch { line, .. }
        | ParseError::Trailing { line, .. } => *line,
        _ => 0,
    }
}

/// Read and parse a generated kernel from disk.
pub fn parse_file(path: impl AsRef<Path>) -> Result<EmittedKernel, ParseError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|e| ParseError::Io(e.to_string()))?;
    parse(&path.display().to_string(), &text)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    All,
    Cardinality,
    Sum,
}

struct Matched {
    function: String,
    params: Vec<CParam>,
    kind: KernelKind,
    spans: Vec<Span>,
}

/// A normalized logical line: continuation lines are joined so that a
/// multi-line statement matches one pattern.
#[derive(Clone, Debug)]
struct LogicalLine {
    first: usize,
    last: usize,
    text: String,
}

fn normalize_lines(raw: &[&str]) -> Vec<LogicalLine> {
    let mut out: Vec<LogicalLine> = Vec::new();
    let mut i = 0usize;
    while i < raw.len() {
        let first = i + 1;
        let mut text = collapse_ws(raw[i]);
        if text.is_empty() {
            i += 1;
            continue;
        }
        let mut last = first;
        while i + 1 < raw.len() {
            let next = collapse_ws(raw[i + 1]);
            if next.is_empty() {
                break;
            }
            if !(continues(&text) || starts_with_operator(&next)) {
                break;
            }
            text.push(' ');
            text.push_str(&next);
            i += 1;
            last = i + 1;
        }
        out.push(LogicalLine { first, last, text });
        i += 1;
    }
    out
}

fn collapse_ws(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn continues(text: &str) -> bool {
    text.ends_with('+')
        || text.ends_with('-')
        || text.ends_with('*')
        || text.ends_with('/')
        || text.ends_with(',')
        || text.ends_with('(')
}

/// Continuation lines in the emitted dialect start with a binary
/// operator (the emitter wraps long sums that way).
fn starts_with_operator(text: &str) -> bool {
    text.starts_with("+ ")
        || text.starts_with("- ")
        || text.starts_with("* ")
        || text.starts_with("/ ")
        || text.starts_with("&&")
        || text.starts_with("||")
        || text.starts_with(")")
}

fn match_shape(shape: Shape, lines: &[LogicalLine]) -> Result<Matched, ParseError> {
    let mut m = Matcher::new(lines);
    let signature = m.peek().ok_or(ParseError::UnknownShape)?.clone();
    if !signature.text.starts_with("uint64_t ") || !signature.text.ends_with('{') {
        return Err(ParseError::Signature {
            line: signature.first,
            text: signature.text.clone(),
        });
    }
    m.consume(&shape_signature_pattern(shape))?;
    let (function, params) = parse_signature(&signature.text)?;

    match shape {
        Shape::All => match_all(&mut m, &function, &params),
        Shape::Cardinality => match_cardinality(&mut m, &function, &params),
        Shape::Sum => match_sum(&mut m, &function, &params),
    }
}

fn parse_signature(text: &str) -> Result<(String, Vec<CParam>), ParseError> {
    // uint64_t name(const uint8_t * p0, uint64_t p1, ...) {
    let open = text.find('(').ok_or_else(|| ParseError::Signature {
        line: 0,
        text: text.to_string(),
    })?;
    let close = text.rfind(')').ok_or_else(|| ParseError::Signature {
        line: 0,
        text: text.to_string(),
    })?;
    let function = text["uint64_t ".len()..open].trim().to_string();
    let params_text = &text[open + 1..close];
    let mut params = Vec::new();
    for part in params_text.split(',') {
        let part = part.trim();
        let (ty, name) = if let Some(rest) = part.strip_prefix("const uint8_t * ") {
            (ParamType::BufPtr, rest.trim())
        } else if let Some(rest) = part.strip_prefix("uint64_t ") {
            (ParamType::U64, rest.trim())
        } else if let Some(rest) = part.strip_prefix("uint8_t ") {
            (ParamType::U8, rest.trim())
        } else {
            return Err(ParseError::Signature {
                line: 0,
                text: part.to_string(),
            });
        };
        params.push(CParam {
            name: name.to_string(),
            ty,
        });
    }
    Ok((function, params))
}

fn shape_signature_pattern(shape: Shape) -> String {
    match shape {
        Shape::All => {
            "uint64_t {fn}(const uint8_t * {buf}, uint64_t {n}, uint8_t {val}) {".to_string()
        }
        Shape::Cardinality => {
            "uint64_t {fn}(const uint8_t * {buf}, uint64_t {n}, uint8_t {mask}, uint8_t {target}) {"
                .to_string()
        }
        Shape::Sum => "uint64_t {fn}(const uint8_t * {buf}, uint64_t {n}) {".to_string(),
    }
}

struct Matcher<'a> {
    lines: &'a [LogicalLine],
    pos: usize,
    captures: HashMap<String, String>,
    spans: Vec<Span>,
}

impl<'a> Matcher<'a> {
    fn new(lines: &'a [LogicalLine]) -> Self {
        Matcher {
            lines,
            pos: 0,
            captures: HashMap::new(),
            spans: Vec::new(),
        }
    }

    fn peek(&self) -> Option<&LogicalLine> {
        self.lines.get(self.pos)
    }

    fn consume(&mut self, pattern: &str) -> Result<(), ParseError> {
        let line = self.lines.get(self.pos).ok_or(ParseError::Mismatch {
            line: 0,
            expected: pattern.to_string(),
            actual: "<eof>".to_string(),
        })?;
        if match_pattern(pattern, &line.text, &mut self.captures) {
            self.spans.push(Span {
                first_line: line.first,
                last_line: line.last,
                text: line.text.clone(),
            });
            self.pos += 1;
            Ok(())
        } else {
            Err(ParseError::Mismatch {
                line: line.first,
                expected: pattern.to_string(),
                actual: line.text.clone(),
            })
        }
    }

    fn expect_end(&self) -> Result<(), ParseError> {
        match self.lines.get(self.pos) {
            None => Ok(()),
            Some(line) => Err(ParseError::Trailing {
                line: line.first,
                text: line.text.clone(),
            }),
        }
    }

    fn cap(&self, key: &str) -> String {
        self.captures
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("capture {key} missing"))
    }
}

/// Match `pattern` against `line`, binding `{name}` placeholders to
/// identifier tokens. Repeated placeholders must match the same text.
fn match_pattern(pattern: &str, line: &str, captures: &mut HashMap<String, String>) -> bool {
    let mut pat_rest = pattern;
    let mut line_rest = line;
    while let Some(open) = next_placeholder(pat_rest) {
        let literal = &pat_rest[..open];
        if !line_rest.starts_with(literal) {
            return false;
        }
        line_rest = &line_rest[literal.len()..];
        pat_rest = &pat_rest[open..];
        let close = match pat_rest.find('}') {
            Some(c) => c,
            None => return false,
        };
        let key = &pat_rest[1..close];
        pat_rest = &pat_rest[close + 1..];
        // Identifier token.
        // Identifiers are ASCII; `count()` is the byte length here.
        let token_len = line_rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
        if token_len == 0 {
            return false;
        }
        let token = &line_rest[..token_len];
        if !token
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        {
            return false;
        }
        match captures.get(key) {
            Some(bound) if bound != token => return false,
            Some(_) => {}
            None => {
                captures.insert(key.to_string(), token.to_string());
            }
        }
        line_rest = &line_rest[token_len..];
    }
    line_rest == pat_rest
}

/// Position of the next `{ident}` placeholder. A bare `{` (the C block
/// opener that ends many patterns) is literal text, not a placeholder.
fn next_placeholder(pattern: &str) -> Option<usize> {
    let bytes = pattern.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'{' {
            continue;
        }
        if let Some(close) = pattern[i + 1..].find('}') {
            let inner = &pattern[i + 1..i + 1 + close];
            if !inner.is_empty()
                && inner
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Some(i);
            }
        }
    }
    None
}

fn match_all(m: &mut Matcher<'_>, _function: &str, _params: &[CParam]) -> Result<Matched, ParseError> {
    m.consume("__m256i target_v = _mm256_set1_epi8((char){val});")?;
    m.consume("uint64_t i = 0;")?;
    m.consume("while (i + 32 <= {n}) {")?;
    m.consume("__m256i chunk = _mm256_loadu_si256((__m256i *)({buf} + i));")?;
    m.consume("__m256i cmp = _mm256_cmpeq_epi8(chunk, target_v);")?;
    m.consume("int mask = _mm256_movemask_epi8(cmp);")?;
    m.consume("if (mask != -1) return 0;")?;
    m.consume("i += 32;")?;
    m.consume("}")?;
    m.consume("while (i + 16 <= {n}) {")?;
    m.consume("__m128i chunk = _mm_loadu_si128((__m128i *)({buf} + i));")?;
    m.consume("__m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char){val}));")?;
    m.consume("int mask = _mm_movemask_epi8(cmp);")?;
    m.consume("if (mask != 0xFFFF) return 0;")?;
    m.consume("i += 16;")?;
    m.consume("}")?;
    m.consume("while (i < {n}) {")?;
    m.consume("if ({buf}[i] != {val}) return 0;")?;
    m.consume("i++;")?;
    m.consume("}")?;
    m.consume("return 1;")?;
    m.consume("}")?;
    m.expect_end()?;
    let kind = KernelKind::AllEquality {
        buf: m.cap("buf"),
        n: m.cap("n"),
        val: m.cap("val"),
    };
    Ok(finish(m, _function, _params, kind))
}

fn match_cardinality(
    m: &mut Matcher<'_>,
    _function: &str,
    _params: &[CParam],
) -> Result<Matched, ParseError> {
    m.consume("__m256i mask_v = _mm256_set1_epi8((char){mask});")?;
    m.consume("__m256i target_v = _mm256_set1_epi8((char){target});")?;
    m.consume("uint64_t count = 0;")?;
    m.consume("uint64_t i = 0;")?;
    m.consume("while (i + 32 <= {n}) {")?;
    m.consume("__m256i chunk = _mm256_loadu_si256((__m256i *)({buf} + i));")?;
    m.consume("__m256i masked = _mm256_and_si256(chunk, mask_v);")?;
    m.consume("__m256i cmp = _mm256_cmpeq_epi8(masked, target_v);")?;
    m.consume("int bits = _mm256_movemask_epi8(cmp);")?;
    m.consume("count += __builtin_popcount(bits);")?;
    m.consume("i += 32;")?;
    m.consume("}")?;
    m.consume("while (i + 16 <= {n}) {")?;
    m.consume("__m128i chunk = _mm_loadu_si128((__m128i *)({buf} + i));")?;
    m.consume("__m128i masked = _mm_and_si128(chunk, _mm_set1_epi8((char){mask}));")?;
    m.consume("__m128i cmp = _mm_cmpeq_epi8(masked, _mm_set1_epi8((char){target}));")?;
    m.consume("int bits = _mm_movemask_epi8(cmp);")?;
    m.consume("count += __builtin_popcount(bits);")?;
    m.consume("i += 16;")?;
    m.consume("}")?;
    m.consume("while (i < {n}) {")?;
    m.consume("count += (({buf}[i] & {mask}) == {target});")?;
    m.consume("i++;")?;
    m.consume("}")?;
    m.consume("return count;")?;
    m.consume("}")?;
    m.expect_end()?;
    let kind = KernelKind::CardinalityMasked {
        buf: m.cap("buf"),
        n: m.cap("n"),
        mask: m.cap("mask"),
        target: m.cap("target"),
    };
    Ok(finish(m, _function, _params, kind))
}

fn match_sum(m: &mut Matcher<'_>, _function: &str, _params: &[CParam]) -> Result<Matched, ParseError> {
    m.consume("__m256i zero = _mm256_setzero_si256();")?;
    m.consume("__m256i acc = zero;")?;
    m.consume("uint64_t i = 0;")?;
    m.consume("while (i + 32 <= {n}) {")?;
    m.consume("__m256i chunk = _mm256_loadu_si256((__m256i *)({buf} + i));")?;
    m.consume("__m256i sums = _mm256_sad_epu8(chunk, zero);")?;
    m.consume("acc = _mm256_add_epi64(acc, sums);")?;
    m.consume("i += 32;")?;
    m.consume("}")?;
    m.consume("__m128i lo = _mm256_castsi256_si128(acc);")?;
    m.consume("__m128i hi = _mm256_extracti128_si256(acc, 1);")?;
    m.consume(
        "uint64_t sum = (uint64_t)_mm_cvtsi128_si64(lo) + (uint64_t)_mm_extract_epi64(lo, 1) \
         + (uint64_t)_mm_cvtsi128_si64(hi) + (uint64_t)_mm_extract_epi64(hi, 1);",
    )?;
    m.consume("while (i + 16 <= {n}) {")?;
    m.consume("__m128i chunk = _mm_loadu_si128((__m128i *)({buf} + i));")?;
    m.consume("__m128i sums = _mm_sad_epu8(chunk, _mm_setzero_si128());")?;
    m.consume("sum += (uint64_t)_mm_cvtsi128_si64(sums) + (uint64_t)_mm_extract_epi64(sums, 1);")?;
    m.consume("i += 16;")?;
    m.consume("}")?;
    m.consume("while (i < {n}) {")?;
    m.consume("sum += {buf}[i];")?;
    m.consume("i++;")?;
    m.consume("}")?;
    m.consume("return sum;")?;
    m.consume("}")?;
    m.expect_end()?;
    let kind = KernelKind::SumAscii {
        buf: m.cap("buf"),
        n: m.cap("n"),
    };
    Ok(finish(m, _function, _params, kind))
}

fn finish(m: &Matcher<'_>, function: &str, params: &[CParam], kind: KernelKind) -> Matched {
    Matched {
        function: function.to_string(),
        params: params.to_vec(),
        kind,
        spans: m.spans.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_path(rel: &str) -> std::path::PathBuf {
        let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // crates/sir_gate4b -> repository root
        dir.pop();
        dir.pop();
        dir.pop();
        dir.join(rel)
    }

    #[test]
    fn matches_committed_k50() {
        let path = repo_path("gate4/k50_absac.c");
        let kernel = parse_file(&path).expect("k50 recognized");
        assert_eq!(kernel.function, "k50_count_masked");
        assert_eq!(
            kernel.kind,
            KernelKind::CardinalityMasked {
                buf: "p0".into(),
                n: "p1".into(),
                mask: "p2".into(),
                target: "p3".into(),
            }
        );
        assert_eq!(kernel.params.len(), 4);
        assert_eq!(kernel.params[0].ty, ParamType::BufPtr);
        // Every statement line was matched and recorded.
        assert!(kernel.spans.len() > 20);
    }

    #[test]
    fn matches_committed_k18() {
        let path = repo_path("gate4/k18_absac.c");
        let kernel = parse_file(&path).expect("k18 recognized");
        assert_eq!(kernel.function, "k18_all_equal");
        assert_eq!(
            kernel.kind,
            KernelKind::AllEquality {
                buf: "p0".into(),
                n: "p1".into(),
                val: "p2".into(),
            }
        );
    }

    #[test]
    fn matches_committed_k43() {
        let path = repo_path("gate4/k43_absac.c");
        let kernel = parse_file(&path).expect("k43 recognized");
        assert_eq!(kernel.function, "k43_sum_ascii");
        assert_eq!(
            kernel.kind,
            KernelKind::SumAscii {
                buf: "p0".into(),
                n: "p1".into(),
            }
        );
    }

    #[test]
    fn rejects_modified_source() {
        let path = repo_path("gate4/k50_absac.c");
        let text = std::fs::read_to_string(&path).expect("read");
        let mutated = text.replace("i + 32 <= p1", "i + 31 <= p1");
        assert!(parse("mutated.c", &mutated).is_err());
        let mutated = text.replace("count += __builtin_popcount(bits);", "count += bits;");
        assert!(parse("mutated.c", &mutated).is_err());
    }

    #[test]
    fn digest_changes_with_content() {
        let a = fnv1a64(b"abc");
        let b = fnv1a64(b"abd");
        assert_ne!(a, b);
    }

    #[test]
    fn signature_pattern_matches() {
        let mut caps = HashMap::new();
        let ok = match_pattern(
            "uint64_t {fn}(const uint8_t * {buf}, uint64_t {n}, uint8_t {val}) {",
            "uint64_t k18_all_equal(const uint8_t * p0, uint64_t p1, uint8_t p2) {",
            &mut caps,
        );
        assert!(ok, "captures: {caps:?}");
        assert_eq!(caps.get("fn").map(String::as_str), Some("k18_all_equal"));
        assert_eq!(caps.get("buf").map(String::as_str), Some("p0"));
    }
}
