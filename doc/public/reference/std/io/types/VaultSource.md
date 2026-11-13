<!-- hql reference: std/io.VaultSource -->
# `VaultSource`

In [`std/io`](../index.md).

```hql
abstract type VaultSource : DataSource
```

A vault as a source: an endpoint that holds its cards, as a catalog
holds tables, with its members as fields rather than names. It is not a
`Seq<Card>` itself; its cards are. What the documents sit on, a
filesystem, a git revision, an HTTP server, is the concrete type's.
<!-- hql reference: end -->
