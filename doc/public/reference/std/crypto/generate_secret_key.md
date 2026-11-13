<!-- hql reference: std/crypto.generate_secret_key -->
# `generate_secret_key`

In [`std/crypto`](index.md).

```hql
effectful fn generate_secret_key<A : SecretAlgorithm>(algorithm: Type<A>) : SecretKey<A>
```

A new symmetric key from the host generator: 32 octets for a cipher and
`HmacSha256`, 64 for `HmacSha512`.
<!-- hql reference: end -->
