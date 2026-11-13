<!-- hql reference: std/crypto/digest.digest -->
# `digest`

In [`std/crypto/digest`](index.md).

```hql
fn digest<A : Algorithm>(self: String, as: Type<A>) : String
```

The digest of `self`'s UTF-8 octets under `as`, written as lowercase
hexadecimal, two digits per octet:
`"abc".digest(Sha256)` is `"ba7816bf…15ad"`.

```hql
fn digest<A : Algorithm>(self: Bytes, as: Type<A>) : String
```

The digest of the octets `self` holds, as lowercase hexadecimal:
`Bytes("abc").digest(Sha256)` is `"abc".digest(Sha256)`.
<!-- hql reference: end -->
