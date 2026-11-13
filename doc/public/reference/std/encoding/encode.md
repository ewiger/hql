<!-- hql reference: std/encoding.encode -->
# `encode`

In [`std/encoding`](index.md).

```hql
fn encode<E : Encoding>(self: String, as: Type<E>) : String
```

`self`'s UTF-8 octets written in `as`: `"hé".encode(Hex)` is `"68c3a9"`.
<!-- hql reference: end -->
