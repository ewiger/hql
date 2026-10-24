# Part 3: `hash` and `encoding`

- **Depends on**: [the proposal](../README.md) for ownership;
  [issue 0029](../../../issues/0029-binary-formats-and-codecs.md) for binary
  values, which this part stays out of
- **Needed by**: none

## Scope

Two modules over text: `hash` gives a digest of a `String` or of a `Data`
tree, and `encoding` converts a `String` to and from a text-safe encoding.
Python's `hashlib`, `zlib.crc32`, `base64`, `binascii` and `urllib.parse.quote`
cover the same ground. A `String` is UTF-8, and its UTF-8 bytes are what each
operation reads.

## `hash`

An algorithm is named by a type, as a format is.

```hql
// lib/std/hash/mod.hql
module hash
import core

abstract type Algorithm
type Md5    : Algorithm   // broken for collision resistance; checksums only
type Sha1   : Algorithm   // broken for collision resistance; checksums only
type Sha256 : Algorithm
type Sha512 : Algorithm
type Blake3 : Algorithm   // 256-bit output
type Crc32  : Algorithm   // IEEE polynomial; not a cryptographic hash

fn digest<A : Algorithm>(self: String, as: Type<A>) : String
fn fingerprint(self: Data) : String
```

- `digest` MUST give the algorithm's output over the text's UTF-8 bytes,
  written as lowercase hexadecimal, two digits per byte. `Crc32` MUST give
  eight digits, big-endian.
- `fingerprint` MUST equal `digest(canonical(x), Sha256)`, where
  `canonical(x)` is the JSON Canonicalization Scheme text of the tree
  (RFC 8785): object keys sorted by their UTF-16 code units, no whitespace,
  numbers in their shortest round-trip form.
- Two trees equal under HQL equality MUST have equal fingerprints. A
  `Decimal` MUST be written as its exact digits, and a time value as its
  ISO 8601 text, so the canonical form loses nothing a tree holds.

## `encoding`

An encoding is named by a type, and a decode that cannot succeed gives
`none`.

```hql
// lib/std/encoding/mod.hql
module encoding
import core

abstract type Encoding
type Hex       : Encoding   // lowercase on encode; either case on decode
type Base64    : Encoding   // RFC 4648, standard alphabet, padded
type Base64Url : Encoding   // RFC 4648, URL-safe alphabet, unpadded
type Percent   : Encoding   // RFC 3986 percent-encoding of a URI component

fn encode<E : Encoding>(self: String, as: Type<E>) : String
fn decode<E : Encoding>(self: String, as: Type<E>) : Option<String>
```

- `decode` MUST give `none` where the text is not valid in the encoding, and
  where the decoded bytes are not UTF-8: a `String` cannot hold them.
- `Base64` decode MUST require padding; `Base64Url` decode MUST accept it
  present or absent.
- `Percent` encode MUST leave the RFC 3986 unreserved characters
  (`A-Z a-z 0-9 - . _ ~`) as they are and encode every other byte as `%XX`
  with uppercase hex. Decode MUST NOT read `+` as a space.
- `s.encode(E).decode(E)` MUST be `some(s)` for every `String` and every
  encoding.

## Examples

```hql
import hash
import encoding

// A stable id for each record, from its content.
source(file("sightings.jsonl"), JsonLines)
| map(r => {id: fingerprint(r), record: r})

// Records that appear more than once.
records | group(by = r => fingerprint(r)) | filter(g => size(g.items) > 1)

// Checksum a file's text, and read a base64 field.
file("data.csv") | text | digest(Sha256)
payload.body | decode(Base64) | unwrap_or("")
```

## Test plan

- Each algorithm against its published test vectors (the empty string,
  `"abc"`, and the million-`a` vector where the standard defines one).
- `fingerprint` of two trees with the same entries in different key order is
  equal; a `Decimal` `1.10` and `1.1` fingerprint as HQL equality says.
- Each encoding against RFC 4648's and RFC 3986's vectors; the round trip
  over text with multi-byte characters.
- `decode` gives `none` for invalid input and for valid input that is not
  UTF-8 (`"/w=="` in `Base64`).
