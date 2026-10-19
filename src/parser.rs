//! Parse tokens into a program: statements, bindings, pipelines and calls.

use crate::ast::{
    ALIAS_INPUT, AliasDecl, AliasParam, Arg, Arith, Arm, BlockStmt, Expr, Field, FnDecl, Kind,
    Logic, Param, Pattern, Program, Relation, Stmt, TEE_INPUT, TypeAnn, TypeDecl,
};
use crate::diagnostics::Diagnostic;
use crate::lexer::{self, Spanned, Token};
use std::ops::Range;

// Bound the left-deep operator chains, including their traversal and destruction.
const MAX_OPERATORS: usize = 255;
// Bound recursion through parentheses, calls, data literals and lambdas.
const MAX_DEPTH: usize = 128;

/// Parse a whole program. The entire input must be consumed.
pub(crate) fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = lexer::scan(source)?;
    let mut parser = Parser {
        tokens,
        index: 0,
        nesting: 0,
        operators: 0,
        depth: 0,
        line_starts: std::iter::once(0)
            .chain(source.match_indices('\n').map(|(index, _)| index + 1))
            .collect(),
    };
    parser.program()
}

struct Parser {
    tokens: Vec<Spanned>,
    index: usize,
    /// Bracket nesting; inside brackets a newline is not a statement end.
    nesting: usize,
    /// Infix arithmetic and connective operators seen so far.
    operators: usize,
    depth: usize,
    /// Where each line of the source starts, for the columns `tee` reads.
    line_starts: Vec<usize>,
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
        if let Some(alias) = self.alias()? {
            return Ok(Stmt::Alias(alias));
        }
        if let Token::Ident(keyword) = self.peek().clone()
            && keyword == "fn"
            && matches!(self.tokens[self.index + 1].token, Token::Ident(_))
        {
            return Ok(Stmt::Fn(self.function()?));
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
                let name_span = self.advance().span;
                let annotation = if self.eat(&Token::Colon) {
                    Some(self.type_annotation()?)
                } else {
                    None
                };
                self.expect(&Token::Equals, "in a binding")?;
                self.continuation();
                return Ok(Stmt::Bind {
                    name,
                    name_span,
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

    /// A type, including a function type: `A -> R`, `(A, B) -> R`, `() -> R`.
    /// The arrow groups to the right.
    fn type_annotation(&mut self) -> Result<TypeAnn, Diagnostic> {
        let start = self.peek_span().start;
        let parameters = if matches!(self.peek(), Token::OpenParen) {
            self.advance();
            self.nesting += 1;
            let mut parameters = Vec::new();
            if !matches!(self.peek(), Token::CloseParen) {
                loop {
                    parameters.push(self.nested(Self::type_annotation)?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Token::CloseParen, "after parameter types")?;
            self.nesting -= 1;
            if !matches!(self.peek(), Token::Arrow) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    "parenthesised types are the parameters of a function type: add `-> R`",
                ));
            }
            parameters
        } else {
            let single = self.simple_type()?;
            if !matches!(self.peek(), Token::Arrow) {
                return Ok(single);
            }
            vec![single]
        };
        self.expect(&Token::Arrow, "in a function type")?;
        let result = self.nested(Self::type_annotation)?;
        let span = start..result.span.end;
        let mut arguments = parameters;
        let name = format!("Fn{}", arguments.len());
        arguments.push(result);
        Ok(TypeAnn {
            name,
            arguments,
            span,
        })
    }

    fn simple_type(&mut self) -> Result<TypeAnn, Diagnostic> {
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

    /// `{ statement* }`, one statement per line: the block body of an
    /// iterating function. Newlines separate statements here, as they do at
    /// the top level, whatever brackets surround the declaration.
    fn block(&mut self) -> Result<Vec<BlockStmt>, Diagnostic> {
        let saved = std::mem::replace(&mut self.nesting, 0);
        self.expect(&Token::OpenBrace, "to open the body")?;
        let mut statements = Vec::new();
        loop {
            self.continuation();
            if self.eat(&Token::CloseBrace) {
                break;
            }
            if matches!(self.peek(), Token::End) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    "expected `}` to close the body",
                ));
            }
            statements.push(self.nested(Self::block_statement)?);
            if !matches!(
                self.tokens[self.index].token,
                Token::Newline | Token::CloseBrace
            ) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    format!(
                        "expected end of statement, found {}",
                        self.peek().describe()
                    ),
                ));
            }
        }
        self.nesting = saved;
        Ok(statements)
    }

    fn block_statement(&mut self) -> Result<BlockStmt, Diagnostic> {
        let head = self.peek().clone();
        let next = self.tokens[self.index + 1].token.clone();
        match head {
            Token::Ident(keyword) if keyword == "yield" => {
                self.advance();
                Ok(BlockStmt::Yield(self.expression()?))
            }
            Token::Ident(keyword) if keyword == "for" => {
                self.advance();
                let named = self.advance();
                let Token::Ident(name) = named.token else {
                    return Err(Diagnostic::syntax(
                        named.span,
                        "expected a name after `for`",
                    ));
                };
                self.expect(&Token::Ident("in".to_owned()), "after the loop variable")?;
                let iterable = self.expression()?;
                let body = self.block()?;
                Ok(BlockStmt::For {
                    name,
                    iterable,
                    body,
                })
            }
            Token::Ident(name) if next == Token::Equals => {
                self.advance();
                self.advance();
                self.continuation();
                Ok(BlockStmt::Bind {
                    name,
                    value: self.expression()?,
                })
            }
            _ => Err(Diagnostic::syntax(
                self.peek_span(),
                "a block body holds `yield`, `for` and bindings",
            )),
        }
    }

    /// `name(p, ..) := body`, when the statement is one: a name, optional
    /// parameters, then `:` and `=` side by side.
    fn alias(&mut self) -> Result<Option<AliasDecl>, Diagnostic> {
        let Token::Ident(name) = self.peek().clone() else {
            return Ok(None);
        };
        let mut after = self.index + 1;
        if matches!(self.tokens[after].token, Token::OpenParen) {
            let mut depth = 0;
            while after < self.tokens.len() {
                match self.tokens[after].token {
                    Token::OpenParen => depth += 1,
                    Token::CloseParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Token::End | Token::Newline if depth == 0 => return Ok(None),
                    _ => {}
                }
                after += 1;
            }
            after += 1;
        }
        let (Some(colon), Some(equals)) = (self.tokens.get(after), self.tokens.get(after + 1))
        else {
            return Ok(None);
        };
        if !(colon.token == Token::Colon
            && equals.token == Token::Equals
            && colon.span.end == equals.span.start)
        {
            return Ok(None);
        }
        let start = self.advance().span.start;
        let mut parameters = Vec::new();
        if self.eat(&Token::OpenParen) {
            self.nesting += 1;
            if !matches!(self.peek(), Token::CloseParen) {
                loop {
                    let written = self.advance();
                    let Token::Ident(parameter) = written.token else {
                        return Err(Diagnostic::syntax(
                            written.span,
                            "expected a parameter name",
                        ));
                    };
                    let annotation = if self.eat(&Token::Colon) {
                        Some(self.type_annotation()?)
                    } else {
                        None
                    };
                    parameters.push(AliasParam {
                        name: parameter,
                        annotation,
                        span: written.span,
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Token::CloseParen, "to close the parameters")?;
            self.nesting -= 1;
        }
        self.expect(&Token::Colon, "in `:=`")?;
        self.expect(&Token::Equals, "in `:=`")?;
        self.continuation();
        let body = self.expression()?;
        // A body that opens with a step is a fragment over an implicit
        // input; any other body is an expression over the first parameter.
        let opening = leftmost(&body);
        let step = match &opening.kind {
            Kind::Ident(step) => Some(step.as_str()),
            Kind::Call { callee, .. } => match &callee.kind {
                Kind::Ident(step) => Some(step.as_str()),
                _ => None,
            },
            _ => None,
        };
        let fragment = step.is_some_and(|step| {
            crate::builtins::exists(step) && !parameters.iter().any(|held| held.name == step)
        });
        let body = if fragment {
            parameters.insert(
                0,
                AliasParam {
                    name: ALIAS_INPUT.to_owned(),
                    annotation: None,
                    span: body.span.start..body.span.start,
                },
            );
            with_input(body)
        } else {
            body
        };
        let span = start..body.span.end;
        Ok(Some(AliasDecl {
            name,
            parameters,
            body,
            span,
        }))
    }

    /// `fn name<T>(p: Type, xs: ...Type) : Result = body`.
    fn function(&mut self) -> Result<FnDecl, Diagnostic> {
        let start = self.advance().span.start;
        let named = self.advance();
        let Token::Ident(name) = named.token else {
            return Err(Diagnostic::syntax(named.span, "expected a function name"));
        };
        let mut generics = Vec::new();
        let mut bounds = Vec::new();
        if self.eat(&Token::Less) {
            loop {
                let parameter = self.advance();
                let Token::Ident(parameter) = parameter.token else {
                    return Err(Diagnostic::syntax(
                        parameter.span,
                        "expected a type parameter",
                    ));
                };
                if self.eat(&Token::Colon) {
                    bounds.push((parameter.clone(), self.simple_type()?));
                }
                generics.push(parameter);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Greater, "after type parameters")?;
        }
        self.expect(&Token::OpenParen, "to open the parameters")?;
        self.nesting += 1;
        let mut parameters: Vec<Param> = Vec::new();
        if !matches!(self.peek(), Token::CloseParen) {
            loop {
                let written = self.advance();
                let Token::Ident(parameter) = written.token else {
                    return Err(Diagnostic::syntax(
                        written.span,
                        "expected a parameter name",
                    ));
                };
                if parameters.iter().any(|seen| seen.name == parameter) {
                    return Err(Diagnostic::syntax(
                        written.span,
                        "parameters must be distinct",
                    ));
                }
                self.expect(
                    &Token::Colon,
                    "after a parameter name: every parameter is typed",
                )?;
                let variadic = self.eat(&Token::Dot);
                if variadic {
                    self.expect(&Token::Dot, "in `...`")?;
                    self.expect(&Token::Dot, "in `...`")?;
                }
                let annotation = self.type_annotation()?;
                parameters.push(Param {
                    name: parameter,
                    span: written.span,
                    annotation,
                    variadic,
                });
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::CloseParen, "to close the parameters")?;
        self.nesting -= 1;
        if let Some(index) = parameters.iter().position(|parameter| parameter.variadic)
            && index + 1 != parameters.len()
        {
            return Err(Diagnostic::syntax(
                start..self.peek_span().end,
                "a variadic parameter is the last one",
            ));
        }
        self.expect(&Token::Colon, "before the result type")?;
        let result = self.type_annotation()?;
        let body = if matches!(self.peek(), Token::OpenBrace) {
            let open = self.peek_span().start;
            let statements = self.block()?;
            let close = self.tokens[self.index.saturating_sub(1)].span.end;
            Expr {
                kind: Kind::Block(statements),
                span: open..close,
            }
        } else {
            self.expect(&Token::Equals, "before the function body")?;
            self.continuation();
            self.expression()?
        };
        let span = start..body.span.end;
        Ok(FnDecl {
            name,
            generics,
            bounds,
            parameters,
            result,
            body,
            span,
        })
    }

    fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.nested(Self::given)
    }

    /// `value given condition otherwise other` binds looser than a pipeline,
    /// and `otherwise` groups to the right, so the short forms chain.
    fn given(&mut self) -> Result<Expr, Diagnostic> {
        let value = self.pipeline()?;
        if !self.at_infix(&Token::Given) {
            return Ok(value);
        }
        self.continuation();
        let condition = self.pipeline()?;
        let otherwise = if self.at_infix(&Token::Otherwise) {
            self.continuation();
            Some(Box::new(self.nested(Self::given)?))
        } else {
            None
        };
        let end = otherwise
            .as_ref()
            .map_or(condition.span.end, |other| other.span.end);
        let span = value.span.start..end;
        Ok(Expr {
            kind: Kind::Given {
                value: Box::new(value),
                condition: Box::new(condition),
                otherwise,
            },
            span,
        })
    }

    /// `match subject { pattern [given guard] => result, .. }`.
    fn match_expression(&mut self, start: usize) -> Result<Expr, Diagnostic> {
        let subject = self.nested(Self::pipeline)?;
        self.expect(&Token::OpenBrace, "to open the arms of a match")?;
        self.nesting += 1;
        let mut arms = Vec::new();
        loop {
            while self.eat(&Token::Newline) {}
            if matches!(self.peek(), Token::CloseBrace) {
                break;
            }
            let arm_start = self.peek_span().start;
            let pattern = self.pattern()?;
            let guard = if self.eat(&Token::Given) {
                // A bare name before `=>` is the guard, not a lambda.
                if let (Token::Ident(name), Token::FatArrow) =
                    (self.peek().clone(), &self.tokens[self.index + 1].token)
                {
                    let span = self.advance().span;
                    Some(Expr {
                        kind: Kind::Ident(name),
                        span,
                    })
                } else {
                    Some(self.nested(Self::pipeline)?)
                }
            } else {
                None
            };
            self.expect(&Token::FatArrow, "after a match pattern")?;
            self.continuation();
            let body = self.expression()?;
            let span = arm_start..body.span.end;
            arms.push(Arm {
                pattern,
                guard,
                body,
                span,
            });
            while self.eat(&Token::Newline) {}
            if !self.eat(&Token::Comma) && !matches!(self.peek(), Token::CloseBrace) {
                return Err(Diagnostic::syntax(
                    self.peek_span(),
                    format!(
                        "expected `,` or `}}` after a match arm, found {}",
                        self.peek().describe()
                    ),
                ));
            }
        }
        let close = self.expect(&Token::CloseBrace, "to close a match")?;
        self.nesting -= 1;
        if arms.is_empty() {
            return Err(Diagnostic::syntax(
                start..close.span.end,
                "a match needs at least one arm",
            ));
        }
        Ok(Expr {
            kind: Kind::Match {
                subject: Box::new(subject),
                arms,
            },
            span: start..close.span.end,
        })
    }

    fn pattern(&mut self) -> Result<Pattern, Diagnostic> {
        let head = self.advance();
        Ok(match head.token {
            Token::Int(value) => Pattern::Int(value),
            Token::Float(value) => Pattern::Float(value),
            Token::Bool(value) => Pattern::Bool(value),
            Token::Str(text) => Pattern::Str(text),
            Token::Minus => match self.advance().token {
                Token::Int(value) => Pattern::Int(-value),
                Token::Float(value) => Pattern::Float(-value),
                _ => return Err(Diagnostic::syntax(head.span, "expected a number after `-`")),
            },
            Token::Ident(name) if name == "_" => Pattern::Wildcard,
            Token::Ident(name) if name == "none" => Pattern::None,
            Token::Ident(name) if name == "some" => {
                self.expect(&Token::OpenParen, "after `some` in a pattern")?;
                let inner = self.pattern()?;
                self.expect(&Token::CloseParen, "to close `some(…)`")?;
                Pattern::Some(Box::new(inner))
            }
            // `U name`: a type name, capitalised, followed by what it binds.
            Token::Ident(name)
                if name.starts_with(|c: char| c.is_uppercase())
                    && matches!(self.peek(), Token::Ident(_)) =>
            {
                let inner = self.pattern()?;
                if !inner.is_irrefutable() {
                    return Err(Diagnostic::syntax(
                        head.span,
                        "a type pattern binds a name or `_`",
                    ));
                }
                Pattern::Type(name, Box::new(inner))
            }
            Token::Ident(name) => Pattern::Bind(name),
            other => {
                return Err(Diagnostic::syntax(
                    head.span,
                    format!("expected a match pattern, found {}", other.describe()),
                ));
            }
        })
    }

    /// The column a byte offset sits at.
    fn column(&self, offset: usize) -> usize {
        let line = self.line_starts.partition_point(|start| *start <= offset);
        offset - self.line_starts[line.saturating_sub(1)]
    }

    /// Whether the next line continues a `tee` branch: it opens with `|`
    /// indented past the column of the `|` that opened the `tee`.
    fn branch_continues(&mut self, pipe_column: usize) -> bool {
        let mut lookahead = self.index;
        if !matches!(self.tokens[lookahead].token, Token::Newline) {
            return false;
        }
        while matches!(self.tokens[lookahead].token, Token::Newline) {
            lookahead += 1;
        }
        let next = &self.tokens[lookahead];
        if next.token == Token::Pipe && self.column(next.span.start) > pipe_column {
            self.index = lookahead + 1;
            return true;
        }
        false
    }

    /// `tee name = branch`. On one line the branch is one step; after a line
    /// break it is every step indented past the `|` that opened the `tee`.
    fn tee(&mut self, pipe_column: usize) -> Result<Expr, Diagnostic> {
        let keyword = self.advance();
        let named = self.advance();
        let Token::Ident(name) = named.token else {
            return Err(Diagnostic::syntax(
                named.span,
                "expected a name after `tee`",
            ));
        };
        self.expect(&Token::Equals, "after the name a `tee` binds")?;
        let multiline = matches!(self.peek(), Token::Newline);
        self.continuation();
        let input = Expr {
            kind: Kind::Ident(TEE_INPUT.to_owned()),
            span: keyword.span.clone(),
        };
        let first = self.link()?;
        let mut branch = Expr {
            span: keyword.span.start..first.span.end,
            kind: Kind::Pipe(Box::new(input), Box::new(first)),
        };
        while multiline && self.branch_continues(pipe_column) {
            self.continuation();
            let next = self.link()?;
            branch = Expr {
                span: branch.span.start..next.span.end,
                kind: Kind::Pipe(Box::new(branch), Box::new(next)),
            };
        }
        let span = keyword.span.start..branch.span.end;
        Ok(Expr {
            kind: Kind::Tee {
                name,
                branch: Box::new(branch),
            },
            span,
        })
    }

    fn at_tee(&self) -> bool {
        matches!(self.peek(), Token::Ident(word) if word == "tee")
            && matches!(self.tokens[self.index + 1].token, Token::Ident(_))
            && matches!(self.tokens[self.index + 2].token, Token::Equals)
    }

    fn pipeline(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.link()?;
        while self.at_infix(&Token::Pipe) {
            let pipe_column = self.column(self.tokens[self.index - 1].span.start);
            self.continuation();
            let right = if self.at_tee() {
                self.tee(pipe_column)?
            } else if self.at_type_step() {
                // `xs | List<Int>`: a constructor with its type arguments and
                // no argument list of its own.
                let annotation = self.simple_type()?;
                let span = annotation.span.clone();
                Expr {
                    kind: Kind::Construct {
                        annotation,
                        arguments: Vec::new(),
                    },
                    span,
                }
            } else {
                self.link()?
            };
            left = piped(left, right);
        }
        Ok(left)
    }

    /// Whether the next step is a type name with type arguments and no
    /// argument list: `List<Int>` ending the step.
    fn at_type_step(&self) -> bool {
        let Token::Ident(name) = self.peek() else {
            return false;
        };
        if !is_type_name(name) || !matches!(self.tokens[self.index + 1].token, Token::Less) {
            return false;
        }
        let mut depth = 0usize;
        for (offset, spanned) in self.tokens[self.index + 1..].iter().enumerate() {
            match spanned.token {
                Token::Less => depth += 1,
                Token::Greater => {
                    depth -= 1;
                    if depth == 0 {
                        let after = self.index + 1 + offset + 1;
                        return !matches!(self.tokens[after].token, Token::OpenParen);
                    }
                }
                Token::Ident(_) | Token::Comma => {}
                _ => return false,
            }
        }
        false
    }

    fn link(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.disjunction()?;
        if !self.at_infix(&Token::Arrow) {
            return Ok(left);
        }
        self.continuation();
        let target = self.disjunction()?;
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

    /// Count one more chained operator against the bootstrap limit.
    fn chained(&mut self) -> Result<(), Diagnostic> {
        self.operators += 1;
        if self.operators > MAX_OPERATORS {
            return Err(Diagnostic::syntax(
                self.peek_span(),
                format!("bootstrap limit: at most {} operands", MAX_OPERATORS + 1),
            ));
        }
        self.continuation();
        Ok(())
    }

    fn disjunction(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.conjunction()?;
        while self.at_infix(&Token::Or) {
            self.chained()?;
            let right = self.conjunction()?;
            left = logic(Logic::Or, left, right);
        }
        Ok(left)
    }

    fn conjunction(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.negation()?;
        while self.at_infix(&Token::And) {
            self.chained()?;
            let right = self.negation()?;
            left = logic(Logic::And, left, right);
        }
        Ok(left)
    }

    /// `not` binds looser than a comparison, so `not a == b` is `not (a == b)`.
    fn negation(&mut self) -> Result<Expr, Diagnostic> {
        if !matches!(self.peek(), Token::Not) {
            return self.comparison();
        }
        let start = self.advance().span.start;
        let operand = self.nested(Self::negation)?;
        let span = start..operand.span.end;
        Ok(Expr {
            kind: Kind::Not(Box::new(operand)),
            span,
        })
    }

    /// A comparison binds looser than arithmetic, so `a + 1 == b` groups as
    /// `(a + 1) == b`, and it does not chain: `a < b < c` is refused.
    fn comparison(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.addition()?;
        let operator = if self.at_infix(&Token::EqualEqual) {
            Comparison::Equal(false)
        } else if self.at_infix(&Token::BangEqual) {
            Comparison::Equal(true)
        } else if let Some(relation) = self.relation() {
            Comparison::Order(relation)
        } else {
            return Ok(left);
        };
        self.continuation();
        let right = self.addition()?;
        let span = left.span.start..right.span.end;
        let (left, right) = (Box::new(left), Box::new(right));
        let kind = match operator {
            Comparison::Equal(negated) => Kind::Compare {
                left,
                right,
                negated,
            },
            Comparison::Order(relation) => Kind::Relate {
                left,
                right,
                relation,
            },
        };
        if self.relation().is_some()
            || self.at_infix(&Token::EqualEqual)
            || self.at_infix(&Token::BangEqual)
        {
            return Err(Diagnostic::syntax(
                span.end..self.tokens[self.index - 1].span.end,
                "comparisons do not chain: combine them with `and`",
            ));
        }
        Ok(Expr { kind, span })
    }

    fn relation(&mut self) -> Option<Relation> {
        [
            (Token::Less, Relation::Less),
            (Token::LessEqual, Relation::LessEqual),
            (Token::Greater, Relation::Greater),
            (Token::GreaterEqual, Relation::GreaterEqual),
        ]
        .into_iter()
        .find_map(|(token, relation)| self.at_infix(&token).then_some(relation))
    }

    fn addition(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.multiplication()?;
        loop {
            let operator = if self.at_infix(&Token::Plus) {
                Arith::Add
            } else if self.at_infix(&Token::Minus) {
                Arith::Sub
            } else {
                return Ok(left);
            };
            self.chained()?;
            let right = self.multiplication()?;
            left = arith(operator, left, right);
        }
    }

    fn multiplication(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.postfix()?;
        loop {
            let operator = if self.at_infix(&Token::Star) {
                Arith::Mul
            } else if self.at_infix(&Token::Slash) {
                Arith::Div
            } else if self.at_infix(&Token::Percent) {
                Arith::Rem
            } else {
                return Ok(left);
            };
            self.chained()?;
            let right = self.postfix()?;
            left = arith(operator, left, right);
        }
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
                expression = holed(Expr {
                    kind: Kind::Call {
                        callee: Box::new(expression),
                        arguments,
                    },
                    span,
                });
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
        if let Token::Ident(keyword) = self.peek()
            && matches!(keyword.as_str(), "yield" | "for")
            && !matches!(
                self.tokens[self.index + 1].token,
                Token::Newline | Token::End | Token::Equals | Token::CloseParen | Token::Comma
            )
        {
            return Err(Diagnostic::syntax(
                self.peek_span(),
                format!("`{keyword}` is written only in the block body of a user `fn`"),
            ));
        }
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
        if matches!(self.peek(), Token::Ident(_)) && self.at_construction() {
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
            // A parenthesised list followed by `=>` is a lambda's parameters.
            let mut look = self.index + 1;
            let mut depth = 1;
            while depth > 0 && !matches!(self.tokens[look].token, Token::End) {
                match self.tokens[look].token {
                    Token::OpenParen => depth += 1,
                    Token::CloseParen => depth -= 1,
                    _ => {}
                }
                look += 1;
            }
            if depth == 0 && matches!(self.tokens[look].token, Token::FatArrow) {
                let start = self.advance().span.start;
                self.nesting += 1;
                let mut parameters = Vec::new();
                let mut annotations = Vec::new();
                while !matches!(self.peek(), Token::CloseParen) {
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
                    annotations.push(if self.eat(&Token::Colon) {
                        Some(self.type_annotation()?)
                    } else {
                        None
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::CloseParen, "after lambda parameters")?;
                self.nesting -= 1;
                self.expect(&Token::FatArrow, "before a lambda body")?;
                let body = self.nested(Self::given)?;
                let span = start..body.span.end;
                return Ok(Expr {
                    kind: Kind::Lambda {
                        parameters,
                        annotations,
                        body: Box::new(body),
                    },
                    span,
                });
            }
            self.advance();
            self.nesting += 1;
            let inner = self.nested(Self::given)?;
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
            Token::Match => return self.match_expression(head.span.start),
            Token::Ident(name) => {
                if matches!(self.peek(), Token::FatArrow) {
                    self.advance();
                    self.continuation();
                    let body = self.nested(Self::given)?;
                    let span = head.span.start..body.span.end;
                    return Ok(Expr {
                        kind: Kind::Lambda {
                            parameters: vec![name],
                            annotations: vec![None],
                            body: Box::new(body),
                        },
                        span,
                    });
                }
                Kind::Ident(name)
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

    /// Whether the name ahead opens a constructor, `List<Int>(…)`, rather
    /// than a comparison, `n < 3`: the angle brackets must hold only type
    /// names and close directly before an argument list.
    fn at_construction(&self) -> bool {
        let mut index = self.index;
        while matches!(self.tokens[index].token, Token::Newline) {
            index += 1;
        }
        if !matches!(self.tokens[index + 1].token, Token::Less) {
            return false;
        }
        let mut depth = 0usize;
        for (offset, spanned) in self.tokens[index + 1..].iter().enumerate() {
            match spanned.token {
                Token::Less => depth += 1,
                Token::Greater => {
                    depth -= 1;
                    if depth == 0 {
                        let after = index + 1 + offset + 1;
                        return matches!(self.tokens[after].token, Token::OpenParen);
                    }
                }
                Token::Ident(_) | Token::Comma | Token::Newline => {}
                _ => return false,
            }
        }
        false
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
            elements.push(self.nested(Self::given)?);
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

/// Which comparison a `comparison` rule has read.
enum Comparison {
    Equal(bool),
    Order(Relation),
}

fn arith(operator: Arith, left: Expr, right: Expr) -> Expr {
    let span = left.span.start..right.span.end;
    Expr {
        kind: Kind::Arith(operator, Box::new(left), Box::new(right)),
        span,
    }
}

fn logic(connective: Logic, left: Expr, right: Expr) -> Expr {
    let span = left.span.start..right.span.end;
    Expr {
        kind: Kind::Logic(connective, Box::new(left), Box::new(right)),
        span,
    }
}

/// Whether a name is written as a type: capitalised, as only a type and its
/// constructor may be.
fn is_type_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_uppercase())
}

/// The name a hole's parameter takes. It cannot be written, so it never
/// meets a name in the program.
pub(crate) const HOLE: &str = "%hole";

/// A call with a hole, `f(a, _)`, is a function of its holes, in order:
/// `(%hole0) => f(a, %hole0)`. A hole stands only as a whole argument.
fn holed(call: Expr) -> Expr {
    let Kind::Call { callee, arguments } = call.kind else {
        return call;
    };
    let is_hole = |argument: &Arg| matches!(&argument.value.kind, Kind::Ident(name) if name == "_");
    if !arguments.iter().any(is_hole) {
        return Expr {
            kind: Kind::Call { callee, arguments },
            span: call.span,
        };
    }
    let mut parameters = Vec::new();
    let arguments = arguments
        .into_iter()
        .map(|argument| {
            if !is_hole(&argument) {
                return argument;
            }
            let name = format!("{HOLE}{}", parameters.len());
            parameters.push(name.clone());
            Arg {
                name: argument.name,
                value: Expr {
                    kind: Kind::Ident(name),
                    span: argument.value.span,
                },
            }
        })
        .collect();
    Expr {
        kind: Kind::Lambda {
            annotations: vec![None; parameters.len()],
            parameters,
            body: Box::new(Expr {
                kind: Kind::Call { callee, arguments },
                span: call.span.clone(),
            }),
        },
        span: call.span,
    }
}

/// The first step of a pipeline, or the whole expression when it is none.
fn leftmost(expression: &Expr) -> &Expr {
    match &expression.kind {
        Kind::Pipe(left, _) => leftmost(left),
        _ => expression,
    }
}

/// A fragment with the alias's implicit input piped into its first step.
fn with_input(fragment: Expr) -> Expr {
    match fragment.kind {
        Kind::Pipe(left, right) => {
            let span = fragment.span;
            Expr {
                kind: Kind::Pipe(Box::new(with_input(*left)), right),
                span,
            }
        }
        _ => {
            let input = Expr {
                kind: Kind::Ident(ALIAS_INPUT.to_owned()),
                span: fragment.span.start..fragment.span.start,
            };
            piped(input, fragment)
        }
    }
}

/// `left | right`, where a type name in step position is its constructor and
/// the piped value its first argument: `x | T` is `T(x)` and `x | T(a)` is
/// `T(x, a)`.
fn piped(left: Expr, right: Expr) -> Expr {
    let span = left.span.start..right.span.end;
    let input = Arg {
        name: None,
        value: left,
    };
    let kind = match right.kind {
        Kind::Ident(name) if is_type_name(&name) => Kind::Call {
            callee: Box::new(Expr {
                kind: Kind::Ident(name),
                span: right.span,
            }),
            arguments: vec![input],
        },
        Kind::Call {
            callee,
            mut arguments,
        } if matches!(&callee.kind, Kind::Ident(name) if is_type_name(name)) => {
            arguments.insert(0, input);
            Kind::Call { callee, arguments }
        }
        Kind::Construct {
            annotation,
            mut arguments,
        } => {
            arguments.insert(0, input);
            Kind::Construct {
                annotation,
                arguments,
            }
        }
        kind => Kind::Pipe(
            Box::new(input.value),
            Box::new(Expr {
                kind,
                span: right.span,
            }),
        ),
    };
    Expr { kind, span }
}
