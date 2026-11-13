<!-- hql reference: std/crypto.read_private_key -->
# `read_private_key`

In [`std/crypto`](index.md).

```hql
effectful fn read_private_key<F : KeyFormat, A : KeyAlgorithm>(file: File, format: Type<F>, algorithm: Type<A>, *, passphrase: String = "") : PrivateKey<A>
```

The private key a file holds in `format`. On a Unix host, a file its
group or others may read gives a warning naming the path and its mode.
<!-- hql reference: end -->
