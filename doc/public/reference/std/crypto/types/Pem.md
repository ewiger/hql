<!-- hql reference: std/crypto.Pem -->
# `Pem`

In [`std/crypto`](../index.md).

```hql
type Pem : KeyFormat
```

How one key is text. `Pem` is RFC 7468 text around DER: SPKI for a
public key, PKCS #8 for a private key. `Jwk` is the RFC 7517 JSON text of
one key. `OpenSsh` is an authorized_keys line or an OpenSSH private key
file. `Age` is age's Bech32 recipient and identity text, for `X25519`
only; `X25519` has no `OpenSsh` text.
<!-- hql reference: end -->
