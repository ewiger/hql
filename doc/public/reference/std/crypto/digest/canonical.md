<!-- hql reference: std/crypto/digest.canonical -->
# `canonical`

In [`std/crypto/digest`](index.md).

```hql
fn canonical(self: Data) : String
```

The canonical JSON text of a tree (RFC 8785), the text `fingerprint`
digests: `fingerprint(x) == canonical(x).digest(Sha256)`. A `Bytes` leaf
is written as its base64 text.
<!-- hql reference: end -->
