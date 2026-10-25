//! Scan HQL source into tokens, keeping a byte range for every one.

use crate::diagnostics::Diagnostic;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Int(i64),
    Float(f64),
    Decimal(crate::decimal::Decimal),
    Bool(bool),
    Str(String),
    Ident(String),
    DocRef(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    Arrow,
    FatArrow,
    Dot,
    Comma,
    Colon,
    Equals,
    EqualEqual,
    BangEqual,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    /// The connectives are words, as `given … otherwise` is, so they are
    /// reserved rather than bound.
    And,
    Or,
    Not,
    /// `match`, `given` and `otherwise` are reserved for branching.
    Match,
    Given,
    Otherwise,
    /// `?`, which marks a field optional.
    Question,
    Newline,
    End,
}

impl Token {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Int(value) => format!("`{value}`"),
            Self::Float(value) => format!("`{value:?}f`"),
            Self::Decimal(value) => format!("`{value}`"),
            Self::Bool(value) => format!("`{value}`"),
            Self::Str(_) => "a string".to_owned(),
            Self::Ident(name) => format!("`{name}`"),
            Self::DocRef(name) => format!("`@[[{name}]]`"),
            Self::Plus => "`+`".to_owned(),
            Self::Minus => "`-`".to_owned(),
            Self::Star => "`*`".to_owned(),
            Self::Slash => "`/`".to_owned(),
            Self::Percent => "`%`".to_owned(),
            Self::Pipe => "`|`".to_owned(),
            Self::Arrow => "`->`".to_owned(),
            Self::FatArrow => "`=>`".to_owned(),
            Self::Dot => "`.`".to_owned(),
            Self::Comma => "`,`".to_owned(),
            Self::Colon => "`:`".to_owned(),
            Self::Equals => "`=`".to_owned(),
            Self::EqualEqual => "`==`".to_owned(),
            Self::BangEqual => "`!=`".to_owned(),
            Self::OpenParen => "`(`".to_owned(),
            Self::CloseParen => "`)`".to_owned(),
            Self::OpenBrace => "`{`".to_owned(),
            Self::CloseBrace => "`}`".to_owned(),
            Self::OpenBracket => "`[`".to_owned(),
            Self::CloseBracket => "`]`".to_owned(),
            Self::Less => "`<`".to_owned(),
            Self::Greater => "`>`".to_owned(),
            Self::LessEqual => "`<=`".to_owned(),
            Self::GreaterEqual => "`>=`".to_owned(),
            Self::And => "`and`".to_owned(),
            Self::Or => "`or`".to_owned(),
            Self::Not => "`not`".to_owned(),
            Self::Match => "`match`".to_owned(),
            Self::Given => "`given`".to_owned(),
            Self::Otherwise => "`otherwise`".to_owned(),
            Self::Question => "`?`".to_owned(),
            Self::Newline => "end of line".to_owned(),
            Self::End => "end of input".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Spanned {
    pub token: Token,
    pub span: Range<usize>,
}

/// Scan a whole source string, ending with exactly one `End` token.
pub(crate) fn scan(source: &str) -> Result<Vec<Spanned>, Diagnostic> {
    let mut scanner = Scanner {
        source,
        offset: 0,
        tokens: Vec::new(),
    };
    scanner.run()?;
    Ok(scanner.tokens)
}

struct Scanner<'a> {
    source: &'a str,
    offset: usize,
    tokens: Vec<Spanned>,
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.source[self.offset..]
    }

    fn push(&mut self, token: Token, start: usize) {
        self.tokens.push(Spanned {
            token,
            span: start..self.offset,
        });
    }

    fn error(&self, message: &str) -> Diagnostic {
        let width = self.rest().chars().next().map_or(0, char::len_utf8);
        Diagnostic::syntax(self.offset..self.offset + width, message)
    }

    fn run(&mut self) -> Result<(), Diagnostic> {
        loop {
            self.trivia()?;
            let start = self.offset;
            let Some(character) = self.rest().chars().next() else {
                self.tokens.push(Spanned {
                    token: Token::End,
                    span: start..start,
                });
                return Ok(());
            };

            if character == '\n' {
                self.offset += 1;
                self.push(Token::Newline, start);
                continue;
            }

            for (text, token) in [("<=", Token::LessEqual), (">=", Token::GreaterEqual)] {
                if self.rest().starts_with(text) {
                    self.offset += 2;
                    self.push(token, start);
                    break;
                }
            }
            if self.offset != start {
                continue;
            }

            let simple = match character {
                '+' => Some(Token::Plus),
                '*' => Some(Token::Star),
                '/' => Some(Token::Slash),
                '%' => Some(Token::Percent),
                '|' => Some(Token::Pipe),
                '.' => Some(Token::Dot),
                ',' => Some(Token::Comma),
                ':' => Some(Token::Colon),
                '(' => Some(Token::OpenParen),
                ')' => Some(Token::CloseParen),
                '{' => Some(Token::OpenBrace),
                '}' => Some(Token::CloseBrace),
                ']' => Some(Token::CloseBracket),
                '>' => Some(Token::Greater),
                _ => None,
            };
            if let Some(token) = simple {
                self.offset += character.len_utf8();
                self.push(token, start);
                continue;
            }

            if character == '<' {
                self.offset += 1;
                self.push(Token::Less, start);
                continue;
            }
            if character == '?' {
                self.offset += 1;
                self.push(Token::Question, start);
                continue;
            }
            if self.rest().starts_with("==") {
                self.offset += 2;
                self.push(Token::EqualEqual, start);
                continue;
            }
            if self.rest().starts_with("!=") {
                self.offset += 2;
                self.push(Token::BangEqual, start);
                continue;
            }
            if self.rest().starts_with("=>") {
                self.offset += 2;
                self.push(Token::FatArrow, start);
                continue;
            }
            if character == '=' {
                self.offset += 1;
                self.push(Token::Equals, start);
                continue;
            }
            if self.rest().starts_with("@[[") {
                self.doc_reference(start)?;
                continue;
            }
            if character == '@' {
                return Err(Diagnostic::syntax(
                    start..start + 1,
                    "`@` opens a document reference: `@[[name]]`",
                ));
            }
            if character == '[' {
                self.offset += 1;
                self.push(Token::OpenBracket, start);
                continue;
            }
            if self.rest().starts_with("->") {
                self.offset += 2;
                self.push(Token::Arrow, start);
                continue;
            }
            if character == '"' {
                self.string(start)?;
                continue;
            }
            // A minus directly before a digit is a literal's sign unless it
            // follows an operand, where it subtracts: `-2 + 3`, `n -2`. HQL
            // has no unary minus, so `- 1` stays a syntax error.
            if character.is_ascii_digit()
                || (character == '-'
                    && !self.after_operand()
                    && self.source[self.offset + 1..]
                        .starts_with(|next: char| next.is_ascii_digit()))
            {
                self.number(start)?;
                continue;
            }
            if character == '-' {
                self.offset += 1;
                self.push(Token::Minus, start);
                continue;
            }
            if character.is_alphabetic() || character == '_' {
                self.word(start);
                continue;
            }
            return Err(self.error("unexpected character"));
        }
    }

    /// Whether the last token can end an operand, so a minus after it is infix.
    fn after_operand(&self) -> bool {
        self.tokens.last().is_some_and(|last| {
            matches!(
                last.token,
                Token::Int(_)
                    | Token::Float(_)
                    | Token::Decimal(_)
                    | Token::Bool(_)
                    | Token::Str(_)
                    | Token::Ident(_)
                    | Token::DocRef(_)
                    | Token::CloseParen
                    | Token::CloseBracket
                    | Token::CloseBrace
            )
        })
    }

    /// Skip spaces, tabs and `//` line comments, but never newlines.
    fn trivia(&mut self) -> Result<(), Diagnostic> {
        loop {
            let rest = self.rest();
            if rest.starts_with("///") && !rest.starts_with("////") {
                let line = self.source[..self.offset]
                    .rfind('\n')
                    .map_or(0, |at| at + 1);
                if !self.source[line..self.offset].trim().is_empty() {
                    return Err(Diagnostic::syntax(
                        self.offset..self.offset + 3,
                        "a doc string stands on its own lines, directly above a declaration",
                    ));
                }
            }
            if rest.starts_with("//") {
                self.offset += rest.find('\n').unwrap_or(rest.len());
                continue;
            }
            match rest.chars().next() {
                Some(c) if c.is_whitespace() && c != '\n' => self.offset += c.len_utf8(),
                _ => return Ok(()),
            }
        }
    }

    fn word(&mut self, start: usize) {
        while self
            .rest()
            .starts_with(|c: char| c.is_alphanumeric() || c == '_')
        {
            self.offset += self.rest().chars().next().map_or(0, char::len_utf8);
        }
        let text = &self.source[start..self.offset];
        let token = match text {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "match" => Token::Match,
            "given" => Token::Given,
            "otherwise" => Token::Otherwise,
            other => Token::Ident(other.to_owned()),
        };
        self.push(token, start);
    }

    fn doc_reference(&mut self, start: usize) -> Result<(), Diagnostic> {
        self.offset += 3;
        let Some(end) = self.rest().find("]]") else {
            return Err(Diagnostic::syntax(
                start..self.source.len(),
                "unterminated document reference: expected `]]`",
            ));
        };
        let name = self.rest()[..end].trim().to_owned();
        self.offset += end + 2;
        if name.is_empty() {
            return Err(Diagnostic::syntax(
                start..self.offset,
                "a document reference needs a name",
            ));
        }
        self.push(Token::DocRef(name), start);
        Ok(())
    }

    fn string(&mut self, start: usize) -> Result<(), Diagnostic> {
        self.offset += 1;
        let mut text = String::new();
        loop {
            let Some(character) = self.rest().chars().next() else {
                return Err(Diagnostic::syntax(
                    start..self.source.len(),
                    "unterminated string literal",
                ));
            };
            self.offset += character.len_utf8();
            match character {
                '"' => break,
                '\n' => {
                    return Err(Diagnostic::syntax(
                        start..self.offset,
                        "a string literal may not span lines",
                    ));
                }
                '\\' => {
                    let Some(escape) = self.rest().chars().next() else {
                        return Err(self.error("unterminated escape"));
                    };
                    self.offset += escape.len_utf8();
                    text.push(match escape {
                        'n' => '\n',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        _ => return Err(self.error("unknown escape")),
                    });
                }
                other => text.push(other),
            }
        }
        self.push(Token::Str(text), start);
        Ok(())
    }

    fn number(&mut self, start: usize) -> Result<(), Diagnostic> {
        if self.rest().starts_with('-') {
            self.offset += 1;
        }
        self.digits();
        let fraction = self.source.as_bytes().get(self.offset) == Some(&b'.')
            && self.source[self.offset + 1..].starts_with(|next: char| next.is_ascii_digit());
        if fraction {
            self.offset += 1;
            self.digits();
            self.exponent();
        }
        let digits_end = self.offset;
        // `f` directly after the digits makes the literal a Float.
        let float = self.rest().starts_with('f')
            && !self.source[self.offset + 1..]
                .starts_with(|next: char| next.is_alphanumeric() || next == '_');
        if float {
            self.offset += 1;
        }
        // A point followed by a name ends the literal: `7.max(3)` calls
        // `max` on `7`. A trailing point is not part of the literal grammar.
        let receiver_call = self.rest().starts_with('.')
            && self.source[self.offset + 1..]
                .starts_with(|next: char| next.is_alphabetic() || next == '_');
        if !receiver_call
            && self
                .rest()
                .starts_with(|c: char| c == '.' || c.is_alphanumeric())
        {
            self.offset += self.rest().chars().next().map_or(0, char::len_utf8);
            return Err(Diagnostic::syntax(
                start..self.offset,
                "malformed numeric literal",
            ));
        }
        let text = &self.source[start..digits_end];
        let token = if float {
            let value = text
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    Diagnostic::syntax(
                        start..self.offset,
                        "float literal is outside the finite double-precision range",
                    )
                })?;
            Token::Float(value)
        } else if fraction {
            let value = crate::decimal::Decimal::parse(text).ok_or_else(|| {
                // The digits are already scanned, so only the bounds refuse it.
                Diagnostic::syntax(
                    start..self.offset,
                    "decimal literal is outside the Decimal range: a significand of at most \
                     512 bits, at most 154 digits after the point",
                )
            })?;
            Token::Decimal(value)
        } else {
            let value = text.parse::<i64>().map_err(|_| {
                Diagnostic::syntax(
                    start..self.offset,
                    "integer literal is outside the signed 64-bit range",
                )
                .with_kind(crate::diagnostics::Kind::NumericOverflow)
            })?;
            Token::Int(value)
        };
        self.push(token, start);
        Ok(())
    }

    /// An exponent after a literal's fraction: `e` or `E`, an optional sign
    /// and digits. Without digits after it, the `e` is not consumed.
    fn exponent(&mut self) {
        let bytes = self.source.as_bytes();
        if !matches!(bytes.get(self.offset), Some(b'e' | b'E')) {
            return;
        }
        let mut at = self.offset + 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        if bytes.get(at).is_some_and(u8::is_ascii_digit) {
            self.offset = at;
            self.digits();
        }
    }

    fn digits(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_digit)
        {
            self.offset += 1;
        }
    }
}
