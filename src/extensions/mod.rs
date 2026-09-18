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
use crate::types::TypeRef;
use crate::types::Value;
use crate::vault::Vault;
use std::ops::Range;
use std::path::Path;

pub(crate) mod collections;
pub(crate) mod graph;
pub mod lexical;
pub(crate) mod present;
pub mod semantic;

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
    fn(&mut dyn CheckCx, &TypeRef, &[Arg], Range<usize>) -> Result<TypeRef, Diagnostic>;

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
    fn infer(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic>;
    /// The result type of a lambda argument, with its parameter bound.
    fn lambda(&mut self, argument: &Arg, parameter: TypeRef) -> Result<TypeRef, Diagnostic>;
    /// Check a two-argument comparison with both parameters bound to the element type.
    fn comparator(&mut self, argument: &Arg, parameter: TypeRef) -> Result<TypeRef, Diagnostic>;
}

/// What evaluating a step may ask of the evaluator around it.
pub(crate) trait EvalCx {
    /// The vault the program runs against.
    fn vault(&self) -> &Vault;
    /// The value of an argument expression.
    fn evaluate(&mut self, expression: &Expr) -> Result<Value, Diagnostic>;
    /// A lambda argument applied to one element.
    fn apply_lambda(&mut self, argument: &Arg, element: Value) -> Result<Value, Diagnostic>;
    /// Apply a comparison to a pair of elements.
    fn apply_comparator(
        &mut self,
        argument: &Arg,
        left: Value,
        right: Value,
    ) -> Result<Value, Diagnostic>;
    /// Queue something worth saying that does not make the result impossible.
    fn warn(&mut self, warning: crate::warnings::Warning);
}

/// One name a pipeline may write.
#[derive(Debug)]
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
#[derive(Debug)]
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

static LEXICAL: Extension = Extension {
    name: "lexical",
    version: env!("CARGO_PKG_VERSION"),
    steps: lexical::STEPS,
};

static SEMANTIC: Extension = Extension {
    name: "semantic",
    version: env!("CARGO_PKG_VERSION"),
    steps: semantic::STEPS,
};

/// Every extension compiled into this binary, core first.
///
/// Registration is not import: a step here is known to diagnostics and to
/// `hql builtins` whether or not the program has imported the extension that
/// provides it, so a missing import reports the import rather than a
/// misspelling.
pub(crate) static REGISTERED: &[&Extension] = &[&CORE, &GRAPH, &PRESENT, &LEXICAL, &SEMANTIC];

impl Extension {
    /// The step of a name this extension provides.
    pub(crate) fn step(&'static self, name: &str) -> Option<&'static Step> {
        self.steps.iter().find(|step| step.name == name)
    }
}

/// The extensions imported unless a vault opts out.
///
/// A prelude exists so that a program that only walks a graph and prints a
/// table does not open with two lines of ceremony. It is a convenience, not a
/// rule, which is why a vault can decline it.
pub(crate) static PRELUDE: &[&str] = &["graph", "present"];

/// The registered extension of a name.
pub(crate) fn extension(name: &str) -> Option<&'static Extension> {
    REGISTERED
        .iter()
        .copied()
        .find(|extension| extension.name == name)
}

/// Every registered extension providing a step name, in registration order.
pub(crate) fn providers(step: &str) -> Vec<&'static Extension> {
    REGISTERED
        .iter()
        .copied()
        .filter(|extension| extension.step(step).is_some())
        .collect()
}

/// What a vault's `hql.toml` says about extensions.
///
/// The prelude is a compatibility convenience, and a vault that wants every
/// capability stated in the program that uses it must be able to decline it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Extensions imported for every program this vault runs.
    pub imports: Vec<String>,
    /// Whether `graph` and `present` arrive without being asked for.
    pub prelude: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            prelude: true,
        }
    }
}

impl Config {
    /// Read `[extensions]` from a vault's configuration file.
    ///
    /// A file that is absent or says nothing about extensions leaves the
    /// defaults in force, which is what a vault that has never heard of an
    /// extension expects.
    #[must_use]
    pub fn read(root: &Path) -> Self {
        let mut config = Self::default();
        let Ok(text) = std::fs::read_to_string(root.join(crate::reporting::CONFIG_FILE)) else {
            return config;
        };
        let Ok(parsed) = toml::from_str::<toml::Value>(&text) else {
            return config;
        };
        let Some(section) = parsed.get("extensions") else {
            return config;
        };
        if let Some(prelude) = section.get("prelude").and_then(toml::Value::as_bool) {
            config.prelude = prelude;
        }
        if let Some(imports) = section.get("import").and_then(toml::Value::as_array) {
            config.imports = imports
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect();
        }
        config
    }
}

/// Whether an extension is available to a program, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The core, which is never imported because it is never absent.
    Core,
    /// In the prelude this vault keeps.
    Prelude,
    /// Named by the vault's `hql.toml`.
    Configured,
    /// Registered, and written out by any program that wants it.
    Absent,
}

impl Availability {
    /// How help says it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Core => "always available",
            Self::Prelude => "prelude",
            Self::Configured => "imported by this vault",
            Self::Absent => "imported where it is used",
        }
    }
}

/// Which extensions a program has imported, and what they let it write.
///
/// Registration and import are different things: every extension in the binary
/// is registered, and a program resolves only what it has imported. That is
/// what lets a diagnostic name the import a program is missing instead of
/// guessing at a misspelling.
#[derive(Debug, Clone)]
pub(crate) struct Resolution {
    imported: Vec<&'static Extension>,
}

impl Resolution {
    /// The core, plus the prelude unless the vault declines it.
    pub(crate) fn bare(prelude: bool) -> Self {
        let mut imported = vec![&CORE];
        if prelude {
            imported.extend(PRELUDE.iter().filter_map(|name| extension(name)));
        }
        Self { imported }
    }

    /// What a vault starts every program with.
    ///
    /// A configured import fails the same way a written one does, at a span
    /// pointing at the program's start, because the fault is in the vault
    /// rather than in the line a reader is looking at.
    pub(crate) fn new(config: &Config) -> Result<Self, Diagnostic> {
        let mut resolution = Self::bare(config.prelude);
        for name in &config.imports {
            resolution.import(name, &(0..0)).map_err(|failed| {
                Diagnostic::name(
                    0..0,
                    format!(
                        "the vault's {}: {}",
                        crate::reporting::CONFIG_FILE,
                        failed.message()
                    ),
                )
            })?;
        }
        Ok(resolution)
    }

    /// Bind an extension's steps into the program.
    ///
    /// A collision is reported here rather than at the use site, where the
    /// failure would depend on which pipeline a reader happened to look at.
    pub(crate) fn import(&mut self, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
        let wanted = extension(name).ok_or_else(|| {
            Diagnostic::name(
                span.clone(),
                format!("no extension named `{name}` is registered"),
            )
        })?;
        self.bind(wanted, span)
    }

    /// Bind an extension whose identity is already known.
    pub(crate) fn bind(
        &mut self,
        wanted: &'static Extension,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        if self.imported.iter().any(|held| held.name == wanted.name) {
            return Ok(());
        }
        for step in wanted.steps {
            if let Some(held) = self
                .imported
                .iter()
                .find(|held| held.step(step.name).is_some())
            {
                return Err(Diagnostic::name(
                    span.clone(),
                    format!(
                        "`{}` and `{}` both provide `{}`, so importing both leaves it ambiguous",
                        held.name, wanted.name, step.name
                    ),
                ));
            }
        }
        self.imported.push(wanted);
        Ok(())
    }

    /// The step a bare name resolves to.
    pub(crate) fn step(
        &self,
        name: &str,
        span: &Range<usize>,
    ) -> Result<&'static Step, Diagnostic> {
        if let Some(step) = self.imported.iter().find_map(|held| held.step(name)) {
            return Ok(step);
        }
        Err(self.unresolved(name, span))
    }

    /// The step `<extension>.<step>` names, which is always available for an
    /// imported extension.
    pub(crate) fn qualified(
        &self,
        extension: &str,
        name: &str,
        span: &Range<usize>,
    ) -> Result<&'static Step, Diagnostic> {
        let held = self
            .imported
            .iter()
            .find(|held| held.name == extension)
            .ok_or_else(|| self.unresolved(extension, span))?;
        held.step(name).ok_or_else(|| {
            Diagnostic::name(
                span.clone(),
                format!("the `{extension}` extension provides no step `{name}`"),
            )
        })
    }

    /// Why a name did not resolve: a missing import, or no such step at all.
    fn unresolved(&self, name: &str, span: &Range<usize>) -> Diagnostic {
        let providers = providers(name);
        let message = match providers.as_slice() {
            [] => {
                let hint = crate::builtins::nearest(name)
                    .map(|near| format!("; did you mean `{near}`?"))
                    .unwrap_or_default();
                format!("`{name}` is not a pipeline step{hint}")
            }
            [only] => format!(
                "`{name}` is provided by the `{}` extension: write `import {}`",
                only.name, only.name
            ),
            many => format!(
                "`{name}` is provided by the {} extensions: import one of them",
                many.iter()
                    .map(|extension| format!("`{}`", extension.name))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
        };
        Diagnostic::name(span.clone(), message)
    }
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

/// One extension and its steps, as `hql builtins` prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// The name an `import` writes.
    pub name: &'static str,
    /// The version this extension reports.
    pub version: &'static str,
    /// Whether a program in this vault has it without asking.
    pub availability: Availability,
    /// What it supplies.
    pub steps: Vec<Listing>,
}

/// Every registered extension and its steps, as one vault resolves them.
///
/// Every registered extension is listed, not only what the vault imports:
/// help and the suggestion index answer the same question, so listing only
/// the imported ones would leave a reader unable to find the import a
/// diagnostic has just told them to write.
#[must_use]
pub fn catalogue(config: &Config) -> Vec<Provider> {
    REGISTERED
        .iter()
        .map(|extension| Provider {
            name: extension.name,
            version: extension.version,
            availability: if extension.name == CORE.name {
                Availability::Core
            } else if config.prelude && PRELUDE.contains(&extension.name) {
                Availability::Prelude
            } else if config.imports.iter().any(|held| held == extension.name) {
                Availability::Configured
            } else {
                Availability::Absent
            },
            steps: extension
                .steps
                .iter()
                .map(|step| Listing {
                    provider: extension.name,
                    version: extension.version,
                    name: step.name,
                    signature: step.signature,
                    summary: step.summary,
                    purity: step.purity,
                })
                .collect(),
        })
        .collect()
}

/// The element type of a collection, or the failure that says it is not one.
pub(crate) fn collection(
    input: &TypeRef,
    name: &str,
    span: &Range<usize>,
) -> Result<TypeRef, Diagnostic> {
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

    fn listing() -> Vec<Listing> {
        catalogue(&Config::default())
            .into_iter()
            .flat_map(|provider| provider.steps)
            .collect()
    }

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
        let listed = listing();
        let count = listed.iter().find(|entry| entry.name == "count").unwrap();
        assert_eq!(count.provider, "core");
        let table = listed.iter().find(|entry| entry.name == "table").unwrap();
        assert_eq!(table.provider, "present");
        assert_eq!(providers("count").len(), 1);
        assert!(providers("nonesuch").is_empty());
        assert!(extension("graph").is_some());
        assert!(extension("nonesuch").is_none());
    }

    #[test]
    fn the_core_resolves_without_an_import_and_retrieval_does_not() {
        let mut resolution = Resolution::bare(true);
        assert!(resolution.step("count", &(0..1)).is_ok());
        assert!(resolution.step("table", &(0..1)).is_ok());

        let missing = resolution.step("lexical", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("import lexical"),
            "{}",
            missing.message()
        );

        resolution.import("lexical", &(0..1)).unwrap();
        assert!(resolution.step("lexical", &(0..1)).is_ok());
        assert!(resolution.qualified("lexical", "lexical", &(0..1)).is_ok());
        assert!(resolution.qualified("lexical", "rank", &(0..1)).is_err());
    }

    #[test]
    fn the_prelude_is_a_convenience_a_vault_may_decline() {
        let bare = Resolution::bare(false);
        let missing = bare.step("table", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("import present"),
            "{}",
            missing.message()
        );
        assert!(bare.step("count", &(0..1)).is_ok());
    }

    /// A second provider of a name the core already has, which no registered
    /// extension is allowed to be — the collision is the point of the test.
    static PROBE: Extension = Extension {
        name: "probe",
        version: "0",
        steps: &[Step {
            name: "count",
            signature: "collection | count",
            summary: "A second `count`, so the collision has two sides.",
            purity: Purity::Pure,
            check: |_, _, _, _| Ok(TypeRef::INT),
            eval: |_, _, _, _| Ok(Value::Int(0)),
        }],
    };

    #[test]
    fn two_extensions_providing_one_name_collide_at_the_import() {
        let mut resolution = Resolution::bare(true);
        let collision = resolution.bind(&PROBE, &(0..1)).unwrap_err();
        assert!(
            collision
                .message()
                .contains("`core` and `probe` both provide `count`"),
            "{}",
            collision.message()
        );
        // The failed import changed nothing: `count` is still the core's.
        assert!(resolution.step("count", &(0..1)).is_ok());
    }

    #[test]
    fn importing_the_same_extension_twice_is_not_a_collision() {
        let mut resolution = Resolution::bare(true);
        resolution.import("lexical", &(0..1)).unwrap();
        resolution.import("lexical", &(0..1)).unwrap();
        assert!(resolution.step("lexical", &(0..1)).is_ok());
    }

    #[test]
    fn a_vault_may_import_for_every_program_and_may_decline_the_prelude() {
        let config = Config {
            imports: vec!["lexical".to_owned()],
            prelude: false,
        };
        let resolution = Resolution::new(&config).expect("the configuration resolves");
        assert!(resolution.step("lexical", &(0..1)).is_ok());
        assert!(resolution.step("table", &(0..1)).is_err());

        let listed = catalogue(&config);
        let by_name = |wanted: &str| {
            listed
                .iter()
                .find(|provider| provider.name == wanted)
                .unwrap()
                .availability
        };
        assert_eq!(by_name("core"), Availability::Core);
        assert_eq!(by_name("present"), Availability::Absent);
        assert_eq!(by_name("lexical"), Availability::Configured);
        assert_eq!(by_name("graph"), Availability::Absent);

        let kept = catalogue(&Config::default());
        let graph = kept.iter().find(|provider| provider.name == "graph");
        assert_eq!(
            graph.map(|provider| provider.availability),
            Some(Availability::Prelude)
        );
    }

    #[test]
    fn a_configured_import_that_is_not_registered_fails_every_program() {
        let config = Config {
            imports: vec!["nonesuch".to_owned()],
            prelude: true,
        };
        let failed = Resolution::new(&config).unwrap_err();
        assert!(
            failed.message().contains("hql.toml"),
            "{}",
            failed.message()
        );
    }

    #[test]
    fn importing_an_extension_that_is_not_registered_says_so() {
        let mut resolution = Resolution::bare(true);
        let failed = resolution.import("nonesuch", &(0..1)).unwrap_err();
        assert!(
            failed.message().contains("no extension named `nonesuch`"),
            "{}",
            failed.message()
        );
    }
}
