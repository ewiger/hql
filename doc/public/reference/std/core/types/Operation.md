<!-- hql reference: std/core.Operation -->
# `Operation`

In [`std/core`](../index.md).

```hql
abstract type Operation<I, O>
```

A function from `I` to `O` that also carries its shape: what an executor
may assume about splitting the step. A value of one is called as an
`I -> O` function is, and a lambda bound at one is an operation on equal
terms with `map` or `filter`.
<!-- hql reference: end -->
