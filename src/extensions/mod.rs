//! The registry: what a pipeline step is, and which extension provides it.
//!
//! A step is a name a pipeline may write, together with the function that
//! checks it and the function that evaluates it. Keeping both beside the name
//! is what lets a capability be added without editing the core's checker or
//! evaluator, and what lets `hql builtins` name a provider rather than print a
//! flat list that stops being true once two extensions can supply one name.
//!
//! The `Extension` type is a host mechanism. It is how this binary is
//! organised; HQL programs see typed functions and never see a trait.

use crate::ast::{Arg, Expr};
use crate::diagnostics::Diagnostic;
use crate::types::Type;
use crate::values::Value;
use crate::vault::Vault;
use std::ops::Range;

pub(crate) mod collections;
pub(crate) mod graph;
pub(crate) mod present;
pub(crate) mod retrieval;

/// Whether the executor may move a step around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purity {
    /// A function of its input and arguments alone: free to reorder, fuse,
    /// parallelise, cache or skip.
    Pure,
    /// Must run once, where it is written.
    Effectful,
}

impl Purity {
    /// The word help prints for this purity.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Effectful => "effectful",
        }
    }
}

/// What a step does to a type.
pub(crate) type Check =
    fn(&mut dyn CheckCx, &Type, &[Arg], Range<usize>) -> Result<Type, Diagnostic>;

/// What a step does to a value.
pub(crate) type Eval =
    fn(&mut dyn EvalCx, Value, &[Arg], Range<usize>) -> Result<Value, Diagnostic>;

/// What checking a step may ask of the checker around it.
///
/// An argument arrives as unevaluated syntax, because a step such as
/// `filter(c => …)` applies its argument once per element rather than once.
pub(crate) trait CheckCx {
    /// The vault the program is checked against.
    fn vault(&self) -> &Vault;
    /// The type of an argument expression.
    fn infer(&mut self, expression: &Expr) -> Result<Type, Diagnostic>;
    /// The result type of a lambda argument, with its parameter bound.
    fn lambda(&mut self, argument: &Arg, parameter: Type) -> Result<Type, Diagnostic>;
}

/// What evaluating a step may ask of the evaluator around it.
pub(crate) trait EvalCx {
    /// The vault the program runs against.
    fn vault(&self) -> &Vault;
    /// The value of an argument expression.
    fn evaluate(&mut self, expression: &Expr) -> Result<Value, Diagnostic>;
    /// A lambda argument applied to one element.
    fn apply_lambda(&mut self, argument: &Arg, element: Value) -> Result<Value, Diagnostic>;
}

/// One name a pipeline may write.
pub struct Step {
    /// How it is written.
    pub(crate) name: &'static str,
    /// How it is called, for help output.
    pub(crate) signature: &'static str,
    /// One sentence about what it does.
    pub(crate) summary: &'static str,
    /// Whether the executor may move it.
    pub(crate) purity: Purity,
    /// What it does to a type.
    pub(crate) check: Check,
    /// What it does to a value.
    pub(crate) eval: Eval,
}

/// A named unit supplying pipeline steps.
pub struct Extension {
    /// The lowercase identifier an `import` names.
    pub(crate) name: &'static str,
    /// The version this extension reports.
    pub(crate) version: &'static str,
    /// What it supplies.
    pub(crate) steps: &'static [Step],
}

/// The core: collection operations, which mention nothing above the core's own
/// types and so are never imported because they are never absent.
pub(crate) static CORE: Extension = Extension {
    name: "core",
    version: env!("CARGO_PKG_VERSION"),
    steps: collections::STEPS,
};

static GRAPH: Extension = Extension {
    name: "graph",
    version: env!("CARGO_PKG_VERSION"),
    steps: graph::STEPS,
};

static PRESENT: Extension = Extension {
    name: "present",
    version: env!("CARGO_PKG_VERSION"),
    steps: present::STEPS,
};

static SEMANTIC: Extension = Extension {
    name: "semantic",
    version: env!("CARGO_PKG_VERSION"),
    steps: retrieval::STEPS,
};

/// Every extension compiled into this binary, core first.
///
/// Registration is not import: a step here is known to diagnostics and to
/// `hql builtins` whether or not the program has imported the extension that
/// provides it, so a missing import reports the import rather than a
/// misspelling.
pub(crate) static REGISTERED: &[&Extension] = &[&CORE, &GRAPH, &PRESENT, &SEMANTIC];

impl Extension {
    /// The step of a name this extension provides.
    pub(crate) fn step(&'static self, name: &str) -> Option<&'static Step> {
        self.steps.iter().find(|step| step.name == name)
    }
}

/// The step a bare name resolves to, searching every registered extension.
pub(crate) fn step(name: &str) -> Option<&'static Step> {
    REGISTERED.iter().find_map(|extension| extension.step(name))
}

/// Every registered step name, for the suggestion index.
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    REGISTERED
        .iter()
        .flat_map(|extension| extension.steps.iter().map(|step| step.name))
}

/// One step, as `hql builtins` prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listing {
    /// Which extension supplies it.
    pub provider: &'static str,
    /// That extension's version.
    pub version: &'static str,
    /// How it is written.
    pub name: &'static str,
    /// How it is called.
    pub signature: &'static str,
    /// One sentence about what it does.
    pub summary: &'static str,
    /// Whether the executor may move it.
    pub purity: Purity,
}

/// Every registered step, grouped by provider in registration order.
#[must_use]
pub fn listing() -> Vec<Listing> {
    REGISTERED
        .iter()
        .flat_map(|extension| {
            extension.steps.iter().map(|step| Listing {
                provider: extension.name,
                version: extension.version,
                name: step.name,
                signature: step.signature,
                summary: step.summary,
                purity: step.purity,
            })
        })
        .collect()
}

/// The element type of a collection, or the failure that says it is not one.
pub(crate) fn collection(
    input: &Type,
    name: &str,
    span: &Range<usize>,
) -> Result<Type, Diagnostic> {
    input.element().ok_or_else(|| {
        Diagnostic::typing(
            span.clone(),
            format!("`{name}` needs a collection, not {input}"),
        )
    })
}

/// The argument written with a name.
pub(crate) fn named<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some(name))
}

/// The argument written with a name, or the first written without one.
pub(crate) fn named_or_first<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    named(arguments, name).or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_step_declares_a_purity_and_a_provider() {
        let listed = listing();
        assert!(!listed.is_empty());
        for entry in &listed {
            assert_eq!(entry.purity, Purity::Pure, "{}", entry.name);
            assert!(!entry.provider.is_empty());
            assert!(!entry.signature.is_empty());
            assert!(!entry.summary.is_empty());
        }
    }

    #[test]
    fn a_step_name_resolves_to_the_extension_that_provides_it() {
        assert!(step("count").is_some());
        assert!(step("nonesuch").is_none());
        let listed = listing();
        let count = listed.iter().find(|entry| entry.name == "count").unwrap();
        assert_eq!(count.provider, "core");
        let table = listed.iter().find(|entry| entry.name == "table").unwrap();
        assert_eq!(table.provider, "present");
    }
}
