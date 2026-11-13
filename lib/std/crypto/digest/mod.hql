// Not in the prelude: a program writes `import crypto.digest` and reaches
// the module as `digest`. Every declaration is external: the `std`
// extension implements it.
//
// See doc/proposals/HQL-0041/parts/03-digest-and-encoding.md.

/// Cryptographic digests of text and of `Data` trees. An algorithm is named
/// by a type, as a format is: `"abc".digest(Sha256)`. Every algorithm is a
/// cryptographic hash function; `Md5` and `Sha1` are broken for collision
/// resistance and serve interoperability with digests other tools wrote.
module crypto.digest

import core

/// A cryptographic hash function.
abstract type Algorithm
type Md5    : Algorithm   // RFC 1321; broken for collision resistance
type Sha1   : Algorithm   // FIPS 180-4; broken for collision resistance
type Sha256 : Algorithm   // FIPS 180-4
type Sha512 : Algorithm   // FIPS 180-4
type Blake3 : Algorithm   // 256-bit output

/// The digest of `self`'s UTF-8 octets under `as`, written as lowercase
/// hexadecimal, two digits per octet:
/// `"abc".digest(Sha256)` is `"ba7816bf…15ad"`.
fn digest<A : Algorithm>(self: String, as: Type<A>) : String

/// The SHA-256 digest of the canonical JSON text of a tree (RFC 8785): keys
/// sorted by their UTF-16 code units, no whitespace, a number as its digits
/// without trailing zeros, a time value as its ISO 8601 text. Two trees
/// equal under HQL equality have equal fingerprints.
fn fingerprint(self: Data) : String
