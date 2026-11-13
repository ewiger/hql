<!-- hql reference: std/crypto.verify_data -->
# `verify_data`

In [`std/crypto`](index.md).

```hql
fn verify_data<A : Signing>(self: Data, signature: Bytes, key: PublicKey<A>) : Bool
```

Whether `signature` is `key`'s signature of `canonical(self)`.
<!-- hql reference: end -->
