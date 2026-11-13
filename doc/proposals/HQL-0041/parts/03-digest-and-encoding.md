# Part 3: `crypto.digest` and `encoding`

- **Depends on**: [the proposal](../README.md) for ownership;
  [issue 0029](../../../issues/0029-binary-formats-and-codecs.md) for binary
  values, which this part stays out of
- **Needed by**: [HQL-0053](../../HQL-0053/README.md), whose `crypto`
  imports `crypto.digest`

## Scope

Two modules over text: `crypto.digest` gives a cryptographic digest of a
`String` or of a `Data` tree, and `encoding` converts a `String` to and from
a text-safe encoding. Python's `hashlib`, `base64`, `binascii` and
`urllib.parse.quote` cover the same ground. A `String` is UTF-8, and its UTF-8
bytes are what each operation reads.

No standard module or type is named "hash". A digest is a function from
octets to a fixed-size octet string; a map is a collection.

## `crypto.digest`

An algorithm is named by a type, as a format is. A program writes
`import crypto.digest` and reaches the module as `digest`. Modules and
functions are separate namespaces, so `digest.digest(s, Sha256)` is the
qualified call.

```hql
// lib/std/crypto/digest/mod.hql
module crypto.digest
import core

abstract type Algorithm
type Md5    : Algorithm   // broken for collision resistance; interoperability only
type Sha1   : Algorithm   // broken for collision resistance; interoperability only
type Sha256 : Algorithm
type Sha512 : Algorithm
type Blake3 : Algorithm   // 256-bit output

fn digest<A : Algorithm>(self: String, as: Type<A>) : String
fn fingerprint(self: Data) : String
```

- `digest` MUST give the algorithm's output over the text's UTF-8 bytes,
  written as lowercase hexadecimal, two digits per byte.
- Every `Algorithm` MUST be a cryptographic hash function. A checksum is
  not one.
- `fingerprint` MUST equal `digest(canonical(x), Sha256)`, where
  `canonical(x)` is the JSON Canonicalization Scheme text of the tree
  (RFC 8785): object keys sorted by their UTF-16 code units, no whitespace,
  strings escaped as RFC 8785 escapes them.
- Two trees equal under HQL equality MUST have equal fingerprints. A number
  MUST be written as its exact digits with trailing zeros removed and no
  exponent, so `1`, `1.0` and `1.00` write `1`, and a time value as its
  ISO 8601 text, so the canonical form loses nothing a tree holds. For a
  number of magnitude at least 1e-6 and below 1e21 this is RFC 8785's
  text; outside that range RFC 8785 writes an exponent and loses digits a
  `Decimal` holds, and `canonical` keeps them.

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
import crypto.digest
import encoding

// A stable id for each record, from its content.
source(file("sightings.jsonl"), JsonLines)
| map(r => {id: fingerprint(r), record: r})

// Records that appear more than once.
records | group(by = r => fingerprint(r)) | filter(g => size(g.items) > 1)

// Digest a file's text, and read a base64 field.
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
