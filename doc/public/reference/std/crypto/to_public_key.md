<!-- hql reference: std/crypto.to_public_key -->
# `to_public_key`

In [`std/crypto`](index.md).

```hql
fn to_public_key<F : KeyFormat, A : KeyAlgorithm>(self: String, format: Type<F>, algorithm: Type<A>) : Option<PublicKey<A>>
```

The public key `self` holds in `format`, or `none`.
<!-- hql reference: end -->
