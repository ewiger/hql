<!-- hql reference: std/encoding.to_bytes -->
# `to_bytes`

In [`std/encoding`](index.md).

```hql
fn to_bytes<E : Encoding>(self: String, as: Type<E>) : Option<Bytes>
```

The octets `self` encodes in `as`, or `none` where the text is not valid
in it: `"6869".to_bytes(Hex)` is `some(0x6869)`. Unlike `decode`, the
octets need not be UTF-8.
<!-- hql reference: end -->
