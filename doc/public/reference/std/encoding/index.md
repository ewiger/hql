<!-- hql reference: std/encoding -->
# `std/encoding`

```hql
module encoding
```

Text-safe encodings of a text's UTF-8 octets. An encoding is named by a
type, as a format is: `"hé".encode(Hex)` is `"68c3a9"`. A decode that
cannot succeed gives `none`.

- [`Encoding`](types/Encoding.md) — A text-safe encoding of octets.
- [`Hex`](types/Hex.md) — A text-safe encoding of octets.
- [`Base64`](types/Base64.md) — A text-safe encoding of octets.
- [`Base64Url`](types/Base64Url.md) — A text-safe encoding of octets.
- [`Percent`](types/Percent.md) — A text-safe encoding of octets.
- [`encode`](encode.md) — `self`'s UTF-8 octets written in `as`: `"hé".encode(Hex)` is `"68c3a9"`.
- [`decode`](decode.md) — The text `self` encodes in `as`, or `none` where `self` is not valid in it or the octets are not UTF-8: `"aGk/".decode(Base64)` is `some("hi?")`.
- [`to_bytes`](to_bytes.md) — The octets `self` encodes in `as`, or `none` where the text is not valid in it: `"6869".to_bytes(Hex)` is `some(0x6869)`.
- [`Bytes`](types/Bytes.md) — The octets `text` encodes in `as`: `Bytes("6869", Hex)`.
<!-- hql reference: end -->
