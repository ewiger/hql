<!-- hql reference: std/crypto.read_public_key -->
# `read_public_key`

In [`std/crypto`](index.md).

```hql
effectful fn read_public_key<F : KeyFormat, A : KeyAlgorithm>(file: File, format: Type<F>, algorithm: Type<A>) : PublicKey<A>
```

The public key a file holds in `format`.
<!-- hql reference: end -->
