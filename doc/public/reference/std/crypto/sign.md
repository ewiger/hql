<!-- hql reference: std/crypto.sign -->
# `sign`

In [`std/crypto`](index.md).

```hql
fn sign<A : Signing>(self: Bytes, key: PrivateKey<A>) : Bytes
```

The signature of the octets of `self` by `key`.

```hql
fn sign<A : Signing>(self: String, key: PrivateKey<A>) : Bytes
```

The signature of the UTF-8 octets of `self`.
<!-- hql reference: end -->
