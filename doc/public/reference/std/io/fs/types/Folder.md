<!-- hql reference: std/io/fs.Folder -->
# `Folder`

In [`std/io/fs`](../index.md).

```hql
type Folder : Catalog<File> { path : Path }
```

A folder of the file system at `path`: an endpoint that holds files and
folders, as a database holds tables. It is not a `Seq<File>` itself;
`entries`, `walk` and `glob` list what it holds.
<!-- hql reference: end -->
