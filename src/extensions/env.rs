//! The module `io.env` (HQL-0041 part 5): environment variables and the
//! invocation's named arguments.
//!
//! The environment is read from the process once, when a program first
//! reads it, and held for the rest of the process: no code of the crate
//! writes it, so every evaluation sees the same environment. What a program
//! reads of it is filtered by the grant: every variable a standalone run, the
//! names `env = [...]` lists where a workspace's `hql.toml` states it, and
//! none under `hql render`, whose host policy is the document's. A variable
//! whose name or value is not UTF-8 is not held, so it reads as unset, as a
//! withheld one does. The arguments are the `--arg name=value` pairs the
//! host fixed before evaluation, always granted.

use super::{EvalCx, Operation, Purity, Shape, Signature};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::io::Policy;
use crate::types::{MapKind, MapValue, TypeRef, Value};
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// An operation that reads the invocation.
const fn reading(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: Shape::Barrier,
        check: None,
        eval,
    }
}

/// Every operation `io.env` provides.
pub(crate) static STEPS: &[Operation] = &[
    reading("var", eval_var),
    reading("vars", eval_vars),
    reading("arg", eval_arg),
    reading("args", eval_args),
];

/// Which environment variables the host grants a program.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Grant {
    /// Every variable: a program run on its own.
    #[default]
    All,
    /// The variables a pattern matches: a name, or a prefix and `*`.
    Only(Vec<String>),
}

impl Grant {
    /// Whether the grant holds the variable `name`.
    #[must_use]
    pub fn holds(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(patterns) => patterns.iter().any(|pattern| {
                pattern
                    .strip_suffix('*')
                    .map_or(pattern == name, |prefix| name.starts_with(prefix))
            }),
        }
    }
}

/// What the host fixed before evaluation: the grant and the arguments.
#[derive(Debug, Clone, Default)]
struct Invocation {
    grant: Grant,
    arguments: BTreeMap<String, String>,
}

static INVOCATION: Mutex<Option<Invocation>> = Mutex::new(None);

/// Fix the grant and the arguments every later evaluation reads.
pub fn set_invocation(grant: Grant, arguments: Vec<(String, String)>) {
    *INVOCATION.lock().unwrap_or_else(PoisonError::into_inner) = Some(Invocation {
        grant,
        arguments: arguments.into_iter().collect(),
    });
}

fn invocation() -> Invocation {
    INVOCATION
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_default()
}

/// The process environment, its UTF-8 variables, read once.
fn environment() -> &'static BTreeMap<String, String> {
    static ENVIRONMENT: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    ENVIRONMENT.get_or_init(|| {
        std::env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .collect()
    })
}

/// The variables a program may read now: none under `hql render`, else the
/// granted ones that are set.
fn granted() -> BTreeMap<String, String> {
    if matches!(crate::io::policy(), Policy::Document { .. }) {
        return BTreeMap::new();
    }
    let grant = invocation().grant;
    environment()
        .iter()
        .filter(|(name, _)| grant.holds(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// Whether `name` is an argument's name: `[a-z_][a-z0-9_]*`.
#[must_use]
pub fn is_argument_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && characters.all(|rest| rest.is_ascii_lowercase() || rest.is_ascii_digit() || rest == '_')
}

/// Read one `--arg name=value`.
///
/// # Errors
///
/// The text has no `=`, or its name is not `[a-z_][a-z0-9_]*`.
pub fn argument(text: &str) -> std::result::Result<(String, String), String> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| format!("{text:?} is not name=value"))?;
    if !is_argument_name(name) {
        return Err(format!(
            "{name:?} is not an argument name: a lowercase letter or `_`, then lowercase \
             letters, digits and `_`"
        ));
    }
    Ok((name.to_owned(), value.to_owned()))
}

/// The first name given twice among the arguments, if any.
#[must_use]
pub fn repeated(arguments: &[(String, String)]) -> Option<&str> {
    arguments.iter().enumerate().find_map(|(at, (name, _))| {
        arguments[..at]
            .iter()
            .any(|(earlier, _)| earlier == name)
            .then_some(name.as_str())
    })
}

type Result<T> = std::result::Result<T, Diagnostic>;

fn text(value: &str) -> Value {
    Value::Str(Arc::from(value))
}

fn optional(value: Option<&String>) -> Value {
    value.map_or(Value::Absent(TypeRef::STR), |value| {
        Value::Present(Arc::new(text(value)))
    })
}

fn map(entries: &BTreeMap<String, String>, span: &Range<usize>) -> Result<Value> {
    let (names, values) = entries
        .iter()
        .map(|(name, value)| (text(name), text(value)))
        .unzip();
    MapValue::new(
        MapKind::Unordered,
        names,
        values,
        TypeRef::STR,
        TypeRef::STR,
    )
    .map(|map| Value::Map(Arc::new(map)))
    .map_err(|error| Diagnostic::runtime(span.clone(), error.to_string()))
}

fn name_of(input: &Value, step: &str, span: &Range<usize>) -> Result<String> {
    match input {
        Value::Str(name) => Ok(name.to_string()),
        _ => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{step}` needs a String name"),
        )),
    }
}

fn eval_var(_: &mut dyn EvalCx, input: Value, _: &[TypedArg], span: Range<usize>) -> Result<Value> {
    let name = name_of(&input, "var", &span)?;
    Ok(optional(granted().get(&name)))
}

fn eval_vars(_: &mut dyn EvalCx, _: Value, _: &[TypedArg], span: Range<usize>) -> Result<Value> {
    map(&granted(), &span)
}

fn eval_arg(_: &mut dyn EvalCx, input: Value, _: &[TypedArg], span: Range<usize>) -> Result<Value> {
    let name = name_of(&input, "arg", &span)?;
    Ok(optional(invocation().arguments.get(&name)))
}

fn eval_args(_: &mut dyn EvalCx, _: Value, _: &[TypedArg], span: Range<usize>) -> Result<Value> {
    map(&invocation().arguments, &span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_grants_a_name_or_a_prefix() {
        let grant = Grant::Only(vec!["HOME".to_owned(), "APP_*".to_owned()]);
        assert!(grant.holds("HOME"));
        assert!(!grant.holds("HOMEPATH"));
        assert!(grant.holds("APP_TOKEN"));
        assert!(grant.holds("APP_"));
        assert!(!grant.holds("APP"));
        assert!(!grant.holds("PATH"));
        assert!(Grant::All.holds("PATH"));
        assert!(!Grant::Only(Vec::new()).holds("PATH"));
    }

    #[test]
    fn an_argument_is_a_lowercase_name_and_a_value() {
        assert_eq!(
            argument("species=owl"),
            Ok(("species".to_owned(), "owl".to_owned()))
        );
        assert_eq!(
            argument("_x1=a=b"),
            Ok(("_x1".to_owned(), "a=b".to_owned()))
        );
        assert_eq!(argument("empty="), Ok(("empty".to_owned(), String::new())));
        for refused in ["Species=owl", "1st=a", "=a", "no-dash=a", "plain", "é=a"] {
            assert!(argument(refused).is_err(), "{refused}");
        }
        let given = vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "2".to_owned()),
            ("a".to_owned(), "3".to_owned()),
        ];
        assert_eq!(repeated(&given), Some("a"));
        assert_eq!(repeated(&given[..2]), None);
    }
}
