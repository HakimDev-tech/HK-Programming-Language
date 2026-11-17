//! Experimental, source-spanned front-end model for the HK 0.5 grammar.
//!
//! This module is deliberately separate from the executable 0.4 parser until
//! declaration collection, type checking, bytecode lowering, and VM support
//! can be integrated atomically. It provides the lossless lexical contract and
//! recursive type-expression model used by the staged 0.5 parser.
use std::ops::Range;

/// Keywords introduced by HK 0.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum V05Keyword {
    Struct,
    Enum,
    Trait,
    Impl,
    Match,
}

/// Punctuation needed by nominal declarations and generic syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum V05Punctuation {
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    DoubleColon,
    Less,
    Greater,
    Plus,
    Equal,
    Arrow,
    Underscore,
}

/// A token retains its exact UTF-8 byte range in the original source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05TokenKind {
    Identifier(String),
    Keyword(V05Keyword),
    Punctuation(V05Punctuation),
    Number(String),
    StringLiteral(String),
    ExistingSyntax(String),
}

/// A source-spanned token. Ranges are half-open UTF-8 byte offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Token {
    pub kind: V05TokenKind,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Lexes the 0.5 declaration/generic vocabulary while preserving existing
/// expression tokens for the established 0.4 lexer/parser to interpret later.
pub fn lex_v05(source: &str) -> Result<Vec<V05Token>, V05LexError> {
    let bytes = source.as_bytes();
    let mut offset = 0;
    let mut line = 1;
    let mut column = 1;
    let mut tokens = Vec::new();

    while offset < bytes.len() {
        let start = offset;
        let start_line = line;
        let start_column = column;
        let ch = source[offset..].chars().next().expect("valid UTF-8 boundary");

        if ch.is_whitespace() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            continue;
        }
        if ch == '/' && bytes.get(offset + 1) == Some(&b'/') {
            while offset < bytes.len() {
                let current = source[offset..].chars().next().expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if current == '\n' { break; }
            }
            continue;
        }

        let kind = if ch == '_' || ch.is_ascii_alphabetic() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            while offset < bytes.len() {
                let next = source[offset..].chars().next().expect("valid UTF-8 boundary");
                if next == '_' || next.is_ascii_alphanumeric() {
                    advance_position(next, &mut offset, &mut line, &mut column);
                } else {
                    break;
                }
            }
            let text = &source[start..offset];
            match text {
                "struct" => V05TokenKind::Keyword(V05Keyword::Struct),
                "enum" => V05TokenKind::Keyword(V05Keyword::Enum),
                "trait" => V05TokenKind::Keyword(V05Keyword::Trait),
                "impl" => V05TokenKind::Keyword(V05Keyword::Impl),
                "match" => V05TokenKind::Keyword(V05Keyword::Match),
                "_" => V05TokenKind::Punctuation(V05Punctuation::Underscore),
                _ => V05TokenKind::Identifier(text.to_owned()),
            }
        } else if ch.is_ascii_digit() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                advance_position(source[offset..].chars().next().expect("ASCII digit"), &mut offset, &mut line, &mut column);
            }
            V05TokenKind::Number(source[start..offset].to_owned())
        } else if ch == '"' {
            advance_position(ch, &mut offset, &mut line, &mut column);
            let mut escaped = false;
            let mut closed = false;
            while offset < bytes.len() {
                let current = source[offset..].chars().next().expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if escaped { escaped = false; }
                else if current == '\\\\' { escaped = true; }
                else if current == '"' { closed = true; break; }
            }
            if !closed {
                return Err(V05LexError { message: "Unterminated string literal.".into(), span: start..offset, line: start_line, column: start_column });
            }
            V05TokenKind::StringLiteral(source[start..offset].to_owned())
        } else {
            let (punctuation, width) = match source[start..].chars().take(2).collect::<String>().as_str() {
                "::" => (Some(V05Punctuation::DoubleColon), 2),
                "->" => (Some(V05Punctuation::Arrow), 2),
                _ => (None, 0),
            };
            if let Some(punctuation) = punctuation {
                for _ in 0..width {
                    let current = source[offset..].chars().next().expect("valid UTF-8 boundary");
                    advance_position(current, &mut offset, &mut line, &mut column);
                }
                V05TokenKind::Punctuation(punctuation)
            } else if let Some(punctuation) = single_punctuation(ch) {
                advance_position(ch, &mut offset, &mut line, &mut column);
                V05TokenKind::Punctuation(punctuation)
            } else if "{}()[]:;,.<>+=-*/%!&|?".contains(ch) {
                advance_position(ch, &mut offset, &mut line, &mut column);
                V05TokenKind::ExistingSyntax(ch.to_string())
            } else {
                return Err(V05LexError { message: format!("Unexpected character '{ch}'."), span: start..start + ch.len_utf8(), line: start_line, column: start_column });
            }
        };

        tokens.push(V05Token { kind, span: start..offset, line: start_line, column: start_column });
    }

    Ok(tokens)
}

fn single_punctuation(ch: char) -> Option<V05Punctuation> {
    Some(match ch {
        '{' => V05Punctuation::LeftBrace,
        '}' => V05Punctuation::RightBrace,
        '(' => V05Punctuation::LeftParen,
        ')' => V05Punctuation::RightParen,
        '[' => V05Punctuation::LeftBracket,
        ']' => V05Punctuation::RightBracket,
        ':' => V05Punctuation::Colon,
        ';' => V05Punctuation::Semicolon,
        ',' => V05Punctuation::Comma,
        '.' => V05Punctuation::Dot,
        '<' => V05Punctuation::Less,
        '>' => V05Punctuation::Greater,
        '+' => V05Punctuation::Plus,
        '=' => V05Punctuation::Equal,
        _ => return None,
    })
}

fn advance_position(ch: char, offset: &mut usize, line: &mut usize, column: &mut usize) {
    *offset += ch.len_utf8();
    if ch == '\n' { *line += 1; *column = 1; }
    else { *column += 1; }
}

/// A recursive type expression; semantic resolution is intentionally deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    Named { name: String, arguments: Vec<TypeExpr>, span: Range<usize> },
    Any { span: Range<usize> },
}

/// A bounded trait constraint such as `T: Display + Debug`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericBound {
    pub parameter: String,
    pub traits: Vec<String>,
    pub span: Range<usize>,
}

/// A lexical failure with stable source coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05LexError {
    pub message: String,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_v05_keywords_and_generic_punctuation() {
        let tokens = lex_v05("struct Box<T> { value: T } enum Result<T, E> { Ok(T) } impl<T> Box<T> {} trait Show {} match value {}").unwrap();
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Struct)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Enum)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Impl)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Trait)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Match)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Less)));
        assert!(tokens.iter().any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Greater)));
    }

    #[test]
    fn preserves_utf8_byte_spans_and_line_columns() {
        let source = "// commentaire\nstruct Café {}";
        let tokens = lex_v05(source).unwrap();
        let cafe = tokens.iter().find(|t| matches!(&t.kind, V05TokenKind::Identifier(name) if name == "Cafe")).unwrap();
        assert_eq!(cafe.line, 2);
        assert_eq!(cafe.column, 8);
        assert_eq!(&source[cafe.span.clone()], "Cafe");
    }

    #[test]
    fn reports_unterminated_strings_without_panicking() {
        let error = lex_v05("struct A { name: \"unfinished").unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 19);
        assert!(error.message.contains("Unterminated"));
    }

    #[test]
    fn token_spans_are_valid_utf8_boundaries() {
        let source = "struct Point { x: number }";
        for token in lex_v05(source).unwrap() {
            assert_eq!(&source[token.span.clone()].len(), &(token.span.end - token.span.start));
        }
    }
}
