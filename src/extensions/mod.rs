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
use crate::execution::{Arg as TypedArg, Expr as TypedExpr};
use crate::types::TypeRef;
use crate::types::Value;
use crate::vault::Vault;

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
    /// The vault the program is checked against.
    fn vault(&self) -> &Vault;
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
    /// The descriptor's `id`: the package's name, `std` or `search`.
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

/// The modules visible in every source unit without an import, prelude or
/// not: `core`, the types every other module is written in, and
/// `collections`, the operations over its collection types, which mention
/// nothing above the core's own types and so are never absent.
pub(crate) const ALWAYS: &[&str] = &["core", "collections"];

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

/// The search package's extension: retrieval over a vault's cards.
pub(crate) static SEARCH: Extension = Extension {
    id: "search",
    abi: ABI,
    origin: Origin::Internal,
    version: env!("CARGO_PKG_VERSION"),
    provides: &[
        Provided {
            module: "lexical",
            steps: lexical::STEPS,
        },
        Provided {
            module: "semantic",
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
pub(crate) static REGISTERED: &[&Extension] = &[&STD, &SEARCH];

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
    PRELUDE.get_or_init(|| {
        crate::library::PRELUDE
            .lines()
            .filter_map(|line| line.trim().strip_prefix("import "))
            .map(|name| name.split_whitespace().next().unwrap_or(name))
            .collect()
    })
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

/// What a vault's `hql.toml` says about extensions.
///
/// The prelude is a compatibility convenience, and a vault that wants every
/// capability stated in the program that uses it must be able to decline it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Config {
    /// What is imported for every program this vault runs: builtin modules
    /// by name, and packages by their id, which imports every module the
    /// package provides.
    pub imports: Vec<String>,
    /// Whether the prelude's modules arrive without being asked for.
    pub prelude: bool,
    /// The module roots workspace modules are discovered under: `[modules]
    /// roots` in `hql.toml`, resolved against the vault, and `--modules` on
    /// the command line.
    pub module_roots: Vec<std::path::PathBuf>,
    /// The crates of packages under development: `[packages] paths` in
    /// `hql.toml`, resolved against the vault. Each is built on first use
    /// and loaded as an installed package is.
    pub package_paths: Vec<std::path::PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            prelude: true,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
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

/// Whether a module is available to a program, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// `core` and `collections`, which are never imported because they are
    /// never absent.
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
    /// Each visible builtin module, under the local name it is bound to.
    imported: Vec<Imported>,
}

/// A builtin module bound into a source unit under a local name: its last
/// segment, or an alias.
#[derive(Debug, Clone, Copy)]
struct Imported {
    local: &'static str,
    module: binding::ModuleId,
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
    /// The always-visible modules, plus the prelude's unless the vault
    /// declines it, over a frozen environment.
    pub(crate) fn bare(environment: binding::Frozen, prelude: bool) -> Self {
        let mut resolution = Self {
            environment,
            imported: Vec::new(),
        };
        let names = ALWAYS
            .iter()
            .copied()
            .chain(prelude.then(self::prelude).into_iter().flatten().copied());
        for name in names {
            if let Some(module) = environment.provided_module(name)
                && !resolution.imported.iter().any(|held| held.module == module)
            {
                resolution.imported.push(Imported {
                    local: environment.module(module).local(),
                    module,
                });
            }
        }
        resolution
    }

    /// What a vault starts every program with: the configured layer of the
    /// process's runtime environment, resolved once per configuration.
    ///
    /// A configured import fails the same way a written one does, at a span
    /// pointing at the program's start, because the fault is in the vault
    /// rather than in the line a reader is looking at.
    pub(crate) fn new(config: &Config) -> Result<Self, Diagnostic> {
        binding::Runtime::process(config).map(|runtime| runtime.resolution.clone())
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
        Ok(true)
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

    /// The operation of a name an imported module offers an importer: one
    /// the module declares, and does not declare private.
    fn offered(&self, held: &Imported, name: &str) -> Option<&'static Operation> {
        self.environment
            .offered(held.module, name)
            .filter(|step| !self.environment.is_private(step))
    }

    /// The canonical name of an imported module, for messages.
    fn name_of(&self, held: &Imported) -> &'static str {
        self.environment.module(held.module).name
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
        named: &[&str],
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        self.step_among(name, input, positional, named, span, false)
    }

    /// The step a receiver call, `value.f()`, resolves to: as
    /// [`Resolution::step_for`], among the visible operations whose
    /// declarations take `self`. A name declared only without `self` is
    /// reached by a call or a pipeline step, and the refusal says so.
    pub(crate) fn receiver_step_for(
        &self,
        name: &str,
        input: &TypeRef,
        positional: usize,
        named: &[&str],
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        let visible = self
            .imported
            .iter()
            .filter_map(|held| self.offered(held, name))
            .collect::<Vec<_>>();
        if !visible.is_empty()
            && !visible
                .iter()
                .any(|step| self.environment.has_receiver(step))
        {
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` takes no receiver: its declaration has no `self`, so \
                     `value.{name}(…)` does not reach it; write `{name}(value, …)` or \
                     `value | {name}(…)`"
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
        let mut candidates: Vec<(&Imported, &'static Operation)> = Vec::new();
        for held in &self.imported {
            if let Some(step) = self.offered(held, name)
                && (!receivers || self.environment.has_receiver(step))
                && !candidates
                    .iter()
                    .any(|(candidate, _)| candidate.module == held.module)
            {
                candidates.push((held, step));
            }
        }
        let environment = self.environment;
        match candidates.as_slice() {
            [] => Err(self.unresolved(name, span)),
            [(_, only)] => Ok(only),
            many => {
                // The names a call writes tell modules apart before its
                // input does: only one module's `join` takes `into`.
                if !named.is_empty() {
                    let naming: Vec<_> = many
                        .iter()
                        .filter(|(_, step)| {
                            environment.specs(step).iter().any(|spec| {
                                named.iter().all(|written| {
                                    spec.arguments
                                        .iter()
                                        .any(|held| held.named && held.name == *written)
                                })
                            })
                        })
                        .collect();
                    if let [(_, only)] = naming.as_slice() {
                        return Ok(only);
                    }
                }
                let input = input.present();
                let fitting: Vec<_> = many
                    .iter()
                    .filter(|(_, step)| {
                        environment.specs(step).iter().any(|spec| {
                            spec.input.accepts(input)
                                || (input.constructor.0 == "Option"
                                    && spec.input.accepts(input.present()))
                        })
                    })
                    .collect();
                let by_arity: Vec<_> = fitting
                    .iter()
                    .filter(|(_, step)| {
                        environment.specs(step).iter().any(|spec| {
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
                    ([(_, step)], _) => Ok(step),
                    ([], [(_, step)]) => Ok(step),
                    ([], []) => Ok(many[0].1),
                    (by_arity, fitting) => {
                        // Several fit: by arity where any does, else by input.
                        let several: Vec<&(&Imported, &'static Operation)> = if by_arity.is_empty()
                        {
                            fitting.to_vec()
                        } else {
                            by_arity.iter().map(|held| **held).collect()
                        };
                        // The modules by their canonical names; the
                        // qualification by the local name the unit bound.
                        Err(Diagnostic::name(
                            span.clone(),
                            format!(
                                "`{name}` over {input} is declared by {}, so the call is \
                             ambiguous; qualify it, `{}.{name}(…)`",
                                several
                                    .iter()
                                    .map(|(held, _)| format!("`{}`", self.name_of(held)))
                                    .collect::<Vec<_>>()
                                    .join(" and "),
                                several[0].0.local
                            ),
                        ))
                    }
                }
            }
        }
    }

    /// The visible modules whose receiver function `name` takes an input of
    /// this type, by local name: the providers a receiver call could reach.
    pub(crate) fn receiver_providers(&self, name: &str, input: &TypeRef) -> Vec<&'static str> {
        let input = input.present();
        let mut found: Vec<&Imported> = Vec::new();
        for held in &self.imported {
            if let Some(step) = self.offered(held, name)
                && self.environment.has_receiver(step)
                && self
                    .environment
                    .specs(step)
                    .iter()
                    .any(|spec| spec.input.accepts(input))
                && !found.iter().any(|known| known.module == held.module)
            {
                found.push(held);
            }
        }
        found.into_iter().map(|held| held.local).collect()
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
            .find_map(|held| self.offered(held, name))
        {
            return Ok(step);
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
        if let Some(step) = self.imported.iter().find_map(|held| {
            self.environment
                .offered(held.module, name)
                .filter(|step| self.environment.is_private(step))
        }) {
            return self.private(name, step, span);
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
            [only] => format!(
                "`{name}` is declared in the module `{only}`: write `import {}`",
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
    ALWAYS.contains(&module) || prelude().contains(&module)
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
    /// Whether a program in this vault has the module without asking.
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
                availability: if ALWAYS.contains(&provided.module) {
                    Availability::Core
                } else if config.prelude && prelude().contains(&provided.module) {
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
                    && matches!(entry.name, "entries" | "walk" | "glob" | "exists" | "entry"))
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
                || entry.module == "io.env";
            let expected = if effectful {
                Purity::Effectful
            } else {
                Purity::Pure
            };
            assert_eq!(entry.purity, expected, "{}", entry.name);
            assert!(
                matches!(entry.extension, "std" | "search"),
                "{}",
                entry.name
            );
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
        assert_eq!((lexical.module, lexical.extension), ("lexical", "search"));
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
            [("std", ABI), ("search", ABI)]
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
        assert_eq!(by_module("core").availability, Availability::Core);
        assert_eq!(by_module("collections").availability, Availability::Core);
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
        let bare = bare(false);
        let missing = bare.step("markdown", &(0..1)).unwrap_err();
        assert!(
            missing.message().contains("import format"),
            "{}",
            missing.message()
        );
        assert!(bare.step("count", &(0..1)).is_ok());
    }

    #[test]
    fn importing_the_same_module_twice_is_not_a_collision() {
        let mut resolution = bare(true);
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
            package_paths: Vec::new(),
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
        assert_eq!(by_name("core"), Availability::Core);
        assert_eq!(by_name("collections"), Availability::Core);
        assert_eq!(by_name("format"), Availability::Absent);
        assert_eq!(by_name("lexical"), Availability::Configured);
        assert_eq!(by_name("semantic"), Availability::Absent);
        assert_eq!(by_name("graph"), Availability::Absent);

        let kept = catalogue(&Config::default());
        let graph = kept.iter().find(|provider| provider.module == "graph");
        assert_eq!(
            graph.map(|provider| provider.availability),
            Some(Availability::Prelude)
        );
    }

    #[test]
    fn a_vault_may_import_a_package_which_imports_every_module_it_provides() {
        let config = Config {
            imports: vec!["search".to_owned(), "logging".to_owned()],
            prelude: true,
            module_roots: Vec::new(),
            package_paths: Vec::new(),
        };
        let resolution = Resolution::new(&config).expect("the configuration resolves");
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
        assert_eq!(by_name("lexical"), Availability::Configured);
        assert_eq!(by_name("semantic"), Availability::Configured);
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
        };
        let failed = Resolution::new(&config).unwrap_err();
        assert!(
            failed.message().contains("hql.toml"),
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
            ("lexical", "write `import lexical`"),
            ("http", "write `import net`"),
            ("Logger", "write `import logging`"),
            ("walk", "write `import io.fs`"),
            ("mean", "write `import stats`"),
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
