//! Scan and parse the bootstrap grammar: literal ("+" literal)*.

use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;

// Bound the left-deep AST, including its traversal and destruction.
const MAX_LITERALS: usize = 256;

pub(crate) fn parse(source: &str) -> Result<Expr, Diagnostic> {
    let mut parser = Parser { source, offset: 0 };
    let mut expression = parser.literal()?;
    let mut count = 1;
    loop {
        parser.whitespace();
        if parser.offset == source.len() {
            return Ok(expression);
        }
        if !source[parser.offset..].starts_with('+') {
            return Err(parser.error("expected '+' or end of expression"));
        }
        if count == MAX_LITERALS {
            return Err(parser.error("bootstrap limit: at most 256 literals"));
        }
        parser.offset += 1;
        let right = parser.literal()?;
        let span = expression.span.start..right.span.end;
        expression = Expr {
            kind: Kind::Add(Box::new(expression), Box::new(right)),
            span,
        };
        count += 1;
    }
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
}

impl Parser<'_> {
    fn whitespace(&mut self) {
        while let Some(c) = self.source[self.offset..].chars().next() {
            if !c.is_whitespace() {
                break;
            }
            self.offset += c.len_utf8();
        }
    }

    fn error(&self, message: &str) -> Diagnostic {
        let width = self.source[self.offset..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
        Diagnostic::Syntax {
            span: self.offset..self.offset + width,
            message: message.to_owned(),
        }
    }

    fn literal(&mut self) -> Result<Expr, Diagnostic> {
        self.whitespace();
        let start = self.offset;
        let rest = &self.source[start..];
        let kind = if rest.starts_with("true") {
            self.offset += 4;
            Kind::Bool(true)
        } else if rest.starts_with("false") {
            self.offset += 5;
            Kind::Bool(false)
        } else {
            // A minus is part of a numeric literal, not a unary operator.
            if rest.starts_with('-') {
                self.offset += 1;
            }
            let digits = self.offset;
            self.digits();
            if self.offset == digits {
                return Err(self.error("expected a number or Boolean literal"));
            }
            // A decimal point makes the literal a Float and must carry digits.
            let fraction = self.source.as_bytes().get(self.offset) == Some(&b'.');
            if fraction {
                self.offset += 1;
                let fractional = self.offset;
                self.digits();
                if self.offset == fractional {
                    return Err(self.error("expected a digit after the decimal point"));
                }
            }
            let text = &self.source[start..self.offset];
            if fraction {
                let value = text.parse::<f64>().ok().filter(|value| value.is_finite());
                Kind::Float(value.ok_or_else(|| Diagnostic::Syntax {
                    span: start..self.offset,
                    message:
                        "float literal is outside the finite double-precision range".to_owned(),
                })?)
            } else {
                Kind::Int(text.parse::<i64>().map_err(|_| Diagnostic::Syntax {
                    span: start..self.offset,
                    message: "integer literal is outside the signed 64-bit range".to_owned(),
                })?)
            }
        };
        Ok(Expr {
            kind,
            span: start..self.offset,
        })
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
