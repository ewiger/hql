<!-- hql reference: std/format.format -->
# `format`

In [`std/format`](index.md).

```hql
fn format<T, F : Format>(x: Collection<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String
fn format<T, F : Format>(x: Stream<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : Stream<String>
fn format<T, F : Format>(x: T, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String
```

`x` encoded in the format `as`: the encoded text, a `String` that keeps
its format for presentation. A collection, which ends, and any other
value give one text; a `Stream`, which may not end, is encoded piece by
piece as it is consumed and gives a stream of texts. `limit`, when not
`0`, keeps that many top-level elements and notes how many were cut;
`renderer` names one of the format's renderers. The arguments after `*`
are supplied by name only: `[1, 2] | format(Json)` is `"[\n  1,\n  2\n]"`.
<!-- hql reference: end -->
