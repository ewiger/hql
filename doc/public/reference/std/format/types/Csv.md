<!-- hql reference: std/format.Csv -->
# `Csv`

In [`std/format`](../index.md).

```hql
type Csv : Format
```

The comma-separated values format, with a header row: read, each row is
one `Data` value whose fields, named by the header, hold `String`s;
written, a collection of records or a `Table` is the header and one row
per element.
<!-- hql reference: end -->
