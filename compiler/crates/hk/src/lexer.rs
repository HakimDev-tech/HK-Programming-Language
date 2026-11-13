use super::{HkError, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum K {
    Id,
    Number,
    String,
    Let,
    Const,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    True,
    False,
    Null,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Semi,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    BangEq,
    Eq,
    EqEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    And,
    Or,
    Eof,
}
#[derive(Clone, Debug)]
pub(super) struct Token {
    pub(super) kind: K,
    pub(super) text: String,
    pub(super) literal: Option<Lit>,
    pub(super) line: usize,
    pub(super) col: usize,
}
#[derive(Clone, Debug)]
pub(super) enum Lit {
    Number(f64),
    String(String),
}
pub(super) struct Lexer<'a> {
    chars: Vec<char>,
    source: &'a str,
    start: usize,
    current: usize,
    line: usize,
    col: usize,
    start_line: usize,
    start_col: usize,
    name: &'a str,
}
impl<'a> Lexer<'a> {
    pub(super) fn new(name: &'a str, source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            source,
            start: 0,
            current: 0,
            line: 1,
            col: 1,
            start_line: 1,
            start_col: 1,
            name,
        }
    }
    pub(super) fn scan(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        while !self.end() {
            self.start = self.current;
            self.start_line = self.line;
            self.start_col = self.col;
            if let Some(t) = self.token()? {
                out.push(t);
            }
        }
        out.push(Token {
            kind: K::Eof,
            text: String::new(),
            literal: None,
            line: self.line,
            col: self.col,
        });
        Ok(out)
    }
    fn token(&mut self) -> Result<Option<Token>> {
        let c = self.advance();
        let kind = match c {
            '(' => Some(K::LParen),
            ')' => Some(K::RParen),
            '{' => Some(K::LBrace),
            '}' => Some(K::RBrace),
            '[' => Some(K::LBracket),
            ']' => Some(K::RBracket),
            ',' => Some(K::Comma),
            '.' => Some(K::Dot),
            ';' => Some(K::Semi),
            '+' => Some(K::Plus),
            '-' => Some(K::Minus),
            '*' => Some(K::Star),
            '%' => Some(K::Percent),
            '!' => Some(if self.take('=') { K::BangEq } else { K::Bang }),
            '=' => Some(if self.take('=') { K::EqEq } else { K::Eq }),
            '>' => Some(if self.take('=') { K::GtEq } else { K::Gt }),
            '<' => Some(if self.take('=') { K::LtEq } else { K::Lt }),
            '&' if self.take('&') => Some(K::And),
            '|' if self.take('|') => Some(K::Or),
            '/' if self.take('/') => {
                while self.peek() != '\n' && !self.end() {
                    self.advance();
                }
                None
            }
            '/' => Some(K::Slash),
            ' ' | '\r' | '\t' | '\n' => None,
            '"' => return self.string().map(Some),
            c if c.is_ascii_digit() => return self.number().map(Some),
            c if c == '_' || c.is_ascii_alphabetic() => return Ok(Some(self.identifier())),
            '&' | '|' => return Err(self.err("E1001", "Expected a doubled logical operator.")),
            _ => return Err(self.err("E1000", &format!("Unexpected character '{c}'."))),
        };
        Ok(kind.map(|kind| self.make(kind, None)))
    }
    fn string(&mut self) -> Result<Token> {
        let mut s = String::new();
        while !self.end() && self.peek() != '"' {
            let c = self.advance();
            if c == '\\' {
                if self.end() {
                    return Err(self.err("E1002", "Unterminated escape sequence."));
                }
                let e = self.advance();
                s.push(match e {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    _ => return Err(self.err("E1003", "Unsupported string escape.")),
                });
            } else {
                s.push(c);
            }
        }
        if self.end() {
            return Err(self.err("E1004", "Unterminated string literal."));
        }
        self.advance();
        Ok(self.make(K::String, Some(Lit::String(s))))
    }
    fn number(&mut self) -> Result<Token> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        if self.peek() == '.' && self.next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        let text: String = self.chars[self.start..self.current].iter().collect();
        let n = text
            .parse::<f64>()
            .map_err(|_| self.err("E1005", "Invalid number."))?;
        if !n.is_finite() {
            return Err(self.err("E1006", "Number must be finite."));
        }
        Ok(self.make(K::Number, Some(Lit::Number(n))))
    }
    fn identifier(&mut self) -> Token {
        while self.peek() == '_' || self.peek().is_ascii_alphanumeric() {
            self.advance();
        }
        let text: String = self.chars[self.start..self.current].iter().collect();
        let kind = match text.as_str() {
            "let" => K::Let,
            "const" => K::Const,
            "fn" => K::Fn,
            "return" => K::Return,
            "if" => K::If,
            "else" => K::Else,
            "while" => K::While,
            "for" => K::For,
            "true" => K::True,
            "false" => K::False,
            "null" => K::Null,
            _ => K::Id,
        };
        self.make(kind, None)
    }
    fn make(&self, kind: K, literal: Option<Lit>) -> Token {
        Token {
            kind,
            text: self.chars[self.start..self.current].iter().collect(),
            literal,
            line: self.start_line,
            col: self.start_col,
        }
    }
    fn err(&self, code: &'static str, msg: &str) -> HkError {
        HkError::new(
            code,
            msg,
            self.name,
            self.start_line,
            self.start_col,
            self.source,
        )
    }
    fn advance(&mut self) -> char {
        let c = self.chars[self.current];
        self.current += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1
        } else {
            self.col += 1
        }
        c
    }
    fn take(&mut self, c: char) -> bool {
        if self.end() || self.chars[self.current] != c {
            return false;
        }
        self.advance();
        true
    }
    fn peek(&self) -> char {
        self.chars.get(self.current).copied().unwrap_or('\0')
    }
    fn next(&self) -> char {
        self.chars.get(self.current + 1).copied().unwrap_or('\0')
    }
    fn end(&self) -> bool {
        self.current >= self.chars.len()
    }
}
