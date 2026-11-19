//! Experimental, source-spanned front-end model for the HK 0.5 grammar.
//!
//! This module is deliberately separate from the executable 0.4 parser until
//! declaration collection, type checking, bytecode lowering, and VM support
//! can be integrated atomically. It provides the lossless lexical contract and
//! recursive type-expression model used by the staged 0.5 parser.
use std::ops::Range;

const MAX_V05_GENERIC_PARAMETERS: usize = 256;
const MAX_V05_TRAIT_BOUNDS_PER_PARAMETER: usize = 256;

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
    let mut parameter_names = std::collections::HashSet::new();
    loop {
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
        if !parameter_names.insert(parameter.clone()) {
            return Err(parse_error(tokens, *cursor, "Duplicate generic parameter."));
        }
        *cursor += 1;
        let mut traits = Vec::new();
        let mut trait_names = std::collections::HashSet::new();
        if is_punctuation(tokens.get(*cursor), V05Punctuation::Colon) {
            *cursor += 1;
            loop {
                let bound = tokens
                    .get(*cursor)
                    .ok_or_else(|| parse_error(tokens, *cursor, "Expected a trait bound."))?;
                match &bound.kind {
                    V05TokenKind::Identifier(name) => {
                        if !trait_names.insert(name.clone()) {
                            return Err(parse_error(tokens, *cursor, "Duplicate trait bound."));
                        }
                        traits.push(name.clone());
                    }
                    _ => {
                        return Err(parse_error(
                            tokens,
                            *cursor,
                            "Expected a trait name in generic bound.",
                        ))
                    }
                }
                *cursor += 1;
                if traits.len() > MAX_V05_TRAIT_BOUNDS_PER_PARAMETER {
                    return Err(parse_error(
                        tokens,
                        *cursor,
                        "Trait-bound list exceeds the supported limit (256).",
                    ));
                }
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
        if parameters.len() > MAX_V05_GENERIC_PARAMETERS {
            return Err(parse_error(
                tokens,
                *cursor,
                "Generic parameter list exceeds the supported limit (256).",
            ));
        }
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
/// top-level constructs instead of silently skipping tokens.
pub fn parse_nominal_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05Declaration>, V05ParseError> {
    let mut parser = V05NominalParser { tokens, cursor: 0 };
    let mut declarations = Vec::new();
    while parser.cursor < tokens.len() {
        let declaration = match parser.peek_keyword() {
            Some(V05Keyword::Struct) => parser.parse_struct().map(V05Declaration::Struct),
            Some(V05Keyword::Enum) => parser.parse_enum().map(V05Declaration::Enum),
            _ => Err(parser.error("Expected a struct or enum declaration.")),
        };
        declarations.push(declaration?);
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

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn identifier(&mut self, message: &str) -> Result<(String, Range<usize>), V05ParseError> {
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error(message))?;
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
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' before struct fields.",
        )?;
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
            fields.push(V05Field {
                name: field_name,
                ty,
                span: field_span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after field declaration."));
            }
        }
        self.expect_punctuation(
            V05Punctuation::RightBrace,
            "Expected '}' after struct fields.",
        )?;
        let end = self.tokens[self.cursor - 1].span.end;
        let _ = name_span;
        Ok(V05StructDecl {
            name,
            generics,
            fields,
            span: start..end,
        })
    }

    fn parse_enum(&mut self) -> Result<V05EnumDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.expect_keyword(V05Keyword::Enum, "Expected 'enum'.")?;
        let (name, _) = self.identifier("Expected enum name.")?;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' before enum variants.",
        )?;
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
                            if is_punctuation(
                                self.tokens.get(self.cursor),
                                V05Punctuation::RightParen,
                            ) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                self.expect_punctuation(
                    V05Punctuation::RightParen,
                    "Expected ')' after enum payload.",
                )?;
            }
            let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
            variants.push(V05EnumVariant {
                name: variant_name,
                payload,
                span: variant_span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
            } else if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
                return Err(self.error("Expected ',' or '}' after enum variant."));
            }
        }
        self.expect_punctuation(
            V05Punctuation::RightBrace,
            "Expected '}' after enum variants.",
        )?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05EnumDecl {
            name,
            generics,
            variants,
            span: start..end,
        })
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
        let tokens = lex_v05(
            "struct Box<T: Display> { value: T, } enum Result<T, E> { Ok(T), Err(E), Empty, }",
        )
        .unwrap();
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
        assert!(parse_nominal_declarations(&duplicate_field)
            .unwrap_err()
            .message
            .contains("Duplicate"));
        let duplicate_variant = lex_v05("enum E { A, A }").unwrap();
        assert!(parse_nominal_declarations(&duplicate_variant)
            .unwrap_err()
            .message
            .contains("Duplicate"));
    }

    #[test]
    fn nominal_parser_rejects_trailing_unknown_syntax_and_unclosed_blocks() {
        assert!(parse_nominal_declarations(&lex_v05("struct S {} fn f() {}").unwrap()).is_err());
        assert!(parse_nominal_declarations(&lex_v05("enum E { A").unwrap()).is_err());
    }

    #[test]
    fn generic_parameter_parser_rejects_duplicates_and_caps_lists() {
        let duplicate = lex_v05("<T, T>").unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&duplicate, &mut cursor)
            .unwrap_err()
            .message
            .contains("Duplicate generic"));

        let repeated_bound = lex_v05("<T: Display + Display>").unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&repeated_bound, &mut cursor)
            .unwrap_err()
            .message
            .contains("Duplicate trait"));

        let many = format!(
            "<{}>",
            (0..257)
                .map(|i| format!("T{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let tokens = lex_v05(&many).unwrap();
        let mut cursor = 0;
        assert!(parse_generic_parameters(&tokens, &mut cursor)
            .unwrap_err()
            .message
            .contains("supported limit"));
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

// ---- Trait and impl declaration parsing (staged M1.1 slice) ----------------

/// A named, typed instance-method parameter. The receiver is represented as the
/// ordinary first parameter and validated by later semantic analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05MethodParameter {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Range<usize>,
}

/// Method signature/body shape. Trait methods have no body; impl methods retain
/// their body token range for a later expression-parser integration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05Method {
    pub name: String,
    pub parameters: Vec<V05MethodParameter>,
    pub return_type: Option<TypeExpr>,
    pub body_span: Option<Range<usize>>,
    pub span: Range<usize>,
}

/// Trait declaration: method signatures only, per the normative v0.5 contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05TraitDecl {
    pub name: String,
    pub methods: Vec<V05Method>,
    pub span: Range<usize>,
}

/// An impl declaration. A missing trait name denotes an inherent impl.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05ImplDecl {
    pub generics: Vec<GenericBound>,
    pub target: TypeExpr,
    pub trait_name: Option<String>,
    pub methods: Vec<V05Method>,
    pub span: Range<usize>,
}

/// Parses trait signatures and impl method headers, preserving impl bodies as
/// token spans. This is syntax-only: bodies are not expression-parsed and no
/// declaration is integrated into the executable compiler by this API.
pub fn parse_trait_and_impl_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05TraitOrImpl>, V05ParseError> {
    let mut parser = V05TraitImplParser { tokens, cursor: 0 };
    let mut declarations = Vec::new();
    while parser.cursor < tokens.len() {
        let declaration = match parser.peek_keyword() {
            Some(V05Keyword::Trait) => parser.parse_trait().map(V05TraitOrImpl::Trait),
            Some(V05Keyword::Impl) => parser.parse_impl().map(V05TraitOrImpl::Impl),
            _ => Err(parser.error("Expected a trait or impl declaration.")),
        };
        declarations.push(declaration?);
    }
    Ok(declarations)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V05TraitOrImpl {
    Trait(V05TraitDecl),
    Impl(V05ImplDecl),
}

struct V05TraitImplParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05TraitImplParser<'_> {
    fn peek_keyword(&self) -> Option<V05Keyword> {
        match self.tokens.get(self.cursor)?.kind {
            V05TokenKind::Keyword(keyword) => Some(keyword),
            _ => None,
        }
    }

    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_identifier(&mut self, what: &str) -> Result<String, V05ParseError> {
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error(what))?;
        if let V05TokenKind::Identifier(name) = &token.kind {
            self.cursor += 1;
            Ok(name.clone())
        } else {
            Err(self.error(what))
        }
    }

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn parse_trait(&mut self) -> Result<V05TraitDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.cursor += 1; // trait
        let name = self.expect_identifier("Expected trait name.")?;
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected '{' after trait name.")?;
        let mut methods = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated trait declaration."));
            }
            let method = self.parse_method(false)?;
            self.validate_receiver(&method)?;
            if !names.insert(method.name.clone()) {
                return Err(self.error("Duplicate trait method."));
            }
            methods.push(method);
        }
        self.cursor += 1;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05TraitDecl {
            name,
            methods,
            span: start..end,
        })
    }

    fn parse_impl(&mut self) -> Result<V05ImplDecl, V05ParseError> {
        let start = self.tokens[self.cursor].span.start;
        self.cursor += 1; // impl
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        let first = parse_type_expr(self.tokens, &mut self.cursor)?;
        let (trait_name, target) = if self
            .tokens
            .get(self.cursor)
            .is_some_and(|t| matches!(&t.kind, V05TokenKind::Identifier(s) if s == "for"))
        {
            self.cursor += 1;
            let target = parse_type_expr(self.tokens, &mut self.cursor)?;
            let trait_name = match first {
                TypeExpr::Named {
                    name, arguments, ..
                } if arguments.is_empty() => name,
                _ => {
                    return Err(self.error("Trait name must be a plain identifier."));
                }
            };
            (Some(trait_name), target)
        } else {
            (None, first)
        };
        self.expect_punctuation(
            V05Punctuation::LeftBrace,
            "Expected '{' to begin impl body.",
        )?;
        let mut methods = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightBrace) {
            if self.cursor >= self.tokens.len() {
                return Err(self.error("Unterminated impl declaration."));
            }
            let method = self.parse_method(true)?;
            self.validate_receiver(&method)?;
            if !names.insert(method.name.clone()) {
                return Err(self.error("Duplicate impl method."));
            }
            methods.push(method);
        }
        self.cursor += 1;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05ImplDecl {
            generics,
            target,
            trait_name,
            methods,
            span: start..end,
        })
    }

    fn validate_receiver(&self, method: &V05Method) -> Result<(), V05ParseError> {
        if method
            .parameters
            .first()
            .is_none_or(|parameter| parameter.name != "self")
        {
            return Err(self.error("Instance methods require `self` as their first parameter."));
        }
        Ok(())
    }

    fn parse_method(&mut self, has_body: bool) -> Result<V05Method, V05ParseError> {
        let start = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected method declaration."))?
            .span
            .start;
        // fn is not a dedicated v0.5 keyword yet; it is an identifier token.
        if !self
            .tokens
            .get(self.cursor)
            .is_some_and(|t| matches!(&t.kind, V05TokenKind::Identifier(name) if name == "fn"))
        {
            return Err(self.error("Expected 'fn' method declaration."));
        }
        self.cursor += 1;
        let name = self.expect_identifier("Expected method name.")?;
        self.expect_punctuation(V05Punctuation::LeftParen, "Expected '(' after method name.")?;
        let mut parameters = Vec::new();
        let mut names = std::collections::HashSet::new();
        if !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
            loop {
                let token = self
                    .tokens
                    .get(self.cursor)
                    .ok_or_else(|| self.error("Expected method parameter."))?;
                let pname = match &token.kind {
                    V05TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected parameter name.")),
                };
                if !names.insert(pname.clone()) {
                    return Err(self.error("Duplicate method parameter."));
                }
                self.cursor += 1;
                let ty = if pname == "self" {
                    if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Colon) {
                        return Err(self.error(
                            "The self receiver type is inferred and must not be annotated.",
                        ));
                    }
                    None
                } else {
                    self.expect_punctuation(
                        V05Punctuation::Colon,
                        "Expected ':' after parameter name.",
                    )?;
                    Some(parse_type_expr(self.tokens, &mut self.cursor)?)
                };
                let end = self
                    .tokens
                    .get(self.cursor.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(token.span.end);
                parameters.push(V05MethodParameter {
                    name: pname,
                    ty,
                    span: token.span.start..end,
                });
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
        self.expect_punctuation(
            V05Punctuation::RightParen,
            "Expected ')' after method parameters.",
        )?;
        let return_type = if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Arrow) {
            self.cursor += 1;
            Some(parse_type_expr(self.tokens, &mut self.cursor)?)
        } else {
            None
        };
        let body_span = if has_body {
            self.expect_punctuation(V05Punctuation::LeftBrace, "Expected method body '{'.")?;
            let body_start = self.tokens[self.cursor - 1].span.start;
            let mut depth = 1usize;
            while self.cursor < self.tokens.len() && depth > 0 {
                match self.tokens[self.cursor].kind {
                    V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => depth += 1,
                    V05TokenKind::Punctuation(V05Punctuation::RightBrace) => depth -= 1,
                    _ => {}
                }
                self.cursor += 1;
                if depth > 64 {
                    return Err(
                        self.error("Maximum method-body brace nesting depth (64) exceeded.")
                    );
                }
            }
            if depth != 0 {
                return Err(self.error("Unterminated method body."));
            }
            Some(body_start..self.tokens[self.cursor - 1].span.end)
        } else {
            self.expect_punctuation(
                V05Punctuation::Semicolon,
                "Trait method signatures must end with ';'.",
            )?;
            None
        };
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        Ok(V05Method {
            name,
            parameters,
            return_type,
            body_span,
            span: start..end,
        })
    }
}

#[cfg(test)]
mod v05_trait_impl_parser_tests {
    use super::{lex_v05, parse_trait_and_impl_declarations, V05TraitOrImpl};

    #[test]
    fn parses_trait_signatures_and_generic_inherent_impl_methods() {
        let tokens = lex_v05("trait Display { fn display(self) -> string; } impl<T: Display> Box<T> { fn get(self) -> T { return self.value; } }").unwrap();
        let declarations = parse_trait_and_impl_declarations(&tokens).unwrap();
        assert_eq!(declarations.len(), 2);
        match &declarations[0] {
            V05TraitOrImpl::Trait(decl) => {
                assert_eq!(decl.name, "Display");
                assert_eq!(decl.methods[0].name, "display");
                assert_eq!(decl.methods[0].parameters[0].name, "self");
                assert_eq!(
                    decl.methods[0].return_type.as_ref().unwrap(),
                    &super::TypeExpr::Named {
                        name: "string".into(),
                        arguments: vec![],
                        span: 36..42,
                    }
                );
                assert!(decl.methods[0].body_span.is_none());
            }
            _ => panic!("expected trait"),
        }
        match &declarations[1] {
            V05TraitOrImpl::Impl(decl) => {
                assert_eq!(decl.generics[0].parameter, "T");
                assert_eq!(
                    decl.target,
                    super::TypeExpr::Named {
                        name: "Box".into(),
                        arguments: vec![super::TypeExpr::Named {
                            name: "T".into(),
                            arguments: vec![],
                            span: 67..68,
                        }],
                        span: 63..69,
                    }
                );
                assert_eq!(decl.methods[0].name, "get");
                assert!(decl.methods[0].body_span.is_some());
            }
            _ => panic!("expected impl"),
        }
    }

    #[test]
    fn parses_trait_impl_and_captures_body_without_parsing_expressions() {
        let tokens = lex_v05("trait Display { fn display(self) -> string; } impl Display for Point { fn display(self) -> string { return \"p\"; } }").unwrap();
        let declarations = parse_trait_and_impl_declarations(&tokens).unwrap();
        match &declarations[1] {
            V05TraitOrImpl::Impl(decl) => {
                assert_eq!(decl.trait_name.as_deref(), Some("Display"));
                assert_eq!(
                    decl.target,
                    super::TypeExpr::Named {
                        name: "Point".into(),
                        arguments: vec![],
                        span: 63..68,
                    }
                );
            }
            _ => panic!("expected trait impl"),
        }
    }

    #[test]
    fn rejects_duplicate_method_names_and_parameter_names() {
        let duplicate_method = lex_v05("trait T { fn x(self); fn x(self); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&duplicate_method).is_err());
        let duplicate_parameter =
            lex_v05("impl T { fn x(self, value: number, value: string) {} }").unwrap();
        assert!(parse_trait_and_impl_declarations(&duplicate_parameter).is_err());
    }

    #[test]
    fn rejects_missing_receiver_and_annotated_receiver() {
        let no_receiver = lex_v05("trait T { fn x(value: number); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&no_receiver).is_err());
        let annotated = lex_v05("trait T { fn x(self: T); }").unwrap();
        assert!(parse_trait_and_impl_declarations(&annotated).is_err());
    }

    #[test]
    fn body_capture_is_bounded_and_requires_closing_brace() {
        let unterminated = lex_v05("impl T { fn x(self) { if (true) { }").unwrap();
        assert!(parse_trait_and_impl_declarations(&unterminated).is_err());
        let excessive =
            lex_v05(&format!("impl T {{ fn x(self) {{ {} }} }}", "{".repeat(70))).unwrap();
        assert!(parse_trait_and_impl_declarations(&excessive).is_err());
    }
}

 

// ---- Generic function declaration syntax (staged M1.1 slice) --------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05FunctionParameter {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V05FunctionDecl {
    pub name: String,
    pub generics: Vec<GenericBound>,
    pub parameters: Vec<V05FunctionParameter>,
    pub return_type: Option<TypeExpr>,
    pub body_span: Range<usize>,
    pub span: Range<usize>,
}

/// Parses generic and nongeneric function declarations while preserving their
/// body token ranges. Expression and statement parsing is a separate stage.
pub fn parse_function_declarations(
    tokens: &[V05Token],
) -> Result<Vec<V05FunctionDecl>, V05ParseError> {
    let mut parser = V05FunctionParser { tokens, cursor: 0 };
    let mut functions = Vec::new();
    let mut names = std::collections::HashSet::new();
    while parser.cursor < tokens.len() {
        let function = parser.parse_function()?;
        if !names.insert(function.name.clone()) {
            return Err(parser.error("Duplicate function declaration."));
        }
        functions.push(function);
    }
    Ok(functions)
}

struct V05FunctionParser<'a> {
    tokens: &'a [V05Token],
    cursor: usize,
}

impl V05FunctionParser<'_> {
    fn error(&self, message: &str) -> V05ParseError {
        parse_error(self.tokens, self.cursor, message)
    }

    fn expect_punctuation(
        &mut self,
        punctuation: V05Punctuation,
        message: &str,
    ) -> Result<(), V05ParseError> {
        if is_punctuation(self.tokens.get(self.cursor), punctuation) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn parse_function(&mut self) -> Result<V05FunctionDecl, V05ParseError> {
        let start_token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected function declaration."))?;
        if !matches!(&start_token.kind, V05TokenKind::Identifier(keyword) if keyword == "fn") {
            return Err(self.error("Expected function declaration keyword."));
        }
        let start = start_token.span.start;
        self.cursor += 1;
        let name_token = self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error("Expected function name."))?;
        let name = match &name_token.kind {
            V05TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected function name.")),
        };
        self.cursor += 1;
        let generics = parse_generic_parameters(self.tokens, &mut self.cursor)?;
        self.expect_punctuation(
            V05Punctuation::LeftParen,
            "Expected '(' after function name/generics.",
        )?;
        let mut parameters = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
            let token = self
                .tokens
                .get(self.cursor)
                .ok_or_else(|| self.error("Expected function parameter."))?;
            let parameter_name = match &token.kind {
                V05TokenKind::Identifier(name) => name.clone(),
                _ => return Err(self.error("Expected parameter name.")),
            };
            if !names.insert(parameter_name.clone()) {
                return Err(self.error("Duplicate function parameter."));
            }
            self.cursor += 1;
            self.expect_punctuation(V05Punctuation::Colon, "Expected ':' after parameter name.")?;
            let ty = parse_type_expr(self.tokens, &mut self.cursor)?;
            let end = self
                .tokens
                .get(self.cursor.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(token.span.end);
            parameters.push(V05FunctionParameter {
                name: parameter_name,
                ty,
                span: token.span.start..end,
            });
            if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Comma) {
                self.cursor += 1;
                if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }
        self.expect_punctuation(V05Punctuation::RightParen, "Expected ')' after parameters.")?;
        let return_type = if is_punctuation(self.tokens.get(self.cursor), V05Punctuation::Arrow) {
            self.cursor += 1;
            Some(parse_type_expr(self.tokens, &mut self.cursor)?)
        } else {
            None
        };
        self.expect_punctuation(V05Punctuation::LeftBrace, "Expected function body.")?;
        let body_start = self.tokens[self.cursor - 1].span.start;
        let mut depth = 1usize;
        while self.cursor < self.tokens.len() && depth > 0 {
            match self.tokens[self.cursor].kind {
                V05TokenKind::Punctuation(V05Punctuation::LeftBrace) => depth += 1,
                V05TokenKind::Punctuation(V05Punctuation::RightBrace) => depth -= 1,
                _ => {}
            }
            self.cursor += 1;
            if depth > 64 {
                return Err(self.error("Maximum function-body brace nesting depth (64) exceeded."));
            }
        }
        if depth != 0 {
            return Err(self.error("Unterminated function body."));
        }
        let body_span = body_start..self.tokens[self.cursor - 1].span.end;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(V05FunctionDecl {
            name,
            generics,
            parameters,
            return_type,
            body_span,
            span: start..end,
        })
    }
}

#[cfg(test)]
mod v05_function_parser_tests {
    use super::{lex_v05, parse_function_declarations};

    #[test]
    fn parses_generic_function_signature_and_captures_body() {
        let tokens = lex_v05("fn identity<T>(value: T) -> T { return value; } fn sum(a: number, b: number) { return a + b; }").unwrap();
        let functions = parse_function_declarations(&tokens).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "identity");
        assert_eq!(functions[0].generics[0].parameter, "T");
        assert_eq!(functions[0].parameters[0].name, "value");
        assert!(functions[0].return_type.is_some());
        assert_eq!(functions[1].name, "sum");
        assert_eq!(functions[1].parameters.len(), 2);
        assert!(functions[1].return_type.is_none());
    }

    #[test]
    fn rejects_duplicate_function_and_parameter_names() {
        let duplicate_functions = lex_v05("fn f() {} fn f() {}").unwrap();
        assert!(parse_function_declarations(&duplicate_functions).is_err());
        let duplicate_parameters = lex_v05("fn f(a: number, a: string) {}").unwrap();
        assert!(parse_function_declarations(&duplicate_parameters).is_err());
    }

    #[test]
    fn rejects_unterminated_and_deep_function_bodies() {
        let unterminated = lex_v05("fn f() { if (true) { }").unwrap();
        assert!(parse_function_declarations(&unterminated).is_err());
        let deep = lex_v05(&format!("fn f() {{ {} }}", "{".repeat(70))).unwrap();
        assert!(parse_function_declarations(&deep).is_err());
    }
}
