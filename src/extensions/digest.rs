//! The `crypto.digest` module (HQL-0041 part 3): the natives behind every
//! declaration of `lib/std/crypto/digest/mod.hql`.
//!
//! An algorithm is the type argument a call names, `Sha256`, read from the
//! argument's checked type; every digest is written as lowercase hex.

use super::{EvalCx, Operation, declared};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::Value;
use sha2::Digest as _;
use std::fmt::Write as _;
use std::ops::Range;
use std::sync::Arc;

/// Every step the `crypto.digest` module provides, in the order help prints
/// them.
pub(crate) static STEPS: &[Operation] = &[
    declared("digest", eval_digest),
    declared("fingerprint", eval_fingerprint),
];

/// A cryptographic hash function, as `Algorithm`'s subtypes name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Blake3,
}

impl Algorithm {
    /// The algorithm a type of `crypto.digest` names.
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Md5" => Self::Md5,
            "Sha1" => Self::Sha1,
            "Sha256" => Self::Sha256,
            "Sha512" => Self::Sha512,
            "Blake3" => Self::Blake3,
            _ => return None,
        })
    }

    /// The digest of `octets`.
    pub(crate) fn digest(self, octets: &[u8]) -> Vec<u8> {
        match self {
            Self::Md5 => md5::Md5::digest(octets).to_vec(),
            Self::Sha1 => sha1::Sha1::digest(octets).to_vec(),
            Self::Sha256 => sha2::Sha256::digest(octets).to_vec(),
            Self::Sha512 => sha2::Sha512::digest(octets).to_vec(),
            Self::Blake3 => blake3::hash(octets).as_bytes().to_vec(),
        }
    }
}

/// Octets as lowercase hexadecimal, two digits each.
pub(crate) fn hex(octets: &[u8]) -> String {
    let mut out = String::with_capacity(octets.len() * 2);
    for octet in octets {
        let _ = write!(out, "{octet:02x}");
    }
    out
}

/// The fingerprint of a tree: the SHA-256 of its canonical JSON text.
pub(crate) fn fingerprint(data: &crate::data::Data) -> String {
    hex(&Algorithm::Sha256.digest(data.canonical_json().as_bytes()))
}

fn input_text(input: &Value, name: &str, span: &Range<usize>) -> Result<String, Diagnostic> {
    match input {
        Value::Str(text) => Ok(text.to_string()),
        Value::Formatted(formatted) => Ok(formatted.text.clone()),
        Value::Present(inner) => input_text(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a String, not {}", other.type_of()),
        )),
    }
}

fn eval_digest(
    _cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let algorithm = super::named(arguments, "as")
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
        .and_then(|argument| Algorithm::named(argument.value.ty.constructor.0))
        .ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                "`digest` needs an algorithm: `Sha256`, `Sha512`, `Blake3`, `Sha1` or `Md5`",
            )
        })?;
    let text = input_text(&input, "digest", &span)?;
    Ok(Value::Str(Arc::from(hex(&algorithm.digest(text.as_bytes())))))
}

fn eval_fingerprint(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    _span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let data = crate::evaluation::as_data(&input);
    Ok(Value::Str(Arc::from(fingerprint(&data))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec()
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }

    #[test]
    fn each_algorithm_gives_its_published_empty_digest() {
        let empty = |algorithm: Algorithm| hex(&algorithm.digest(b""));
        assert_eq!(empty(Algorithm::Md5), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            empty(Algorithm::Sha1),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            empty(Algorithm::Sha256),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            empty(Algorithm::Blake3),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn the_million_a_vector() {
        let input = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&Algorithm::Sha256.digest(&input)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(
            hex(&Algorithm::Sha1.digest(&input)),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }

    #[test]
    fn canonical_json_sorts_keys_by_utf16_and_writes_numbers_by_value() {
        let tree = crate::io::data_of(
            serde_json::from_str(r#"{"b": 1.50, "a": [1, 1.0, "x\n\u0001"], "é": null, "😀": true, "｡": false}"#)
                .unwrap(),
        );
        // U+1F600 is the surrogate pair D83D DE00, which sorts before U+FF61
        // in UTF-16 and after it in UTF-8.
        assert_eq!(
            tree.canonical_json(),
            "{\"a\":[1,1,\"x\\n\\u0001\"],\"b\":1.5,\"é\":null,\"😀\":true,\"｡\":false}"
        );
        assert_eq!(Data::Float(-0.0).canonical_json(), "0");
    }
}
