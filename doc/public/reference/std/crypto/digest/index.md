<!-- hql reference: std/crypto/digest -->
# `std/crypto/digest`

```hql
module crypto.digest
```

Cryptographic digests of text and of `Data` trees. An algorithm is named
by a type, as a format is: `"abc".digest(Sha256)`. Every algorithm is a
cryptographic hash function; `Md5` and `Sha1` are broken for collision
resistance and serve interoperability with digests other tools wrote.

- [`Algorithm`](types/Algorithm.md) — A cryptographic hash function.
- [`Md5`](types/Md5.md) — A cryptographic hash function.
- [`Sha1`](types/Sha1.md) — A cryptographic hash function.
- [`Sha256`](types/Sha256.md) — A cryptographic hash function.
- [`Sha512`](types/Sha512.md) — A cryptographic hash function.
- [`Blake3`](types/Blake3.md) — A cryptographic hash function.
- [`digest`](digest.md) — The digest of `self`'s UTF-8 octets under `as`, written as lowercase hexadecimal, two digits per octet: `"abc".digest(Sha256)` is `"ba7816bf…15ad"`.
- [`fingerprint`](fingerprint.md) — The SHA-256 digest of the canonical JSON text of a tree (RFC 8785): keys sorted by their UTF-16 code units, no whitespace, a number as its digits without trailing zeros, a time value as its ISO 8601 text.
- [`canonical`](canonical.md) — The canonical JSON text of a tree (RFC 8785), the text `fingerprint` digests: `fingerprint(x) == canonical(x).digest(Sha256)`.
<!-- hql reference: end -->
