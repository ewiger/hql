<!-- hql reference: std/format.yaml -->
# `yaml`

In [`std/format`](index.md).

```hql
fn yaml<T>(x: Collection<T>) : String
fn yaml<T>(x: Stream<T>) : Stream<String>
fn yaml<T>(x: T) : String
```

`x` encoded as `Yaml`: `x | yaml` is `format(x, Yaml)`.
<!-- hql reference: end -->
