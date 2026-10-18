# A tour of HQL

Every program on this page runs with the current release. Run one with
`hql eval '<program>'`, or ask for its type without running it with
`hql check -` and the program on standard input.

## Values

The scalar types are `Int`, `Float`, `String` and `Bool`. `hql check` reports
the type of a program without evaluating it.

| Program | Value | Type |
| --- | --- | --- |
| `1 + 2` | `3` | `Int` |
| `1.5 + 2.25` | `3.75` | `Float` |
| `"hello"` | `hello` | `String` |
| `true` | `true` | `Bool` |

A program is type-checked as a whole before it runs. A type error is reported
with its byte range, and nothing is evaluated:

```text
$ hql eval '1 + true'
type error: addition requires two Int or two Float operands
 --> <expression>:1:5
  |
1 | 1 + true
  |     ^^^^
```

## Collections

A list keeps every element and its position. A set keeps one occurrence of each
distinct value. A map pairs keys with values.

```hql
[1, 2, 3]                      // List<Int>
Set([1, 2, 2, 3])              // Set<Int>: [1, 2, 3]
Map(["robin", "owl"], [1, 2])  // Map<String, Int>
```

Looking up a key yields an `Option`, because the key may be absent:

```hql
counts = Map(["robin", "owl"], [1, 2])
counts | get("owl")            // 2, of type Option<Int>
```

`counts | get("swallow")` is `none` rather than an error. A record literal
such as `{ name: "owl", count: 2 }` is `Data` — the tree type YAML and JSON
documents share.

## Bindings

A line `name = expression` binds a name; the program's value is its last
expression. Types are inferred, and an annotation states a view of the value:

```hql
xs : Collection<Int> = [1, 2, 2]
xs | count(2)                  // 2
```

## Pipelines and lambdas

`xs | f(a)` means `f(xs, a)`. A lambda, `x => …`, is written where a step takes
a function:

```hql
[1, 2, 3] | map(x => x + 1)    // [2, 3, 4]
[3, 1, 2] | sort               // [1, 2, 3]
["a", "b", "a"] | count("a")   // 2
[1, 2, 3] | contains(2)        // true
```

The steps are ordinary functions, so `size([1, 2, 3])` and `[1, 2, 3] | size`
are the same program. `hql builtins` lists them all with their signatures.

## Cards

Against a vault, `cards` is the set of its cards. The
[birds example](https://github.com/ewiger/hql/tree/main/examples/birds) is a
small one; pass it with `--vault`:

```sh
hql --vault examples/birds/wiki eval 'cards | count'    # 6
```

A card's front matter is its `metadata`, and `filter` keeps the cards a
predicate accepts. Filtering a set yields a set; sorting it yields a list:

```hql
cards
| filter(c => c.metadata.category == "bird")
| sort(by = c => c.name)
| map(c => c.title)
```

```text
[Barn owl, Common raven, European robin]
```

The type of that program is `List<String>`, determined by type checking before
evaluation.

## Presenting a result

A program returns a value. How it is shown is a last, separate step that writes
it in a presentation format — `text`, `markdown`, `json` and others:

```hql
cards
| filter(c => c.metadata.category == "bird")
| sort(by = c => c.name)
| map(c => c.title)
| text
```

```text
value
--------------
Barn owl
Common raven
European robin
```

Without one, the host chooses. `--format json` before the command emits the
type, the value and the report queue for another program to read.

## Next

- [Queries in documents](documents.md): put a query in a card.
- [Retrieval and graphs](retrieval.md): rank cards and walk their links.
- [The expression grammar](../models/behavior/expressions.md): everything the
  parser accepts.
