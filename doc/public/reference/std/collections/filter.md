<!-- hql reference: std/collections.filter -->
# `filter`

In [`std/collections`](index.md).

```hql
elementwise fn filter<F<_> : Iterable<_>, T, U>(self: F<T>, by: T -> Is<U>) : F<U>
elementwise fn filter<T, U>(self: Option<T>, by: T -> Is<U>) : Option<U>
```

The elements for which a type test holds, each at the tested type. A
`by` of type `T -> Is<U>`, such as `x => x | is(Int)`, makes the result
an `F<U>`. Over an `Option<T>`, the result is the value when it is
present and the test holds, and `none` otherwise.

```hql
elementwise fn filter<S : Iterable<T>, T>(self: S, by: T -> Bool) : S
elementwise fn filter<T>(self: Option<T>, by: T -> Bool) : Option<T>
```

The elements for which `by` is `true`, in their order, in a collection
of the input's type. Over an `Option<T>`, the result is the value when
it is present and `by` holds, and `none` otherwise.
<!-- hql reference: end -->
