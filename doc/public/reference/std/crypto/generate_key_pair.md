<!-- hql reference: std/crypto.generate_key_pair -->
# `generate_key_pair`

In [`std/crypto`](index.md).

```hql
effectful fn generate_key_pair<A : KeyAlgorithm>(algorithm: Type<A>, *, bits: Int = 3072) : KeyPair<A>
```

A new key pair from the host's cryptographically secure generator.
`bits` is read for `Rsa` only, 2048 to 8192.
<!-- hql reference: end -->
