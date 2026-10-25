<!-- hql reference: std/collections.get -->
# `get`

In [`std/collections`](index.md).

```hql
fn get<K, V>(self: Map<K, V>, key: K, /) : Option<V>
```

The value a map holds for `key`, or `none` when the key is missing:
`{"a": 1} | get("a")` is `1`, and `{"a": 1} | get("b")` is `none`.

```hql
fn get<T : Doc>(self: Collection<T>, name: String, /) : Option<T>
```

The document named `name` in a collection of documents, or `none` when
none has that name: `cards.get("alice")` is the card named alice.
<!-- hql reference: end -->
