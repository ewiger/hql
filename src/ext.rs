//! What a package's crate implements an extension against (HQL-0057).
//!
//! A package is a crate built as a library `hql` loads. Its `modules/` root
//! declares functions without bodies; its Rust supplies one [`Operation`] per
//! declared name, collects them per module in a [`Provided`], and states the
//! whole in an [`Extension`] that [`extension!`](crate::extension) exports
//! under the one symbol the loader calls, `hql_extension_v1`.
//!
//! ```ignore
//! use hql::ext::{self, Diagnostic, EvalCx, Operation, Span, TypedArg, Value};
//!
//! fn shout(
//!     _cx: &mut dyn EvalCx,
//!     input: Value,
//!     _arguments: &[TypedArg],
//!     span: Span,
//! ) -> Result<Value, Diagnostic> {
//!     match input {
//!         Value::Str(text) => Ok(Value::Str(text.to_uppercase().into())),
//!         _ => Err(ext::failure(span, "`shout` takes a string")),
//!     }
//! }
//!
//! static STEPS: &[Operation] = &[hql::native!("shout", shout)];
//!
//! hql::extension!(ext::extension(
//!     "hello",
//!     env!("CARGO_PKG_VERSION"),
//!     &[ext::provided("hello", STEPS)],
//! ));
//! ```
//!
//! The library holds its own compiled copy of this crate. The two copies
//! exchange [`Value`], [`Diagnostic`] and `&mut dyn EvalCx` as plain Rust
//! types, which are the same types only when both copies come from one `hql`
//! version, one set of dependency versions and one compiler: the [`Header`]
//! states those, and the loader compares them before it reads anything else.
//! A function of the library reaches the running program only through its
//! [`EvalCx`]; the statics of its own copy of this crate are not the
//! process's.

use std::ops::Range;
use std::sync::Arc;

pub use crate::diagnostics::Diagnostic;
pub use crate::execution::Arg as TypedArg;
pub use crate::extensions::{Eval, EvalCx, Extension, Operation, Provided, Purity, Shape};
pub use crate::types::{Record, TypeConstructor, TypeRef, Value};

use crate::extensions::{Origin, Signature};

/// A place in the program's source: what a diagnostic points at.
pub type Span = Range<usize>;

/// The extension ABI of this crate: `1`, an in-process library exchanging
/// the Rust types of one `hql` build.
pub const ABI: u32 = crate::extensions::ABI;

/// The version of this build of `hql`.
pub const HQL: &str = env!("CARGO_PKG_VERSION");

/// The compiler this build of `hql` was compiled with, `1.95.0 (59807616e
/// 2026-04-14)`.
pub const RUSTC: &str = env!("HQL_RUSTC");

/// The target this build of `hql` was compiled for, `aarch64-apple-darwin`.
pub const TARGET: &str = env!("HQL_TARGET");

/// The symbol a package's library exports.
pub const ENTRY: &str = "hql_extension_v1";

/// An operation implementing the declarations of one name in a module of
/// the package: pure and a barrier, as a declaration written `fn` is.
/// Binding checks both against the declaration, so one written
/// `effectful fn` takes [`Operation::effectful`] and one written with a
/// shape takes [`Operation::shaped`].
#[must_use]
pub const fn operation(name: &'static str, eval: Eval) -> Operation {
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
    /// The same operation marked effectful, for one its declaration writes
    /// `effectful fn`.
    #[must_use]
    pub const fn effectful(self) -> Self {
        Self {
            purity: Purity::Effectful,
            ..self
        }
    }

    /// The name the operation implements.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

/// One module of the package, by its canonical name, with the operations
/// implementing its declarations.
#[must_use]
pub const fn provided(module: &'static str, steps: &'static [Operation]) -> Provided {
    Provided { module, steps }
}

/// The extension of a package: its name, its crate's version, and the
/// modules it provides.
#[must_use]
pub const fn extension(
    id: &'static str,
    version: &'static str,
    provides: &'static [Provided],
) -> Extension {
    Extension {
        id,
        abi: ABI,
        origin: Origin::External,
        version,
        provides,
        metadata: None,
    }
}

impl Extension {
    /// The package's name.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// The version the extension reports.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// The ABI the extension states.
    #[must_use]
    pub const fn abi(&self) -> u32 {
        self.abi
    }

    /// The modules the extension provides, in the order written.
    pub fn modules(&self) -> impl Iterator<Item = &'static str> {
        self.provides.iter().map(|provided| provided.module)
    }
}

/// A failure of the function while the program runs, pointing at the call.
#[must_use]
pub fn failure(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span, message)
}

/// The argument written with a name, `target = …`.
#[must_use]
pub fn named<'a>(arguments: &'a [TypedArg], name: &str) -> Option<&'a TypedArg> {
    crate::extensions::named(arguments, name)
}

/// The argument at a position among those written without a name, counted
/// from zero. The call's input is not among them.
#[must_use]
pub fn positional(arguments: &[TypedArg], position: usize) -> Option<&TypedArg> {
    arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .nth(position)
}

/// The value of an argument.
///
/// # Errors
///
/// Returns the failure evaluating the argument's expression met.
pub fn value_of(cx: &mut dyn EvalCx, argument: &TypedArg) -> Result<Value, Diagnostic> {
    cx.evaluate(&argument.value)
}

/// The type a module of the package declares under a name, applied to its
/// arguments: `ty("Dataset", [])`.
#[must_use]
pub fn ty(name: &'static str, arguments: impl IntoIterator<Item = TypeRef>) -> TypeRef {
    TypeConstructor(name).apply(arguments)
}

/// A value of a record type a module of the package declares: its fields in
/// declaration order, an optional field holding an `Option` value.
#[must_use]
pub fn record(ty: TypeRef, fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(Arc::new(Record {
        ty,
        fields: fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        carrier: None,
    }))
}

/// Run a function of the package, turning a panic into a failure of the
/// call. A panic must not leave the library: the frames above it are the
/// running `hql`'s. [`native!`](crate::native) wraps each function in this.
///
/// # Errors
///
/// Returns the function's own failure, or one naming the function where it
/// panicked.
pub fn guard(
    name: &str,
    span: Span,
    run: impl FnOnce() -> Result<Value, Diagnostic>,
) -> Result<Value, Diagnostic> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(result) => result,
        Err(payload) => {
            let said = payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "no message".to_owned());
            Err(failure(
                span,
                format!("the extension function `{name}` panicked: {said}"),
            ))
        }
    }
}

/// How many layout facts a [`Header`] states.
pub const LAYOUT: usize = 6;

/// The sizes of the types the two copies of this crate exchange, as this
/// copy was compiled: a check on what the versions alone cannot show, a
/// dependency compiled with other features.
#[must_use]
pub fn layout() -> [usize; LAYOUT] {
    [
        size_of::<Value>(),
        size_of::<Diagnostic>(),
        size_of::<crate::diagnostics::Context>(),
        size_of::<TypeRef>(),
        size_of::<Operation>(),
        size_of::<serde_json::Value>(),
    ]
}

/// What `hql_extension_v1` returns: what the library was built against, in
/// a layout that does not depend on it, and the extension, which does.
///
/// The loader reads `abi`, `hql`, `rustc` and `layout` and compares them with
/// its own. Only where all four agree does it read `extension`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    /// [`ABI`] of the library's copy of this crate.
    pub abi: u32,
    /// [`HQL`] of the library's copy, as UTF-8 bytes and their count.
    pub hql: *const u8,
    pub hql_len: usize,
    /// [`RUSTC`] of the library's copy, as UTF-8 bytes and their count.
    pub rustc: *const u8,
    pub rustc_len: usize,
    /// [`layout`] of the library's copy.
    pub layout: [usize; LAYOUT],
    /// The extension the library carries.
    pub extension: *const Extension,
}

impl Header {
    /// The header of an extension, stating this copy of the crate.
    #[must_use]
    pub fn of(extension: &'static Extension) -> Self {
        Self {
            abi: ABI,
            hql: HQL.as_ptr(),
            hql_len: HQL.len(),
            rustc: RUSTC.as_ptr(),
            rustc_len: RUSTC.len(),
            layout: layout(),
            extension: std::ptr::from_ref(extension),
        }
    }
}

/// An [`Operation`] over a function of the package, with the function run
/// under [`guard`](crate::ext::guard): `native!("shout", shout)`.
#[macro_export]
macro_rules! native {
    ($name:expr, $function:path) => {{
        fn guarded(
            cx: &mut dyn $crate::ext::EvalCx,
            input: $crate::ext::Value,
            arguments: &[$crate::ext::TypedArg],
            span: $crate::ext::Span,
        ) -> ::core::result::Result<$crate::ext::Value, $crate::ext::Diagnostic> {
            $crate::ext::guard($name, span.clone(), move || {
                $function(cx, input, arguments, span)
            })
        }
        $crate::ext::operation($name, guarded)
    }};
}

/// Export a package's extension under the symbol the loader calls.
///
/// Written once in the crate, over the [`Extension`](crate::ext::Extension)
/// [`ext::extension`](crate::ext::extension) builds. It is the crate's one
/// use of `unsafe`: an exported symbol is an `unsafe` attribute.
#[macro_export]
macro_rules! extension {
    ($descriptor:expr) => {
        #[doc(hidden)]
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn hql_extension_v1() -> $crate::ext::Header {
            static EXTENSION: $crate::ext::Extension = $descriptor;
            $crate::ext::Header::of(&EXTENSION)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fails(
        _cx: &mut dyn EvalCx,
        _input: Value,
        _arguments: &[TypedArg],
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        panic!("out of range")
    }

    static STEPS: &[Operation] = &[crate::native!("fails", fails)];
    static EXTENSION: Extension = extension("sample", "0.1.0", &[provided("sample", STEPS)]);

    #[test]
    fn a_header_states_the_build_of_this_crate() {
        let header = Header::of(&EXTENSION);
        assert_eq!(header.abi, 1);
        assert_eq!(header.hql_len, env!("CARGO_PKG_VERSION").len());
        assert_eq!(header.rustc_len, RUSTC.len());
        assert_ne!(RUSTC, "unknown");
        assert_eq!(header.layout, layout());
        assert!(std::ptr::eq(header.extension, &raw const EXTENSION));
    }

    #[test]
    fn an_external_extension_states_its_modules() {
        assert_eq!(EXTENSION.id(), "sample");
        assert_eq!(EXTENSION.origin, Origin::External);
        assert_eq!(EXTENSION.modules().collect::<Vec<_>>(), ["sample"]);
        assert_eq!(STEPS[0].name(), "fails");
    }

    #[test]
    fn a_panic_becomes_a_failure_of_the_call() {
        let failed = guard("fails", 3..7, || panic!("out of range")).unwrap_err();
        assert_eq!(failed.span(), 3..7);
        assert!(
            failed.message().contains("`fails` panicked: out of range"),
            "{}",
            failed.message()
        );
    }
}
