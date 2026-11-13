<!-- hql reference: std/crypto.decrypted_with -->
# `decrypted_with`

In [`std/crypto`](index.md).

```hql
fn decrypted_with(endpoint: Replayable, identity: PrivateKey<X25519>) : DecryptedEnd
```

`endpoint`, its contents age-decrypted with `identity`.

```hql
fn decrypted_with(endpoint: Readable, identity: PrivateKey<X25519>) : DecryptedLive
```

`endpoint`, its contents age-decrypted with `identity`.
<!-- hql reference: end -->
