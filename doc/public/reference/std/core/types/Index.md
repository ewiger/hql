<!-- hql reference: std/core.Index -->
# `Index`

In [`std/core`](../index.md).

```hql
trait Index<T>
```

A position: `xs[i]` over a type implementing `Index` is `xs.at(i)`, the
element there or its absence. `T` is the element type, which each
implementation names: `E : Index<T>` leaves `T` to the type `E` is.

## Members

```hql
fn at(self, position: Int) : Option<T>
```
<!-- hql reference: end -->
