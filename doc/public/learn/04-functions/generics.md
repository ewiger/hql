# Generic functions

A function written once for every type names a **type parameter**, one capital
letter in angle brackets after its name. Each call fills it from its
arguments:

```hql
fn first<T>(xs: List<T>) : Option<T> = xs | reduce((a, b) => a)
first(["Dune", "Emma"])
```

```text
"Dune"
```

The same function over a list of numbers gives an `Option<Int>`:

<!-- run: check -->
```hql
fn first<T>(xs: List<T>) : Option<T> = xs | reduce((a, b) => a)
first([412, 280])
```

```text
Option<Int>
```

## Bounds

Inside the body `T` is a type of its own: the body may rely only on what the
parameter promises. A **bound**, `T : Orderable`, promises an order, so the
body may sort or find a minimum, and each call is checked against the bound:

```hql
fn shortest<T : Orderable>(xs: List<T>) : Option<T> = xs | min
shortest([412, 280, 310])
```

```text
280
```

```hql
fn shortest<T : Orderable>(xs: List<T>) : Option<T> = xs | min
shortest([{1}, {2}])
```

```text
type error: `shortest` needs T : Orderable, and T is Set<Int> here
 --> <expression>:2:1
  |
2 | shortest([{1}, {2}])
  | ^^^^^^^^^^^^^^^^^^^^
```

A bound is a type or a trait. [Traits](../17-the-type-system/traits.md)
declares traits of a program's own and bounds by them.
