<!-- hql reference: std/format.Lines -->
# `Lines`

In [`std/format`](../index.md).

```hql
type Lines : Format
```

The format of text framed one record per line: read, each line is one
`String`; written, each element of a collection is one line:
`[1, 2] | lines` is `"1\n2"`.
<!-- hql reference: end -->
