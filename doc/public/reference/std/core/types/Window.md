<!-- hql reference: std/core.Window -->
# `Window`

In [`std/core`](../index.md).

```hql
type Window<T>
```

One row's place in its ordered partition under `over`: the rows with
its key, in the order `over` gives them, and its position among them.
It has no fields: the `seq` module's window functions read it, and a
window does not leave the `into` it is handed to. See
[Windows](../../../../learn/20-olap/windows.md).
<!-- hql reference: end -->
