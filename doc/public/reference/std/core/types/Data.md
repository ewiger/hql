<!-- hql reference: std/core.Data -->
# `Data`

In [`std/core`](../index.md).

```hql
type Data = union { Scalar, Seq<Data>, Map<String, Data> }
```

An open tree whose keys belong to whoever wrote them: what a YAML or
JSON document is. It is a union of exactly three alternatives: a
`Scalar` leaf, a sequence of trees, or a `String`-keyed map of trees.

```hql
fn Data<F : Format>(text: String, as: Type<F>) : Data
```

`text` parsed in the format `as`: `Data("[1, 2]", Json)` is `[1, 2]`,
and `s | Data(Yaml)` is one call. The format is one whose decoder reads
`Data`: `Json`, `JsonLines`, `Yaml` or `Csv`. A framed format,
`JsonLines` or `Csv`, gives the list of its records; `Json` or `Yaml`
gives one document. Text that does not parse fails at run time.
<!-- hql reference: end -->
