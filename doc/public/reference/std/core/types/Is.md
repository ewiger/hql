<!-- hql reference: std/core.Is -->
# `Is`

In [`std/core`](../index.md).

```hql
type Is<T>
```

What a predicate that tests its parameter's type, `x => x | is(U)`,
gives: a `Bool` that, where true, holds the parameter at `U`. A
parameter of type `T -> Is<U>` takes only such a predicate, and binds
`U` to the type it tests.
<!-- hql reference: end -->
