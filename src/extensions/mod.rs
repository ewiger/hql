//! The registry: what a pipeline step is, and which extension provides it.
//!
//! A step is a name a pipeline may write, together with the function that
//! checks it and the function that evaluates it. Keeping both beside the name
//! is what lets a capability be added without editing the core's checker or
//! evaluator, and what lets `hql builtins` name a provider rather than print a
//! flat list that stops being true once two modules can declare one name.
//!
//! The binary carries two extensions, one per builtin package: `std`,
//! providing every module of `lib/std/` with an external declaration, and
//! `search`, providing `lexical` and `semantic`. Each is a descriptor, its
//! `id`, its `abi` and the modules it `provides`, beside the implementations
//! of those modules' declarations, keyed by the module and the declared
//! name. The Rust source modules of this directory contribute their step
//! tables to one descriptor; how the code is organised is never a reason for
//! a second extension.
//!
//! The `Extension` type is a host mechanism. It is how this binary is
//! organised; HQL programs see typed functions and never see a trait.

use crate::ast::{Arg, Expr};
use crate::diagnostics::Diagnostic;
use crate::environment::RunEnvironment;
use crate::execution::{Arg as TypedArg, Expr as TypedExpr};
use crate::types::TypeRef;
use crate::types::Value;

use std::ops::Range;
use std::path::Path;

pub(crate) mod binding;
pub(crate) mod bytes;
pub(crate) mod collections;
pub(crate) mod contract;
pub(crate) mod crypto;
pub(crate) mod digest;
pub(crate) mod encoding;
pub(crate) mod env;
pub mod format;
pub(crate) mod fs;
pub(crate) mod graph;
pub(crate) mod io;
pub mod lexical;
pub mod logging;
pub(crate) mod math;
pub mod nats;
pub mod net;
pub(crate) mod option;
pub mod postgres;
pub mod process;
pub(crate) mod random;
pub(crate) mod regex;
pub(crate) mod result;
pub mod semantic;
pub(crate) mod seq;
pub mod spark;
pub mod spec;
pub mod sqlite;
pub(crate) mod stats;
pub(crate) mod string;
pub(crate) mod time;
pub(crate) mod traits;
pub(crate) mod transaction;
pub(crate) mod window;
pub use collections::{COMBINERS, Combiner};
use spec::{CheckedCall, OperationSpec};

/// The combiner of a name: `+`, `*`, `min`, `max`, `union`, `merge` or
/// `concat`.
#[must_use]
pub fn combiner(name: &str) -> Option<&'static Combiner> {
    COMBINERS.iter().find(|combiner| combiner.name == name)
}

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

/// How the executor may split a step's input: the whole of what a backend
/// may assume about partitioning it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    /// The result for each element depends on that element alone: any split
    /// of the input may run separately and be recombined in the input's
    /// order.
    Elementwise,
    /// The input is regrouped by a key, and the groups may complete in any
    /// order.
    Partition,
    /// The whole input collapses to one value through a combiner the
    /// executor holds, so splits may be reduced and their results combined.
    Reduction,
    /// The whole input is needed before any output: nothing may be split.
    Barrier,
}

impl Shape {
    /// Every shape, in the order help explains them.
    pub const ALL: [Self; 4] = [
        Self::Elementwise,
        Self::Partition,
        Self::Reduction,
        Self::Barrier,
    ];

    /// The word help prints for this shape, and the word a declaration
    /// writes before `fn`.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Elementwise => "elementwise",
            Self::Partition => "partition",
            Self::Reduction => "reduction",
            Self::Barrier => "barrier",
        }
    }

    /// The shape a declaration's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|shape| shape.label() == word)
    }
}

/// What a step does to a type.
pub(crate) type Check =
    fn(&mut dyn CheckCx, &CheckedCall<'_>, Range<usize>) -> Result<(), Diagnostic>;

/// What a step does to a value.
pub type Eval = fn(&mut dyn EvalCx, Value, &[TypedArg], Range<usize>) -> Result<Value, Diagnostic>;

/// What checking a step may ask of the checker around it.
///
/// An argument arrives as unevaluated syntax, because a step such as
/// `filter(c => …)` applies its argument once per element rather than once.
pub(crate) trait CheckCx: Send {
    /// What the program is checked against.
    fn environment(&self) -> &RunEnvironment;
    /// The configured id a binding holds, where the binding's value is
    /// `configured("id")` (HQL-0051), so a read or a write through the
    /// binding is checked as one by the id.
    fn configured_binding(&self, _name: &str) -> Option<String> {
        None
    }
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
    fn warn(&mut self, warning: crate::diagnostics::Diagnostic);
    /// Fix an alias parameter's type to what a step argument requires;
    /// answers whether `actual` was such a parameter.
    fn placeholder(&mut self, actual: &TypeRef, expected: Option<TypeRef>) -> bool;
}

/// What evaluating a step may ask of the evaluator around it.
pub trait EvalCx: Send {
    /// What the program runs against.
    fn environment(&self) -> &RunEnvironment;
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
    /// Whether two values are equal as `==` says: through `Option`, and by
    /// the `eq` an implementation of `Eq` provides where one applies.
    fn equal_values(
        &mut self,
        left: &Value,
        right: &Value,
        span: &Range<usize>,
    ) -> Result<bool, Diagnostic>;
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
    fn warn(&mut self, warning: crate::diagnostics::Diagnostic);
    /// Whether the run asks for continuation: under `collect` a step may go
    /// on past an element its function failed on.
    fn collecting(&self) -> bool;
    /// A function value applied to values: one an argument evaluated to,
    /// such as an element of a list of functions.
    fn call(
        &mut self,
        function: &Value,
        values: Vec<Value>,
        span: &Range<usize>,
    ) -> Result<Value, Diagnostic>;
    /// 64 bits drawn at the call from the evaluation seed, which `--seed`
    /// fixes: what a step without a `seed` argument of its own draws from.
    fn draw(&mut self, span: &Range<usize>) -> u64;
}

/// One name a pipeline may write.
#[derive(Debug)]
pub struct Operation {
    /// How it is written.
    pub(crate) name: &'static str,
    /// Where its checking signature comes from: its HQL declaration, or a
    /// native one for what a contract cannot state yet. Read it with
    /// [`Operation::spec`].
    pub(crate) spec: Signature,
    /// The extension's own marking of whether the executor may move it:
    /// binding checks it against the declaration, which is what
    /// [`Operation::purity`] reads.
    pub(crate) purity: Purity,
    /// The extension's own marking of how the executor may split it:
    /// binding checks it against the declaration, which is what
    /// [`Operation::shape`] reads.
    pub(crate) shape: Shape,
    /// What it does to a type.
    pub(crate) check: Option<Check>,
    /// What it does to a value.
    pub(crate) eval: Eval,
}

/// Where an operation's checking signature comes from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Signature {
    /// Its external declarations in a standard module: binding derives one
    /// signature from each, an overloaded name having several, and Rust
    /// writes no type.
    Declared,
    /// Written here, for an operation no contract declares or one whose
    /// result is computed from the call. Binding compares it with the
    /// declaration wherever the declaration states one; a native operation
    /// has one signature, so its name is declared once.
    Native(OperationSpec),
}

/// An operation whose signature, purity and shape are its declaration's in a
/// standard module: a name and what it does to a value, marked
/// pure and a barrier, which binding checks against the declaration.
pub(crate) const fn declared(name: &'static str, eval: Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Pure,
        shape: Shape::Barrier,
        check: None,
        eval,
    }
}

impl Operation {
    /// The same operation marked with another shape, for one its
    /// declaration writes `elementwise fn`, `partition fn` or
    /// `reduction fn`.
    pub const fn shaped(self, shape: Shape) -> Self {
        Self { shape, ..self }
    }

    /// The same operation with a check of its own, for a rule its
    /// declaration cannot state.
    pub(crate) const fn checked(self, check: Check) -> Self {
        Self {
            check: Some(check),
            ..self
        }
    }

    /// The structural signature used for checking and help: the first
    /// overload of a name declared more than once.
    pub(crate) fn spec(&self) -> &OperationSpec {
        &self.specs()[0]
    }

    /// The signature a call takes: the first overload whose input, arity and
    /// value arguments the call fits. Where the input and the arity leave
    /// several, the arguments' types decide; where a call fits a form by its
    /// arity and no form by its types, and another form takes those
    /// arguments, the refusal names every form of that arity. Otherwise the
    /// first form the call fits by arity, or failing that by its input and
    /// the argument names it writes, is checked, so checking names the
    /// argument that is wrong; where the input meets none, the refusal lists
    /// what it may be.
    pub(crate) fn overload(
        &self,
        cx: &mut dyn CheckCx,
        input: &crate::types::TypeRef,
        arguments: &[crate::ast::Arg],
        span: &std::ops::Range<usize>,
    ) -> Result<&OperationSpec, Diagnostic> {
        let specs = self.specs();
        if let [only] = specs {
            return Ok(only);
        }
        let fitting: Vec<&OperationSpec> = specs
            .iter()
            .filter(|spec| spec.fits(cx, input, arguments))
            .collect();
        for spec in &fitting {
            if spec.fits_types(cx, Some(input), arguments)? {
                return Ok(spec);
            }
        }
        if let Some(first) = fitting.first() {
            // Forms of this arity that differ by their arguments' types: a
            // form for another input takes these arguments.
            let forms: Vec<&OperationSpec> = specs
                .iter()
                .filter(|spec| spec.fits_arity(cx, arguments))
                .collect();
            let mut taken = false;
            for form in &forms {
                if !fitting.iter().any(|held| std::ptr::eq(*held, *form))
                    && form.fits_types(cx, None, arguments)?
                {
                    taken = true;
                }
            }
            if taken && forms.len() > 1 {
                return Err(spec::no_form(
                    self.name, &forms, cx, input, arguments, span,
                )?);
            }
            return Ok(first);
        }
        if let Some(message) = collections::join_refusal(self.name, input, arguments) {
            return Err(Diagnostic::typing(span.clone(), message));
        }
        if let Some(spec) = specs
            .iter()
            .find(|spec| spec.input.accepts(input) && spec.declares_names(arguments))
            .or_else(|| specs.iter().find(|spec| spec.input.accepts(input)))
        {
            return Ok(spec);
        }
        let inputs: Vec<_> = specs.iter().map(|spec| spec.input).collect();
        Err(spec::refused(self.name, &inputs, input, span))
    }

    /// Every signature the operation takes, most specific first: a call
    /// takes the first one it fits.
    /// Whether a form of the operation is declared without an input, its
    /// input `Unit`: a call to it writes no positional argument.
    pub(crate) fn takes_no_input(&self) -> bool {
        self.specs().iter().any(|spec| {
            matches!(spec.input, spec::TypePattern::Type(unit) if unit == TypeRef::UNIT.constructor)
        })
    }

    pub(crate) fn specs(&self) -> &[OperationSpec] {
        match binding::Environment::get() {
            Ok(environment) => environment.specs(self),
            Err(_) => match &self.spec {
                Signature::Native(spec) => std::slice::from_ref(spec),
                Signature::Declared => std::slice::from_ref(&binding::UNBOUND),
            },
        }
    }

    /// Whether the executor may move it: what its declaration states, `fn`
    /// pure and `effectful fn` effectful. An operation of a module without a
    /// contract has no declaration, and its extension's marking stands.
    pub(crate) fn purity(&self) -> Purity {
        binding::Environment::get().map_or(self.purity, |environment| environment.purity(self))
    }

    /// Whether every form of the operation is effectful, so a call is
    /// whichever form it takes. Where forms differ, the form a call takes
    /// decides.
    pub(crate) fn effectful_in_every_form(&self) -> bool {
        match binding::Environment::get() {
            Ok(environment) => {
                environment.purity(self) == Purity::Effectful && !environment.mixed_purity(self)
            }
            Err(_) => self.purity == Purity::Effectful,
        }
    }

    /// How the executor may split it: what its declaration states, `fn` a
    /// barrier and `elementwise fn`, `partition fn` or `reduction fn` the
    /// shape it names. An operation without a declaration keeps its
    /// extension's marking.
    pub fn shape(&self) -> Shape {
        binding::Environment::get().map_or(self.shape, |environment| environment.shape(self))
    }

    /// Whether the step receives an optional input as it is, absence
    /// included: one declared over `Option<T>`, and `str`, whose text is of
    /// the whole value.
    pub(crate) fn takes_raw_input(&self) -> bool {
        self.name == "str"
            || matches!(self.spec().input, spec::TypePattern::Apply(constructor, _) if matches!(constructor.0, "Option" | "Result"))
    }
}

/// One module an extension provides: the canonical name of its contract
/// under `lib/`, and the operations implementing that contract's external
/// declarations. An implementation is keyed by the declaration identity it
/// satisfies, the module and the declared name, so two modules of one
/// extension may each declare a `contains`.
#[derive(Debug)]
pub struct Provided {
    /// The canonical module name, `io.net`.
    pub(crate) module: &'static str,
    /// The operations implementing the module's external declarations. Two
    /// modules may share one table: `core` declares types alone, and the
    /// operations over its collection types are `collections`'s.
    pub(crate) steps: &'static [Operation],
}

/// The ABI every extension compiled into this binary reaches the process
/// by: `1`, an in-process library with the types of this crate.
pub(crate) const ABI: u32 = 1;

/// Where an extension's code comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Compiled into the binary: the extension of a builtin package under
    /// `lib/`.
    Internal,
    /// Supplied by an installed package and reached through its transport
    /// (HQL-0057).
    External,
}

/// An extension: the implementation unit of a package. Its descriptor is the
/// `id`, the `abi` and the modules it `provides`; beside the descriptor are
/// the implementations of those modules' external declarations. Binding
/// cross-checks each provided contract against them.
#[derive(Debug)]
pub struct Extension {
    /// The descriptor's `id`: the package's name, `std` or `hmd`.
    pub(crate) id: &'static str,
    /// The descriptor's `abi`: how its code reaches the process.
    pub(crate) abi: u32,
    /// Whether the code is compiled in or installed.
    pub(crate) origin: Origin,
    /// The version this extension reports.
    pub(crate) version: &'static str,
    /// The descriptor's `provides`, each module with its implementations.
    pub(crate) provides: &'static [Provided],
    pub(crate) metadata: Option<fn(&mut crate::document::Card)>,
}

/// The one-word imports two modules keep beside their canonical paths:
/// `import net` is `import io.net`, and `import logging` is `import
/// io.logging`. A transport's module has no one-word form (HQL-0048).
const SHORT_NAMES: &[(&str, &str)] = &[("net", "io.net"), ("logging", "io.logging")];

/// The canonical module name an import path names: the path itself, or
/// the module a kept one-word form stands for.
pub(crate) fn canonical(path: &str) -> &str {
    SHORT_NAMES
        .iter()
        .find(|(short, _)| *short == path)
        .map_or(path, |(_, module)| module)
}

/// The import a diagnostic tells a program to write for a module: its kept
/// one-word form where it has one, else its canonical path.
fn import_of(module: &str) -> &str {
    // A module of `hmd` is reached by activating the package, whose prelude
    // imports it.
    if is_hmd(module) {
        return "hmd";
    }
    SHORT_NAMES
        .iter()
        .find(|(_, held)| *held == module)
        .map_or(module, |(short, _)| short)
}

/// The standard package's extension: every module of `lib/std/` whose
/// contract declares externally.
pub(crate) static STD: Extension = Extension {
    id: "std",
    abi: ABI,
    origin: Origin::Internal,
    version: env!("CARGO_PKG_VERSION"),
    provides: &[
        Provided {
            module: "core",
            steps: collections::STEPS,
        },
        Provided {
            module: "collections",
            steps: collections::STEPS,
        },
        Provided {
            module: "string",
            steps: string::STEPS,
        },
        Provided {
            module: "seq",
            steps: seq::STEPS,
        },
        Provided {
            module: "math",
            steps: math::STEPS,
        },
        Provided {
            module: "stats",
            steps: stats::STEPS,
        },
        Provided {
            module: "time",
            steps: time::STEPS,
        },
        Provided {
            module: "random",
            steps: random::STEPS,
        },
        Provided {
            module: "option",
            steps: option::STEPS,
        },
        Provided {
            module: "result",
            steps: result::STEPS,
        },
        Provided {
            module: "regex",
            steps: regex::STEPS,
        },
        Provided {
            module: "graph",
            steps: graph::STEPS,
        },
        Provided {
            module: "format",
            steps: format::STEPS,
        },
        Provided {
            module: "io",
            steps: io::STEPS,
        },
        Provided {
            module: "io.net",
            steps: net::STEPS,
        },
        Provided {
            module: "io.logging",
            steps: logging::STEPS,
        },
        Provided {
            module: "io.fs",
            steps: fs::STEPS,
        },
        Provided {
            module: "io.env",
            steps: env::STEPS,
        },
        Provided {
            module: "io.process",
            steps: process::STEPS,
        },
        Provided {
            module: "io.db.sqlite",
            steps: sqlite::STEPS,
        },
        Provided {
            module: "io.db.postgres",
            steps: postgres::STEPS,
        },
        Provided {
            module: "io.db.spark",
            steps: spark::STEPS,
        },
        Provided {
            module: "io.transaction",
            steps: transaction::STEPS,
        },
        Provided {
            module: "io.msg.nats",
            steps: nats::CORE_STEPS,
        },
        Provided {
            module: "io.msg.nats_jetstream",
            steps: nats::JETSTREAM_STEPS,
        },
        Provided {
            module: "bytes",
            steps: bytes::STEPS,
        },
        Provided {
            module: "encoding",
            steps: encoding::STEPS,
        },
        Provided {
            module: "crypto",
            steps: crypto::STEPS,
        },
        Provided {
            module: "crypto.digest",
            steps: digest::STEPS,
        },
    ],
    metadata: None,
};

/// The `hmd` package's extension (HQL-0059): HyperMarkdown in HQL, and
/// among it retrieval over a vault's cards. `hmd`, `hmd.vault` and
/// `hmd.search` declare nothing external, so only the two retrieval modules
/// are provided.
pub(crate) static HMD: Extension = Extension {
    id: "hmd",
    abi: ABI,
    origin: Origin::Internal,
    version: env!("CARGO_PKG_VERSION"),
    provides: &[
        Provided {
            module: "hmd.search.lexical",
            steps: lexical::STEPS,
        },
        Provided {
            module: "hmd.search.semantic",
            steps: semantic::STEPS,
        },
    ],
    metadata: Some(lexical::contribute_metadata),
};

/// Every extension compiled into this binary, one per builtin package.
///
/// Registration is not import: a step here is known to diagnostics and to
/// `hql builtins` whether or not the program has imported the module that
/// declares it, so a missing import reports the import rather than a
/// misspelling.
pub(crate) static REGISTERED: &[&Extension] = &[&STD, &HMD];

impl Extension {
    /// Whether the descriptor provides a module.
    pub(crate) fn provides(&self, module: &str) -> bool {
        self.provides
            .iter()
            .any(|provided| provided.module == module)
    }

    /// The operation implementing a declaration of a provided module, by
    /// the declaration's identity: the module and the name.
    pub(crate) fn implementation(&self, module: &str, name: &str) -> Option<&'static Operation> {
        self.provides
            .iter()
            .find(|provided| provided.module == module)
            .and_then(|provided| provided.steps.iter().find(|step| step.name == name))
    }

    /// The step tables the extension carries, each once: a table two
    /// modules share is one table.
    fn tables(&self) -> Vec<&'static [Operation]> {
        let mut tables: Vec<&'static [Operation]> = Vec::new();
        for provided in self.provides {
            if !tables
                .iter()
                .any(|held| std::ptr::eq(held.as_ptr(), provided.steps.as_ptr()))
            {
                tables.push(provided.steps);
            }
        }
        tables
    }

    /// Every operation the extension carries, each once.
    pub(crate) fn steps(&self) -> impl Iterator<Item = &'static Operation> {
        self.tables().into_iter().flatten()
    }
}

/// The modules imported unless a vault opts out: the imports written in
/// `lib/std/prelude.hql`, which is the standard library's implicit import list
/// and not a module of its own.
///
/// A prelude exists so that a program that trims a string or walks a graph
/// does not open with lines of ceremony. It is a convenience, not a rule,
/// which is why a vault can decline it.
pub(crate) fn prelude() -> &'static [&'static str] {
    static PRELUDE: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    PRELUDE.get_or_init(|| modules_of(prelude_entries()))
}

/// The statements of `lib/std/prelude.hql`, each as an import entry.
pub(crate) fn prelude_entries() -> &'static [String] {
    static ENTRIES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ENTRIES.get_or_init(|| {
        prelude_imports(crate::library::PRELUDE).expect("lib/std/prelude.hql is an import list")
    })
}

/// The modules a list of import entries names, each once, in order.
fn modules_of(entries: &'static [String]) -> Vec<&'static str> {
    let mut modules: Vec<&'static str> = Vec::new();
    for entry in entries {
        let module = entry_module(entry);
        if !modules.contains(&module) {
            modules.push(module);
        }
    }
    modules
}

/// The module an import entry names: `m` itself, or the `m` of
/// `from m import …`.
pub(crate) fn entry_module(entry: &str) -> &str {
    match entry.strip_prefix("from ") {
        Some(rest) => rest.split_whitespace().next().unwrap_or(rest),
        None => entry,
    }
}

/// The registered extension of an id.
pub(crate) fn extension(id: &str) -> Option<&'static Extension> {
    crate::package::extensions()
        .iter()
        .copied()
        .find(|extension| extension.id == id)
}

/// Whether a loaded package's extension implements a constructor of a
/// name: an operation named as a type is, `Dataset`.
pub(crate) fn constructs(name: &str) -> bool {
    crate::constructors::is_constructor_name(name)
        && crate::package::extensions()
            .iter()
            .filter(|extension| extension.origin == Origin::External)
            .any(|extension| extension.steps().any(|step| step.name == name))
}

/// The registered extension whose descriptor provides a module, if any.
pub(crate) fn provider_of(module: &str) -> Option<&'static Extension> {
    crate::package::extensions()
        .iter()
        .copied()
        .find(|extension| extension.provides(module))
}

/// Every builtin module declaring a step name publicly, in loading order.
#[cfg(test)]
pub(crate) fn providers(step: &str) -> Vec<&'static str> {
    let Ok(environment) = binding::Environment::get() else {
        return Vec::new();
    };
    environment
        .modules()
        .filter(|(id, _)| {
            environment
                .offered(*id, step)
                .is_some_and(|operation| !environment.is_private(operation))
        })
        .map(|(_, module)| module.name)
        .collect()
}

/// What a project's `.hql/config.toml` says about modules and packages.
///
/// The prelude is a compatibility convenience, and a project that wants every
/// capability stated in the program that uses it must be able to decline it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Config {
    /// What is imported for every program this project runs: builtin modules
    /// by name, and packages by their id, which imports every module the
    /// package provides.
    pub imports: Vec<String>,
    /// Whether the prelude's modules arrive without being asked for.
    pub prelude: bool,
    /// The module roots project modules are discovered under: `[modules]
    /// roots` in `.hql/config.toml`, resolved against the project root, and `--modules` on
    /// the command line.
    pub module_roots: Vec<std::path::PathBuf>,
    /// The crates of packages under development: `[packages] paths` in
    /// `.hql/config.toml`, resolved against the project root. Each is built on first use
    /// and loaded as an installed package is.
    pub package_paths: Vec<std::path::PathBuf>,
    /// The preludes of the packages active in the run, `std`'s apart: what
    /// [`Config::activated`] gathers, and no configuration file states.
    pub preludes: Vec<Prelude>,
}

/// The prelude of an active package: the modules its `prelude.hql` imports
/// into every unit of the run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prelude {
    /// The file, as a diagnostic names it.
    pub file: String,
    /// Its import statements, in the order written, each an import entry:
    /// a module's name for `import m`, or the text of a `from` statement.
    pub imports: Vec<String>,
}

/// The import statements of a package's `prelude.hql`, each as an import
/// entry: the module's name for `import m`, the statement's text for
/// `from m import …`. A prelude is an import list: a statement of another
/// kind, or an import under an alias, is refused.
///
/// # Errors
///
/// Returns why the text is no import list.
pub(crate) fn prelude_imports(source: &str) -> Result<Vec<String>, String> {
    let program = crate::parser::parse(source).map_err(|error| error.message())?;
    program
        .statements
        .iter()
        .map(|statement| match statement {
            crate::ast::Stmt::Import {
                path, alias: None, ..
            } => Ok(path.join(".")),
            crate::ast::Stmt::Import { path, .. } => Err(format!(
                "`import {}` is written with `as`: a prelude imports a module under its own \
                 name",
                path.join(".")
            )),
            crate::ast::Stmt::From { span, .. } => Ok(source[span.clone()].to_owned()),
            _ => Err("a prelude holds imports and nothing else".to_owned()),
        })
        .collect()
}

/// The module every import of a run names: the program's, and those of
/// each project module it reaches. A module that cannot be found, read or
/// parsed contributes nothing here; checking reports it.
fn run_imports(program: &crate::ast::Program, roots: &[std::path::PathBuf]) -> Vec<String> {
    fn written(program: &crate::ast::Program) -> impl Iterator<Item = String> + '_ {
        program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                crate::ast::Stmt::Import { path, .. } | crate::ast::Stmt::From { path, .. } => {
                    Some(path.join("."))
                }
                _ => None,
            })
    }
    let mut found: Vec<String> = Vec::new();
    let mut pending: Vec<String> = written(program).collect();
    while let Some(module) = pending.pop() {
        if found.contains(&module) {
            continue;
        }
        // A module of a package is no file under a root.
        let project = !roots.is_empty()
            && !crate::package::graph()
                .iter()
                .any(|(held, _)| *held == canonical(&module));
        if project
            && let Ok(Some(file)) = crate::modules::locate(roots, &module)
            && let Ok(source) = std::fs::read_to_string(file)
            && let Ok(unit) = crate::parser::parse(&source)
        {
            pending.extend(written(&unit));
        }
        found.push(module);
    }
    found
}

impl Default for Config {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            prelude: true,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
            preludes: Vec::new(),
        }
    }
}

/// Whether a module path names the `hmd` package's root module or one of its
/// modules: what importing activates the package (HQL-0059).
#[must_use]
pub fn is_hmd(path: &str) -> bool {
    path == "hmd" || path.starts_with("hmd.")
}

thread_local! {
    /// Whether `hmd` is active for the program being checked on this thread:
    /// what a step's check reads when its meaning is HyperMarkdown's, as a
    /// configured endpoint of kind `vault` is.
    static HMD_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Record whether `hmd` is active for the program about to be checked.
pub(crate) fn set_checking_hmd(active: bool) {
    HMD_ACTIVE.with(|held| held.set(active));
}

/// Whether `hmd` is active for the program being checked.
pub(crate) fn checking_hmd() -> bool {
    HMD_ACTIVE.with(std::cell::Cell::get)
}

/// The statements of `lib/hmd/prelude.hql`, each as an import entry: what
/// every unit of a run imports while `hmd` is active.
pub(crate) fn hmd_prelude_entries() -> &'static [String] {
    static ENTRIES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ENTRIES.get_or_init(|| {
        prelude_imports(crate::library::HMD_PRELUDE).expect("lib/hmd/prelude.hql is an import list")
    })
}

impl Config {
    /// Whether `hmd` is active under this configuration: a configured
    /// import, `[modules] import` or `--import`, names a module of it.
    #[must_use]
    pub fn hmd_active(&self) -> bool {
        self.imports
            .iter()
            .any(|import| is_hmd(entry_module(import)))
    }

    /// The configuration a program runs under: this one, with the prelude
    /// of every package active in the run, unless preludes are declined.
    ///
    /// A package is active when a unit of the run, the program or a project
    /// module it reaches, or a configured import names one of its modules.
    /// It is active for the whole run, and its prelude is imported by every
    /// unit of the run: the effective prelude is the union per run
    /// (HQL-0024, part 3). `hmd` is a package among them (HQL-0059).
    #[must_use]
    pub(crate) fn activated(&self, program: &crate::ast::Program) -> Self {
        let written = run_imports(program, &self.module_roots);
        let mut config = self.clone();
        config.preludes.clear();
        if self.hmd_active() || written.iter().any(|module| is_hmd(module)) {
            if !config.hmd_active() {
                config.imports.push("hmd".to_owned());
            }
            if config.prelude {
                config.preludes.push(Prelude {
                    file: "lib/hmd/prelude.hql".to_owned(),
                    imports: hmd_prelude_entries().to_vec(),
                });
            }
        }
        if !config.prelude {
            return config;
        }
        let loaded = crate::package::active().map_or(&[][..], |active| &active.loaded);
        for package in loaded {
            let installed = &package.installed;
            let active = installed.provides.iter().any(|module| {
                written.contains(module)
                    || self
                        .imports
                        .iter()
                        .any(|entry| entry_module(entry) == module)
            }) || self
                .imports
                .iter()
                .any(|entry| entry_module(entry) == installed.name);
            if active && !package.prelude.is_empty() {
                config.preludes.push(Prelude {
                    file: package
                        .directory
                        .join("modules/prelude.hql")
                        .display()
                        .to_string(),
                    imports: package.prelude.clone(),
                });
            }
        }
        config
    }

    /// Read `[modules]` and `[packages]` from a project's `.hql/config.toml`.
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
        if let Some(paths) = parsed
            .get("packages")
            .and_then(|section| section.get("paths"))
            .and_then(toml::Value::as_array)
        {
            config.package_paths = paths
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|path| root.join(path))
                .collect();
        }
        let Some(section) = parsed.get("modules") else {
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

/// Whether a module is available to a program, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// In a prelude the run keeps.
    Prelude,
    /// Named by `[modules] import` or `--import`.
    Configured,
    /// Registered, and written out by any program that wants it.
    Absent,
}

impl Availability {
    /// How help says it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Prelude => "prelude",
            Self::Configured => "imported by this project",
            Self::Absent => "imported where it is used",
        }
    }
}

/// Which builtin modules a source unit has imported, and what they let it
/// write.
///
/// Registration and import are different things: every module of the
/// binary's packages is registered, and a unit resolves only what it has
/// imported. That is what lets a diagnostic name the import a program is
/// missing instead of guessing at a misspelling. Visibility is the module's,
/// not its extension's: `import io.net` makes `io.net` visible and nothing
/// else `std` provides.
#[derive(Debug, Clone)]
pub(crate) struct Resolution {
    /// The frozen environment the program runs against: its modules are
    /// what an import may name, and their declarations say what an
    /// importer reaches.
    environment: binding::Frozen,
    /// Each builtin module `import` bound, under the local name it is bound
    /// to: reached qualified, and by receiver call (HQL-0060).
    imported: Vec<Imported>,
    /// What each `from` statement bound unqualified, in the order written.
    bound: Vec<Bound>,
    /// Whether the run's configuration makes `hmd` active, so that `vault`,
    /// its bindings and document references belong to the program.
    hmd: bool,
}

/// A builtin module bound into a source unit under a local name: its last
/// segment, or an alias.
#[derive(Debug, Clone, Copy)]
struct Imported {
    local: &'static str,
    module: binding::ModuleId,
}

/// The receiver types a binding admits a form at: one type, every type but
/// some, or every type.
#[derive(Debug, Clone, Default)]
struct Receivers {
    /// `T.*` and `T.f`: only forms whose receiver type is `T`.
    only: Option<String>,
    /// `except T.*` and `except T.f`: not forms at these types.
    not: Vec<String>,
}

impl Receivers {
    /// Whether a form is admitted: one that takes `self` at the receiver
    /// type `ty`, `None` where the type is a variable, or one without `self`.
    /// A type selector matches its type exactly.
    fn admits(&self, receiver: bool, ty: Option<&str>) -> bool {
        if let Some(only) = &self.only {
            return receiver && ty == Some(only.as_str());
        }
        !(receiver && ty.is_some_and(|held| self.not.iter().any(|not| not == held)))
    }

    fn restricts(&self) -> bool {
        self.only.is_some() || !self.not.is_empty()
    }
}

/// One name, or one group of names, a `from` statement binds from a module
/// (HQL-0060).
#[derive(Debug, Clone)]
struct Bound {
    module: binding::ModuleId,
    /// The canonical name of the module, for messages.
    module_name: &'static str,
    /// How a written name maps to the module's own: one name, or a pattern.
    names: Names,
    /// The receiver types whose forms it binds.
    receivers: Receivers,
    /// The module's names a group leaves out, `except f`.
    except: Vec<String>,
    /// The forms a group leaves out by type and name, `except T.f`.
    except_at: Vec<(String, String)>,
    /// Whether it is reached unqualified, `f(x)` and `x | f`.
    function: bool,
    /// Whether it is reached by receiver call, `x.f()`.
    receiver: bool,
}

/// An operation a written name reaches, with the name to qualify it by
/// and the receiver types it is admitted at.
struct Reached {
    step: &'static Operation,
    qualifier: &'static str,
    admitted: Receivers,
}

#[derive(Debug, Clone)]
enum Names {
    /// `f` or `f as g`: the bound name and the module's.
    One { bound: String, original: String },
    /// `*` or `T.*`, renamed by a pattern `prefix*suffix`, or not renamed.
    Group { prefix: String, suffix: String },
}

impl Bound {
    /// The module's name a written name stands for, where this binding binds
    /// the written name.
    fn original<'n>(&'n self, written: &'n str) -> Option<&'n str> {
        match &self.names {
            Names::One { bound, original } => (bound == written).then_some(original.as_str()),
            Names::Group { prefix, suffix } => {
                let original = written.strip_prefix(prefix.as_str())?;
                let original = original.strip_suffix(suffix.as_str())?;
                (!original.is_empty() && !self.except.iter().any(|held| held == original))
                    .then_some(original)
            }
        }
    }
}

/// Bind the standard library's contracts to the extensions implementing them.
///
/// HQL declares, an extension implements: the process's runtime environment
/// loads the modules of `lib/std/`, allocating module, declaration, type, trait,
/// implementation and constructor identities, binds every compiled-in
/// extension to them and validates the result once. For every module written
/// as external declarations the extension whose descriptor provides the
/// module must implement each declaration and nothing more. A declaration's
/// signature, its receiver, its purity and its privacy are the declaration's;
/// the extension's own marking of purity, and a native signature where one is
/// written, are checked against it. A mismatch is a defect of the binary, and
/// refuses every program with a message naming the module, the declaration
/// and the extension.
///
/// # Errors
///
/// Returns the first disagreement between a contract and its implementation.
pub fn bind() -> Result<(), String> {
    binding::Frozen::process().map(drop)
}

impl Resolution {
    /// The prelude's imports, unless the vault declines them, over a frozen
    /// environment: an ordinary list of import statements (HQL-0060).
    pub(crate) fn bare(environment: binding::Frozen, prelude: bool) -> Self {
        let mut resolution = Self {
            environment,
            imported: Vec::new(),
            bound: Vec::new(),
            hmd: false,
        };
        if prelude {
            for entry in prelude_entries() {
                resolution
                    .entry(entry, &(0..0))
                    .expect("lib/std/prelude.hql imports what the standard library declares");
            }
        }
        resolution
    }

    /// Bind one import entry: `m`, read as `import m`, or the text of a
    /// `from` statement. Answers whether the entry named a builtin module.
    pub(crate) fn entry(&mut self, entry: &str, span: &Range<usize>) -> Result<bool, Diagnostic> {
        if !entry.starts_with("from ") {
            return self.import(entry, None, span);
        }
        let program = crate::parser::parse(entry)
            .map_err(|error| Diagnostic::name(span.clone(), error.message()))?;
        let [crate::ast::Stmt::From { path, entries, .. }] = program.statements.as_slice() else {
            return Err(Diagnostic::name(
                span.clone(),
                format!("`{entry}` is no import statement"),
            ));
        };
        self.bind_from(&path.join("."), entries, span)
    }

    /// `from m import …` of a builtin module: bind what each entry selects,
    /// unqualified, and its receivers under the names they are bound as.
    /// Answers whether the path named a builtin module.
    pub(crate) fn bind_from(
        &mut self,
        path: &str,
        entries: &[crate::ast::ImportEntry],
        span: &Range<usize>,
    ) -> Result<bool, Diagnostic> {
        let Some(module) = self.environment.provided_module(canonical(path)) else {
            return Ok(false);
        };
        for entry in entries {
            let bound = self.selected(module, entry)?;
            self.bound.extend(bound);
        }
        let _ = span;
        Ok(true)
    }

    /// What one entry of a `from` statement binds, checked against the
    /// module's declarations.
    fn selected(
        &self,
        module: binding::ModuleId,
        entry: &crate::ast::ImportEntry,
    ) -> Result<Vec<Bound>, Diagnostic> {
        use crate::ast::Selector;
        let environment = self.environment;
        let module_name = environment.module(module).name;
        let at = &entry.span;
        let invalid = |message: String| {
            Diagnostic::name(at.clone(), message).with_kind(crate::diagnostics::Kind::InvalidImport)
        };
        // The public operations of the module, by name.
        let public = |name: &str| {
            environment
                .offered(module, name)
                .filter(|step| !environment.is_private(step))
        };
        let missing = |name: &str, span: &Range<usize>| {
            if environment
                .offered(module, name)
                .is_some_and(|step| environment.is_private(step))
            {
                return Diagnostic::name(
                    span.clone(),
                    format!(
                        "`{name}` is private to the module `{module_name}`, and cannot be \
                         imported"
                    ),
                );
            }
            Diagnostic::name(
                span.clone(),
                format!("the module `{module_name}` declares no public `{name}`"),
            )
        };
        let selects_at = |ty: &str| {
            environment.operation_names(module).any(|name| {
                public(name).is_some_and(|step| {
                    environment
                        .receiver_forms(step)
                        .iter()
                        .any(|(held, _)| *held == ty)
                })
            })
        };
        let nothing_at = |ty: &str, span: &Range<usize>| {
            let mut over: Vec<&str> = Vec::new();
            for name in environment.operation_names(module) {
                if let Some(step) = public(name) {
                    for (held, _) in environment.receiver_forms(step) {
                        if !over.contains(&held) {
                            over.push(held);
                        }
                    }
                }
            }
            over.sort_unstable();
            let found = match over.as_slice() {
                [] => "it declares no receiver".to_owned(),
                types => format!(
                    "its receivers are over {}",
                    types
                        .iter()
                        .map(|held| format!("`{held}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            };
            Diagnostic::name(
                span.clone(),
                format!(
                    "`{ty}.*` selects nothing in the module `{module_name}`: no public function \
                     has the receiver type `{ty}`; {found}"
                ),
            )
        };
        let mut receivers = Receivers::default();
        let mut except = Vec::new();
        for (excluded, span) in &entry.except {
            match excluded {
                Selector::Name(name) => {
                    if public(name).is_none() {
                        return Err(missing(name, span));
                    }
                    except.push(name.clone());
                }
                Selector::TypeAll(ty) => {
                    if !selects_at(ty) {
                        return Err(nothing_at(ty, span));
                    }
                    receivers.not.push(ty.clone());
                }
                Selector::TypeName(ty, name) => {
                    let Some(step) = public(name) else {
                        return Err(missing(name, span));
                    };
                    if !environment
                        .receiver_forms(step)
                        .iter()
                        .any(|(held, _)| held == ty)
                    {
                        return Err(Diagnostic::name(
                            span.clone(),
                            format!(
                                "`{ty}.{name}` selects nothing in the module `{module_name}`: \
                                 its `{name}` has no form whose receiver type is `{ty}`"
                            ),
                        ));
                    }
                    // A group leaves out one form of a name by leaving out
                    // the name at that type only.
                    except.push(format!("{ty}.{name}"));
                }
                Selector::All => unreachable!("the parser refuses `except *`"),
            }
        }
        let alias = entry.alias.as_ref();
        let pattern = |alias: &crate::ast::Rename| -> Result<(String, String), Diagnostic> {
            match alias.name.split_once('*') {
                Some((prefix, suffix)) => Ok((prefix.to_owned(), suffix.to_owned())),
                None => Err(invalid(format!(
                    "`{} as {alias}` renames a group to one name; write a pattern with one `*`, \
                     such as `{}`",
                    entry.selector,
                    match &alias.ty {
                        Some(ty) => format!("{ty}.{}_*", alias.name),
                        None => format!("{}_*", alias.name),
                    }
                ))),
            }
        };
        let item_alias = |alias: &crate::ast::Rename| -> Result<(), Diagnostic> {
            if alias.name.contains('*') {
                return Err(invalid(format!(
                    "`{} as {alias}` renames one name by a pattern; a pattern renames a group, \
                     `*` or `T.*`",
                    entry.selector
                )));
            }
            Ok(())
        };
        let (names_out, typed_out): (Vec<String>, Vec<String>) =
            except.into_iter().partition(|held| !held.contains('.'));
        let except_at: Vec<(String, String)> = typed_out
            .iter()
            .filter_map(|held| held.split_once('.'))
            .map(|(ty, name)| (ty.to_owned(), name.to_owned()))
            .collect();
        let mut bound = Vec::new();
        match &entry.selector {
            Selector::All | Selector::TypeAll(_) => {
                if let Selector::TypeAll(ty) = &entry.selector {
                    if !selects_at(ty) {
                        return Err(nothing_at(ty, at));
                    }
                    receivers.only = Some(ty.clone());
                }
                let (prefix, suffix, function, receiver) = match alias {
                    None => (String::new(), String::new(), true, true),
                    Some(alias) => {
                        let (prefix, suffix) = pattern(alias)?;
                        match (&alias.ty, &entry.selector) {
                            (None, _) => (prefix, suffix, true, false),
                            (Some(ty), Selector::TypeAll(selected)) if ty == selected => {
                                (prefix, suffix, true, true)
                            }
                            (Some(ty), Selector::TypeAll(selected)) => {
                                return Err(invalid(format!(
                                    "`{} as {alias}` changes the receiver type from `{selected}` \
                                     to `{ty}`; a receiver alias keeps its type: write \
                                     `{selected}.{}`",
                                    entry.selector, alias.name
                                )));
                            }
                            (Some(_), _) => {
                                return Err(invalid(format!(
                                    "`* as {alias}` binds functions of every receiver type; \
                                     name the type in the selector, `{}.* as {alias}`",
                                    alias.ty.as_deref().unwrap_or_default()
                                )));
                            }
                        }
                    }
                };
                bound.push(Bound {
                    module,
                    module_name,
                    names: Names::Group { prefix, suffix },
                    receivers: receivers.clone(),
                    except: names_out,
                    except_at,
                    function,
                    receiver,
                });
            }
            Selector::Name(name) => {
                if public(name).is_none() {
                    if environment.declares_type(module, name) {
                        // Types stay visible unqualified, as they are; the
                        // import names one, checked against the module.
                        if let Some(alias) = alias {
                            return Err(invalid(format!(
                                "`{name} as {alias}` renames a type; a type keeps its name"
                            )));
                        }
                        return Ok(Vec::new());
                    }
                    return Err(missing(name, at));
                }
                let (bound_name, receiver) = match alias {
                    None => (name.clone(), true),
                    Some(alias) => {
                        item_alias(alias)?;
                        if let Some(ty) = &alias.ty {
                            return Err(invalid(format!(
                                "`{name} as {alias}` binds a receiver at `{ty}`; select the \
                                 receiver by its type, `{ty}.{name} as {alias}`"
                            )));
                        }
                        (alias.name.clone(), false)
                    }
                };
                bound.push(Bound {
                    module,
                    module_name,
                    names: Names::One {
                        bound: bound_name,
                        original: name.clone(),
                    },
                    receivers,
                    except: Vec::new(),
                    except_at: Vec::new(),
                    function: true,
                    receiver,
                });
            }
            Selector::TypeName(ty, name) => {
                let Some(step) = public(name) else {
                    return Err(missing(name, at));
                };
                if !environment
                    .receiver_forms(step)
                    .iter()
                    .any(|(held, _)| held == ty)
                {
                    return Err(Diagnostic::name(
                        at.clone(),
                        format!(
                            "`{ty}.{name}` selects nothing in the module `{module_name}`: its \
                             `{name}` has no form whose receiver type is `{ty}`"
                        ),
                    ));
                }
                receivers.only = Some(ty.clone());
                let (bound_name, receiver) = match alias {
                    None => (name.clone(), true),
                    Some(alias) => {
                        item_alias(alias)?;
                        match &alias.ty {
                            None => (alias.name.clone(), false),
                            Some(held) if held == ty => (alias.name.clone(), true),
                            Some(held) => {
                                return Err(invalid(format!(
                                    "`{ty}.{name} as {alias}` changes the receiver type from \
                                     `{ty}` to `{held}`; a receiver alias keeps its type: write \
                                     `{ty}.{name} as {ty}.{}`",
                                    alias.name
                                )));
                            }
                        }
                    }
                };
                bound.push(Bound {
                    module,
                    module_name,
                    names: Names::One {
                        bound: bound_name,
                        original: name.clone(),
                    },
                    receivers,
                    except: Vec::new(),
                    except_at: Vec::new(),
                    function: true,
                    receiver,
                });
            }
        }
        Ok(bound)
    }

    /// Whether a name is bound unqualified or as a receiver by a `from`
    /// statement: a name the checker resolves among the imports though no
    /// extension declares it, `bar_upper`.
    pub(crate) fn binds(&self, name: &str) -> bool {
        self.bound.iter().any(|bound| {
            bound
                .original(name)
                .is_some_and(|original| self.public(bound.module, original).is_some())
        })
    }

    /// The canonical names of the modules whose operations a `from`
    /// statement bound under a name, each once.
    pub(crate) fn binding_modules(&self, name: &str) -> Vec<&'static str> {
        let mut found: Vec<&'static str> = Vec::new();
        for reached in self.reached(name, false) {
            let module =
                self.environment
                    .of(reached.step)
                    .first()
                    .map_or(reached.qualifier, |id| {
                        self.environment
                            .module(self.environment.declaration(*id).module)
                            .name
                    });
            if !found.contains(&module) {
                found.push(module);
            }
        }
        found
    }

    /// A module's public operation of a name.
    fn public(&self, module: binding::ModuleId, name: &str) -> Option<&'static Operation> {
        self.environment
            .offered(module, name)
            .filter(|step| !self.environment.is_private(step))
    }

    /// Every operation a written name reaches: by the `from` bindings, as a
    /// function or as a receiver, and by receiver call through every module
    /// `import` bound. Each comes with the module's local or canonical name
    /// for messages and the receiver types it is admitted at.
    fn reached(&self, name: &str, receivers: bool) -> Vec<Reached> {
        let mut found: Vec<Reached> = Vec::new();
        let add = |found: &mut Vec<Reached>, reached: Reached| {
            if let Some(held) = found
                .iter_mut()
                .find(|held| std::ptr::eq(held.step, reached.step))
            {
                // One operation reached twice is one: the widest admission
                // stands.
                if !reached.admitted.restricts() {
                    held.admitted = Receivers::default();
                }
                return;
            }
            found.push(reached);
        };
        if receivers {
            for held in &self.imported {
                if let Some(step) = self.public(held.module, name)
                    && self.environment.has_receiver(step)
                {
                    add(
                        &mut found,
                        Reached {
                            step,
                            qualifier: held.local,
                            admitted: Receivers::default(),
                        },
                    );
                }
            }
        }
        for bound in &self.bound {
            if !(if receivers {
                bound.receiver
            } else {
                bound.function
            }) {
                continue;
            }
            let Some(original) = bound.original(name) else {
                continue;
            };
            let Some(step) = self.public(bound.module, original) else {
                continue;
            };
            let mut admitted = bound.receivers.clone();
            for (ty, held) in &bound.except_at {
                if held == original {
                    admitted.not.push(ty.clone());
                }
            }
            let forms = self.environment.forms(step);
            if !forms.iter().any(|(receiver, ty, _)| {
                admitted.admits(*receiver, *ty) && (!receivers || *receiver)
            }) {
                continue;
            }
            add(
                &mut found,
                Reached {
                    step,
                    qualifier: bound.module_name,
                    admitted,
                },
            );
        }
        found
    }

    /// What a run starts every program with: the configured layer of the
    /// process's runtime environment, resolved once per configuration.
    ///
    /// A configured import fails the same way a written one does, at a span
    /// pointing at the program's start, because the fault is in the
    /// configuration rather than in the line a reader is looking at.
    pub(crate) fn new(config: &Config) -> Result<Self, Diagnostic> {
        binding::Runtime::process(config).map(|runtime| runtime.resolution.clone())
    }

    /// Record whether the run's configuration makes `hmd` active.
    pub(crate) fn set_hmd(&mut self, active: bool) {
        self.hmd = active;
    }

    /// Whether `hmd` is active for the run this resolution starts.
    pub(crate) fn hmd_active(&self) -> bool {
        self.hmd
    }

    /// The frozen environment this resolution runs against.
    pub(crate) fn environment(&self) -> binding::Frozen {
        self.environment
    }

    /// Bind a builtin module an extension provides into the unit, under its
    /// last segment or an alias; answers whether the path named one.
    ///
    /// A collision is reported here rather than at the use site, where the
    /// failure would depend on which pipeline a reader happened to look at.
    pub(crate) fn import(
        &mut self,
        path: &str,
        alias: Option<&str>,
        span: &Range<usize>,
    ) -> Result<bool, Diagnostic> {
        let Some(wanted) = self.environment.provided_module(canonical(path)) else {
            return Ok(false);
        };
        let local = match alias {
            Some(alias) => crate::types::program::intern(alias),
            None => crate::types::program::intern(path.rsplit('.').next().unwrap_or(path)),
        };
        self.bind_as(local, wanted, span)?;
        self.bind_constructors(wanted);
        Ok(true)
    }

    /// A type's name stays usable unqualified wherever its module is
    /// imported, and its constructor is written with that name: `import m`
    /// binds each constructor of a type `m` declares (HQL-0060).
    fn bind_constructors(&mut self, module: binding::ModuleId) {
        let environment = self.environment;
        let module_name = environment.module(module).name;
        let constructors: Vec<String> = environment
            .operation_names(module)
            .filter(|name| environment.declares_type(module, name))
            .filter(|name| self.public(module, name).is_some())
            .map(str::to_owned)
            .collect();
        for name in constructors {
            self.bound.push(Bound {
                module,
                module_name,
                names: Names::One {
                    bound: name.clone(),
                    original: name,
                },
                receivers: Receivers::default(),
                except: Vec::new(),
                except_at: Vec::new(),
                function: true,
                receiver: false,
            });
        }
    }

    /// Bind every module a registered extension provides, each under its
    /// last segment, for `[extensions] import` naming a package; answers
    /// whether the id named one.
    pub(crate) fn import_package(
        &mut self,
        id: &str,
        span: &Range<usize>,
    ) -> Result<bool, Diagnostic> {
        let Some(package) = extension(id) else {
            return Ok(false);
        };
        for provided in package.provides {
            self.import(provided.module, None, span)?;
        }
        Ok(true)
    }

    /// Whether a path names a standard module the runtime implements: one
    /// whose contract declares nothing external, `doc` or `knowledge`, so no
    /// extension provides it. Importing one binds no operation; its
    /// declarations are types the environment already holds.
    pub(crate) fn is_runtime_module(&self, path: &str) -> bool {
        self.environment.modules().any(|(_, module)| {
            module.name == path && path != "prelude" && module.provider.is_none()
        })
    }

    /// Bind a module under a local name. One module may be bound under
    /// several names; one name may not stand for two modules.
    fn bind_as(
        &mut self,
        local: &'static str,
        wanted: binding::ModuleId,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        if let Some(held) = self.imported.iter().find(|held| held.local == local) {
            if held.module == wanted {
                return Ok(());
            }
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{local}` already names the module `{}` here; import `{}` under another \
                     name with `as`",
                    self.environment.module(held.module).name,
                    self.environment.module(wanted).name
                ),
            ));
        }
        // Two modules may declare one function name: `contains` of a String
        // and of a collection are two declarations, told apart at the call by
        // the input, and a call that fits both is refused there.
        self.imported.push(Imported {
            local,
            module: wanted,
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
    /// operation a `from` statement binds under it: the one whose input the
    /// type fits, told apart from the others by that and by how many
    /// positional arguments are written. Several that fit are an ambiguity,
    /// naming each provider so the call can be qualified.
    pub(crate) fn step_for(
        &self,
        name: &str,
        input: &TypeRef,
        positional: usize,
        named: &[&str],
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        self.step_among(name, input, positional, named, span, false)
    }

    /// The step a receiver call, `value.f()`, resolves to: as
    /// [`Resolution::step_for`], among the receivers reachable under the
    /// name: every receiver of a module `import` bound, and those a `from`
    /// statement bound as receivers. A name reached only as a function is
    /// reached by a call or a pipeline step, and the refusal says so.
    pub(crate) fn receiver_step_for(
        &self,
        name: &str,
        input: &TypeRef,
        positional: usize,
        named: &[&str],
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        let declared_without_self = self
            .imported
            .iter()
            .any(|held| self.public(held.module, name).is_some());
        if self.reached(name, true).is_empty()
            && (declared_without_self || !self.reached(name, false).is_empty())
        {
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` takes no receiver here: its declaration has no `self`, or it is \
                     imported as a function only, so `value.{name}(…)` does not reach it; write \
                     `{name}(value, …)` or `value | {name}(…)`"
                ),
            ));
        }
        self.step_among(name, input, positional, named, span, true)
    }

    fn step_among(
        &self,
        name: &str,
        input: &TypeRef,
        positional: usize,
        named: &[&str],
        span: &Range<usize>,
        receivers: bool,
    ) -> Result<&'static Operation, Diagnostic> {
        let environment = self.environment;
        let present = input.present();
        // A binding restricted to some receiver types reaches a call only
        // through a form at one of them.
        let candidates: Vec<Reached> = self
            .reached(name, receivers)
            .into_iter()
            .filter(|reached| {
                !reached.admitted.restricts()
                    || *input == TypeRef::UNIT
                    || environment
                        .forms(reached.step)
                        .iter()
                        .any(|(receiver, ty, spec)| {
                            reached.admitted.admits(*receiver, *ty)
                                && (spec.input.accepts(present) || spec.input.accepts(input))
                        })
            })
            .collect();
        match candidates.as_slice() {
            [] => Err(self.unresolved_as(name, span, receivers)),
            [only] => Ok(only.step),
            many => {
                // The names a call writes tell modules apart before its
                // input does: only one module's `join` takes `into`.
                if !named.is_empty() {
                    let naming: Vec<_> = many
                        .iter()
                        .filter(|reached| {
                            environment.specs(reached.step).iter().any(|spec| {
                                named.iter().all(|written| {
                                    spec.arguments
                                        .iter()
                                        .any(|held| held.named && held.name == *written)
                                })
                            })
                        })
                        .collect();
                    if let [only] = naming.as_slice() {
                        return Ok(only.step);
                    }
                }
                let fitting: Vec<&Reached> = many
                    .iter()
                    .filter(|reached| {
                        environment.specs(reached.step).iter().any(|spec| {
                            spec.input.accepts(present)
                                || (present.constructor.0 == "Option"
                                    && spec.input.accepts(present.present()))
                        })
                    })
                    .collect();
                let by_arity: Vec<&Reached> = fitting
                    .iter()
                    .copied()
                    .filter(|reached| {
                        environment.specs(reached.step).iter().any(|spec| {
                            // The arguments written by position fill the
                            // ones a call may write so, and each name
                            // written must be one of the form's.
                            let specs = spec.arguments;
                            let by_position: Vec<_> =
                                specs.iter().filter(|spec| spec.positional).collect();
                            let variadic = specs.iter().any(|spec| spec.variadic);
                            let filled = |name: &str| {
                                named.contains(&name)
                                    || by_position
                                        .iter()
                                        .take(positional)
                                        .any(|spec| spec.name == name)
                            };
                            (variadic || positional <= by_position.len())
                                && specs
                                    .iter()
                                    .filter(|spec| spec.required && !spec.variadic)
                                    .all(|spec| filled(spec.name))
                                && named.iter().all(|written| {
                                    specs.iter().any(|spec| spec.named && spec.name == *written)
                                })
                        })
                    })
                    .collect();
                match (by_arity.as_slice(), fitting.as_slice()) {
                    ([reached], _) => Ok(reached.step),
                    ([], [reached]) => Ok(reached.step),
                    ([], []) => Ok(many[0].step),
                    (by_arity, fitting) => {
                        // Several fit: by arity where any does, else by input.
                        let several: &[&Reached] = if by_arity.is_empty() {
                            fitting
                        } else {
                            by_arity
                        };
                        let module_of = |reached: &Reached| {
                            environment
                                .of(reached.step)
                                .first()
                                .map_or(reached.qualifier, |id| {
                                    environment.module(environment.declaration(*id).module).name
                                })
                        };
                        Err(Diagnostic::name(
                            span.clone(),
                            format!(
                                "`{name}` over {present} is declared by {}, so the call is \
                                 ambiguous; qualify it, `{}.{name}(…)`",
                                several
                                    .iter()
                                    .map(|reached| format!("`{}`", module_of(reached)))
                                    .collect::<Vec<_>>()
                                    .join(" and "),
                                several[0].qualifier
                            ),
                        ))
                    }
                }
            }
        }
    }

    /// The modules whose receiver function `name` takes an input of this
    /// type and is reachable here, by local name: the providers a receiver
    /// call could reach.
    pub(crate) fn receiver_providers(&self, name: &str, input: &TypeRef) -> Vec<&'static str> {
        let input = input.present();
        self.reached(name, true)
            .into_iter()
            .filter(|reached| {
                self.environment
                    .forms(reached.step)
                    .iter()
                    .any(|(receiver, ty, spec)| {
                        *receiver && reached.admitted.admits(true, *ty) && spec.input.accepts(input)
                    })
            })
            .map(|reached| reached.qualifier)
            .collect()
    }

    /// The step a bare name resolves to, whatever its input: the first
    /// operation bound under it.
    #[cfg(test)]
    pub(crate) fn step(
        &self,
        name: &str,
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        if let Some(reached) = self.reached(name, false).first() {
            return Ok(reached.step);
        }
        Err(self.unresolved(name, span))
    }

    /// The step `<module>.<step>` names, which is always available for an
    /// imported module unless the module declares it private.
    pub(crate) fn qualified(
        &self,
        module: &str,
        name: &str,
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        let held = self
            .imported
            .iter()
            .find(|held| held.local == module)
            .ok_or_else(|| self.unresolved(module, span))?;
        let declared = self.environment.offered(held.module, name);
        if let Some(step) = declared
            && self.environment.is_private(step)
        {
            return Err(self.private(name, step, span));
        }
        declared.ok_or_else(|| {
            let refused = Diagnostic::name(
                span.clone(),
                format!("the `{module}` module provides no function `{name}`"),
            );
            match crate::builtins::explained(name) {
                Some(help) => refused.with_help(help),
                None => refused,
            }
        })
    }

    /// The refusal of a private operation reached from outside its module.
    fn private(&self, name: &str, step: &Operation, span: &Range<usize>) -> Diagnostic {
        let module = self.environment.of(step).first().map_or("", |id| {
            self.environment
                .module(self.environment.declaration(*id).module)
                .name
        });
        Diagnostic::name(
            span.clone(),
            format!(
                "`{name}` is private to the module `{module}`: it is reachable inside that \
                 module only"
            ),
        )
    }

    /// Why a name did not resolve: a private operation of an imported
    /// module, a missing import, or no such step at all.
    fn unresolved(&self, name: &str, span: &Range<usize>) -> Diagnostic {
        self.unresolved_as(name, span, false)
    }

    /// Why a name did not resolve, for a receiver call where `receiver`:
    /// `import m` makes every receiver of `m` reachable, so that is the
    /// import a receiver call is missing.
    fn unresolved_as(&self, name: &str, span: &Range<usize>, receiver: bool) -> Diagnostic {
        let modules = self
            .imported
            .iter()
            .map(|held| held.module)
            .chain(self.bound.iter().map(|bound| bound.module));
        if let Some(step) = modules.clone().find_map(|module| {
            self.environment
                .offered(module, name)
                .filter(|step| self.environment.is_private(step))
        }) {
            return self.private(name, step, span);
        }
        // A module imported qualified declares it: name both fixes.
        if let Some(held) = self
            .imported
            .iter()
            .find(|held| self.public(held.module, name).is_some())
        {
            let module = self.environment.module(held.module).name;
            return Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` is declared in the module `{module}`, which is imported qualified: \
                     write `{}.{name}(…)`, or `from {module} import {name}`",
                    held.local
                ),
            );
        }
        let declaring: Vec<&'static str> = self
            .environment
            .modules()
            .filter(|(id, _)| {
                self.environment
                    .offered(*id, name)
                    .is_some_and(|step| !self.environment.is_private(step))
            })
            .map(|(_, module)| module.name)
            .collect();
        let message = match declaring.as_slice() {
            [] => {
                let refused =
                    Diagnostic::name(span.clone(), format!("`{name}` is not a pipeline step"))
                        .with_kind(crate::diagnostics::Kind::UnknownStep);
                return match crate::builtins::hint(name) {
                    Some(help) => refused.with_help(help),
                    None => refused,
                };
            }
            // A module of `hmd` is reached by activating the package, whose
            // prelude imports it.
            [only] if is_hmd(only) => {
                format!("`{name}` is declared in the module `{only}`: write `import hmd`")
            }
            // A constructor comes with its type's module, and a receiver
            // with its module's import.
            [only]
                if receiver
                    || self
                        .environment
                        .provided_module(only)
                        .is_some_and(|module| self.environment.declares_type(module, name)) =>
            {
                format!(
                    "`{name}` is declared in the module `{only}`: write `import {}`",
                    import_of(only)
                )
            }
            [only] => format!(
                "`{name}` is declared in the module `{only}`: write `from {} import {name}`",
                import_of(only)
            ),
            many => format!(
                "`{name}` is declared in the modules {}: import one of them",
                many.iter()
                    .map(|module| format!("`{module}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
        };
        Diagnostic::name(span.clone(), message)
    }
}

/// Every registered step name, for the suggestion index.
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    let environment = binding::Environment::get().ok();
    crate::package::extensions()
        .iter()
        .flat_map(move |extension| {
            extension
                .steps()
                .filter(move |step| !environment.is_some_and(|held| held.is_private(step)))
                .map(|step| step.name)
        })
}

/// Whether a module is visible in every unit of a vault keeping the
/// prelude: always, or named by `lib/std/prelude.hql`.
fn in_reach(module: &str) -> bool {
    prelude().contains(&module)
}

/// Every step name, each with whether a module in reach of every program
/// provides it.
pub(crate) fn names_with_reach() -> impl Iterator<Item = (&'static str, bool)> {
    let environment = binding::Environment::get().ok();
    let mut reach: Vec<(&'static str, bool)> = Vec::new();
    for provided in crate::package::extensions()
        .iter()
        .flat_map(|extension| extension.provides)
    {
        for step in provided.steps {
            if environment.is_some_and(|held| held.is_private(step)) {
                continue;
            }
            match reach.iter_mut().find(|(name, _)| *name == step.name) {
                Some((_, held)) => *held |= in_reach(provided.module),
                None => reach.push((step.name, in_reach(provided.module))),
            }
        }
    }
    reach.into_iter()
}

/// One step, as `hql builtins` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The extension implementing it, by its descriptor's id.
    pub extension: &'static str,
    /// The module declaring it.
    pub module: &'static str,
    /// That extension's version.
    pub version: &'static str,
    /// How it is written.
    pub name: &'static str,
    /// How it is called.
    pub signature: String,
    /// The structural contract behind the rendered signature: the first
    /// overload of an overloaded step.
    pub spec: &'static OperationSpec,
    /// Every overload, in the order a call tries them.
    pub specs: &'static [OperationSpec],
    /// The first sentence of its doc string in the standard library.
    pub description: String,
    /// Whether the executor may move it.
    pub purity: Purity,
    /// How the executor may split it.
    pub shape: Shape,
}

/// One builtin module and its steps, as `hql builtins` prints them: the
/// module an `import` names, and the extension implementing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// The canonical module name an `import` writes.
    pub module: &'static str,
    /// The extension providing the module, by its descriptor's id.
    pub extension: &'static str,
    /// Whether that extension is compiled in or installed.
    pub origin: Origin,
    /// The version that extension reports.
    pub version: &'static str,
    /// Whether a program of this run has the module without asking.
    pub availability: Availability,
    /// The steps the module declares.
    pub steps: Vec<Listing>,
}

/// What a step does, as `hql builtins` says it: the first sentence of the
/// doc string of the first declaration of its name, in loading order, that
/// has one.
fn description(name: &str) -> String {
    crate::help::library()
        .iter()
        .flat_map(|module| &module.declarations)
        .filter(|declaration| {
            declaration.name == name && declaration.kind == crate::help::Kind::Function
        })
        .find_map(|declaration| declaration.doc.as_ref())
        .map(|doc| crate::docs::first_sentence(&doc.text, crate::help::VERSION))
        .unwrap_or_default()
}

/// Every module the registered extensions provide and its steps, as one
/// vault resolves them.
///
/// Every provided module is listed, not only what the vault imports: help
/// and the suggestion index answer the same question, so listing only the
/// imported ones would leave a reader unable to find the import a
/// diagnostic has just told them to write.
#[must_use]
pub fn catalogue(config: &Config) -> Vec<Provider> {
    let environment = binding::Environment::get().ok();
    // The steps a module lists are the ones it declares: of a table two
    // modules share, each lists its own. A private operation is its
    // module's own, and no program reaches it. Without an environment, a
    // defect of the binary, the table stands.
    let listed = |module: &str, step: &Operation| match environment {
        Some(environment) => {
            !environment.is_private(step)
                && environment.provided_module(module).is_some_and(|id| {
                    environment
                        .of(step)
                        .iter()
                        .any(|declared| environment.declaration(*declared).module == id)
                })
        }
        None => true,
    };
    let configured = |module: &str| {
        config.imports.iter().any(|held| {
            let held = entry_module(held);
            canonical(held) == module || extension(held).is_some_and(|held| held.provides(module))
        })
    };
    crate::package::extensions()
        .iter()
        .flat_map(|extension| {
            extension.provides.iter().map(|provided| Provider {
                module: provided.module,
                extension: extension.id,
                origin: extension.origin,
                version: extension.version,
                availability: if config.prelude && prelude().contains(&provided.module) {
                    Availability::Prelude
                } else if configured(provided.module) {
                    Availability::Configured
                } else {
                    Availability::Absent
                },
                steps: provided
                    .steps
                    .iter()
                    .filter(|step| listed(provided.module, step))
                    .map(|step| Listing {
                        extension: extension.id,
                        module: provided.module,
                        version: extension.version,
                        name: step.name,
                        signature: step
                            .specs()
                            .iter()
                            .map(|spec| spec.signature(step.name))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        spec: step.spec(),
                        specs: step.specs(),
                        description: description(step.name),
                        purity: step.purity(),
                        shape: step.shape(),
                    })
                    .collect(),
            })
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

/// Let each registered extension contribute the metadata it owns: `search`
/// writes the lexical model and index entries.
pub(crate) fn contribute_metadata(card: &mut crate::document::Card) {
    for extension in crate::package::extensions() {
        if let Some(contribute) = extension.metadata {
            contribute(card);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare(prelude: bool) -> Resolution {
        Resolution::bare(binding::Frozen::process().unwrap(), prelude)
    }

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
        // Reading and writing an endpoint, listing the file system,
        // logging, reading the evaluation clock and drawing from the
        // evaluation seed are the effects; every other step, constructing an
        // endpoint, a path or a logger included, is a function of its input.
        // Every name of `random` has a form that draws from the evaluation
        // seed, and its seeded forms are pure. Every name of `io.env` reads
        // the invocation.
        for entry in &listed {
            let effectful = (entry.module == "io"
                && !matches!(
                    entry.name,
                    "file"
                        | "configured"
                        | "Path"
                        | "join"
                        | "parent"
                        | "name"
                        | "stem"
                        | "extension"
                ))
                || entry.module == "io.transaction"
                || (entry.module == "io.fs"
                    && matches!(
                        entry.name,
                        "entries" | "walk" | "glob" | "exists" | "entry" | "working_dir"
                    ))
                || (entry.module == "io.logging" && entry.name != "Logger")
                || (entry.module == "time" && matches!(entry.name, "now" | "today" | "local_zone"))
                || entry.module == "random"
                // Reading a key file and drawing from the host generator.
                || (entry.module == "crypto"
                    && matches!(
                        entry.name,
                        "read_public_key"
                            | "read_private_key"
                            | "generate_key_pair"
                            | "generate_secret_key"
                            | "hash_password"
                            | "encrypt"
                            | "encrypt_to"
                            | "encrypt_with_passphrase"
                    ))
                || entry.module == "io.env"
                || (entry.module == "io.process" && entry.name == "run");
            let expected = if effectful {
                Purity::Effectful
            } else {
                Purity::Pure
            };
            assert_eq!(entry.purity, expected, "{}", entry.name);
            assert!(matches!(entry.extension, "std" | "hmd"), "{}", entry.name);
            assert!(!entry.signature.is_empty());
            assert!(!entry.description.is_empty(), "{}", entry.name);
        }
    }

    #[test]
    fn a_step_name_resolves_to_the_module_declaring_it_and_the_extension_providing_it() {
        let listed = listing();
        let count = listed.iter().find(|entry| entry.name == "count").unwrap();
        assert_eq!((count.module, count.extension), ("collections", "std"));
        let markdown = listed
            .iter()
            .find(|entry| entry.name == "markdown")
            .unwrap();
        assert_eq!((markdown.module, markdown.extension), ("format", "std"));
        let lexical = listed.iter().find(|entry| entry.name == "lexical").unwrap();
        assert_eq!(
            (lexical.module, lexical.extension),
            ("hmd.search.lexical", "hmd")
        );
        assert_eq!(providers("count"), ["collections"]);
        assert!(providers("nonesuch").is_empty());
        assert!(extension("std").is_some());
        assert!(extension("graph").is_none());
        assert!(provider_of("io.net").is_some_and(|held| held.id == "std"));
        assert!(provider_of("net").is_none());
    }

    #[test]
    fn the_binary_carries_one_extension_per_builtin_package() {
        assert_eq!(
            REGISTERED
                .iter()
                .map(|extension| (extension.id, extension.abi))
                .collect::<Vec<_>>(),
            [("std", ABI), ("hmd", ABI)]
        );
        // `core` and `collections` share one table, and each lists what it
        // declares: `Date` is `core`'s, `count` is `collections`'s, and
        // neither is listed twice.
        let listed = catalogue(&Config::default());
        let by_module = |wanted: &str| {
            listed
                .iter()
                .find(|provider| provider.module == wanted)
                .unwrap()
        };
        let names = |provider: &Provider| {
            provider
                .steps
                .iter()
                .map(|step| step.name)
                .collect::<Vec<_>>()
        };
        assert!(names(by_module("core")).contains(&"Date"));
        assert!(!names(by_module("core")).contains(&"count"));
        assert!(names(by_module("collections")).contains(&"count"));
        assert!(!names(by_module("collections")).contains(&"Date"));
        assert_eq!(by_module("core").availability, Availability::Prelude);
        assert_eq!(by_module("collections").availability, Availability::Prelude);
        assert!(
            listed
                .iter()
                .all(|provider| provider.origin == Origin::Internal)
        );
        // A module is provided by one extension, and every module a
        // contract declares externally is provided.
        for (module, _) in crate::library::contracts().unwrap() {
            let providing: Vec<_> = REGISTERED
                .iter()
                .filter(|extension| extension.provides(module))
                .collect();
            assert_eq!(providing.len(), 1, "{module}");
        }
    }

    #[test]
    fn the_core_resolves_without_an_import_and_retrieval_does_not() {
        let mut resolution = bare(true);
        assert!(resolution.step("count", &(0..1)).is_ok());
        assert!(resolution.step("markdown", &(0..1)).is_ok());

        let missing = resolution.step("lexical", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("write `import hmd`"),
            "{}",
            missing.message()
        );

        // `import` binds the module's name only (HQL-0060).
        resolution
            .import("hmd.search.lexical", None, &(0..1))
            .unwrap();
        let qualified = resolution.step("lexical", &(0..1)).unwrap_err();
        assert!(
            qualified.message().contains("imported qualified")
                && qualified.message().contains("`lexical.lexical(…)`")
                && qualified
                    .message()
                    .contains("`from hmd.search.lexical import lexical`"),
            "{}",
            qualified.message()
        );
        assert!(resolution.qualified("lexical", "lexical", &(0..1)).is_ok());
        assert!(resolution.qualified("lexical", "rank", &(0..1)).is_err());
        resolution
            .entry("from hmd.search.lexical import lexical", &(0..1))
            .unwrap();
        assert!(resolution.step("lexical", &(0..1)).is_ok());
    }

    #[test]
    fn the_prelude_is_a_convenience_a_vault_may_decline() {
        let bare = bare(false);
        let missing = bare.step("markdown", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("from format import markdown"),
            "{}",
            missing.message()
        );
        // `core` and `collections` are prelude lines like any other.
        assert!(bare.step("count", &(0..1)).is_err());
    }

    #[test]
    fn importing_the_same_module_twice_is_not_a_collision() {
        let mut resolution = bare(true);
        resolution
            .import("hmd.search.lexical", None, &(0..1))
            .unwrap();
        resolution
            .import("hmd.search.lexical", None, &(0..1))
            .unwrap();
        // One declaration bound twice is one.
        for _ in 0..2 {
            resolution
                .entry("from hmd.search.lexical import *", &(0..1))
                .unwrap();
        }
        assert!(resolution.step("lexical", &(0..1)).is_ok());
    }

    #[test]
    fn a_vault_may_import_for_every_program_and_may_decline_the_prelude() {
        let config = Config {
            imports: vec!["from hmd.search.lexical import lexical".to_owned()],
            prelude: false,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
            preludes: Vec::new(),
        };
        let resolution = Resolution::new(&config).expect("the configuration resolves");
        assert!(resolution.step("lexical", &(0..1)).is_ok());
        assert!(resolution.step("table", &(0..1)).is_err());

        let listed = catalogue(&config);
        let by_name = |wanted: &str| {
            listed
                .iter()
                .find(|provider| provider.module == wanted)
                .unwrap()
                .availability
        };
        // `prelude = false` withholds `core` and `collections` with the rest.
        assert_eq!(by_name("core"), Availability::Absent);
        assert_eq!(by_name("collections"), Availability::Absent);
        assert_eq!(by_name("format"), Availability::Absent);
        assert_eq!(by_name("hmd.search.lexical"), Availability::Configured);
        assert_eq!(by_name("hmd.search.semantic"), Availability::Absent);
        assert_eq!(by_name("graph"), Availability::Absent);

        let kept = catalogue(&Config::default());
        let graph = kept.iter().find(|provider| provider.module == "graph");
        assert_eq!(
            graph.map(|provider| provider.availability),
            Some(Availability::Prelude)
        );
    }

    #[test]
    fn importing_hmd_activates_it_and_its_prelude_reaches_every_unit() {
        let config = Config {
            imports: vec!["hmd".to_owned(), "logging".to_owned()],
            prelude: true,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
            preludes: Vec::new(),
        };
        assert!(config.hmd_active());
        let empty = crate::ast::Program {
            statements: Vec::new(),
            docs: crate::docs::Docs::default(),
        };
        let config = config.activated(&empty);
        let resolution = Resolution::new(&config).expect("the configuration resolves");
        assert!(resolution.hmd_active());
        assert!(resolution.step("lexical", &(0..1)).is_ok());
        assert!(resolution.step("semantic", &(0..1)).is_ok());
        assert!(resolution.has("logging"));

        let listed = catalogue(&config);
        let by_name = |wanted: &str| {
            listed
                .iter()
                .find(|provider| provider.module == wanted)
                .unwrap()
                .availability
        };
        assert_eq!(by_name("hmd.search.lexical"), Availability::Configured);
        assert_eq!(by_name("hmd.search.semantic"), Availability::Configured);
        assert_eq!(by_name("io.logging"), Availability::Configured);
        assert_eq!(by_name("io.net"), Availability::Absent);
    }

    #[test]
    fn a_configured_import_that_is_not_registered_fails_every_program() {
        let config = Config {
            imports: vec!["nonesuch".to_owned()],
            prelude: true,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
            preludes: Vec::new(),
        };
        let failed = Resolution::new(&config).unwrap_err();
        assert!(
            failed.message().contains(".hql/config.toml"),
            "{}",
            failed.message()
        );
    }

    #[test]
    fn the_standard_contracts_bind_to_their_extensions() {
        assert_eq!(bind(), Ok(()));
        // Every declared operation takes a signature its declaration states.
        let environment = binding::Environment::get().unwrap();
        for (id, declaration) in environment.declarations() {
            assert!(
                declaration.signature.is_ok(),
                "{}: {:?}",
                environment.qualified(id),
                declaration.signature
            );
        }
        // Every module declared externally has a provider, and the provider's
        // id is how `hql builtins` lists it beside the module.
        for (module, _) in crate::library::contracts().unwrap() {
            assert!(
                provider_of(module).is_some(),
                "{module} has no extension providing it"
            );
        }
    }

    #[test]
    fn a_missing_import_names_the_module_declaring_the_step() {
        let bare = bare(true);
        for (step, written) in [
            ("lexical", "write `import hmd`"),
            ("http", "write `from net import http`"),
            // A constructor comes with its type's module.
            ("Logger", "write `import logging`"),
            ("walk", "write `from io.fs import walk`"),
            ("mean", "write `from stats import mean`"),
        ] {
            let missing = bare.step(step, &(0..1)).unwrap_err();
            assert!(
                missing.message().contains("is declared in the module")
                    && missing.message().contains(written),
                "{}",
                missing.message()
            );
        }
    }

    #[test]
    fn importing_a_name_no_builtin_module_has_binds_nothing() {
        let mut resolution = bare(true);
        assert!(!resolution.import("nonesuch", None, &(0..1)).unwrap());
        assert!(resolution.import("io.net", None, &(0..1)).unwrap());
        assert!(resolution.has("net"));
    }
}
