<!-- hql reference: std/io.join -->
# `join`

In [`std/io`](index.md).

```hql
fn join(self: Path, segment: String) : Path
```

`self` with `segment` appended as its last segment, separated by `/`:
`Path("data").join("birds.csv")` is `Path("data/birds.csv")`. Onto
`Path(".")` or `Path("")` it is `Path(segment)`.
<!-- hql reference: end -->
