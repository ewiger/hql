<!-- hql reference: std/io.parent -->
# `parent`

In [`std/io`](index.md).

```hql
fn parent(self: Path) : Option<Path>
```

`self` without its last segment, `some(Path)`, or `none` for a path of
one segment: `Path("a/b").parent()` is `some(Path("a"))`, and
`Path("a").parent()` is `none`.
<!-- hql reference: end -->
