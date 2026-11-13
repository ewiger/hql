<!-- hql reference: std/core.Elementwise -->
# `Elementwise`

In [`std/core`](../index.md).

```hql
abstract type Elementwise<I, O> : Operation<I, O>
```

An `Operation` whose result for each element depends on that element
alone, so it may run over any split of its input: `map` and `filter`
are elementwise. Piped a collection of `I`, it runs over each element,
and its body reads its one element and nothing else:
`long : Elementwise<Int, Bool> = n => n > 300`.
<!-- hql reference: end -->
