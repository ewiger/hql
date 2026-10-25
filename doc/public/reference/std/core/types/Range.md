<!-- hql reference: std/core.Range -->
# `Range`

In [`std/core`](../index.md).

```hql
type Range { start? : Int end? : Int }
```

A window of positions, what a slice takes: `xs[a:b]` is
`xs[Range(start = a, end = b)]`, and a bound left out is absent. A
negative bound counts from the end: `"hello"[-2:]` is `"lo"`.
<!-- hql reference: end -->
