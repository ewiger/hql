<!-- hql reference: std/crypto.verify -->
# `verify`

In [`std/crypto`](index.md).

```hql
fn verify<A : Signing>(self: Bytes, signature: Bytes, key: PublicKey<A>) : Bool
```

Whether `signature` is `key`'s signature of `self`: `false`, never an
error, for a signature of another key, of the wrong length or malformed.

```hql
fn verify<A : Signing>(self: String, signature: Bytes, key: PublicKey<A>) : Bool
```

Whether `signature` is `key`'s signature of the UTF-8 octets of `self`.
<!-- hql reference: end -->
