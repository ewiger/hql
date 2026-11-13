<!-- hql reference: std/io.extension -->
# `extension`

In [`std/io`](index.md).

```hql
fn extension(self: Path) : Option<String>
```

The text after the last `.` of the last segment, `some(String)`, or
`none` where the segment has none: `Path("a/b.csv").extension()` is
`some("csv")`. A segment that opens with its only `.`, `.env`, has none.
<!-- hql reference: end -->
