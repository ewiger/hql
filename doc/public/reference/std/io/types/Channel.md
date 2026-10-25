<!-- hql reference: std/io.Channel -->
# `Channel`

In [`std/io`](../index.md).

```hql
type Channel<T> : {Readable, Writable} { name : String }
```

An endpoint of values rather than bytes, held in memory for one
execution scope: a program run, or one cell. It holds `T`, so it takes
no format, and its values are read in the order they were written. One
`name` is one channel within a scope.

```hql
fn Channel<T>(name: String) : Channel<T>
```

The channel named `name` in the current scope, holding `T`. The element
type is written at the call, `Channel<LogEvent>("logs")`, since nothing
written to the channel yet determines it.
<!-- hql reference: end -->
