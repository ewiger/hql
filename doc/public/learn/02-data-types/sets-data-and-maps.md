# Sets, Data and maps

Three kinds of value are written in braces, or hold keys and values, and they
are easy to mix up. They are different types with different jobs:

| Type | What it is | Written |
| --- | --- | --- |
| `Set<T>` | distinct values of one type, with no positions | `{1, 2, 3}` |
| `Data` | a tree of named fields of any types, the shape of JSON | `{title: "Dune", pages: 412}` |
| `Map<K, V>` | keys of one type, each to a value of one type | `Map(["dune"], [412])` |

## The colon tells braces apart

Braces holding values are a set; braces holding `key: value` entries are
`Data`. The colon is the only thing that tells them apart, and the empty forms
keep it: `{}` is the empty set and `{:}` the empty `Data`.

```hql
{3, 1, 3, 2}
```

```text
{3, 1, 2}
```

```hql
{title: "Dune", pages: 412}
```

```text
{pages: 412, title: "Dune"}
```

```hql
{}
```

```text
{}
```

```hql
{:}
```

```text
{:}
```

A set keeps each value once. A `Data` record prints its keys in order, so the
same tree always prints the same way.

`{}` has a definite meaning, the empty set, so it is not an empty record:

```hql
d : Data = {}
d
```

```text
type error: `d` is declared Data but its value is Set<Never>
 --> <expression>:1:12
  |
1 | d : Data = {}
  |            ^^
```

`{}` is a `Set<Never>`: a set with no elements, and so no element type yet. A
declared set type gives it one:

```hql
seen : Set<Int> = {}
seen | size
```

```text
0
```

## Data is not a map

`Data` is the type of values whose shape is not known to the program: what a
JSON or YAML document decodes to. A record's fields may hold different types,
a number beside a text beside a list, and nesting goes as deep as the
document does. It is not a hash map, and not a `Map<String, Any>`.

A `Map<K, V>` is a typed mapping: every key is a `K` and every value a `V`.
The constructor pairs a list of keys with a list of values:

```hql
Map(["dune", "emma"], [412, 474])
```

```text
Map {"dune": 412, "emma": 474}
```

```hql
Map(["dune", "emma"], [412, 474]).get("dune")
```

```text
412
```

So the three answer different needs: a set, *which values*; a map, *which
value for each key*, all of one type; `Data`, *whatever structure arrived*.

## `{:}` is Data, or a declared map

`{:}` has no type of its own. Where nothing says otherwise it is the empty
`Data`; where a map type is declared for it, it is that empty map:

| Written | Type | Value |
| --- | --- | --- |
| `{:}` with nothing around it | `Data` | the empty `Data` record, printed `{:}` |
| `d : Data = {:}` | `Data` | the same |
| `m : Map<String, Int> = {:}` | `Map<String, Int>` | an empty map, equal to `Map<String, Int>([], [])` |
| `m : SortedMap<String, Int> = {:}`, or `OrderedMap` | that map type | an empty map of that kind |
| `f({:})` where `f` takes a `Map<String, Int>` | `Map<String, Int>` | an empty map |

```hql
counts : Map<String, Int> = {:}
counts == Map<String, Int>([], [])
```

```text
true
```

`Data` is the default, and a declared `Map`, `OrderedMap` or `SortedMap` turns
it into that empty map. An empty literal carries no information of its own,
so the type around it decides: HQL infers a type while the literal says
enough, and asks for an explicit one where it does not.

This holds for the empty literal only. A non-empty keyed literal is always
`Data`, and a declared map type refuses it:

```hql
m : Map<String, Int> = {a: 1}
m
```

```text
type error: `m` is declared Map<String, Int> but its value is Data
 --> <expression>:1:24
  |
1 | m : Map<String, Int> = {a: 1}
  |                        ^^^^^^
```

A non-empty map is built with the constructor, `Map(["a"], [1])`.

## What is next

[Chapter 3](../03-collections.md) uses lists, sets and maps as collections, and
the steps they share. The rules this page follows are stated in
[HQL-0029](../../../proposals/HQL-0029/README.md) and its note on
[Data and type boundaries](../../../proposals/HQL-0029/data-and-type-boundaries.md).
