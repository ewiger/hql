<!-- hql reference: std/format.JsonLines -->
# `JsonLines`

In [`std/format`](../index.md).

```hql
type JsonLines : Format
```

The JSON Lines format, one JSON document per line: read, each line is one
`Data` value; written, each element of a collection is one line of
compact JSON: `[{a: 1}, {a: 2}] | jsonlines` is `"{\"a\":1}\n{\"a\":2}"`.
<!-- hql reference: end -->
