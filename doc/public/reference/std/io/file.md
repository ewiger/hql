<!-- hql reference: std/io.file -->
# `file`

In [`std/io`](index.md).

```hql
fn file(path: String) : File
```

The `File` at `path`. Constructing it is pure and opens nothing:
`file("birds.json").path` is `"birds.json"`. A URL is refused; a network
resource is read with `http`.
<!-- hql reference: end -->
