# Part 1: `Bytes`

- **Depends on**: [the proposal](../README.md) for ownership;
  [HQL-0041 part 3](../../HQL-0041/parts/03-digest-and-encoding.md) for
  `crypto.digest` and `encoding`, whose `String` forms this part extends
- **Needed by**: every other part; [issue 0029](../../../issues/0029-binary-formats-and-codecs.md)
  for the in-memory form of a binary format's encoded value

## Scope

`Bytes` is a finite sequence of octets: what a key in DER, a signature, a
nonce, a ciphertext and a MAC tag are. A `String` cannot hold them, since a
`String` is UTF-8 ([HQL-0024 part 7](../../HQL-0024/parts/07-formats-and-codecs.md),
"A String is UTF-8"). Python's `bytes` covers the same ground.

## Declarations

```hql
// lib/std/core/mod.hql
/// A finite sequence of octets, ordered octet by octet as unsigned numbers,
/// a shorter prefix first. No literal produces one: `Bytes("hello")` is the
/// UTF-8 encoding of a text, and `encoding` converts hex and base64 text.
type Bytes : {Scalar, Orderable}

/// The UTF-8 encoding of `text`: `Bytes("hé")` holds three octets.
fn Bytes(text: String) : Bytes

impl Eq for Bytes
impl Ord for Bytes
impl Add for Bytes            // concatenation
impl Index<Int> for Bytes     // the octet at a position, 0..255
impl Slice for Bytes          // `b[0:12]` is a Bytes
```

```hql
// lib/std/bytes/mod.hql
/// The functions of a `Bytes` value, in the prelude.
module bytes for Bytes

import core

/// How many octets `self` holds.
fn size(self) : Int

/// `self` read as UTF-8 text, or `none` where it is not valid UTF-8:
/// `Bytes("hé").to_text()` is `some("hé")`.
fn to_text(self) : Option<String>
```

```hql
// lib/std/encoding/mod.hql, beside the String forms of HQL-0041 part 3
/// `self` written in the encoding `as`: `Bytes("hi").encode(Hex)` is
/// `"6869"`.
fn encode<E : Encoding>(self: Bytes, as: Type<E>) : String

/// The octets `self` encodes in `as`, or `none` where the text is not valid
/// in it: `"6869".to_bytes(Hex)` is `some(0x6869)`.
fn to_bytes<E : Encoding>(self: String, as: Type<E>) : Option<Bytes>

/// The octets `text` encodes in `as`: `Bytes("6869", Hex)`. Text that is
/// not valid in the encoding is an evaluation error naming the encoding
/// and the first invalid position; `to_bytes` gives `none` instead.
fn Bytes<E : Encoding>(text: String, as: Type<E>) : Bytes
```

```hql
// lib/std/crypto/digest/mod.hql, beside the String form of HQL-0041 part 3
/// The digest of the octets `self` holds, as lowercase hexadecimal.
fn digest<A : Algorithm>(self: Bytes, as: Type<A>) : String

/// The canonical JSON text of a tree: RFC 8785, the text `fingerprint`
/// digests.
fn canonical(self: Data) : String
```

- `Bytes(s)` MUST be the UTF-8 encoding of `s`, and
  `Bytes(s).to_text()` MUST be `some(s)` for every `String`.
- `a + b` MUST be the octets of `a` followed by the octets of `b`.
- `b[i]` MUST be the octet at `i` as an `Int` in `0..255`, and a negative
  `i` counts from the end, as for every `Index`. `b[a:b]` MUST follow the
  slice rules of `core`'s `Range`.
- `b.encode(E).to_bytes(E)` MUST be `some(b)` for every `Bytes` and every
  encoding of HQL-0041 part 3. `to_bytes` reads the same text `decode`
  reads, without `decode`'s requirement that the octets be UTF-8.
- `digest` of a `Bytes` and of a `String` MUST agree:
  `Bytes(s).digest(A) == s.digest(A)`.
- `canonical(x)` MUST be the text `fingerprint` digests, so
  `fingerprint(x) == canonical(x).digest(Sha256)`. A verifier in another
  language reproduces it with any RFC 8785 implementation.

## Text of a `Bytes` value

- The REPL, `str` and `fmt` MUST write a `Bytes` value as `0x` followed by
  its octets in lowercase hexadecimal, two digits each: `0x68656c6c6f`. An
  empty value is `0x`. This text is a rendering and not a literal; a
  program builds the value with `Bytes(s, Hex)`.
- Every text format, `Json`, `JsonLines`, `Yaml`, `Csv` and the canonical
  JSON of `canonical`, MUST write a `Bytes` value as its base64 text, RFC
  4648 standard alphabet, padded.
- A decoder MUST NOT produce a `Bytes` from text. A declared field of type
  `Bytes` reads base64 text, and text that is not base64 is a boundary error
  naming the field. This is the rule HQL-0041 states for the time types.

## `Bytes` and binary

`Bytes` names a value; "binary" names a property of a format. The two words
are not interchangeable, and each has one place.

| Word | Names | Where |
| --- | --- | --- |
| `Bytes` | a value: a finite sequence of octets, held, indexed and sliced | the type in `core`, the receiver module `bytes` |
| binary | how a format encodes a value: as octets, not as UTF-8 text | a binary format kind, [issue 0029](../../../issues/0029-binary-formats-and-codecs.md) |

- A type name in HQL is a noun for what a value is, as `String`, `Int` and
  `Decimal` are. `Bytes` is that noun; `Binary` would be an adjective, and
  also reads as base 2, as in `0b1010`.
- The receiver module is named after its type, `module bytes for Bytes`,
  as `module string for String` is.
- A binary format reads and writes `Bytes`, where a text format reads and
  writes `String`. No standard type, module or function is named
  `binary` or `Binary` for a value.
- A digest reads `Bytes` and is a value, not a format; "binary" does not
  apply to it.

## The order against issue 0029

- This part is the in-memory representation of octets. Issue 0029 decides
  binary formats as a format kind; it MUST take `Bytes` as the value
  `format(x, F)` gives for a binary `F`, where a text format gives a
  `String`.
- Issue 0029 is ordered after this part and before nothing in this
  proposal. The encrypted endpoints of [part 4](04-encryption.md)
  transform the octets a format writes, below the format, and need no
  binary format kind.

## Examples

```hql
import encoding
import crypto.digest

b = Bytes("hello")
{size: b.size(), hex: b.encode(Hex), base64: b.encode(Base64),
 text: b.to_text() | unwrap_or(""), back: Bytes("68656c6c6f", Hex) == b}

// A webhook body's digest, over the octets received.
body.to_bytes(Base64) | map(b => b.digest(Sha256))
```

Case: [bytes and text](../../../../examples/crypto/queries/bytes-and-text.hql).

## Test plan

- `Bytes` of the empty text, of ASCII and of multi-byte text; `to_text`
  of `0xff` is `none`.
- Order: `0x00 < 0x0000 < 0x01 < 0xff`; concatenation and slices at both
  ends and with negative bounds.
- `to_bytes` over each encoding's RFC vectors, including `"/w=="` in
  `Base64`, which `decode` refuses and `to_bytes` reads as `0xff`.
- A record with a `Bytes` field written as JSON and read back through a
  declared type; read as `Data`, the field is a `String`.
- `canonical` against RFC 8785's published examples.
