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
        let ch = source[offset..]
            .chars()
            .next()
            .expect("valid UTF-8 boundary");

        if ch.is_whitespace() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            continue;
        }
        if ch == '/' && bytes.get(offset + 1) == Some(&b'/') {
            while offset < bytes.len() {
                let current = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if current == '\n' {
                    break;
                }
            }
            continue;
        }

        let kind = if ch == '_' || ch.is_ascii_alphabetic() {
            advance_position(ch, &mut offset, &mut line, &mut column);
            while offset < bytes.len() {
                let next = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
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
                advance_position(
                    source[offset..].chars().next().expect("ASCII digit"),
                    &mut offset,
                    &mut line,
                    &mut column,
                );
            }
            V05TokenKind::Number(source[start..offset].to_owned())
        } else if ch == '"' {
            advance_position(ch, &mut offset, &mut line, &mut column);
            let mut escaped = false;
            let mut closed = false;
            while offset < bytes.len() {
                let current = source[offset..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                advance_position(current, &mut offset, &mut line, &mut column);
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == '"' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(V05LexError {
                    message: "Unterminated string literal.".into(),
                    span: start..offset,
                    line: start_line,
                    column: start_column,
                });
            }
            V05TokenKind::StringLiteral(source[start..offset].to_owned())
        } else {
            let (punctuation, width) =
                match source[start..].chars().take(2).collect::<String>().as_str() {
                    "::" => (Some(V05Punctuation::DoubleColon), 2),
                    "->" => (Some(V05Punctuation::Arrow), 2),
                    _ => (None, 0),
                };
            if let Some(punctuation) = punctuation {
                for _ in 0..width {
                    let current = source[offset..]
                        .chars()
                        .next()
                        .expect("valid UTF-8 boundary");
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
                return Err(V05LexError {
                    message: format!("Unexpected character '{ch}'."),
                    span: start..start + ch.len_utf8(),
                    line: start_line,
                    column: start_column,
                });
            }
        };

        tokens.push(V05Token {
            kind,
            span: start..offset,
            line: start_line,
            column: start_column,
        });
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
    if ch == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

/// A recursive type expression; semantic resolution is intentionally deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    Named {
        name: String,
        arguments: Vec<TypeExpr>,
        span: Range<usize>,
    },
    Any {
        span: Range<usize>,
    },
}

/// A bounded trait constraint such as `T: Display + Debug`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericBound {
    pub parameter: String,
    pub traits: Vec<String>,
    pub span: Range<usize>,
}

/// A parser failure with a precise token location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05ParseError {
    pub message: String,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Parses one recursive type expression at `*cursor` and advances the cursor
/// only after successfully consuming the complete expression.
pub fn parse_type_expr(tokens: &[V05Token], cursor: &mut usize) -> Result<TypeExpr, V05ParseError> {
    let original = *cursor;
    match parse_type_expr_at(tokens, cursor, 0) {
        Ok(ty) => Ok(ty),
        Err(error) => {
            *cursor = original;
            Err(error)
        }
    }
}

fn parse_type_expr_at(
    tokens: &[V05Token],
    cursor: &mut usize,
    depth: usize,
) -> Result<TypeExpr, V05ParseError> {
    if depth >= 64 {
        return Err(parse_error(
            tokens,
            *cursor,
            "Maximum type-expression nesting depth (64) exceeded.",
        ));
    }

    let token = tokens
        .get(*cursor)
        .ok_or_else(|| parse_error(tokens, *cursor, "Expected a type name."))?;
    let name = match &token.kind {
        V05TokenKind::Identifier(name) => name.clone(),
        _ => return Err(parse_error(tokens, *cursor, "Expected a type name.")),
    };
    *cursor += 1;

    if name == "any" {
        return Ok(TypeExpr::Any {
            span: token.span.clone(),
        });
    }

    let mut arguments = Vec::new();
    if is_punctuation(tokens.get(*cursor), V05Punctuation::Less) {
        *cursor += 1;
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Generic argument lists cannot be empty.",
            ));
        }
        loop {
            arguments.push(parse_type_expr_at(tokens, cursor, depth + 1)?);
            if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
                *cursor += 1;
                if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
                    return Err(parse_error(tokens, *cursor, "Expected a type after ','."));
                }
            } else {
                break;
            }
        }
        if !is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
            return Err(parse_error(
                tokens,
                *cursor,
                "Expected '>' after generic arguments.",
            ));
        }
        *cursor += 1;
    }

    let end = tokens
        .get(cursor.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(token.span.end);
    Ok(TypeExpr::Named {
        name,
        arguments,
        span: token.span.start..end,
    })
}

/// Parses generic parameters in `<T: Trait + Debug, U>` form.
pub fn parse_generic_parameters(
    tokens: &[V05Token],
    cursor: &mut usize,
) -> Result<Vec<GenericBound>, V05ParseError> {
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::Less) {
        return Ok(Vec::new());
    }
    *cursor += 1;
    let mut parameters = Vec::new();
    loop {
        let start = *cursor;
        let token = tokens
            .get(*cursor)
            .ok_or_else(|| parse_error(tokens, *cursor, "Expected a generic parameter."))?;
        let parameter = match &token.kind {
            V05TokenKind::Identifier(name) => name.clone(),
            _ => {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Expected a generic parameter name.",
                ))
            }
        };
        *cursor += 1;
        let mut traits = Vec::new();
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Colon) {
            *cursor += 1;
            loop {
                let bound = tokens
                    .get(*cursor)
                    .ok_or_else(|| parse_error(tokens, *cursor, "Expected a trait bound."))?;
                match &bound.kind {
                    V05TokenKind::Identifier(name) => traits.push(name.clone()),
                    _ => {
                        return Err(parse_error(
                            tokens,
                            *cursor,
                            "Expected a trait name in generic bound.",
                        ))
                    }
                }
                *cursor += 1;
                if is_punctuation(tokens.get(*cursor), V05Punctuation::Plus) {
                    *cursor += 1;
                } else {
                    break;
                }
            }
        }
        let end = tokens
            .get(cursor.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(token.span.end);
        parameters.push(GenericBound {
            parameter,
            traits,
            span: token.span.start..end,
        });
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Comma) {
            *cursor += 1;
            if is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
                return Err(parse_error(
                    tokens,
                    *cursor,
                    "Expected a generic parameter after ','.",
                ));
            }
        } else {
            break;
        }
        if *cursor - start > 256 {
            return Err(parse_error(
                tokens,
                *cursor,
                "Generic parameter list exceeds the supported limit.",
            ));
        }
    }
    if !is_punctuation(tokens.get(*cursor), V05Punctuation::Greater) {
        return Err(parse_error(
            tokens,
            *cursor,
            "Expected '>' after generic parameters.",
        ));
    }
    *cursor += 1;
    Ok(parameters)
}

fn is_punctuation(token: Option<&V05Token>, expected: V05Punctuation) -> bool {
    matches!(token.map(|t| &t.kind), Some(V05TokenKind::Punctuation(actual)) if *actual == expected)
}

fn parse_error(tokens: &[V05Token], cursor: usize, message: &str) -> V05ParseError {
    let token = tokens.get(cursor).or_else(|| tokens.last());
    let (span, line, column) = token
        .map(|t| (t.span.clone(), t.line, t.column))
        .unwrap_or((0..0, 1, 1));
    V05ParseError {
        message: message.into(),
        span,
        line,
        column,
    }
}

/// A lexical failure with stable source coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05LexError {
    pub message: String,
    pub span: Range<usize>,
    pub line: usize,
    pub column: usize,
}


/// Parsed field of a nominal struct declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Range<usize>,
}

/// Struct declaration AST. Semantic resolution (identity, duplicate names across
/// modules, bounds and field type validity) is deliberately a later compiler phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05StructDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub fields: Vec<V05Field>,
    pub span: Range<usize>,
}

/// Enum variant with unit or positional payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05EnumVariant {
    pub name: String,
    pub payload: Vec<TypeExpr>,
    pub span: Range<usize>,
}

/// Enum declaration AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05EnumDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub variants: Vec<V05EnumVariant>,
    pub span: Range<usize>,
}

/// The supported declaration subset parsed by this staged parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05Declaration {
    Struct(V05StructDecl),
    Enum(V05EnumDecl),
}

/// Parse consecutive struct and enum declarations. The parser rejects unsupported
/// top-level constructs instead of silently skipping tokens. Cursor rollback is
/// guaranteed on failure.
pub fn parse_nominal_declarations(tokens: &[V05Token]) -> Result<Vec<V05Declaration>, V05ParseError> {
    let mut parser = V05NominalParser { tokens, cursor: 0 };
    let mut declarations = Vec::new();
    while parser.cursor < tokens.len() {
        let start = parser.cursor;
        let declaration = match parser.peek_keyword() {
            Some(V05Keyword::Struct) => parser.parse_struct().map(V05Declaration::Struct),
            Some(V05Keyword::Enum) => parser.parse_enum().map(V05Declaration::Enum),
            _ => Err(parser.error("Expected a struct or enum declaration.")),
        };
        match declaration {
            Ok(value) => declarations.push(value),
            Err(error) => {
                parser.cursor = start;
                return Err(error);
            }
        }
    }
    Ok(declarations)
}

struct V05NominalParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05NominalParser<'_> {
    fn peek_keyword(&self) -> Option<V05Keyword> {
        match self.tokens.get(self.cursor).map(|t| &t.kind) {
            Some(V05TokenKind::Keyword(keyword)) => Some(*keyword),
            _ => None,
        }
    }

    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_keyword(&mut self, keyword: V05Keyword, message: &str) -> Result<(), V05ParseError> {
        if self.peek_keyword() == Some(keyword) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn expect_punctuation(&mut self, punctuation: V05Punctuation, message: &str) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn identifier(&mut self, message: &str) -> Result<(String, Range<usize>), V05ParseError> {
        let token = self.tokens.get(self.cursor).ok_or_else(|| self.error(message))?;
        if let V05TokenKind::Identifier(name) = &token.kind {
            let value = name.clone();
            let span = token.span.clone();
            self.cursor += 1;
            Ok((value, span))
        } else {
            Err(self.error(message))
        }
    }

    fn parse_struct(&mut self) -> Result<V05StructDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.expect_keyword(V05Keyword::Struct, "Expected 'struct'.")?;
        let (name, name_span) = self.identifier("Expected struct name.")?;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected '{' before struct fields.")?;
        let mut fields = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated struct declaration; expected '}'."));
            }
            let (field_name, field_span) = self.identifier("Expected field name.")?;
            if !names.insert(field_name.clone()) {
                return Err(V05ParseError {
                    message: format!("Duplicate struct field '{field_name}'."),
                    span: field_span.clone(),
                    line: self.tokens[self.cursor - 1].line,
                    column: self.tokens[self.cursor - 1].column,
                });
            }
            self.expect_punctuation(V05Punctuation::Colon, "Expected ':' after field name.")?;
            let ty = parse_type_expr(self.tokens, &mut self.cursor)?;
            let end = ty_span_end(&ty);
            fields.push(V05Field { name: field_name, ty, span: field_span.start..end });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after field declaration."));
            }
        }
        self.expect_punctuation(V05Punctuation::RightBrace, "Expected '}' after struct fields.")?;
        let end = self.tokens[self.cursor - 1].span.end;
        let _ = name_span;
        Ok(V05StructDecl { name, generics, fields, span: start..end })
    }

    fn parse_enum(&mut self) -> Result<V05EnumDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.expect_keyword(V05Keyword::Enum, "Expected 'enum'.")?;
        let (name, _) = self.identifier("Expected enum name.")?;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected '{' before enum variants.")?;
        let mut variants = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated enum declaration; expected '}'."));
            }
            let (variant_name, variant_span) = self.identifier("Expected enum variant name.")?;
            if !names.insert(variant_name.clone()) {
                return Err(V05ParseError {
                    message: format!("Duplicate enum variant '{variant_name}'."),
                    span: variant_span.clone(),
                    line: self.tokens[self.cursor - 1].line,
                    column: self.tokens[self.cursor - 1].column,
                });
            }
            let mut payload = Vec::new();
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::LeftParen) {
                self.cursor += 1;
                if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                    loop {
                        payload.push(parse_type_expr(self.tokens, &mut self.cursor)?);
                        if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                            self.cursor += 1;
                            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                self.expect_punctuation(V05Punctuation::RightParen, "Expected ')' after enum payload.")?;
            }
            let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
            variants.push(V05EnumVariant { name: variant_name, payload, span: variant_span.start..end });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after enum variant."));
            }
        }
        self.expect_punctuation(V05Punctuation::RightBrace, "Expected '}' after enum variants.")?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05EnumDecl { name, generics, variants, span: start..end })
    }
}

fn ty_span_end(ty: &TypeExpr) -> usize {
    match ty {
        TypeExpr::Named { span, .. } | TypeExpr::Any { span } => span.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn parses_structs_and_enum_unit_and_tuple_variants() {
        let tokens = lex_v05("struct Box<T: Display> { value: T, } enum Result<T, E> { Ok(T), Err(E), Empty, }").unwrap();
        let declarations = parse_nominal_declarations(&tokens).unwrap();
        assert_eq!(declarations.len(), 2);
        match &declarations[0] {
            V05Declaration::Struct(decl) => {
                assert_eq!(decl.name, "Box");
                assert_eq!(decl.generics[0].traits, vec!["Display"]);
                assert_eq!(decl.fields[0].name, "value");
            }
            _ => panic!("expected struct AST"),
        }
        match &declarations[1] {
            V05Declaration::Enum(decl) => {
                assert_eq!(decl.name, "Result");
                assert_eq!(decl.variants.len(), 3);
                assert_eq!(decl.variants[0].payload.len(), 1);
                assert_eq!(decl.variants[2].payload.len(), 0);
            }
            _ => panic!("expected enum AST"),
        }
    }

    #[test]
    fn rejects_duplicate_fields_and_variants() {
        let duplicate_field = lex_v05("struct S { a: number, a: string }").unwrap();
        assert!(parse_nominal_declarations(&duplicate_field).unwrap_err().message.contains("Duplicate"));
        let duplicate_variant = lex_v05("enum E { A, A }").unwrap();
        assert!(parse_nominal_declarations(&duplicate_variant).unwrap_err().message.contains("Duplicate"));
    }

    #[test]
    fn nominal_parser_rejects_trailing_unknown_syntax_and_unclosed_blocks() {
        assert!(parse_nominal_declarations(&lex_v05("struct S {} fn f() {}").unwrap()).is_err());
        assert!(parse_nominal_declarations(&lex_v05("enum E { A").unwrap()).is_err());
    }
    #[test]
    fn recognizes_v05_keywords_and_generic_punctuation() {
        let tokens = lex_v05("struct Box<T> { value: T } enum Result<T, E> { Ok(T) } impl<T> Box<T> {} trait Show {} match value {}").unwrap();
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Struct)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Enum)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Impl)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Trait)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Keyword(V05Keyword::Match)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Less)));
        assert!(tokens
            .iter()
            .any(|t| t.kind == V05TokenKind::Punctuation(V05Punctuation::Greater)));
    }

    #[test]
    fn preserves_utf8_byte_spans_and_line_columns() {
        let source = "// café\nstruct Cafe {}";
        let tokens = lex_v05(source).unwrap();
        let cafe = tokens
            .iter()
            .find(|t| matches!(&t.kind, V05TokenKind::Identifier(name) if name == "Cafe"))
            .unwrap();
        assert_eq!(cafe.line, 2);
        assert_eq!(cafe.column, 8);
        assert_eq!(&source[cafe.span.clone()], "Cafe");
    }

    #[test]
    fn reports_unterminated_strings_without_panicking() {
        let error = lex_v05("struct A { name: \"unfinished").unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 18);
        assert!(error.message.contains("Unterminated"));
    }

    #[test]
    fn token_spans_are_valid_utf8_boundaries() {
        let source = "struct Point { x: number }";
        for token in lex_v05(source).unwrap() {
            assert_eq!(
                &source[token.span.clone()].len(),
                &(token.span.end - token.span.start)
            );
        }
    }
}
