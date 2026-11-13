<!-- hql reference: std/crypto.DecryptedEnd -->
# `DecryptedEnd`

In [`std/crypto`](../index.md).

```hql
type DecryptedEnd : Replayable
```

An endpoint whose contents are those of a replayable `endpoint`,
age-decrypted. `source` authenticates every chunk before it gives any
document; `tap` gives a chunk's documents once that chunk is
authenticated.
<!-- hql reference: end -->
