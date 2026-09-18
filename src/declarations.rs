//! Declared types: what a `type` statement says, and whether it holds.
//!
//! A declaration is checked and yields `Unit`. It introduces no value and no
//! case in the [lattice](crate::types) — constructing a value of a declared
//! record type is separate work. What is checked here is that the declaration
//! is coherent: its parents exist, its parents can be narrowed together, and
//! where it restates a type this binary already has, it agrees with it.

use crate::ast::{TypeAnn, TypeDecl};
use crate::diagnostics::Diagnostic;
use crate::types::{self, Type};
use std::collections::HashMap;
use std::ops::Range;

/// The lattice type a written annotation denotes, when the binary has one.
///
/// A type parameter has no lattice counterpart, so it stands as `Data`: the
/// question a declaration answers is about the shape, not about what a
/// parameter will later be bound to.
fn lattice(written: &TypeAnn) -> Option<Type> {
    let arguments: Vec<Type> = written
        .arguments
        .iter()
        .map(|argument| lattice(argument).unwrap_or(Type::Data))
        .collect();
    types::named(&written.name, &arguments)
}

/// What a program has declared so far.
#[derive(Debug, Default)]
pub(crate) struct Declarations {
    known: HashMap<String, Declared>,
}

/// One declaration, reduced to what a later one may need to know.
#[derive(Debug)]
struct Declared {
    parameters: usize,
    /// The declared parents, by name.
    parents: Vec<String>,
    /// Each field, and whether the key may be absent.
    fields: Vec<(String, bool)>,
    span: Range<usize>,
}

impl Declarations {
    /// Check a declaration and record it.
    pub(crate) fn declare(&mut self, declaration: &TypeDecl) -> Result<(), Diagnostic> {
        self.name_is_free(declaration)?;
        self.parameters_are_distinct(declaration)?;
        let scope: Vec<&str> = declaration.parameters.iter().map(String::as_str).collect();

        let mut parents = Vec::new();
        for supertype in &declaration.supertypes {
            self.resolve(supertype, &scope)?;
            parents.push(supertype);
        }
        self.agrees_with_the_lattice(declaration, &parents)?;
        Self::parents_are_jointly_inhabitable(&parents)?;

        let mut seen: Vec<&str> = Vec::new();
        for field in &declaration.fields {
            if seen.contains(&field.name.as_str()) {
                return Err(Diagnostic::typing(
                    field.span.clone(),
                    format!(
                        "`{}` declares `{}` twice, and a record holds one entry per key",
                        declaration.name, field.name
                    ),
                ));
            }
            seen.push(&field.name);
            self.resolve(&field.annotation, &scope)?;
            if field.optional {
                self.field_stays_required(declaration, &field.name, &field.span)?;
            }
        }

        self.known.insert(
            declaration.name.clone(),
            Declared {
                parameters: declaration.parameters.len(),
                parents: declaration
                    .supertypes
                    .iter()
                    .map(|parent| parent.name.clone())
                    .collect(),
                fields: declaration
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.optional))
                    .collect(),
                span: declaration.span.clone(),
            },
        );
        Ok(())
    }

    /// A subtype narrows its parent, and a key the parent requires stays
    /// required. Making it optional would widen the shape, so a value of the
    /// subtype could fail to be a value of the parent.
    fn field_stays_required(
        &self,
        declaration: &TypeDecl,
        field: &str,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        let mut pending: Vec<&str> = declaration
            .supertypes
            .iter()
            .map(|parent| parent.name.as_str())
            .collect();
        let mut seen: Vec<&str> = Vec::new();
        while let Some(name) = pending.pop() {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
            let Some(parent) = self.known.get(name) else {
                continue;
            };
            if parent
                .fields
                .iter()
                .any(|(held, optional)| held == field && !optional)
            {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!(
                        "`{name}` requires `{field}`, so `{}` may not make it optional",
                        declaration.name
                    ),
                ));
            }
            pending.extend(parent.parents.iter().map(String::as_str));
        }
        Ok(())
    }

    /// Whether a name is one this program declared.
    pub(crate) fn holds(&self, name: &str) -> bool {
        self.known.contains_key(name)
    }

    fn name_is_free(&self, declaration: &TypeDecl) -> Result<(), Diagnostic> {
        if let Some(previous) = self.known.get(&declaration.name) {
            return Err(Diagnostic::name(
                declaration.span.clone(),
                format!(
                    "`{}` is declared twice, and the earlier one is still in force \
                     (it begins at byte {})",
                    declaration.name, previous.span.start
                ),
            ));
        }
        Ok(())
    }

    fn parameters_are_distinct(&self, declaration: &TypeDecl) -> Result<(), Diagnostic> {
        let mut seen: Vec<&str> = Vec::new();
        for parameter in &declaration.parameters {
            if seen.contains(&parameter.as_str()) {
                return Err(Diagnostic::name(
                    declaration.span.clone(),
                    format!("`{}` takes `{parameter}` twice", declaration.name),
                ));
            }
            // A parameter standing for any type must not be spelled like one
            // that already exists, or a reader cannot tell which is meant.
            if types::named(parameter, &[]).is_some() || self.holds(parameter) {
                return Err(Diagnostic::name(
                    declaration.span.clone(),
                    format!(
                        "`{parameter}` is a type, so it cannot also be a parameter of `{}`",
                        declaration.name
                    ),
                ));
            }
            seen.push(parameter);
        }
        Ok(())
    }

    /// A written type must name something: a built-in, a declaration, or a
    /// parameter of the declaration being checked.
    fn resolve(&self, written: &TypeAnn, scope: &[&str]) -> Result<(), Diagnostic> {
        for argument in &written.arguments {
            self.resolve(argument, scope)?;
        }
        if scope.contains(&written.name.as_str()) {
            if written.arguments.is_empty() {
                return Ok(());
            }
            return Err(Diagnostic::typing(
                written.span.clone(),
                format!(
                    "`{}` is a type parameter and takes no arguments of its own",
                    written.name
                ),
            ));
        }
        if let Some(declared) = self.known.get(&written.name) {
            // A bare name is the constructor itself — `Graph<Card, Link>`
            // passes `Link`, not a `Link` of something. Whether a constructor
            // may stand where a type is expected is not settled, so arity is
            // checked when arguments are written and not otherwise.
            if written.arguments.is_empty() || declared.parameters == written.arguments.len() {
                return Ok(());
            }
            return Err(Diagnostic::typing(
                written.span.clone(),
                format!(
                    "`{}` takes {} type argument{}, and {} {} written",
                    written.name,
                    declared.parameters,
                    if declared.parameters == 1 { "" } else { "s" },
                    written.arguments.len(),
                    if written.arguments.len() == 1 {
                        "is"
                    } else {
                        "are"
                    },
                ),
            ));
        }
        if types::named(&written.name, &[]).is_some() {
            return Ok(());
        }
        Err(Diagnostic::name(
            written.span.clone(),
            format!("unknown type `{}`", written.name),
        ))
    }

    /// A declaration may restate a type the binary already has — that is what
    /// `std/` is for — but it may not contradict it.
    fn agrees_with_the_lattice(
        &self,
        declaration: &TypeDecl,
        parents: &[&TypeAnn],
    ) -> Result<(), Diagnostic> {
        let Some(built_in) = types::named(&declaration.name, &[]) else {
            return Ok(());
        };
        for parent in parents {
            // A parent this binary does not know is a declaration of its own,
            // and there is nothing in the lattice to disagree with.
            let Some(expected) = lattice(parent) else {
                continue;
            };
            if !built_in.is(&expected) {
                return Err(Diagnostic::typing(
                    parent.span.clone(),
                    format!(
                        "`{}` is built in and does not narrow {expected}, \
                         so this declaration contradicts it",
                        declaration.name
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Two parents a value could never satisfy at once make the declaration
    /// uninhabitable, which is a mistake rather than an empty type.
    fn parents_are_jointly_inhabitable(parents: &[&TypeAnn]) -> Result<(), Diagnostic> {
        for (position, left) in parents.iter().enumerate() {
            let Some(one) = lattice(left) else {
                continue;
            };
            for right in &parents[position + 1..] {
                let Some(other) = lattice(right) else {
                    continue;
                };
                // Related either way is fine: `{Card, Doc}` is redundant, not
                // contradictory. Sharing nothing is the failure.
                if one.is(&other) || other.is(&one) {
                    continue;
                }
                if one.join(&other) == Type::Data && one != Type::Data && other != Type::Data {
                    return Err(Diagnostic::typing(
                        right.span.clone(),
                        format!(
                            "nothing is both {one} and {other}, so this supertype set \
                             declares a type no value can have"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}
