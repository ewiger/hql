<!-- hql reference: std/core.Diagnostic -->
# `Diagnostic`

In [`std/core`](../index.md).

```hql
type Diagnostic : DataContract { phase : String kind : String severity : String disposition : String message : String origin? : String span : Data labels? : List<Data> help? : List<String> expected? : String found? : Data element? : Data bindings? : Data trace? : List<Data> source? : Data violations? : List<Violation> }
```

A diagnostic a run delivered, read back as data: its `phase`, `kind`,
`severity` and `disposition`, its `message`, and where it points,
`origin` and `span`, a `{start, end}` byte range. The fields a
producer fills where they apply are optional: `labels`, `help`,
`expected`, `found`, `element`, `bindings`, `trace`, `source` and the
`violations` of a contract read. `source(file(p), JsonLines) |
Diagnostic` reads what `--diagnostics-format json` wrote.
<!-- hql reference: end -->
