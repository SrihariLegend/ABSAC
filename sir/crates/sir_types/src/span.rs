use serde::{Deserialize, Serialize};

/// A position within a source file.
///
/// Fields track line, column, and byte offset from the start of the file.
/// Lines and columns are 1-indexed; offset is 0-indexed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number (byte column, not grapheme column).
    pub column: usize,
    /// 0-indexed byte offset from the start of the file.
    pub offset: usize,
}

impl Position {
    /// Create a new position. All fields default to 0 (unknown location).
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    /// Return a position representing an unknown location.
    pub fn unknown() -> Self {
        Self {
            line: 0,
            column: 0,
            offset: 0,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A span covering a range of source text.
///
/// Spans are half-open: `start` is inclusive, `end` is exclusive.
/// For a span covering `offset 0..5`, `start.offset = 0`, `end.offset = 5`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// The start of the span (inclusive).
    pub start: Position,
    /// The end of the span (exclusive).
    pub end: Position,
}

impl Span {
    /// Create a new span from start and end positions.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a span representing an unknown source location.
    pub fn unknown() -> Self {
        Self {
            start: Position::unknown(),
            end: Position::unknown(),
        }
    }

    /// Return the byte length of this span.
    pub fn len(&self) -> usize {
        self.end.offset.saturating_sub(self.start.offset)
    }

    /// Return true if this span covers zero bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_display() {
        let pos = Position::new(1, 10, 9);
        assert_eq!(format!("{pos}"), "1:10");
    }

    #[test]
    fn position_unknown() {
        let pos = Position::unknown();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn span_length() {
        let span = Span::new(
            Position::new(1, 1, 0),
            Position::new(1, 5, 4),
        );
        assert_eq!(span.len(), 4);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_empty() {
        let span = Span::new(
            Position::new(1, 1, 5),
            Position::new(1, 1, 5),
        );
        assert!(span.is_empty());
    }

    #[test]
    fn span_display() {
        let span = Span::new(
            Position::new(1, 1, 0),
            Position::new(1, 5, 4),
        );
        assert_eq!(format!("{span}"), "1:1-1:5");
    }

    #[test]
    fn span_unknown() {
        let span = Span::unknown();
        assert_eq!(format!("{span}"), "0:0-0:0");
    }

    #[test]
    fn serde_roundtrip() {
        let span = Span::new(
            Position::new(2, 3, 10),
            Position::new(2, 8, 15),
        );
        let json = serde_json::to_string(&span).unwrap();
        let parsed: Span = serde_json::from_str(&json).unwrap();
        assert_eq!(span, parsed);
    }
}
