//! The `encoding` module (HQL-0041 part 3): the natives behind every
//! declaration of `lib/std/encoding/mod.hql`.
//!
//! An encoding is the type argument a call names, `Hex`, read from the
//! argument's checked type. Every encoding reads and writes a text's UTF-8
//! octets; a decode that cannot succeed is `none`.

use super::{EvalCx, Operation, declared};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeRef, Value};
use base64::Engine as _;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `encoding` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("encode", eval_encode),
    declared("decode", eval_decode),
];

/// Base64 over the URL-safe alphabet: unpadded on encode, padding optional
/// on decode.
const BASE64_URL: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// A text-safe encoding, as `Encoding`'s subtypes name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    Hex,
    Base64,
    Base64Url,
    Percent,
}

impl Encoding {
    /// The encoding a type of `encoding` names.
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Hex" => Self::Hex,
            "Base64" => Self::Base64,
            "Base64Url" => Self::Base64Url,
            "Percent" => Self::Percent,
            _ => return None,
        })
    }

    /// `octets` written in this encoding.
    pub(crate) fn encode(self, octets: &[u8]) -> String {
        match self {
            Self::Hex => super::digest::hex(octets),
            Self::Base64 => STANDARD.encode(octets),
            Self::Base64Url => BASE64_URL.encode(octets),
            Self::Percent => percent_encode(octets),
        }
    }

    /// The octets `text` encodes, or `None` where it is not valid in this
    /// encoding.
    pub(crate) fn decode(self, text: &str) -> Option<Vec<u8>> {
        match self {
            Self::Hex => hex_decode(text),
            Self::Base64 => STANDARD.decode(text).ok(),
            Self::Base64Url => BASE64_URL.decode(text).ok(),
            Self::Percent => percent_decode(text),
        }
    }
}

/// The value of one hexadecimal digit, either case.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let digits = text.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| Some(nibble(*high)? << 4 | nibble(*low)?))
        .collect()
}

/// RFC 3986's unreserved characters, which percent-encoding leaves as they
/// are.
fn unreserved(octet: u8) -> bool {
    octet.is_ascii_alphanumeric() || matches!(octet, b'-' | b'.' | b'_' | b'~')
}

fn percent_encode(octets: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(octets.len());
    for &octet in octets {
        if unreserved(octet) {
            out.push(char::from(octet));
        } else {
            out.push('%');
            out.push(char::from(DIGITS[usize::from(octet >> 4)]));
            out.push(char::from(DIGITS[usize::from(octet & 0x0f)]));
        }
    }
    out
}

/// Every `%XX` replaced by its octet and every other character kept as its
/// UTF-8 octets; `+` is not a space. A `%` without two hex digits after it
/// is not valid.
fn percent_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let high = nibble(*bytes.get(at + 1)?)?;
            let low = nibble(*bytes.get(at + 2)?)?;
            out.push(high << 4 | low);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    Some(out)
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

/// The encoding a call names, as its `as` argument or its first one.
fn encoding(
    arguments: &[TypedArg],
    name: &str,
    span: &Range<usize>,
) -> Result<Encoding, Diagnostic> {
    super::named(arguments, "as")
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
        .and_then(|argument| Encoding::named(argument.value.ty.constructor.0))
        .ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                format!("`{name}` needs an encoding: `Hex`, `Base64`, `Base64Url` or `Percent`"),
            )
        })
}

fn eval_encode(
    _cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let encoding = encoding(arguments, "encode", &span)?;
    let text = input_text(&input, "encode", &span)?;
    Ok(Value::Str(Arc::from(encoding.encode(text.as_bytes()))))
}

fn eval_decode(
    _cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let encoding = encoding(arguments, "decode", &span)?;
    let text = input_text(&input, "decode", &span)?;
    Ok(
        match encoding
            .decode(&text)
            .and_then(|octets| String::from_utf8(octets).ok())
        {
            Some(decoded) => Value::Present(Arc::new(Value::Str(Arc::from(decoded)))),
            None => Value::Absent(TypeRef::STR),
        },
    )
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

    #[test]
    fn rfc_4648_vectors() {
        let vectors = [
            ("", "", ""),
            ("f", "Zg==", "666f"),
            ("fo", "Zm8=", "666f6f"),
            ("foo", "Zm9v", "666f6f"),
            ("foob", "Zm9vYg==", "666f6f62"),
            ("fooba", "Zm9vYmE=", "666f6f6261"),
            ("foobar", "Zm9vYmFy", "666f6f626172"),
        ];
        for (text, base64, _) in vectors {
            assert_eq!(Encoding::Base64.encode(text.as_bytes()), base64);
            assert_eq!(
                Encoding::Base64.decode(base64).as_deref(),
                Some(text.as_bytes())
            );
            let unpadded = base64.trim_end_matches('=');
            assert_eq!(Encoding::Base64Url.encode(text.as_bytes()), unpadded);
            assert_eq!(
                Encoding::Base64Url.decode(base64).as_deref(),
                Some(text.as_bytes())
            );
        }
        assert_eq!(Encoding::Hex.encode(b"foobar"), "666f6f626172");
        assert_eq!(Encoding::Base64.decode("Zg"), None, "padding is required");
    }

    #[test]
    fn percent_encoding_keeps_the_unreserved_characters() {
        assert_eq!(
            Encoding::Percent.encode(b"Az09-._~ +/"),
            "Az09-._~%20%2B%2F"
        );
        assert_eq!(Encoding::Percent.decode("%4a%4A+"), Some(b"JJ+".to_vec()));
        assert_eq!(Encoding::Percent.decode("%4"), None);
        assert_eq!(Encoding::Percent.decode("%zz"), None);
    }

    #[test]
    fn every_encoding_round_trips_multi_byte_text() {
        let text = "héllo, 世界 😀";
        for encoding in [
            Encoding::Hex,
            Encoding::Base64,
            Encoding::Base64Url,
            Encoding::Percent,
        ] {
            assert_eq!(
                encoding
                    .decode(&encoding.encode(text.as_bytes()))
                    .as_deref(),
                Some(text.as_bytes()),
                "{encoding:?}"
            );
        }
    }
}
