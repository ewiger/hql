# 3. Collections

A `Data` list holds trees. Most collections in HQL hold values of one known
type, and say what they promise about order and repetition. This chapter meets
them, and the steps every collection shares.

## Lists, sets and maps

A **list** keeps every element at its position, duplicates included:

```hql
[412, 280, 310, 145]
```

```text
[412, 280, 310, 145]
```

A **set** keeps each distinct value once and promises no order. Braces around
values, rather than fields, write one:

```hql
{"novel", "essay", "novel", "poetry"}
```

```text
{"novel", "essay", "poetry"}
```

A value prints as the literal that writes it, so a set prints in braces and a
string inside a collection keeps its quotes. `hql check` writes a program's
type without running it:

<!-- run: check -->
```hql
{"novel", "essay", "novel", "poetry"}
```

```text
Set<String>
```

A **map** pairs each key with one value. `Map(keys, values)` builds one, and
`get` looks a key up — an `Option`, since the key may be missing:

```hql
pages = Map(["Dune", "Frankenstein"], [412, 280])
get(pages, "Dune")
```

```text
412
```

## A step keeps its input's shape

Steps work on any collection, and each returns the kind of collection it was
given wherever it can: a list stays a list, a set stays a set.

```hql
[412, 280, 310, 145] | sort | take(2)
```

```text
[145, 280]
```

```hql
[412, 280, 310, 145] | drop(1)
```

```text
[280, 310, 145]
```

A set has no order of its own, so `sort` gives it one and the result is a
list. And because a set keeps each value once, mapping it can make it smaller:

```hql
{3, 1, 2} | map(n => n * 0)
```

```text
{0}
```

```hql
[3, 1, 2] | map(n => n * 0)
```

```text
[0, 0, 0]
```

`size`, `count` and `contains` ask about a whole collection:

```hql
genres = ["novel", "essay", "novel"]
[genres | size, genres | count("novel")]
```

```text
[3, 2]
```

## Ordering

`sort` orders by a value's own order. `sort(by = key)` orders by a key, and a
function of two values that returns an `Ordering` orders by comparison:

```hql
[412, 280, 310] | sort((a, b) => compare(b, a))
```

```text
[412, 310, 280]
```

`min` and `max` find the least and greatest value. Over an empty list there is
none, and the type says so: the result is an `Option`.

```hql
[412, 280, 310] | min
```

```text
280
```

```hql
[] | List<Int> | min
```

```text
none
```

`[] | List<Int>` is a **constructor** written as a step: `x | T` is `T(x)`. A
constructor builds a value of its type — `Set(xs)`, `List(xs)`,
`Map(keys, values)` — and the pipeline form reads left to right:

```hql
["novel", "essay", "novel", "poetry"] | Set | size
```

```text
3
```

## Positions are promises

A list, a ranking and a sequence of cards have positions; a set does not. A
step that needs positions refuses a set, and says what to write instead:

```hql
{3, 1, 2} | drop(1)
```

```text
type error: `drop` needs values in a stable order, and Set<Int> has none; sort it first
 --> <expression>:1:13
  |
1 | {3, 1, 2} | drop(1)
  |             ^^^^^^^
```

The checker knows each collection's promises from its type, so this is found
before anything runs, not after a wrong answer.

## What is next

Steps take functions — `n => n * 0`, `(a, b) => compare(b, a)`.
[Chapter 4](04-functions.md) is about functions themselves: naming them,
passing them, and making them generic.
