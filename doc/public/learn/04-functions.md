# 4. Functions

A function is a value like any other. It can be bound to a name, passed to a
step, returned from another function, and written once for every type it
works on.

## Lambdas and their types

`n => n * 2` is a lambda. Written as an argument, it takes its parameter's
type from the step it is passed to. Written on its own, it states the type:

```hql
double = (n: Int) => n * 2
[412, 280] | map(double)
```

```text
[824, 560]
```

A function's type is written with an arrow: `Int -> Int`, `(Int, Int) -> Int`.

<!-- run: check -->
```hql
double = (n: Int) => n * 2
double
```

```text
Int -> Int
```

Piping into a function calls it: `x | f` is `f(x)`.

```hql
double = (n: Int) => n * 2
280 | double
```

```text
560
```

## Declared functions

`fn` declares a function with typed parameters and a result type. Its name is
bound before its body is checked, so it may call itself:

```hql
fn factorial(n: Int) : Int = 1 given n <= 1 otherwise n * factorial(n - 1)
[1, 2, 3, 4, 5] | map(factorial)
```

```text
[1, 2, 6, 24, 120]
```

A function may take a function, and return one. The returned function keeps
the values it captured:

```hql
fn adder(k: Int) : Int -> Int = n => n + k
fn twice(f: Int -> Int, x: Int) : Int = f(f(x))
twice(adder(3), 30)
```

```text
36
```

A last parameter written `...T` collects any number of trailing arguments into
a list:

```hql
fn total(xs: ...Int) : Int = xs | sum
[total(), total(1, 2), total(1, 2, 3, 4)]
```

```text
[0, 3, 10]
```

## Generic functions

A function written once for every type names a **type parameter**, one capital
letter in angle brackets. Each call fills it from its arguments:

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

Inside the body `T` is a type of its own: the body may rely only on what the
parameter promises. `T : Orderable` promises an order, so the body may sort or
find a minimum, and each call checks that the promise holds:

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

## Aliases

Most small functions are a pipeline fragment with a name. `:=` defines one, and
the checker writes its type: the input is the most general one the fragment
accepts, and each parameter takes the type of the step argument it fills.

```hql
top(n) := sort | take(n)
[412, 280, 310, 145] | top(2)
```

```text
[145, 280]
```

`hql check` prints the signature it wrote:

<!-- run: check -->
```hql
top(n) := sort | take(n)
1
```

```text
top : (Collection<T>, Int) -> List<T> where T : Orderable
Int
```

## Holes

A call with `_` for an argument is a function of the arguments left open — a
**partial application**:

```hql
fn add(a: Int, b: Int) : Int = a + b
[1, 2, 3] | map(add(10, _))
```

```text
[11, 12, 13]
```

## Absence, once more

`Option<T>` is how a function says a value may be missing. `some(x)` and
`none` build one, `match` takes one apart, and `somes` keeps the present values
of a collection:

```hql
fn rating_of(r: Int) : Option<Int> = some(r) given r > 0 otherwise none
[5, 0, 4] | map(rating_of) | somes
```

```text
[5, 4]
```

## What is next

With functions and collections in hand, [chapter 5](05-questions.md) asks the
questions SQL asks: where, order by, group by, and the aggregates.
