<!-- hql reference: std/bytes -->
# `std/bytes`

```hql
module bytes for Bytes : Index<Int>, Slice
```

The functions of a `Bytes` value, in the prelude. A receiver module:
`b.size()`, `bytes.size(b)` and `b | size` are one declaration. `Bytes`
implements `Index<Int>` and `Slice` through `at` and `slice`: `b[0]` is
`b.at(0)`, the octet as an `Int` in `0..255`, and `b[0:12]` is a `Bytes`.

- [`at`](at.md) — The octet of `self` at `position`, as an `Int` in `0..255`, or `none` outside it; a negative position counts from the end: `Bytes("hi").at(0)` and `Bytes("hi")[0]` are `104`.
- [`slice`](slice.md) — The octets of `self` from `start` to `end`, clamped to its size; a negative bound counts from the end.
- [`size`](size.md) — How many octets `self` holds: `Bytes("hé").size()` is `3`.
- [`to_text`](to_text.md) — `self` read as UTF-8 text, or `none` where it is not valid UTF-8: `Bytes("hé").to_text()` is `some("hé")`.
<!-- hql reference: end -->
