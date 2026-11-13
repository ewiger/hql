//! The `crypto` module (HQL-0053 parts 2 to 4): the natives behind
//! `lib/std/crypto/mod.hql`.
//!
//! An algorithm and a key format are type arguments, `Pem` and `Ed25519`,
//! read from each argument's checked type. A key is a [`Value::Key`]; a key
//! pair is a `KeyPair<A>` record of its two keys. Every error names the
//! format and the algorithm and never quotes the key text.

use super::{CheckCx, EvalCx, Operation, Purity, declared};
use crate::crypto::keys::{self, ReadError};
use crate::crypto::{Algorithm, Key, KeyFormat, PrivateKey, PublicKey, SecretAlgorithm, SecretKey};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeConstructor, TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Diagnostic>;

/// An operation that reads the host: a file or the host generator.
const fn effectful(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        purity: Purity::Effectful,
        ..declared(name, eval)
    }
}

/// An operation whose key format is held to its algorithm at check time.
const fn formatted(operation: Operation) -> Operation {
    Operation {
        check: Some(check_key_format),
        ..operation
    }
}

/// Every step the `crypto` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    formatted(declared("PublicKey", eval_public_key)),
    formatted(declared("to_public_key", eval_to_public_key)),
    formatted(declared("PrivateKey", eval_private_key)),
    formatted(declared("to_private_key", eval_to_private_key)),
    declared("SecretKey", eval_secret_key),
    formatted(effectful("read_public_key", eval_read_public_key)),
    formatted(effectful("read_private_key", eval_read_private_key)),
    formatted(declared("to_text", eval_to_text)),
    declared("to_der", eval_to_der),
    formatted(declared("export_private", eval_export_private)),
    declared("public", eval_public),
    declared("thumbprint", eval_thumbprint),
    effectful("generate_key_pair", eval_generate_key_pair),
    effectful("generate_secret_key", eval_generate_secret_key),
];

/// `F` and `A` of a call that names both: a format that holds no key of
/// the algorithm is refused before anything runs.
fn check_key_format(
    _: &mut dyn CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<()> {
    let format = call
        .bindings
        .get("F")
        .and_then(|ty| KeyFormat::named(ty.constructor.0));
    let algorithm = call
        .bindings
        .get("A")
        .and_then(|ty| Algorithm::named(ty.constructor.0));
    if let (Some(format), Some(algorithm)) = (format, algorithm)
        && !format.holds(algorithm)
    {
        return Err(Diagnostic::typing(
            span,
            format!(
                "{} text holds no {} key: {}",
                format.name(),
                algorithm.name(),
                if format == KeyFormat::Age {
                    "`Age` holds `X25519` keys alone"
                } else {
                    "an `X25519` key has no `OpenSsh` text; use `Pem`, `Jwk` or `Age`"
                }
            ),
        ));
    }
    Ok(())
}

// The values.

fn fail(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message)
}

/// The type of a key of `algorithm`: `PublicKey<Ed25519>`.
pub(crate) fn key_type(kind: &'static str, algorithm: &'static str) -> TypeRef {
    TypeConstructor(kind).apply([TypeConstructor(algorithm).apply([])])
}

fn public_value(key: PublicKey) -> Value {
    Value::Key(Arc::new(Key::Public(key)))
}

fn private_value(key: PrivateKey) -> Value {
    Value::Key(Arc::new(Key::Private(key)))
}

/// A `KeyPair<A>` record of a private key and its public key.
fn pair_value(key: PrivateKey) -> Value {
    let algorithm = key.algorithm().name();
    let public = public_value(key.public());
    Value::Record(Arc::new(crate::types::Record {
        ty: key_type("KeyPair", algorithm),
        fields: vec![
            ("public".to_owned(), public),
            ("private".to_owned(), private_value(key)),
        ],
        carrier: None,
    }))
}

/// The key a value holds.
fn key_of<'a>(value: &'a Value, name: &str, span: &Range<usize>) -> Result<&'a Key> {
    match value {
        Value::Key(key) => Ok(key),
        Value::Present(inner) => key_of(inner, name, span),
        other => Err(fail(
            span,
            format!("`{name}` needs a key, not {}", other.type_of()),
        )),
    }
}

pub(crate) fn public_of<'a>(
    value: &'a Value,
    name: &str,
    span: &Range<usize>,
) -> Result<&'a PublicKey> {
    match key_of(value, name, span)? {
        Key::Public(key) => Ok(key),
        _ => Err(fail(span, format!("`{name}` needs a PublicKey"))),
    }
}

pub(crate) fn private_of<'a>(
    value: &'a Value,
    name: &str,
    span: &Range<usize>,
) -> Result<&'a PrivateKey> {
    match key_of(value, name, span)? {
        Key::Private(key) => Ok(key),
        _ => Err(fail(span, format!("`{name}` needs a PrivateKey"))),
    }
}

#[allow(
    dead_code,
    reason = "the MACs and ciphers of HQL-0053 parts 3 and 4 read it"
)]
pub(crate) fn secret_of<'a>(
    value: &'a Value,
    name: &str,
    span: &Range<usize>,
) -> Result<&'a SecretKey> {
    match key_of(value, name, span)? {
        Key::Secret(key) => Ok(key),
        _ => Err(fail(span, format!("`{name}` needs a SecretKey"))),
    }
}

// Arguments.

/// The arguments written without a name, in order.
fn positional(arguments: &[TypedArg]) -> Vec<&TypedArg> {
    arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .collect()
}

/// The constructor a type argument names.
fn named_type(argument: &TypedArg) -> &'static str {
    argument.value.ty.constructor.0
}

fn format_of(argument: Option<&&TypedArg>, name: &str, span: &Range<usize>) -> Result<KeyFormat> {
    argument
        .and_then(|argument| KeyFormat::named(named_type(argument)))
        .ok_or_else(|| {
            fail(
                span,
                format!("`{name}` needs a key format: `Pem`, `Jwk`, `OpenSsh` or `Age`"),
            )
        })
}

fn algorithm_of(
    argument: Option<&&TypedArg>,
    name: &str,
    span: &Range<usize>,
) -> Result<Algorithm> {
    argument
        .and_then(|argument| Algorithm::named(named_type(argument)))
        .ok_or_else(|| {
            fail(
                span,
                format!("`{name}` needs an algorithm: `Ed25519`, `P256`, `Rsa` or `X25519`"),
            )
        })
}

fn secret_algorithm_of(
    argument: Option<&&TypedArg>,
    name: &str,
    span: &Range<usize>,
) -> Result<SecretAlgorithm> {
    argument
        .and_then(|argument| SecretAlgorithm::named(named_type(argument)))
        .ok_or_else(|| {
            fail(span, format!("`{name}` needs an algorithm: `Aes256Gcm`, `ChaCha20Poly1305`, `HmacSha256` or `HmacSha512`"))
        })
}

/// The text of a `String` value.
pub(crate) fn text_of(value: &Value, name: &str, span: &Range<usize>) -> Result<String> {
    super::bytes::input_text(value, name, span)
}

/// A named `String` argument, or `""`.
fn named_text(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    key: &str,
    span: &Range<usize>,
) -> Result<Zeroizing<String>> {
    match super::named(arguments, key) {
        Some(argument) => {
            let value = cx.evaluate(&argument.value)?;
            text_of(&value, key, span).map(Zeroizing::new)
        }
        None => Ok(Zeroizing::new(String::new())),
    }
}

/// The input of a constructor: what was piped, else its first positional
/// argument, with the positional arguments after it.
fn constructor_input<'a>(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &'a [TypedArg],
) -> Result<(Value, Vec<&'a TypedArg>)> {
    let mut rest = positional(arguments);
    if !matches!(input, Value::Unit) {
        return Ok((input, rest));
    }
    if rest.is_empty() {
        return Ok((Value::Unit, rest));
    }
    let first = rest.remove(0);
    Ok((cx.evaluate(&first.value)?, rest))
}

fn read_error(error: &ReadError, span: &Range<usize>) -> Diagnostic {
    fail(span, error.message().to_owned())
}

// Keys.

fn eval_public_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (first, rest) = constructor_input(cx, input, arguments)?;
    if let Value::Bytes(der) = &first {
        let algorithm = algorithm_of(rest.first(), "PublicKey", &span)?;
        return keys::public_from_der(der, algorithm)
            .map(public_value)
            .map_err(|message| fail(&span, message));
    }
    let text = text_of(&first, "PublicKey", &span)?;
    let format = format_of(rest.first(), "PublicKey", &span)?;
    let algorithm = algorithm_of(rest.get(1), "PublicKey", &span)?;
    keys::read_public(&text, format, algorithm)
        .map(public_value)
        .map_err(|message| fail(&span, message))
}

fn eval_to_public_key(
    _cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let text = text_of(&input, "to_public_key", &span)?;
    let rest = positional(arguments);
    let format = format_of(rest.first(), "to_public_key", &span)?;
    let algorithm = algorithm_of(rest.get(1), "to_public_key", &span)?;
    Ok(match keys::read_public(&text, format, algorithm) {
        Ok(key) => Value::Present(Arc::new(public_value(key))),
        Err(_) => Value::Absent(key_type("PublicKey", algorithm.name())),
    })
}

fn eval_private_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (first, rest) = constructor_input(cx, input, arguments)?;
    if let Value::Bytes(der) = &first {
        let algorithm = algorithm_of(rest.first(), "PrivateKey", &span)?;
        return keys::private_from_der(der, algorithm)
            .map(private_value)
            .map_err(|message| fail(&span, message));
    }
    let text = Zeroizing::new(text_of(&first, "PrivateKey", &span)?);
    let format = format_of(rest.first(), "PrivateKey", &span)?;
    let algorithm = algorithm_of(rest.get(1), "PrivateKey", &span)?;
    let passphrase = named_text(cx, arguments, "passphrase", &span)?;
    keys::read_private(&text, format, algorithm, &passphrase)
        .map(private_value)
        .map_err(|error| read_error(&error, &span))
}

fn eval_to_private_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let text = Zeroizing::new(text_of(&input, "to_private_key", &span)?);
    let rest = positional(arguments);
    let format = format_of(rest.first(), "to_private_key", &span)?;
    let algorithm = algorithm_of(rest.get(1), "to_private_key", &span)?;
    let passphrase = named_text(cx, arguments, "passphrase", &span)?;
    Ok(
        match keys::read_private(&text, format, algorithm, &passphrase) {
            Ok(key) => Value::Present(Arc::new(private_value(key))),
            Err(_) => Value::Absent(key_type("PrivateKey", algorithm.name())),
        },
    )
}

fn eval_secret_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (first, rest) = constructor_input(cx, input, arguments)?;
    let octets = super::bytes::input_octets(&first, "SecretKey", &span)?;
    let algorithm = secret_algorithm_of(rest.first(), "SecretKey", &span)?;
    let length = octets.len();
    if algorithm.is_cipher() && length != 32 {
        return Err(fail(
            &span,
            format!(
                "a {} key is 32 octets, and this one is {length}",
                algorithm.name()
            ),
        ));
    }
    if length == 0 {
        return Err(fail(
            &span,
            format!("a {} key holds at least one octet", algorithm.name()),
        ));
    }
    Ok(Value::Key(Arc::new(Key::Secret(SecretKey {
        algorithm,
        octets: Zeroizing::new(octets.to_vec()),
    }))))
}

/// The text the `File` a call reads holds, and the file's mode on a Unix
/// host.
fn file_text(
    value: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<(String, Zeroizing<String>, Option<u32>)> {
    let Value::Endpoint(endpoint) = value else {
        return Err(fail(
            span,
            format!("`{name}` needs a file, not {}", value.type_of()),
        ));
    };
    let crate::io::Endpoint::File(path) = endpoint.as_ref() else {
        return Err(fail(
            span,
            format!("`{name}` reads a file, not {}", endpoint.describe()),
        ));
    };
    let (text, mode) = crate::io::read_key_file(path).map_err(|message| fail(span, message))?;
    Ok((path.display().to_string(), Zeroizing::new(text), mode))
}

fn eval_read_public_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (file, rest) = constructor_input(cx, input, arguments)?;
    let (path, text, _) = file_text(&file, "read_public_key", &span)?;
    let format = format_of(rest.first(), "read_public_key", &span)?;
    let algorithm = algorithm_of(rest.get(1), "read_public_key", &span)?;
    keys::read_public(&text, format, algorithm)
        .map(public_value)
        .map_err(|message| fail(&span, format!("{path}: {message}")))
}

fn eval_read_private_key(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (file, rest) = constructor_input(cx, input, arguments)?;
    let (path, text, mode) = file_text(&file, "read_private_key", &span)?;
    if let Some(mode) = mode
        && mode & 0o077 != 0
    {
        crate::io::write_diagnostic(Diagnostic::new(
            crate::diagnostics::Phase::Execute,
            crate::diagnostics::Kind::Effect,
            crate::diagnostics::Severity::Warning,
            crate::diagnostics::Disposition::Warning,
            span.clone(),
            format!(
                "{path} holds a private key and has mode {:04o}: its group or others may read it; `chmod 600` it",
                mode & 0o7777
            ),
        ));
    }
    let format = format_of(rest.first(), "read_private_key", &span)?;
    let algorithm = algorithm_of(rest.get(1), "read_private_key", &span)?;
    let passphrase = named_text(cx, arguments, "passphrase", &span)?;
    keys::read_private(&text, format, algorithm, &passphrase)
        .map(private_value)
        .map_err(|error| fail(&span, format!("{path}: {}", error.message())))
}

fn eval_to_text(
    _cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let key = public_of(&input, "to_text", &span)?;
    let format = format_of(positional(arguments).first(), "to_text", &span)?;
    keys::write_public(key, format)
        .map(|text| Value::Str(Arc::from(text)))
        .map_err(|message| fail(&span, message))
}

fn eval_to_der(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let key = public_of(&input, "to_der", &span)?;
    keys::to_der(key)
        .map(|der| Value::Bytes(der.into()))
        .map_err(|message| fail(&span, message))
}

fn eval_export_private(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let key = private_of(&input, "export_private", &span)?;
    let format = format_of(positional(arguments).first(), "export_private", &span)?;
    let passphrase = named_text(cx, arguments, "passphrase", &span)?;
    keys::export_private(key, format, &passphrase)
        .map(|text| Value::Str(Arc::from(text)))
        .map_err(|message| fail(&span, message))
}

fn eval_public(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    Ok(public_value(private_of(&input, "public", &span)?.public()))
}

fn eval_thumbprint(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    Ok(Value::Str(Arc::from(keys::thumbprint(public_of(
        &input,
        "thumbprint",
        &span,
    )?))))
}

fn eval_generate_key_pair(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let _ = input;
    let algorithm = algorithm_of(positional(arguments).first(), "generate_key_pair", &span)?;
    let bits = match super::named(arguments, "bits") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Int(bits) => bits,
            _ => return Err(fail(&span, "`bits` needs an Int")),
        },
        None => 3072,
    };
    keys::generate(algorithm, bits)
        .map(pair_value)
        .map_err(|message| fail(&span, message))
}

fn eval_generate_secret_key(
    _cx: &mut dyn EvalCx,
    _input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let algorithm =
        secret_algorithm_of(positional(arguments).first(), "generate_secret_key", &span)?;
    let mut octets = Zeroizing::new(vec![0_u8; algorithm.generated_length()]);
    crate::crypto::random(&mut octets).map_err(|message| fail(&span, message))?;
    Ok(Value::Key(Arc::new(Key::Secret(SecretKey {
        algorithm,
        octets,
    }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec()
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }
}
