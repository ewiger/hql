<!-- hql reference: std/crypto.thumbprint -->
# `thumbprint`

In [`std/crypto`](index.md).

```hql
fn thumbprint<A : KeyAlgorithm>(self: PublicKey<A>) : String
```

The RFC 7638 JWK thumbprint of `self`: base64url, unpadded, of the
SHA-256 of its canonical JWK. It names a key without disclosing it, and
is the same whichever format the key was read from.
<!-- hql reference: end -->
