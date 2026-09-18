//! Declared types: what a `type` statement says, and whether it holds.
//!
//! A declaration is checked and yields `Unit`. It introduces no value and no
//! case in the [lattice](crate::types) — constructing a value of a declared
//! record type is separate work. What is checked here is that the declaration
//! is coherent: its parents exist, its parents can be narrowed together, a
//! union's members exist and are distinct, and where it restates a type this
//! binary already has, it agrees with it.

use crate::ast::{TypeAnn, TypeDecl};
use crate::diagnostics::Diagnostic;
use crate::types::{self, TypeKind, TypeRef, builtin};
use std::collections::{BTreeSet, HashMap};
use std::ops::Range;

/// The lattice type a written annotation denotes, when the binary has one.
///
/// A type parameter has no lattice counterpart, so it stands as `Data`: the
/// question a declaration answers is about the shape, not about what a
/// parameter will later be bound to.
fn lattice(written: &TypeAnn) -> Option<TypeRef> {
    let arguments: Vec<TypeRef> = written
        .arguments
        .iter()
        .map(|argument| lattice(argument).unwrap_or(TypeRef::DATA))
        .collect();
    types::named(&written.name, &arguments)
}

/// Resolve a runtime annotation recursively and validate every supplied argument.
pub(crate) fn annotation(written: &TypeAnn) -> Result<TypeRef, Diagnostic> {
    let args = written
        .arguments
        .iter()
        .map(annotation)
        .collect::<Result<Vec<_>, _>>()?;
    let constructor = builtin::constructor(&written.name).ok_or_else(|| {
        Diagnostic::name(
            written.span.clone(),
            format!("unknown type `{}`", written.name),
        )
    })?;
    let system = builtin::system()
        .map_err(|error| Diagnostic::typing(written.span.clone(), error.to_string()))?;
    let reference = system
        .apply(constructor, args)
        .map_err(|error| Diagnostic::typing(written.span.clone(), error.to_string()))?;
    Ok(reference)
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
        let scope: HashMap<String, TypeRef> = declaration
            .parameters
            .iter()
            .map(|parameter| {
                let bound = declaration
                    .bounds
                    .iter()
                    .find(|(name, _)| name == parameter)
                    .and_then(|(_, bound)| lattice(bound))
                    .unwrap_or(TypeRef::DATA);
                (parameter.clone(), bound)
            })
            .collect();
        let mut bounded = Vec::new();
        for (parameter, bound) in &declaration.bounds {
            if !scope.contains_key(parameter) || bounded.contains(&parameter) {
                return Err(Diagnostic::typing(
                    bound.span.clone(),
                    format!(
                        "`{parameter}` must name one distinct parameter of `{}`",
                        declaration.name
                    ),
                ));
            }
            self.resolve(bound, &scope)?;
            bounded.push(parameter);
        }
        if let Ok(system) = builtin::system() {
            if let Some(constructor) = system.constructor(&declaration.name) {
                let definition = system.definition(constructor).map_err(|error| {
                    Diagnostic::typing(declaration.span.clone(), error.to_string())
                })?;
                if matches!(
                    declaration.name.as_str(),
                    "Collection"
                        | "Seq"
                        | "List"
                        | "Set"
                        | "Map"
                        | "OrderedMap"
                        | "SortedMap"
                        | "Orderable"
                        | "Scalar"
                ) {
                    if declaration.abstract_type != (definition.kind == TypeKind::Abstract)
                        || declaration.parameters.len() != definition.parameters.len()
                    {
                        return Err(Diagnostic::typing(
                            declaration.span.clone(),
                            format!(
                                "`{}` contradicts its built-in kind or arity",
                                declaration.name
                            ),
                        ));
                    }
                    for (name, parameter) in
                        declaration.parameters.iter().zip(&definition.parameters)
                    {
                        let bound = declaration
                            .bounds
                            .iter()
                            .find(|(held, _)| held == name)
                            .and_then(|(_, bound)| lattice(bound));
                        if bound != parameter.bound {
                            return Err(Diagnostic::typing(
                                declaration.span.clone(),
                                format!(
                                    "`{name}` must preserve the bound declared by `{}`",
                                    declaration.name
                                ),
                            ));
                        }
                    }
                }
            }
        }

        if let Some(members) = &declaration.union {
            return self.declare_union(declaration, members, &scope);
        }

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

    /// Check `type Name = union {A, B}` and record it.
    fn declare_union(
        &mut self,
        declaration: &TypeDecl,
        members: &[TypeAnn],
        scope: &HashMap<String, TypeRef>,
    ) -> Result<(), Diagnostic> {
        // A member may apply the union being declared — `Seq<Data>` inside
        // `Data` is what makes a tree — so the name has to resolve while the
        // members are read, and is withdrawn again if they do not hold.
        self.known.insert(
            declaration.name.clone(),
            Declared {
                parameters: declaration.parameters.len(),
                parents: Vec::new(),
                fields: Vec::new(),
                span: declaration.span.clone(),
            },
        );
        let checked = self.union_holds(declaration, members, scope);
        if checked.is_err() {
            self.known.remove(&declaration.name);
        }
        checked
    }

    fn union_holds(
        &self,
        declaration: &TypeDecl,
        members: &[TypeAnn],
        scope: &HashMap<String, TypeRef>,
    ) -> Result<(), Diagnostic> {
        let mut seen: Vec<String> = Vec::new();
        for member in members {
            // Recursion through an argument terminates; a bare self-member
            // admits nothing the other members do not.
            if member.name == declaration.name {
                return Err(Diagnostic::typing(
                    member.span.clone(),
                    format!(
                        "`{}` lists itself as a member, which admits nothing new",
                        declaration.name
                    ),
                ));
            }
            self.resolve(member, scope)?;
            let written = spelling(member);
            if seen.contains(&written) {
                return Err(Diagnostic::typing(
                    member.span.clone(),
                    format!("`{}` lists {written} twice", declaration.name),
                ));
            }
            seen.push(written);
        }
        Self::union_agrees_with_the_lattice(declaration, members)
    }

    /// A union the binary already holds is restated member for member: one
    /// alternative more or fewer is a different type.
    fn union_agrees_with_the_lattice(
        declaration: &TypeDecl,
        members: &[TypeAnn],
    ) -> Result<(), Diagnostic> {
        let Some(constructor) = builtin::constructor(&declaration.name) else {
            return Ok(());
        };
        let contradiction = |held: String| {
            Diagnostic::typing(
                declaration.span.clone(),
                format!(
                    "`{}` is built in as {held}, so this declaration contradicts it",
                    declaration.name
                ),
            )
        };
        let system = builtin::system()
            .map_err(|error| Diagnostic::typing(declaration.span.clone(), error.to_string()))?;
        let definition = system
            .definition(constructor)
            .map_err(|error| Diagnostic::typing(declaration.span.clone(), error.to_string()))?;
        let TypeKind::Union(held) = &definition.kind else {
            return Err(contradiction("a type that is not a union".to_owned()));
        };
        let written: Option<BTreeSet<TypeRef>> = members.iter().map(lattice).collect();
        if written != Some(held.iter().cloned().collect()) {
            let held: Vec<String> = held.iter().map(ToString::to_string).collect();
            return Err(contradiction(format!("union {{{}}}", held.join(", "))));
        }
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
            if builtin::constructor(parameter).is_some() || self.holds(parameter) {
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
    fn resolve(
        &self,
        written: &TypeAnn,
        scope: &HashMap<String, TypeRef>,
    ) -> Result<(), Diagnostic> {
        for argument in &written.arguments {
            self.resolve(argument, scope)?;
        }
        if scope.contains_key(&written.name) {
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
        if let Ok(system) = builtin::system() {
            if let Some(constructor) = system.constructor(&written.name) {
                if !written.arguments.is_empty() && !matches!(constructor.0, "Graph" | "Edge") {
                    let args = written
                        .arguments
                        .iter()
                        .map(|argument| self.scoped_type(argument, scope))
                        .collect::<Option<Vec<_>>>();
                    if let Some(args) = args {
                        system.apply(constructor, args).map_err(|error| {
                            Diagnostic::typing(written.span.clone(), error.to_string())
                        })?;
                    }
                }
            }
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
        if builtin::constructor(&written.name).is_some() {
            return Ok(());
        }
        Err(Diagnostic::name(
            written.span.clone(),
            format!("unknown type `{}`", written.name),
        ))
    }

    fn scoped_type(&self, written: &TypeAnn, scope: &HashMap<String, TypeRef>) -> Option<TypeRef> {
        if let Some(bound) = scope.get(&written.name) {
            return Some(bound.clone());
        }
        let args = written
            .arguments
            .iter()
            .map(|argument| self.scoped_type(argument, scope))
            .collect::<Option<Vec<_>>>()?;
        if let Some(reference) = types::named(&written.name, &args) {
            return Some(reference);
        }
        let declaration = self.known.get(&written.name)?;
        Some(
            declaration
                .parents
                .iter()
                .find_map(|parent| types::named(parent, &[]))
                .unwrap_or(TypeRef::DATA),
        )
    }

    /// A declaration may restate a type the binary already has — that is what
    /// `std/` is for — but it may not contradict it.
    fn agrees_with_the_lattice(
        &self,
        declaration: &TypeDecl,
        parents: &[&TypeAnn],
    ) -> Result<(), Diagnostic> {
        let parameters = declaration
            .parameters
            .iter()
            .map(|parameter| {
                declaration
                    .bounds
                    .iter()
                    .find(|(name, _)| name == parameter)
                    .and_then(|(_, bound)| lattice(bound))
                    .unwrap_or(TypeRef::DATA)
            })
            .collect::<Vec<_>>();
        let Some(built_in) = types::named(&declaration.name, &parameters) else {
            return Ok(());
        };
        for parent in parents {
            // A parent this binary does not know is a declaration of its own,
            // and there is nothing in the lattice to disagree with.
            fn instantiate(
                written: &TypeAnn,
                names: &[String],
                parameters: &[TypeRef],
            ) -> Option<TypeRef> {
                if let Some(index) = names.iter().position(|name| name == &written.name) {
                    return parameters.get(index).cloned();
                }
                let args = written
                    .arguments
                    .iter()
                    .map(|argument| instantiate(argument, names, parameters))
                    .collect::<Option<Vec<_>>>()?;
                types::named(&written.name, &args)
            }
            let Some(expected) = instantiate(parent, &declaration.parameters, &parameters) else {
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
    ///
    /// Only two concrete types can exclude each other. A contract such as
    /// `Scalar` or `Orderable`, or a union such as `Data`, is satisfied *by*
    /// other types, so `{Scalar, Orderable}` is two claims one value can meet.
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
                if one.is_concrete()
                    && other.is_concrete()
                    && one.declared_common_type(&other).is_none()
                {
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

/// A written type as its author spelled it, for naming it in a diagnostic.
fn spelling(written: &TypeAnn) -> String {
    if written.arguments.is_empty() {
        return written.name.clone();
    }
    let arguments: Vec<String> = written.arguments.iter().map(spelling).collect();
    format!("{}<{}>", written.name, arguments.join(", "))
}
