<!-- hql reference: std/format.Markdown -->
# `Markdown`

In [`std/format`](../index.md).

```hql
type Markdown : Format
```

The Markdown format: read, the whole input is one `String`; written, a
collection is a list, one item per element:
`[1, 2] | markdown` is `"- 1\n- 2"`.
<!-- hql reference: end -->
