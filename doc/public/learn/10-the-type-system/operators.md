# Operators

The operators of the language resolve through the **core traits**, which
are in scope in every program. `+` resolves `Add` for its operand type,
`==` resolves `Eq`, `<` resolves `Ord`, `xs[i]` resolves `Index` and
`xs[a:b]` resolves `Slice`. The built-in types implement the traits they
have natively; a type of the program implements one to take the operator
for its own values, and changes nothing else.

| Operator | Trait | Function |
| --- | --- | --- |
| `a + b` | `Add` | `add(self, other: Self) : Self` |
| `a - b` | `Sub` | `sub(self, other: Self) : Self` |
| `a * b` | `Mul` | `mul(self, other: Self) : Self` |
| `a / b` | `Div` | `div(self, other: Self) : Self` |
| `a % b` | `Rem` | `rem(self, other: Self) : Self` |
| `a == b`, `a != b` | `Eq` | `eq(self, other: Self) : Bool` |
| `a < b` and the other comparisons, `sort`, `min`, `max` | `Ord` | `compare(self, other: Self) : Ordering` |
| `xs[i]` | `Index<T>` | `at(self, position: Int) : Option<T>` |
| `xs[a:b]` | `Slice` | `slice(self, range: Range) : Self` |

`and`, `or` and `not` are language-defined and resolve through nothing, and
so are `|`, `=` and `:`.

## Arithmetic

An arithmetic operator takes two values of one type. For `Int`, `Float` and
`Decimal` that is built in, except that an `Int` beside a `Decimal` widens
to one; `+` over two strings is concatenation, since `String` implements
`Add`. Any other type resolves the operator through its trait:

```hql
type Money { cents : Int }
impl Sub for Money {
    fn sub(self, other: Money) : Money = Money(cents = self.cents - other.cents)
}
impl Mul for Money {
    fn mul(self, other: Money) : Money = Money(cents = self.cents * other.cents)
}
[(Money(cents = 500) - Money(cents = 120)).cents, (Money(cents = 3) * Money(cents = 4)).cents]
```

```text
[380, 12]
```

A type without the implementation does not take the operator, and the
message names the trait:

```hql
type Vec2 { x : Int, y : Int }
Vec2(x = 1, y = 2) + Vec2(x = 3, y = 4)
```

```text
type error: addition requires two Int, two Float or two Decimal operands, or two values of a type implementing `Add`, not Vec2
 --> <expression>:2:1
  |
2 | Vec2(x = 1, y = 2) + Vec2(x = 3, y = 4)
  | ^^^^^^^^^^^^^^^^^^
```

An operator is a function value where a function is expected, so
`reduce(0, +)` sums a list and `reduce("", +)` joins strings:

```hql
[[1, 2, 3] | reduce(0, +), ["a", "b"] | reduce("", +)]
```

```text
[6, "ab"]
```

## Equality

`==` over a type implementing `Eq` calls its `eq`. Without one, equality is
the type's own rule: a record field by field, a list position by position,
a set when each holds an equal of every element of the other, a map key by
key. A type that wants another equality declares it:

```hql
type Email { address : String }
impl Eq for Email {
    fn eq(self, other: Email) : Bool = self.address.lower() == other.address.lower()
}
[Email(address = "Ada@Example.org") == Email(address = "ada@example.org"), Email(address = "a") != Email(address = "b")]
```

```text
[true, true]
```

## Order

A type implementing `Ord` supplies `compare`, which gives an `Ordering`,
`Less`, `Equal` or `Greater`, and with it the type is `Orderable`: `<` and
the other comparisons, `sort`, `min` and `max` take it, and it may be a
`SortedMap` key.

```hql
type Version { major : Int, minor : Int }

impl Ord for Version {
    fn compare(self, other: Version) : Ordering =
        compare(self.major * 100 + self.minor, other.major * 100 + other.minor)
}

versions = [Version(major = 1, minor = 10), Version(major = 1, minor = 2), Version(major = 0, minor = 9)]
[versions | sort | map(v => fmt("{}.{}", v.major, v.minor)), [Version(major = 1, minor = 2) < Version(major = 1, minor = 10)] | map(str)]
```

```text
[["0.9", "1.2", "1.10"], ["true"]]
```

A record without `Ord` has no order of its own. `sort` over it is a type
error, and the message says to sort with an explicit key, `sort(by = v => v.minor)`,
which needs only the key's order.

## Positions

`xs[i]` over a type implementing `Index<T>` is `xs.at(i)`: the element at
the position, or `none` past the end. The standard library implements it
for sequences, so a `List` and a `String` take it, and a program's type
takes it with one implementation, as [generics](generics.md) did for a
`Ring<T>`.

```hql
[["a", "b", "c"][1], ["a", "b", "c"][5]]
```

```text
["b", none]
```

A `Set` has no positions and a `Map` has keys, and neither implements
`Index`, so the bracket over them is a type error.

## Parts

`xs[a:b]` over a type implementing `Slice` is a part of the value, of the
value's own type. A bound left out is open, and a negative bound counts
from the end:

```hql
["hello"[1:3], "hello"[-2:]]
```

```text
["el", "lo"]
```

The bracket holds either an `Int`, a position, or a `Range`, a slice, and
the type of what is inside decides which. `Range(start = 1, end = 3)` is
the value a slice takes, and a type that says what a part of its values is
implements `Slice` for them:

```hql
type Playlist { name : String, tracks : List<String> }

impl Slice for Playlist {
    fn slice(self, range: Range) : Playlist = Playlist(name = self.name, tracks = self.tracks[range])
}

mix = Playlist(name = "mix", tracks = ["first", "second", "third"])
mix[1:].tracks
```

```text
["second", "third"]
```

## The core traits are owned by the language

A program implements a core trait for its own types, and never for a
built-in one: `impl Add for Int` is a type error, since `Add` is already
implemented for `Int` in the standard library, natively. The implementations
the built-in types have are declared there as bodyless headers, and a
built-in type implements exactly what its headers name: `Bool` has `Eq` and
no `Add`, so `true + false` is a type error. A program declares no new
operator traits either; the operators of the grammar are the operators
there are.
