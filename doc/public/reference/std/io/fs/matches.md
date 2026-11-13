<!-- hql reference: std/io/fs.matches -->
# `matches`

In [`std/io/fs`](index.md).

```hql
fn matches(self: Glob, path: Path) : Bool
```

Whether `path`, read as relative to the folder the glob is applied to,
matches `self`. It reads the text and never the file system.
<!-- hql reference: end -->
