//! Parse tokens into a program: statements, bindings, pipelines and calls.

use crate::ast::{Arg, Expr, Field, Kind, Program, Stmt, TypeAnn, TypeDecl};
use crate::diagnostics::Diagnostic;
use crate::lexer::{self, Spanned, Token};
use std::ops::Range;

// Bound the left-deep addition chain, including its traversal and destruction.
const MAX_ADDITIONS: usize = 255;
// Bound recursion through parentheses, calls, data literals and lambdas.
const MAX_DEPTH: usize = 128;

/// Parse a whole program. The entire input must be consumed.
pub(crate) fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = lexer::scan(source)?;
    let mut parser = Parser {
        tokens,
        index: 0,
        nesting: 0,
        additions: 0,
        depth: 0,
    };
    parser.program()
}

struct Parser {
    tokens: Vec<Spanned>,
    index: usize,
    /// Bracket nesting; inside brackets a newline is not a statement end.
    nesting: usize,
    additions: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        // Inside brackets, newlines are insignificant and skipped on sight.
        let mut index = self.index;
        if self.nesting > 0 {
            while matches!(self.tokens[index].token, Token::Newline) {
                index += 1;
            }
        }
        &self.tokens[index].token
    }

    fn peek_span(&self) -> Range<usize> {
        let mut index = self.index;
        if self.nesting > 0 {
            while matches!(self.tokens[index].token, Token::Newline) {
                index += 1;
            }
        }
        self.tokens[index].span.clone()
    }

    fn advance(&mut self) -> Spanned {
        if self.nesting > 0 {
            while matches!(self.tokens[self.index].token, Token::Newline) {
                self.index += 1;
            }
        }
        let token = self.tokens[self.index].clone();
        if !matches!(token.token, Token::End) {
            self.index += 1;
        }
        token
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &Token, context: &str) -> Result<Spanned, Diagnostic> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(Diagnostic::syntax(
                self.peek_span(),
                format!(
                    "expected {} {context}, found {}",
                    expected.describe(),
                    self.peek().describe()
                ),
            ))
        }
    }

    /// Whether an infix operator continues the expression, looking past a line
    /// break so a pipeline can be written with a leading `|` on each line.
    fn at_infix(&mut self, operator: &Token) -> bool {
        if self.peek() == operator {
            self.advance();
            return true;
        }
        if self.nesting == 0 && matches!(self.tokens[self.index].token, Token::Newline) {
            let mut lookahead = self.index;
            while matches!(self.tokens[lookahead].token, Token::Newline) {
                lookahead += 1;
            }
            if &self.tokens[lookahead].token == operator {
                self.index = lookahead + 1;
                return true;
            }
        }
        false
    }

    /// Skip line breaks after an infix operator, so `1 +\n2` is one expression.
    fn continuation(&mut self) {
        while matches!(self.tokens[self.index].token, Token::Newline) {
            self.index += 1;
        }
    }

    fn nested<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Diagnostic::syntax(
                self.peek_span(),
                format!("expression nests deeper than the {MAX_DEPTH} the parser accepts"),
            ));
        }
        let result = body(self);
        self.depth -= 1;
        result
    }

    fn program(&mut self) -> Result<Program, Diagnostic> {
        let mut statements = Vec::new();
        self.continuation();
        while !matches!(self.peek(), Token::End) {
            statements.push(self.statement()?);
            if matches!(self.tokens[self.index].token, Token::Newline) {
                self.continuation();
            } else if !matches!(self.peek(), Token::End) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    format!(
                        "expected end of statement, found {}",
                        self.peek().describe()
                    ),
                ));
            }
        }
        if statements.is_empty() {
            let span = self.peek_span();
            return Err(Diagnostic::syntax(span, "expected an expression"));
        }
        Ok(Program {
            statements: std::mem::take(&mut statements),
        })
    }

    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.peek() == &Token::Ident("abstract".to_owned()) {
            self.advance();
            self.expect(&Token::Ident("type".to_owned()), "after abstract")?;
            self.index -= 1;
            let mut declaration = self.declaration()?;
            if declaration.union.is_some() {
                return Err(Diagnostic::syntax(
                    declaration.span,
                    "a union has no constructor of its own, so `abstract` says nothing about it",
                ));
            }
            declaration.abstract_type = true;
            return Ok(Stmt::Type(declaration));
        }
        if let Token::Ident(keyword) = self.peek().clone()
            && keyword == "import"
            && matches!(self.tokens[self.index + 1].token, Token::Ident(_))
        {
            let head = self.advance();
            let named = self.advance();
            let Token::Ident(name) = named.token else {
                return Err(Diagnostic::syntax(named.span, "expected an extension name"));
            };
            return Ok(Stmt::Import {
                name,
                span: head.span.start..named.span.end,
            });
        }
        if let Token::Ident(keyword) = self.peek().clone()
            && keyword == "type"
            && matches!(self.tokens[self.index + 1].token, Token::Ident(_))
        {
            return Ok(Stmt::Type(self.declaration()?));
        }
        if let Token::Ident(name) = self.peek().clone() {
            let follows = &self.tokens[self.index + 1].token;
            if matches!(follows, Token::Equals | Token::Colon) {
                self.advance();
                let annotation = if self.eat(&Token::Colon) {
                    Some(self.type_annotation()?)
                } else {
                    None
                };
                self.expect(&Token::Equals, "in a binding")?;
                self.continuation();
                return Ok(Stmt::Bind {
                    name,
                    annotation,
                    value: self.expression()?,
                });
            }
        }
        Ok(Stmt::Expr(self.expression()?))
    }

    /// `type Name<P, Q> : Parent { field : Type }`, with every part optional
    /// after the name, or `type Name<P> = union { A, B }`.
    fn declaration(&mut self) -> Result<TypeDecl, Diagnostic> {
        let keyword = self.advance();
        let named = self.advance();
        let Token::Ident(name) = named.token else {
            return Err(Diagnostic::syntax(named.span, "expected a type name"));
        };
        let mut end = named.span.end;

        let mut parameters = Vec::new();
        if self.eat(&Token::Less) {
            self.nesting += 1;
            loop {
                let parameter = self.advance();
                let Token::Ident(parameter_name) = parameter.token else {
                    return Err(Diagnostic::syntax(
                        parameter.span,
                        "expected a type parameter name",
                    ));
                };
                parameters.push(parameter_name);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            end = self
                .expect(&Token::Greater, "after type parameters")?
                .span
                .end;
            self.nesting -= 1;
        }

        // `=` defines the type as a choice among others. It says what the type
        // is, where `:` says what it narrows, so the two are not combined.
        let mut union = None;
        if self.eat(&Token::Equals) {
            self.continuation();
            self.expect(&Token::Ident("union".to_owned()), "after `=` in a type")?;
            self.expect(&Token::OpenBrace, "to open a union")?;
            self.nesting += 1;
            let mut members = Vec::new();
            loop {
                members.push(self.type_annotation()?);
                // One member per line reads best, and a trailing comma is
                // accepted for anyone who writes one.
                if !self.eat(&Token::Comma) || matches!(self.peek(), Token::CloseBrace) {
                    break;
                }
            }
            end = self
                .expect(&Token::CloseBrace, "to close a union")?
                .span
                .end;
            self.nesting -= 1;
            union = Some(members);
        }

        let mut supertypes = Vec::new();
        if union.is_none() && self.eat(&Token::Colon) {
            self.continuation();
            // In supertype position a set of types is a supertype set, which
            // is what keeps `{A, B}` and `{a: A, b: B}` apart here.
            if matches!(self.peek(), Token::OpenBrace) {
                self.advance();
                self.nesting += 1;
                loop {
                    supertypes.push(self.type_annotation()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                end = self
                    .expect(&Token::CloseBrace, "after a supertype set")?
                    .span
                    .end;
                self.nesting -= 1;
            } else {
                let only = self.type_annotation()?;
                end = only.span.end;
                supertypes.push(only);
            }
        }

        let mut fields = Vec::new();
        if union.is_none() && matches!(self.tokens[self.index].token, Token::OpenBrace) {
            self.advance();
            self.nesting += 1;
            while !matches!(self.peek(), Token::CloseBrace) {
                fields.push(self.field()?);
                // A record body separates its fields by line, and a comma is
                // accepted for anyone who writes one.
                self.eat(&Token::Comma);
            }
            end = self
                .expect(&Token::CloseBrace, "to close a record body")?
                .span
                .end;
            self.nesting -= 1;
        }

        let mut bounds = Vec::new();
        if self.at_infix(&Token::Ident("where".to_owned())) {
            self.continuation();
            loop {
                let parameter = self.advance();
                let Token::Ident(parameter) = parameter.token else {
                    return Err(Diagnostic::syntax(
                        parameter.span,
                        "expected a constrained parameter",
                    ));
                };
                self.expect(&Token::Colon, "in a type bound")?;
                let bound = self.type_annotation()?;
                end = bound.span.end;
                bounds.push((parameter, bound));
                if !self.eat(&Token::Comma) {
                    break;
                }
                self.continuation();
            }
        }
        Ok(TypeDecl {
            name,
            abstract_type: false,
            bounds,
            parameters,
            supertypes,
            fields,
            union,
            span: keyword.span.start..end,
        })
    }

    fn field(&mut self) -> Result<Field, Diagnostic> {
        let named = self.advance();
        let Token::Ident(name) = named.token else {
            return Err(Diagnostic::syntax(named.span, "expected a field name"));
        };
        // Optionality is part of the schema: `metadata? : Data` says the path
        // may be absent, not that the field is a different type.
        let optional = self.eat(&Token::Question);
        self.expect(&Token::Colon, "after a field name")?;
        let annotation = self.type_annotation()?;
        let span = named.span.start..annotation.span.end;
        Ok(Field {
            name,
            optional,
            annotation,
            span,
        })
    }

    fn type_annotation(&mut self) -> Result<TypeAnn, Diagnostic> {
        let head = self.advance();
        let Token::Ident(name) = head.token else {
            return Err(Diagnostic::syntax(head.span, "expected a type name"));
        };
        let mut arguments = Vec::new();
        let mut span = head.span;
        if self.eat(&Token::Less) {
            self.nesting += 1;
            loop {
                arguments.push(self.nested(Self::type_annotation)?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            let close = self.expect(&Token::Greater, "after type arguments")?;
            self.nesting -= 1;
            span = span.start..close.span.end;
        }
        Ok(TypeAnn {
            name,
            arguments,
            span,
        })
    }

    fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.nested(Self::pipeline)
    }

    fn pipeline(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.link()?;
        while self.at_infix(&Token::Pipe) {
            self.continuation();
            let right = self.link()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::Pipe(Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn link(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.equality()?;
        if !self.at_infix(&Token::Arrow) {
            return Ok(left);
        }
        self.continuation();
        let target = self.equality()?;
        let mut end = target.span.end;
        let data = if matches!(self.peek(), Token::OpenBrace) {
            let annotation = self.data_literal()?;
            end = annotation.span.end;
            Some(Box::new(annotation))
        } else {
            None
        };
        let span = left.span.start..end;
        Ok(Expr {
            kind: Kind::Link {
                source: Box::new(left),
                target: Box::new(target),
                data,
            },
            span,
        })
    }

    /// Equality binds looser than addition, so `a + 1 == b` groups as
    /// `(a + 1) == b`, and it does not chain.
    fn equality(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.addition()?;
        let negated = if self.at_infix(&Token::EqualEqual) {
            false
        } else if self.at_infix(&Token::BangEqual) {
            true
        } else {
            return Ok(left);
        };
        self.continuation();
        let right = self.addition()?;
        let span = left.span.start..right.span.end;
        Ok(Expr {
            kind: Kind::Compare {
                left: Box::new(left),
                right: Box::new(right),
                negated,
            },
            span,
        })
    }

    fn addition(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.postfix()?;
        while self.at_infix(&Token::Plus) {
            self.additions += 1;
            if self.additions > MAX_ADDITIONS {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    format!("bootstrap limit: at most {} literals", MAX_ADDITIONS + 1),
                ));
            }
            self.continuation();
            let right = self.postfix()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::Add(Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.primary()?;
        loop {
            if self.eat(&Token::Dot) {
                let field = self.advance();
                let Token::Ident(name) = field.token else {
                    return Err(Diagnostic::syntax(field.span, "expected a field name"));
                };
                let span = expression.span.start..field.span.end;
                expression = Expr {
                    kind: Kind::Field(Box::new(expression), name),
                    span,
                };
                continue;
            }
            if matches!(self.peek(), Token::OpenParen) {
                let (arguments, end) = self.arguments()?;
                let span = expression.span.start..end;
                expression = Expr {
                    kind: Kind::Call {
                        callee: Box::new(expression),
                        arguments,
                    },
                    span,
                };
                continue;
            }
            return Ok(expression);
        }
    }

    fn arguments(&mut self) -> Result<(Vec<Arg>, usize), Diagnostic> {
        self.expect(&Token::OpenParen, "to open an argument list")?;
        self.nesting += 1;
        let mut arguments = Vec::new();
        if !matches!(self.peek(), Token::CloseParen) {
            loop {
                arguments.push(self.argument()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        let close = self.expect(&Token::CloseParen, "after arguments")?;
        self.nesting -= 1;
        Ok((arguments, close.span.end))
    }

    fn argument(&mut self) -> Result<Arg, Diagnostic> {
        if let Token::Ident(name) = self.peek().clone() {
            let mut lookahead = self.index;
            while matches!(self.tokens[lookahead].token, Token::Newline) {
                lookahead += 1;
            }
            if matches!(self.tokens[lookahead + 1].token, Token::Equals) {
                self.advance();
                self.advance();
                let value = self.expression()?;
                return Ok(Arg {
                    name: Some(name),
                    value,
                });
            }
        }
        Ok(Arg {
            name: None,
            value: self.expression()?,
        })
    }

    fn data_literal(&mut self) -> Result<Expr, Diagnostic> {
        let open = self.expect(&Token::OpenBrace, "to open a data literal")?;
        self.nesting += 1;
        let mut entries = Vec::new();
        while !matches!(self.peek(), Token::CloseBrace) {
            let key = self.advance();
            let name = match key.token {
                Token::Ident(name) => name,
                Token::Str(text) => text,
                other => {
                    return Err(Diagnostic::syntax(
                        key.span,
                        format!("expected a data key, found {}", other.describe()),
                    ));
                }
            };
            self.expect(&Token::Colon, "after a data key")?;
            let value = self.expression()?;
            entries.push((name, value));
            if !self.eat(&Token::Comma) && !matches!(self.peek(), Token::CloseBrace) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    format!(
                        "expected `,` or `}}` in a data literal, found {}",
                        self.peek().describe()
                    ),
                ));
            }
        }
        let close = self.expect(&Token::CloseBrace, "to close a data literal")?;
        self.nesting -= 1;
        Ok(Expr {
            kind: Kind::Data(entries),
            span: open.span.start..close.span.end,
        })
    }

    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek(), Token::OpenBracket) {
            return self.collection_literal(false);
        }
        if matches!(self.peek(), Token::OpenBrace) {
            let mut next = self.index + 1;
            while matches!(self.tokens[next].token, Token::Newline) {
                next += 1;
            }
            let mut after = next + 1;
            while self
                .tokens
                .get(after)
                .is_some_and(|token| matches!(token.token, Token::Newline))
            {
                after += 1;
            }
            if !matches!(self.tokens[next].token, Token::CloseBrace | Token::End)
                && !self
                    .tokens
                    .get(after)
                    .is_some_and(|token| matches!(token.token, Token::Colon))
            {
                return self.collection_literal(true);
            }
            return self.data_literal();
        }
        if matches!(self.peek(), Token::Ident(_))
            && matches!(self.tokens[self.index + 1].token, Token::Less)
        {
            let annotation = self.type_annotation()?;
            let (arguments, end) = self.arguments()?;
            let span = annotation.span.start..end;
            return Ok(Expr {
                kind: Kind::Construct {
                    annotation,
                    arguments,
                },
                span,
            });
        }
        if matches!(self.peek(), Token::OpenParen) {
            let mut look = self.index + 1;
            while matches!(
                self.tokens[look].token,
                Token::Ident(_) | Token::Comma | Token::Newline
            ) {
                look += 1;
            }
            if matches!(self.tokens[look].token, Token::CloseParen)
                && matches!(self.tokens[look + 1].token, Token::FatArrow)
            {
                let start = self.advance().span.start;
                self.nesting += 1;
                let mut parameters = Vec::new();
                loop {
                    let name = self.advance();
                    let Token::Ident(name) = name.token else {
                        return Err(Diagnostic::syntax(name.span, "expected a lambda parameter"));
                    };
                    if parameters.contains(&name) {
                        return Err(Diagnostic::syntax(
                            start..self.peek_span().end,
                            "lambda parameters must be distinct",
                        ));
                    }
                    parameters.push(name);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::CloseParen, "after lambda parameters")?;
                self.nesting -= 1;
                self.expect(&Token::FatArrow, "before a lambda body")?;
                let body = self.nested(Self::pipeline)?;
                let span = start..body.span.end;
                return Ok(Expr {
                    kind: Kind::Lambda {
                        parameters,
                        body: Box::new(body),
                    },
                    span,
                });
            }
            self.advance();
            self.nesting += 1;
            let inner = self.nested(Self::pipeline)?;
            self.expect(&Token::CloseParen, "to close a group")?;
            self.nesting -= 1;
            return Ok(inner);
        }
        let head = self.advance();
        let kind = match head.token {
            Token::OpenBracket => {
                return Err(Diagnostic::syntax(
                    head.span,
                    "square brackets are value-level; sequence literals are not implemented",
                ));
            }
            Token::Int(value) => Kind::Int(value),
            Token::Float(value) => Kind::Float(value),
            Token::Bool(value) => Kind::Bool(value),
            Token::Str(text) => Kind::Str(text),
            Token::DocRef(name) => Kind::DocRef(name),
            Token::Ident(name) => {
                if matches!(self.peek(), Token::FatArrow) {
                    self.advance();
                    self.continuation();
                    let body = self.nested(Self::pipeline)?;
                    let span = head.span.start..body.span.end;
                    return Ok(Expr {
                        kind: Kind::Lambda {
                            parameters: vec![name],
                            body: Box::new(body),
                        },
                        span,
                    });
                }
                if name == "cards" {
                    Kind::Cards
                } else {
                    Kind::Ident(name)
                }
            }
            other => {
                return Err(Diagnostic::syntax(
                    head.span,
                    format!("expected an expression, found {}", other.describe()),
                ));
            }
        };
        Ok(Expr {
            kind,
            span: head.span,
        })
    }

    fn collection_literal(&mut self, set: bool) -> Result<Expr, Diagnostic> {
        let open = self.advance();
        let close = if set {
            Token::CloseBrace
        } else {
            Token::CloseBracket
        };
        self.nesting += 1;
        let mut elements = Vec::new();
        while self.peek() != &close {
            elements.push(self.nested(Self::pipeline)?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        let end = self.expect(&close, "after collection elements")?.span.end;
        self.nesting -= 1;
        Ok(Expr {
            kind: if set {
                Kind::Set(elements)
            } else {
                Kind::List(elements)
            },
            span: open.span.start..end,
        })
    }
}
