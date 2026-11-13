<!-- hql reference: std/io.Message -->
# `Message`

In [`std/io`](../index.md).

```hql
type Message<T> { value : T position : Position key? : Data time? : Date headers : Data }
```

One element of a stream with where and when it sits in it: its `value`,
its `position`, an optional `key` and `time`, and its `headers`.
<!-- hql reference: end -->
