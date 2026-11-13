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
    declared("sign", eval_sign),
    declared("sign_data", eval_sign_data),
    declared("verify", eval_verify),
    declared("verify_data", eval_verify_data),
    declared("p256_signature_from_der", eval_p256_signature_from_der),
    declared("mac", eval_mac),
    declared("verify_mac", eval_verify_mac),
    effectful("hash_password", eval_hash_password),
    declared("verify_password", eval_verify_password),
    effectful("encrypt", eval_encrypt),
    declared("decrypt", eval_decrypt),
    effectful("encrypt_to", eval_encrypt_to),
    declared("decrypt_with", eval_decrypt_with),
    effectful("encrypt_with_passphrase", eval_encrypt_with_passphrase),
    declared("decrypt_with_passphrase", eval_decrypt_with_passphrase),
    declared("armor", eval_armor),
    declared("unarmor", eval_unarmor),
    Operation {
        check: Some(check_encrypted_to),
        ..declared("encrypted_to", eval_encrypted_to)
    },
    declared("decrypted_with", eval_decrypted_with),
];

/// Whether a step is one of `crypto`'s that draws from the host
/// generator: a key, a nonce, a salt or an age file key.
/// The six names are declared by `crypto` alone.
pub(crate) fn reads_host_generator(step: &Operation) -> bool {
    matches!(
        step.name,
        "generate_key_pair"
            | "generate_secret_key"
            | "encrypt"
            | "encrypt_to"
            | "encrypt_with_passphrase"
            | "hash_password"
    )
}

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

// Signatures, MACs and password hashes (part 3).

/// The octets a message is: a `Bytes` value's, or a `String`'s UTF-8.
pub(crate) fn message_octets(
    value: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Zeroizing<Vec<u8>>> {
    if let Ok(octets) = super::bytes::input_octets(value, name, span) {
        return Ok(Zeroizing::new(octets.to_vec()));
    }
    text_of(value, name, span).map(|text| Zeroizing::new(text.into_bytes()))
}

/// The octets `sign_data` signs: the canonical JSON of the tree.
pub(crate) fn canonical_octets(value: &Value) -> Vec<u8> {
    crate::evaluation::as_data(value)
        .canonical_json()
        .into_bytes()
}

/// The argument at `index` among the positional ones, evaluated.
fn argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    index: usize,
    name: &str,
    what: &str,
    span: &Range<usize>,
) -> Result<Value> {
    let argument = positional(arguments)
        .get(index)
        .copied()
        .ok_or_else(|| fail(span, format!("`{name}` needs {what}")))?;
    cx.evaluate(&argument.value)
}

fn signed(message: &[u8], key: &Value, span: &Range<usize>) -> Result<Value> {
    let key = private_of(key, "sign", span)?;
    crate::crypto::sign::sign(message, key)
        .map(|signature| Value::Bytes(signature.into()))
        .map_err(|message| fail(span, message))
}

fn eval_sign(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let message = message_octets(&input, "sign", &span)?;
    let key = argument(cx, arguments, 0, "sign", "a private key", &span)?;
    signed(&message, &key, &span)
}

fn eval_sign_data(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let key = argument(cx, arguments, 0, "sign_data", "a private key", &span)?;
    signed(&canonical_octets(&input), &key, &span)
}

fn verified(
    cx: &mut dyn EvalCx,
    message: &[u8],
    arguments: &[TypedArg],
    name: &str,
    span: &Range<usize>,
) -> Result<Value> {
    let signature = argument(cx, arguments, 0, name, "a signature", span)?;
    let signature = super::bytes::input_octets(&signature, name, span)?;
    let key = argument(cx, arguments, 1, name, "a public key", span)?;
    let key = public_of(&key, name, span)?;
    Ok(Value::Bool(crate::crypto::sign::verify(
        message, &signature, key,
    )))
}

fn eval_verify(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let message = message_octets(&input, "verify", &span)?;
    verified(cx, &message, arguments, "verify", &span)
}

fn eval_verify_data(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    verified(
        cx,
        &canonical_octets(&input),
        arguments,
        "verify_data",
        &span,
    )
}

fn eval_p256_signature_from_der(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let der = super::bytes::input_octets(&input, "p256_signature_from_der", &span)?;
    Ok(match crate::crypto::sign::p256_from_der(&der) {
        Some(signature) => Value::Present(Arc::new(Value::Bytes(signature.into()))),
        None => Value::Absent(TypeRef::BYTES),
    })
}

fn eval_mac(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let message = message_octets(&input, "mac", &span)?;
    let key = argument(cx, arguments, 0, "mac", "a MAC key", &span)?;
    crate::crypto::sign::mac(&message, secret_of(&key, "mac", &span)?)
        .map(|tag| Value::Bytes(tag.into()))
        .map_err(|message| fail(&span, message))
}

fn eval_verify_mac(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let message = message_octets(&input, "verify_mac", &span)?;
    let tag = argument(cx, arguments, 0, "verify_mac", "a tag", &span)?;
    let tag = super::bytes::input_octets(&tag, "verify_mac", &span)?;
    let key = argument(cx, arguments, 1, "verify_mac", "a MAC key", &span)?;
    Ok(Value::Bool(crate::crypto::sign::verify_mac(
        &message,
        &tag,
        secret_of(&key, "verify_mac", &span)?,
    )))
}

fn int_named(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    key: &str,
    default: i64,
    span: &Range<usize>,
) -> Result<i64> {
    match super::named(arguments, key) {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Int(value) => Ok(value),
            _ => Err(fail(span, format!("`{key}` needs an Int"))),
        },
        None => Ok(default),
    }
}

fn eval_hash_password(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let password = Zeroizing::new(text_of(&input, "hash_password", &span)?);
    let memory = int_named(
        cx,
        arguments,
        "memory_kib",
        crate::crypto::sign::MINIMUM_MEMORY_KIB,
        &span,
    )?;
    let iterations = int_named(cx, arguments, "iterations", 2, &span)?;
    let parallelism = int_named(cx, arguments, "parallelism", 1, &span)?;
    crate::crypto::sign::hash_password(&password, memory, iterations, parallelism)
        .map(|hash| Value::Str(Arc::from(hash)))
        .map_err(|message| fail(&span, message))
}

fn eval_verify_password(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let password = Zeroizing::new(text_of(&input, "verify_password", &span)?);
    let hash = argument(cx, arguments, 0, "verify_password", "a PHC hash", &span)?;
    let hash = text_of(&hash, "verify_password", &span)?;
    crate::crypto::sign::verify_password(&password, &hash)
        .map(Value::Bool)
        .map_err(|message| fail(&span, message))
}

// Encryption (part 4).

/// The `aad` a call binds, or no octets.
fn aad_of(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    span: &Range<usize>,
) -> Result<Zeroizing<Vec<u8>>> {
    match super::named(arguments, "aad") {
        Some(argument) => {
            let value = cx.evaluate(&argument.value)?;
            message_octets(&value, "aad", span)
        }
        None => Ok(Zeroizing::new(Vec::new())),
    }
}

/// The X25519 public keys a list of recipients holds.
fn recipient_keys(value: &Value, name: &str, span: &Range<usize>) -> Result<Vec<[u8; 32]>> {
    let elements = value
        .elements()
        .ok_or_else(|| fail(span, format!("`{name}` needs a list of X25519 public keys")))?;
    elements
        .iter()
        .map(|element| match public_of(element, name, span)? {
            PublicKey::X25519(key) => Ok(*key),
            other => Err(fail(
                span,
                format!(
                    "`{name}` encrypts to X25519 keys, and one is {}",
                    other.algorithm().name()
                ),
            )),
        })
        .collect()
}

/// The X25519 octets of a private key.
pub(crate) fn identity_octets(
    value: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Zeroizing<[u8; 32]>> {
    match private_of(value, name, span)? {
        PrivateKey::X25519(key) => Ok(key.clone()),
        other => Err(fail(
            span,
            format!(
                "`{name}` decrypts with an X25519 key, and this is {}",
                other.algorithm().name()
            ),
        )),
    }
}

fn optional_bytes(octets: Option<Zeroizing<Vec<u8>>>) -> Value {
    match octets {
        Some(octets) => Value::Present(Arc::new(Value::Bytes(octets.as_slice().into()))),
        None => Value::Absent(TypeRef::BYTES),
    }
}

fn eval_encrypt(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let plaintext = message_octets(&input, "encrypt", &span)?;
    let key = argument(cx, arguments, 0, "encrypt", "a cipher key", &span)?;
    let aad = aad_of(cx, arguments, &span)?;
    crate::crypto::encrypt::encrypt(&plaintext, secret_of(&key, "encrypt", &span)?, &aad)
        .map(|sealed| Value::Bytes(sealed.into()))
        .map_err(|message| fail(&span, message))
}

fn eval_decrypt(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let sealed = super::bytes::input_octets(&input, "decrypt", &span)?;
    let key = argument(cx, arguments, 0, "decrypt", "a cipher key", &span)?;
    let aad = aad_of(cx, arguments, &span)?;
    crate::crypto::encrypt::decrypt(&sealed, secret_of(&key, "decrypt", &span)?, &aad)
        .map(optional_bytes)
        .map_err(|message| fail(&span, message))
}

fn eval_encrypt_to(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let plaintext = message_octets(&input, "encrypt_to", &span)?;
    let recipients = argument(
        cx,
        arguments,
        0,
        "encrypt_to",
        "a list of recipients",
        &span,
    )?;
    let keys = recipient_keys(&recipients, "encrypt_to", &span)?;
    crate::crypto::encrypt::encrypt_to(&plaintext, &keys)
        .map(|sealed| Value::Bytes(sealed.into()))
        .map_err(|message| fail(&span, message))
}

fn eval_decrypt_with(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let sealed = super::bytes::input_octets(&input, "decrypt_with", &span)?;
    let identity = argument(
        cx,
        arguments,
        0,
        "decrypt_with",
        "an X25519 private key",
        &span,
    )?;
    let key = identity_octets(&identity, "decrypt_with", &span)?;
    Ok(optional_bytes(crate::crypto::encrypt::decrypt_with(
        &sealed, &key,
    )))
}

fn eval_encrypt_with_passphrase(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let plaintext = super::bytes::input_octets(&input, "encrypt_with_passphrase", &span)?;
    let passphrase = argument(
        cx,
        arguments,
        0,
        "encrypt_with_passphrase",
        "a passphrase",
        &span,
    )?;
    let passphrase = Zeroizing::new(text_of(&passphrase, "encrypt_with_passphrase", &span)?);
    crate::crypto::encrypt::encrypt_with_passphrase(&plaintext, &passphrase)
        .map(|sealed| Value::Bytes(sealed.into()))
        .map_err(|message| fail(&span, message))
}

fn eval_decrypt_with_passphrase(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let sealed = super::bytes::input_octets(&input, "decrypt_with_passphrase", &span)?;
    let passphrase = argument(
        cx,
        arguments,
        0,
        "decrypt_with_passphrase",
        "a passphrase",
        &span,
    )?;
    let passphrase = Zeroizing::new(text_of(&passphrase, "decrypt_with_passphrase", &span)?);
    crate::crypto::encrypt::decrypt_with_passphrase(&sealed, &passphrase)
        .map(optional_bytes)
        .map_err(|message| fail(&span, message))
}

fn eval_armor(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let message = super::bytes::input_octets(&input, "armor", &span)?;
    crate::crypto::encrypt::armor(&message)
        .map(|text| Value::Str(Arc::from(text)))
        .map_err(|message| fail(&span, message))
}

fn eval_unarmor(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let text = text_of(&input, "unarmor", &span)?;
    Ok(match crate::crypto::encrypt::unarmor(&text) {
        Some(message) => Value::Present(Arc::new(Value::Bytes(message.into()))),
        None => Value::Absent(TypeRef::BYTES),
    })
}

// Encrypted endpoints (part 4).

/// The endpoint an argument or input holds.
fn endpoint_of(value: &Value, name: &str, span: &Range<usize>) -> Result<Arc<crate::io::Endpoint>> {
    match value {
        Value::Endpoint(endpoint) => Ok(Arc::clone(endpoint)),
        other => Err(fail(
            span,
            format!("`{name}` wraps an endpoint, not {}", other.type_of()),
        )),
    }
}

/// `encrypted_to(file(p).end, …)` is refused before anything runs.
fn check_encrypted_to(
    _: &mut dyn CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<()> {
    if call.input.is(&TypeRef::FILE_END) {
        return Err(Diagnostic::typing(
            span,
            "an age file is one header and one stream of chunks, and nothing is appended to it: \
             encrypt to the whole file, `file(p)`, not its `.end`",
        ));
    }
    Ok(())
}

fn eval_encrypted_to(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (endpoint, rest) = constructor_input(cx, input, arguments)?;
    let inner = endpoint_of(&endpoint, "encrypted_to", &span)?;
    let recipients = match rest.first() {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => return Err(fail(&span, "`encrypted_to` needs a list of recipients")),
    };
    let recipients = recipient_keys(&recipients, "encrypted_to", &span)?;
    if recipients.is_empty() {
        return Err(fail(
            &span,
            "an age file has at least one recipient, and the list is empty",
        ));
    }
    Ok(Value::Endpoint(Arc::new(crate::io::Endpoint::Encrypted {
        inner: Box::new(inner.as_ref().clone()),
        recipients,
    })))
}

fn eval_decrypted_with(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (endpoint, rest) = constructor_input(cx, input, arguments)?;
    let inner = endpoint_of(&endpoint, "decrypted_with", &span)?;
    let identity = match rest.first() {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => return Err(fail(&span, "`decrypted_with` needs an X25519 private key")),
    };
    let key = identity_octets(&identity, "decrypted_with", &span)?;
    Ok(Value::Endpoint(Arc::new(crate::io::Endpoint::Decrypted {
        inner: Box::new(inner.as_ref().clone()),
        identity: crate::crypto::Identity(Arc::new(key)),
    })))
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
