<!-- hql reference: std/crypto.mac -->
# `mac`

In [`std/crypto`](index.md).

```hql
fn mac<A : Mac>(self: Bytes, key: SecretKey<A>) : Bytes
```

The HMAC tag of `self` under `key`: 32 octets for `HmacSha256`, 64 for
`HmacSha512`. `mac(m, k) == tag` compares with `==`, which is not
constant-time; `verify_mac` is.

```hql
fn mac<A : Mac>(self: String, key: SecretKey<A>) : Bytes
```

The HMAC tag of the UTF-8 octets of `self` under `key`.
<!-- hql reference: end -->
