<!-- hql reference: std/io/logging.LogLevel -->
# `LogLevel`

In [`std/io/logging`](../index.md).

```hql
abstract type LogLevel
```

How severe an event is. The five levels are ordered from least to most
severe: `Trace`, `Debug`, `Info`, `Warn`, `Error`. A level is tested
with `e.level | is(Error)`, and a threshold is passed as a type:
`level = Info`.
<!-- hql reference: end -->
