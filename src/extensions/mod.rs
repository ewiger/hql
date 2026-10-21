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
use crate::execution::{Arg as TypedArg, Expr as TypedExpr};
use crate::types::TypeRef;
use crate::types::Value;
use crate::vault::Vault;
use std::ops::Range;
use std::path::Path;

pub(crate) mod collections;
pub mod format;
pub(crate) mod graph;
pub(crate) mod io;
pub mod lexical;
pub mod logging;
pub(crate) mod math;
pub mod net;
pub(crate) mod option;
pub(crate) mod regex;
pub(crate) mod result;
pub mod semantic;
pub(crate) mod seq;
pub mod spec;
pub(crate) mod string;
use spec::{CheckedCall, OperationSpec};

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
    fn(&mut dyn CheckCx, &CheckedCall<'_>, Range<usize>) -> Result<(), Diagnostic>;

/// What a step does to a value.
pub(crate) type Eval =
    fn(&mut dyn EvalCx, Value, &[TypedArg], Range<usize>) -> Result<Value, Diagnostic>;

/// What checking a step may ask of the checker around it.
///
/// An argument arrives as unevaluated syntax, because a step such as
/// `filter(c => …)` applies its argument once per element rather than once.
pub(crate) trait CheckCx: Send {
    /// The vault the program is checked against.
    fn vault(&self) -> &Vault;
    /// The type of an argument expression.
    fn infer(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic>;
    /// Check a lambda with the parameter types declared by its step contract.
    fn lambda_parameters(
        &mut self,
        argument: &Arg,
        parameters: Vec<TypeRef>,
    ) -> Result<TypeRef, Diagnostic>;
    /// Resolve a type argument and retain its identity in the execution tree.
    fn type_argument(&mut self, argument: &Arg) -> Result<TypeRef, Diagnostic>;
    /// How many parameters an argument takes, when it is written as a lambda
    /// or names a function value.
    fn function_arity(&mut self, argument: &Arg) -> Option<usize>;
    /// Report something worth saying that does not make the program wrong.
    fn warn(&mut self, warning: crate::warnings::Warning);
    /// Fix an alias parameter's type to what a step argument requires;
    /// answers whether `actual` was such a parameter.
    fn placeholder(&mut self, actual: &TypeRef, expected: Option<TypeRef>) -> bool;
}

/// What evaluating a step may ask of the evaluator around it.
pub(crate) trait EvalCx: Send {
    /// The vault the program runs against.
    fn vault(&self) -> &Vault;
    /// The value of an argument expression.
    fn evaluate(&mut self, expression: &TypedExpr) -> Result<Value, Diagnostic>;
    /// Drive a traversal, handing each value to `sink` until the values run
    /// out or the sink stops.
    fn traverse(
        &mut self,
        traversal: &crate::traversal::Traversal,
        sink: &mut dyn FnMut(&mut dyn EvalCx, Value) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic>;
    /// A lambda argument applied to one element.
    fn apply_lambda(&mut self, argument: &TypedArg, element: Value) -> Result<Value, Diagnostic>;
    /// How two values of one Orderable type compare: by their intrinsic
    /// keys, or by the `compare` an implementation of `Ord` provides.
    fn compare_values(
        &mut self,
        left: &Value,
        right: &Value,
        span: &Range<usize>,
    ) -> Result<std::cmp::Ordering, Diagnostic>;
    /// A function argument applied to any number of values, none included.
    fn apply_function(
        &mut self,
        argument: &TypedArg,
        values: Vec<Value>,
    ) -> Result<Value, Diagnostic>;
    /// Apply a comparison to a pair of elements.
    fn apply_comparator(
        &mut self,
        argument: &TypedArg,
        left: Value,
        right: Value,
    ) -> Result<Value, Diagnostic>;
    /// Queue something worth saying that does not make the result impossible.
    fn warn(&mut self, warning: crate::warnings::Warning);
}

/// One name a pipeline may write.
#[derive(Debug)]
pub struct Operation {
    /// How it is written.
    pub(crate) name: &'static str,
    /// Structural signature used for checking and help.
    pub(crate) spec: OperationSpec,
    /// One sentence about what it does.
    pub(crate) summary: &'static str,
    /// Whether the executor may move it.
    pub(crate) purity: Purity,
    /// What it does to a type.
    pub(crate) check: Option<Check>,
    /// What it does to a value.
    pub(crate) eval: Eval,
}

impl Operation {
    /// Whether the step returns its input as it is, at whatever type it has —
    /// `debug`. Such a step sees an optional value unlifted, absence included,
    /// because it neither consumes the value nor promises anything about it.
    pub(crate) fn keeps_input(&self) -> bool {
        self.spec.output == spec::Output::Input
            && matches!(self.spec.input, spec::TypePattern::Var(_))
    }

    /// Whether the step receives an optional input as it is, absence
    /// included: a step that keeps its input, one declared over `Option<T>`,
    /// and `str`, whose text is of the whole value.
    pub(crate) fn takes_raw_input(&self) -> bool {
        self.keeps_input()
            || self.name == "str"
            || matches!(self.spec.input, spec::TypePattern::Apply(constructor, _) if matches!(constructor.0, "Option" | "Result"))
    }
}

/// A named unit supplying pipeline steps.
#[derive(Debug)]
pub struct Extension {
    /// The lowercase identifier an `import` names.
    pub(crate) name: &'static str,
    /// The version this extension reports.
    pub(crate) version: &'static str,
    /// What it supplies.
    pub(crate) steps: &'static [Operation],
    pub(crate) metadata: Option<fn(&mut crate::document::Card)>,
}

/// The core: collection operations, which mention nothing above the core's own
/// types and so are never imported because they are never absent.
pub(crate) static CORE: Extension = Extension {
    name: "core",
    version: env!("CARGO_PKG_VERSION"),
    steps: collections::STEPS,
    metadata: None,
};

static GRAPH: Extension = Extension {
    name: "graph",
    version: env!("CARGO_PKG_VERSION"),
    steps: graph::STEPS,
    metadata: None,
};

static FORMAT: Extension = Extension {
    name: "format",
    version: env!("CARGO_PKG_VERSION"),
    steps: format::STEPS,
    metadata: None,
};

static IO: Extension = Extension {
    name: "io",
    version: env!("CARGO_PKG_VERSION"),
    steps: io::STEPS,
    metadata: None,
};

static LOGGING: Extension = Extension {
    name: "logging",
    version: env!("CARGO_PKG_VERSION"),
    steps: logging::STEPS,
    metadata: None,
};

static NET: Extension = Extension {
    name: "net",
    version: env!("CARGO_PKG_VERSION"),
    steps: net::STEPS,
    metadata: None,
};

static LEXICAL: Extension = Extension {
    name: "lexical",
    version: env!("CARGO_PKG_VERSION"),
    steps: lexical::STEPS,
    metadata: Some(lexical::contribute_metadata),
};

static SEMANTIC: Extension = Extension {
    name: "semantic",
    version: env!("CARGO_PKG_VERSION"),
    steps: semantic::STEPS,
    metadata: None,
};

static STRING: Extension = Extension {
    name: "string",
    version: env!("CARGO_PKG_VERSION"),
    steps: string::STEPS,
    metadata: None,
};

static SEQ: Extension = Extension {
    name: "seq",
    version: env!("CARGO_PKG_VERSION"),
    steps: seq::STEPS,
    metadata: None,
};

static MATH: Extension = Extension {
    name: "math",
    version: env!("CARGO_PKG_VERSION"),
    steps: math::STEPS,
    metadata: None,
};

static OPTION: Extension = Extension {
    name: "option",
    version: env!("CARGO_PKG_VERSION"),
    steps: option::STEPS,
    metadata: None,
};

static RESULT: Extension = Extension {
    name: "result",
    version: env!("CARGO_PKG_VERSION"),
    steps: result::STEPS,
    metadata: None,
};

static REGEX: Extension = Extension {
    name: "regex",
    version: env!("CARGO_PKG_VERSION"),
    steps: regex::STEPS,
    metadata: None,
};

/// Every extension compiled into this binary, core first.
///
/// Registration is not import: a step here is known to diagnostics and to
/// `hql builtins` whether or not the program has imported the extension that
/// provides it, so a missing import reports the import rather than a
/// misspelling.
pub(crate) static REGISTERED: &[&Extension] = &[
    &CORE, &STRING, &SEQ, &MATH, &OPTION, &RESULT, &GRAPH, &FORMAT, &IO, &NET, &LOGGING, &LEXICAL,
    &SEMANTIC, &REGEX,
];

impl Extension {
    /// The step of a name this extension provides.
    pub(crate) fn step(&'static self, name: &str) -> Option<&'static Operation> {
        self.steps.iter().find(|step| step.name == name)
    }
}

/// The modules imported unless a vault opts out: the imports written in
/// `std/prelude.hql`, which is the standard library's implicit import list
/// and not a module of its own.
///
/// A prelude exists so that a program that trims a string or walks a graph
/// does not open with lines of ceremony. It is a convenience, not a rule,
/// which is why a vault can decline it.
pub(crate) fn prelude() -> &'static [&'static str] {
    static PRELUDE: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    PRELUDE.get_or_init(|| {
        crate::library::PRELUDE
            .lines()
            .filter_map(|line| line.trim().strip_prefix("import "))
            .map(|name| name.split_whitespace().next().unwrap_or(name))
            .collect()
    })
}

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
    /// Whether the prelude's modules arrive without being asked for.
    pub prelude: bool,
    /// The module roots workspace modules are discovered under: `[modules]
    /// roots` in `hql.toml`, resolved against the vault, and `--modules` on
    /// the command line.
    pub module_roots: Vec<std::path::PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            prelude: true,
            module_roots: Vec::new(),
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
        if let Some(roots) = parsed
            .get("modules")
            .and_then(|section| section.get("roots"))
            .and_then(toml::Value::as_array)
        {
            config.module_roots = roots
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|path| root.join(path))
                .collect();
        }
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
    /// Each visible builtin module, under the local name it is bound to.
    imported: Vec<Imported>,
}

/// A builtin module bound into a source unit under a local name: its last
/// segment, or an alias.
#[derive(Debug, Clone, Copy)]
struct Imported {
    local: &'static str,
    extension: &'static Extension,
}

/// The canonical module names a builtin extension answers to, beyond its own
/// one-word name: the `io` family is reached through `io.net` and
/// `io.logging` as `std/io/` lays it out.
const CANONICAL: &[(&str, &str)] = &[("io.net", "net"), ("io.logging", "logging")];

/// The builtin module a dotted path names, if any.
pub(crate) fn builtin_module(path: &str) -> Option<&'static Extension> {
    let name = CANONICAL
        .iter()
        .find(|(canonical, _)| *canonical == path)
        .map_or(path, |(_, name)| name);
    extension(name)
}

impl Resolution {
    /// The core, plus the prelude unless the vault declines it.
    pub(crate) fn bare(prelude: bool) -> Self {
        let mut imported = vec![Imported {
            local: CORE.name,
            extension: &CORE,
        }];
        if prelude {
            for name in self::prelude() {
                if let Some(found) = builtin_module(name)
                    && !imported
                        .iter()
                        .any(|held| held.extension.name == found.name)
                {
                    imported.push(Imported {
                        local: found.name,
                        extension: found,
                    });
                }
            }
        }
        Self { imported }
    }

    /// What a vault starts every program with.
    ///
    /// A configured import fails the same way a written one does, at a span
    /// pointing at the program's start, because the fault is in the vault
    /// rather than in the line a reader is looking at.
    pub(crate) fn new(config: &Config) -> Result<Self, Diagnostic> {
        for extension in REGISTERED {
            for step in extension.steps {
                step.spec.validate().map_err(|error| {
                    Diagnostic::typing(0..0, format!("{}.{}, {error}", extension.name, step.name))
                })?;
            }
        }
        let mut resolution = Self::bare(config.prelude);
        for name in &config.imports {
            let configured = |message: String| {
                Diagnostic::name(
                    0..0,
                    format!("the vault's {}: {message}", crate::reporting::CONFIG_FILE),
                )
            };
            // A vault's import list names standard modules; a workspace
            // module is imported where it is used.
            let bound = resolution
                .import(name, None, &(0..0))
                .map_err(|failed| configured(failed.message()))?;
            if !bound {
                return Err(configured(format!("no standard module named `{name}`")));
            }
        }
        Ok(resolution)
    }

    /// Bind an extension's steps into the program.
    ///
    /// A collision is reported here rather than at the use site, where the
    /// failure would depend on which pipeline a reader happened to look at.
    pub(crate) fn import(
        &mut self,
        path: &str,
        alias: Option<&str>,
        span: &Range<usize>,
    ) -> Result<bool, Diagnostic> {
        let Some(wanted) = builtin_module(path) else {
            return Ok(false);
        };
        let local = match alias {
            Some(alias) => crate::types::program::intern(alias),
            None => crate::types::program::intern(path.rsplit('.').next().unwrap_or(path)),
        };
        self.bind_as(local, wanted, span)?;
        Ok(true)
    }

    /// Bind an extension under a local name. One extension may be bound
    /// under several names; one name may not stand for two extensions.
    pub(crate) fn bind_as(
        &mut self,
        local: &'static str,
        wanted: &'static Extension,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        if let Some(held) = self.imported.iter().find(|held| held.local == local) {
            if held.extension.name == wanted.name {
                return Ok(());
            }
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{local}` already names the module `{}` here; import `{}` under another \
                     name with `as`",
                    held.extension.name, wanted.name
                ),
            ));
        }
        // Two modules may declare one function name: `contains` of a String
        // and of a collection are two declarations, told apart at the call by
        // the input, and a call that fits both is refused there.
        self.imported.push(Imported {
            local,
            extension: wanted,
        });
        Ok(())
    }

    /// Whether a builtin module is visible under this local name.
    pub(crate) fn has(&self, name: &str) -> bool {
        self.imported.iter().any(|held| held.local == name)
    }

    /// Whether a local name is taken by a visible builtin module.
    pub(crate) fn names_module(&self, name: &str) -> bool {
        self.has(name)
    }

    /// The step a name resolves to for an input of a type, among every
    /// visible module declaring the name: the one whose input the type fits,
    /// told apart from the others by that and by how many positional
    /// arguments are written. Several that fit are an ambiguity, naming
    /// each provider so the call can be qualified.
    pub(crate) fn step_for(
        &self,
        name: &str,
        input: &TypeRef,
        positional: usize,
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        let mut candidates: Vec<(&'static Extension, &'static Operation)> = Vec::new();
        for held in &self.imported {
            if let Some(step) = held.extension.step(name)
                && !candidates
                    .iter()
                    .any(|(extension, _)| extension.name == held.extension.name)
            {
                candidates.push((held.extension, step));
            }
        }
        match candidates.as_slice() {
            [] => Err(self.unresolved(name, span)),
            [(_, only)] => Ok(only),
            many => {
                let input = input.present();
                let fitting: Vec<_> = many
                    .iter()
                    .filter(|(_, step)| {
                        step.spec.input.accepts(input)
                            || (input.constructor.0 == "Option"
                                && step.spec.input.accepts(input.present()))
                    })
                    .collect();
                let by_arity: Vec<_> = fitting
                    .iter()
                    .filter(|(_, step)| {
                        let specs = step.spec.arguments;
                        let required = specs.iter().filter(|spec| spec.required).count();
                        let variadic = specs.iter().any(|spec| spec.variadic);
                        positional >= required && (variadic || positional <= specs.len())
                    })
                    .collect();
                match (by_arity.as_slice(), fitting.as_slice()) {
                    ([(_, step)], _) => Ok(step),
                    ([], [(_, step)]) => Ok(step),
                    ([], []) => Ok(many[0].1),
                    (several, _) => Err(Diagnostic::name(
                        span.clone(),
                        format!(
                            "`{name}` over {input} is declared by {}, so the call is \
                             ambiguous; qualify it, `{}.{name}(…)`",
                            several
                                .iter()
                                .map(|(extension, _)| format!("`{}`", extension.name))
                                .collect::<Vec<_>>()
                                .join(" and "),
                            several[0].0.name
                        ),
                    )),
                }
            }
        }
    }

    /// The step a bare name resolves to, whatever its input: the first
    /// visible module declaring it.
    #[cfg(test)]
    pub(crate) fn step(
        &self,
        name: &str,
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        if let Some(step) = self
            .imported
            .iter()
            .find_map(|held| held.extension.step(name))
        {
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
    ) -> Result<&'static Operation, Diagnostic> {
        let held = self
            .imported
            .iter()
            .find(|held| held.local == extension)
            .ok_or_else(|| self.unresolved(extension, span))?;
        held.extension.step(name).ok_or_else(|| {
            Diagnostic::name(
                span.clone(),
                format!("the `{extension}` module provides no function `{name}`"),
            )
        })
    }

    /// Why a name did not resolve: a missing import, or no such step at all.
    fn unresolved(&self, name: &str, span: &Range<usize>) -> Diagnostic {
        let providers = providers(name);
        let message = match providers.as_slice() {
            [] => {
                let hint = crate::builtins::hint(name);
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// Which extension supplies it.
    pub provider: &'static str,
    /// That extension's version.
    pub version: &'static str,
    /// How it is written.
    pub name: &'static str,
    /// How it is called.
    pub signature: String,
    /// The structural contract behind the rendered signature.
    pub spec: &'static OperationSpec,
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
            } else if config.prelude && prelude().contains(&extension.name) {
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
                    signature: step.spec.signature(step.name),
                    spec: &step.spec,
                    summary: step.summary,
                    purity: step.purity,
                })
                .collect(),
        })
        .collect()
}

/// The argument written with a name.
pub(crate) fn named<'a, E>(arguments: &'a [Arg<E>], name: &str) -> Option<&'a Arg<E>> {
    arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some(name))
}

/// The argument written with a name, or the first written without one.
pub(crate) fn named_or_first<'a, E>(arguments: &'a [Arg<E>], name: &str) -> Option<&'a Arg<E>> {
    named(arguments, name).or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
}

/// Let each registered extension contribute the metadata it owns.
pub(crate) fn contribute_metadata(card: &mut crate::document::Card) {
    for extension in REGISTERED {
        if let Some(contribute) = extension.metadata {
            contribute(card);
        }
    }
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
        // Reading and writing an endpoint, and logging, are the effects;
        // every other step, constructing an endpoint or a logger included, is
        // a function of its input.
        for entry in &listed {
            let effectful = (entry.provider == "io" && entry.name != "file")
                || (entry.provider == "logging" && entry.name != "Logger");
            let expected = if effectful {
                Purity::Effectful
            } else {
                Purity::Pure
            };
            assert_eq!(entry.purity, expected, "{}", entry.name);
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
        let markdown = listed
            .iter()
            .find(|entry| entry.name == "markdown")
            .unwrap();
        assert_eq!(markdown.provider, "format");
        assert_eq!(providers("count").len(), 1);
        assert!(providers("nonesuch").is_empty());
        assert!(extension("graph").is_some());
        assert!(extension("nonesuch").is_none());
    }

    #[test]
    fn the_core_resolves_without_an_import_and_retrieval_does_not() {
        let mut resolution = Resolution::bare(true);
        assert!(resolution.step("count", &(0..1)).is_ok());
        assert!(resolution.step("markdown", &(0..1)).is_ok());

        let missing = resolution.step("lexical", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("import lexical"),
            "{}",
            missing.message()
        );

        resolution.import("lexical", None, &(0..1)).unwrap();
        assert!(resolution.step("lexical", &(0..1)).is_ok());
        assert!(resolution.qualified("lexical", "lexical", &(0..1)).is_ok());
        assert!(resolution.qualified("lexical", "rank", &(0..1)).is_err());
    }

    #[test]
    fn the_prelude_is_a_convenience_a_vault_may_decline() {
        let bare = Resolution::bare(false);
        let missing = bare.step("markdown", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("import format"),
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
        metadata: None,
        steps: &[Operation {
            name: "count",
            spec: OperationSpec {
                parameters: &[],
                input: spec::TypePattern::Any,
                arguments: &[],
                output: spec::Output::Type(spec::TypePattern::Type(crate::types::TypeConstructor(
                    "Int",
                ))),
            },
            summary: "A second `count`, so the collision has two sides.",
            purity: Purity::Pure,
            check: None,
            eval: |_, _, _, _| Ok(Value::Int(0)),
        }],
    };

    #[test]
    fn two_modules_declaring_one_name_are_told_apart_at_the_call() {
        let mut resolution = Resolution::bare(true);
        resolution.bind_as(PROBE.name, &PROBE, &(0..1)).unwrap();
        // A collection fits the core's `count` and the probe's `Any`, with
        // no arity to tell them apart: the call is ambiguous and says so.
        let both = resolution
            .step_for("count", &TypeRef::list(TypeRef::INT), 0, &(0..1))
            .unwrap_err();
        assert!(
            both.message().contains("`core` and `probe`"),
            "{}",
            both.message()
        );
        // An Int fits only the probe's.
        let probe = resolution
            .step_for("count", &TypeRef::INT, 0, &(0..1))
            .unwrap();
        assert_eq!(
            probe.summary,
            "A second `count`, so the collision has two sides."
        );
        assert!(resolution.step("count", &(0..1)).is_ok());
    }

    #[test]
    fn importing_the_same_extension_twice_is_not_a_collision() {
        let mut resolution = Resolution::bare(true);
        resolution.import("lexical", None, &(0..1)).unwrap();
        resolution.import("lexical", None, &(0..1)).unwrap();
        assert!(resolution.step("lexical", &(0..1)).is_ok());
    }

    #[test]
    fn a_vault_may_import_for_every_program_and_may_decline_the_prelude() {
        let config = Config {
            imports: vec!["lexical".to_owned()],
            prelude: false,
            module_roots: Vec::new(),
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
        assert_eq!(by_name("format"), Availability::Absent);
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
            module_roots: Vec::new(),
        };
        let failed = Resolution::new(&config).unwrap_err();
        assert!(
            failed.message().contains("hql.toml"),
            "{}",
            failed.message()
        );
    }

    #[test]
    fn importing_a_name_no_builtin_module_has_binds_nothing() {
        let mut resolution = Resolution::bare(true);
        assert!(!resolution.import("nonesuch", None, &(0..1)).unwrap());
        assert!(resolution.import("io.net", None, &(0..1)).unwrap());
        assert!(resolution.has("net"));
    }
}
