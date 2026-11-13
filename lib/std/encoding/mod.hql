// Not in the prelude: a program writes `import encoding`. Every declaration
// is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/03-digest-and-encoding.md.

/// Text-safe encodings of a text's UTF-8 octets. An encoding is named by a
/// type, as a format is: `"hé".encode(Hex)` is `"68c3a9"`. A decode that
/// cannot succeed gives `none`.
module encoding

import core

/// A text-safe encoding of octets. `Percent` leaves RFC 3986's unreserved
/// characters `A-Z a-z 0-9 - . _ ~` as they are and writes every other
/// octet as `%XX` in uppercase hex.
abstract type Encoding
type Hex       : Encoding   // lowercase on encode; either case on decode
type Base64    : Encoding   // RFC 4648, standard alphabet, padded
type Base64Url : Encoding   // RFC 4648, URL-safe alphabet; unpadded on encode, padding optional on decode
type Percent   : Encoding   // RFC 3986 percent-encoding of a URI component

/// `self`'s UTF-8 octets written in `as`: `"hé".encode(Hex)` is `"68c3a9"`.
fn encode<E : Encoding>(self: String, as: Type<E>) : String

/// The text `self` encodes in `as`, or `none` where `self` is not valid in
/// it or the octets are not UTF-8: `"aGk/".decode(Base64)` is `some("hi?")`.
/// Percent-decoding does not read `+` as a space.
fn decode<E : Encoding>(self: String, as: Type<E>) : Option<String>

/// `self` written in the encoding `as`: `Bytes("hi").encode(Hex)` is
/// `"6869"`.
fn encode<E : Encoding>(self: Bytes, as: Type<E>) : String

/// The octets `self` encodes in `as`, or `none` where the text is not valid
/// in it: `"6869".to_bytes(Hex)` is `some(0x6869)`. Unlike `decode`, the
/// octets need not be UTF-8.
fn to_bytes<E : Encoding>(self: String, as: Type<E>) : Option<Bytes>

/// The octets `text` encodes in `as`: `Bytes("6869", Hex)`. Text that is
/// not valid in the encoding is an evaluation error naming the encoding and
/// the first invalid position; `to_bytes` gives `none` instead.
fn Bytes<E : Encoding>(text: String, as: Type<E>) : Bytes
