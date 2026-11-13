<!-- hql reference: std/crypto.to_private_key -->
# `to_private_key`

In [`std/crypto`](index.md).

```hql
fn to_private_key<F : KeyFormat, A : KeyAlgorithm>(self: String, format: Type<F>, algorithm: Type<A>, *, passphrase: String = "") : Option<PrivateKey<A>>
```

The private key `self` holds in `format`, or `none`.
<!-- hql reference: end -->
