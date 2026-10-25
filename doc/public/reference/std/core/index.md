<!-- hql reference: std/core -->
# `std/core`

```hql
module core
```

The types every other module is written in: the scalars, the collection
types, the open tree `Data`, absence, failure, value order, the format
category and the core traits. A type sits here when a module below it
would otherwise import a module above it: `Data` is a tree of sequences
and maps, so the collection types are core types, and the operations
over them are the `collections` module's.

- [`Orderable`](types/Orderable.md) — A type whose values have a total order: of any two values of the type, one comes first or the two are equal.
- [`Scalar`](types/Scalar.md) — A leaf: a value with no parts of its own.
- [`Bool`](types/Bool.md) — The truth values `true` and `false`: a `Scalar` with equality and no order.
- [`Int`](types/Int.md) — A 64-bit whole number, ordered by value: `7 / 2` is `3`, and `7 % 2` is `1`.
- [`Float`](types/Float.md) — A 64-bit binary floating-point number, written with the suffix `f` (`2.5f`), ordered by value.
- [`Decimal`](types/Decimal.md) — An exact decimal number, written with a decimal point and no suffix (`2.50`), ordered by value: `0.1 + 0.2 == 0.3` is `true`.
- [`String`](types/String.md) — Text, ordered character by character: `"ab" + "cd"` is `"abcd"`, and `"hello"[1]` is `"e"`.
- [`Date`](types/Date.md) — A calendar date, ordered by time.
- [`Iterable`](types/Iterable.md) — A value whose elements can each be visited: the bound an operation states when it only visits each value.
- [`Collection`](types/Collection.md) — A finite multiset of occurrences, unordered, with no runtime representation.
- [`Stream`](types/Stream.md) — An ordered, possibly unending run of values `x0, x1, x2, ...`.
- [`Seq`](types/Seq.md) — A finite stream: a collection in which every occurrence has exactly one position in `0 .. size(c) - 1`.
- [`List`](types/List.md) — A sequence of materialized occurrences; repeated traversal preserves positions.
- [`Set`](types/Set.md) — Materialized distinct values under HQL equality: `count(s, x) <= 1`.
- [`Map`](types/Map.md) — Materialized associations from distinct keys of `K` to values of `V`; not a `Collection`.
- [`OrderedMap`](types/OrderedMap.md) — A `Map` whose keys keep their construction positions: its keys are a `Seq<K>`.
- [`SortedMap`](types/SortedMap.md) — An `OrderedMap` whose key positions follow the intrinsic total order of `K`.
- [`Data`](types/Data.md) — An open tree whose keys belong to whoever wrote them: what a YAML or JSON document is.
- [`TypeContract`](types/TypeContract.md) — The supertype of a closed contract: a structural type declared `type C : TypeContract = {…}` has exactly its declared fields, and is validated as a contract.
- [`DataContract`](types/DataContract.md) — The supertype of an open contract: a structural type declared `type D : DataContract = {…}` is a view over `Data` with at least its declared fields, and is validated as a contract.
- [`Option`](types/Option.md) — An optional value of `T`: present, holding one `T`, or absent.
- [`Result`](types/Result.md) — A computation that may have failed: `ok(value)` with a `T`, or `err(error)` with an `E`.
- [`Ordering`](types/Ordering.md) — The result of `compare(a, b)`: `Less`, `Equal` or `Greater`.
- [`Type`](types/Type.md) — A type written as an argument: `is(Card)`, `of_type(Card)`.
- [`Format`](types/Format.md) — The category of formats: how bytes are documents.
- [`Is`](types/Is.md) — What a predicate that tests its parameter's type, `x => x | is(U)`, gives: a `Bool` that, where true, holds the parameter at `U`.
- [`Eq`](types/Eq.md) — Equality: `a == b` and `a != b` over a type implementing `Eq` call its `eq`.
- [`Ord`](types/Ord.md) — Order: `a < b` and the other comparisons over a type implementing `Ord` call its `compare`.
- [`Add`](types/Add.md) — Addition: `a + b` over a type implementing `Add` is `a.add(b)`.
- [`Sub`](types/Sub.md) — Subtraction: `a - b` over a type implementing `Sub` is `a.sub(b)`.
- [`Mul`](types/Mul.md) — Multiplication: `a * b` over a type implementing `Mul` is `a.mul(b)`.
- [`Div`](types/Div.md) — Division: `a / b` over a type implementing `Div` is `a.div(b)`.
- [`Rem`](types/Rem.md) — Remainder: `a % b` over a type implementing `Rem` is `a.rem(b)`.
- [`Range`](types/Range.md) — A window of positions, what a slice takes: `xs[a:b]` is `xs[Range(start = a, end = b)]`, and a bound left out is absent.
- [`Index`](types/Index.md) — A position: `xs[i]` over a type implementing `Index` is `xs.at(i)`, the element there or its absence.
- [`Slice`](types/Slice.md) — A part: `xs[a:b]` over a type implementing `Slice` is `xs.slice(Range(start = a, end = b))`, a value of the type itself.
- [`Validator`](types/Validator.md) — A check a value passes: a trait extending `Validator` is a validator type.
- [`Operation`](types/Operation.md) — A function from `I` to `O` that also carries its shape: what an executor may assume about splitting the step.
- [`Elementwise`](types/Elementwise.md) — An `Operation` whose result for each element depends on that element alone, so it may run over any split of its input: `map` and `filter` are elementwise.
- [`Partition`](types/Partition.md) — An `Operation` that regroups its input by a key, each group running independently and completing in any order: `group` is one.
- [`Reduction`](types/Reduction.md) — An `Operation` that collapses its whole input to one value through a combiner the runtime holds, so each split may be reduced and the results combined: `size` combines by addition, `contains` by disjunction.
- [`Barrier`](types/Barrier.md) — An `Operation` that needs its whole input before any output, so it is not split: `sort`, `take` and `reduce` are barriers.
- [`Pair`](types/Pair.md) — Two values held together, each at its own type: what `zip`, `enumerate`, `entries` and `join` produce, and what `Pair(first = a, second = b)` builds.
- [`Violation`](types/Violation.md) — One reason a value does not read as a contract, as `validate(T)` reports it.
- [`Invalid`](types/Invalid.md) — A value that did not read as a contract, and every reason why: its `contract`, the `value`, and the `violations`.

## Implementations

```hql
impl Eq for Bool
impl Eq for Int
impl Eq for Float
impl Eq for Decimal
impl Eq for String
impl Eq for Date
```

`Bool`, `Int`, `Float`, `Decimal`, `String` and `Date` compare with `==`
by value.

```hql
impl Ord for Int
impl Ord for Float
impl Ord for Decimal
impl Ord for String
impl Ord for Date
```

`Int`, `Float` and `Decimal` are ordered by value, `String` character by
character, and `Date` by time.

```hql
impl Add for Int
impl Add for Float
impl Add for Decimal
impl Add for String
```

`+` adds `Int`s, `Float`s and `Decimal`s, and concatenates two
`String`s: `"ab" + "cd"` is `"abcd"`.

```hql
impl Sub for Int
impl Sub for Float
impl Sub for Decimal
```

`-` subtracts `Int`s, `Float`s and `Decimal`s.

```hql
impl Mul for Int
impl Mul for Float
impl Mul for Decimal
```

`*` multiplies `Int`s, `Float`s and `Decimal`s.

```hql
impl Div for Int
impl Div for Float
impl Div for Decimal
```

`/` divides `Int`s, `Float`s and `Decimal`s: `7 / 2` is `3`.

```hql
impl Rem for Int
impl Rem for Float
impl Rem for Decimal
```

`%` is the remainder of `Int`s, `Float`s and `Decimal`s: `7 % 2` is `1`.

```hql
impl<T> Index<T> for Seq<T>
impl Index<String> for String
impl Index<Data> for Data
```

`xs[i]` over every sequence, a `List` among them, is the element at
`i`; over a `String` it is the character there, as text (`"hello"[1]`
is `"e"`); over `Data` it is the document's own element.

```hql
impl<T> Slice for Seq<T>
impl Slice for String
impl Slice for Data
```

`xs[a:b]` over every sequence, a `String` and `Data` is the part of the
same type: `[1, 2, 3][1:]` is `[2, 3]`, and `"hello"[1:3]` is `"el"`.
<!-- hql reference: end -->
