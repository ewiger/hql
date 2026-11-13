<!-- hql reference: std/io/fs.glob -->
# `glob`

In [`std/io/fs`](index.md).

```hql
effectful fn glob(self: Folder, pattern: Glob) : Seq<Entry>
```

The entries of `walk(self)` whose path below `self` matches `pattern`,
in walk order.
<!-- hql reference: end -->
