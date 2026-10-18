# Operation signatures compared

Status: audit of the design as of 2026-10-18; **decides nothing by itself**.
Decisions it points at are owned by the proposals and cards it names, and the
work is tracked by [issue 0018](../../issues/0018-operator-synonymy-audit.md).

HQL names its operations on several fronts at once: the implemented core that
`hql builtins` prints, [HQL-0004](../../proposals/HQL-0004/README.md),
[HQL-0006](../../proposals/HQL-0006/README.md) and
[HQL-0007](../../proposals/HQL-0007/README.md), the
[queries](queries.md) model, and the
[type interpretation](../../wiki/hql/type-interpretation.hmd) card. This
document puts every declared signature side by side and compares them by
**signature and result**, not by what the names sound like. Two operations are
the same operation when they accept the same input, take the same arguments,
return the same type and produce the same value. A resemblance between names
is not evidence of anything.

## Inventory

Signatures are written as the sources write them, with the pipe input as the
first parameter. `Input` means the result keeps the input's collection kind.

### Implemented (`hql builtins`)

```hql
size<T>(xs: Collection<T>)                              : Int
count<T>(xs: Collection<T>, value?: T)                  : Int
contains<T>(xs: Collection<T>, value: T)                : Bool
get<K, V>(xs: Map<K, V>, key: K)                        : Option<V>
sort<T>(xs: Collection<T>, by?: (T -> Orderable) | ((T, T) -> Ordering)) : List<T>
take<T>(xs: Collection<T>, n: Int)                      : List<T>   // a concrete Seq keeps its type
filter<T>(xs: Collection<T>, by: T -> Bool)             : Input
map<T, R>(xs: Collection<T>, by: T -> R)                : List<R>
typed<T, U>(xs: Collection<T>, as: Type<U>)             : Input<U>

uplinks(x: Doc | Hit<Card> | Collection<Doc | Hit<Card>>)              : List<Edge>
downlinks(x: Doc | Hit<Card> | Collection<Doc | Hit<Card>>)            : List<Edge>
expand(x: Graph | Doc | Collection<…>, depth?: Int, direction?: String) : Graph
graph(x: Graph | Doc | Collection<…>)                                   : Graph

table(x: Any) : Presentation
json(x: Any)  : Presentation
text(x: Any)  : Presentation

lexical<T : Doc>(xs: Collection<T>, query: String)  : Ranking<Card>
semantic<T : Doc>(xs: Collection<T>, query: String) : Ranking<Card>
```

### Proposed

```hql
// HQL-0004
reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R) : R
group<T, K>(xs: Collection<T>, by: T -> K)           : Set<Group<K, T>>
collect<T>(xs: Seq<T>)                                : List<T>   // renamed by HQL-0007

// HQL-0006
map<T, U>(xs: Stream<T>, by: T -> U)    : Stream<U>
filter<T>(xs: Stream<T>, by: T -> Bool) : Stream<T>
take<T>(xs: Stream<T>, n: Int)          : List<T>

// HQL-0007
collect<T>(xs: Stream<T>, n: Int) : Stream<List<T>>
collect<T>(xs: Seq<T>,    n: Int) : List<List<T>>
emit<T>(xs: Stream<Seq<T>>)       : Stream<T>
emit<T>(xs: Seq<Seq<T>>)          : List<T>
list<T>(xs: Seq<T>)               : List<T>

// queries model
first<T>(xs: Stream<T>)             : Option<T>
take<T>(xs: Stream<T>, n: Int)      : Seq<T>
drop<T>(xs: Stream<T>, n: Int)      : Stream<T>
drop<T>(xs: Seq<T>, n: Int)         : Seq<T>
map<T, U>(xs: Stream<T>, f: T -> U) : Stream<U>
map<T, U>(xs: Seq<T>, f: T -> U)    : Seq<U>
filter<T>(xs: Stream<T>, f: T -> Bool) : Stream<T>
filter<T>(xs: Seq<T>, f: T -> Bool)    : Seq<T>
size<T>(xs: Collection<T>)          : Int
count<T>(xs: Collection<T>, x: T)   : Int
sort<T : Orderable>(xs: Collection<T>) : Seq<T>
list<T>(xs: Seq<T>)                 : List<T>
next<T>(xs: Stream<T>)              : Option<(T, Stream<T>)>

// type interpretation card
somes<T>(xs: List<Option<T>>) : List<T>

// core.hql, over a single Option
map<T, U>(o: Option<T>, f: T -> U)          : Option<U>
filter<T>(o: Option<T>, p: T -> Bool)       : Option<T>
and_then<T, U>(o: Option<T>, f: T -> Option<U>) : Option<U>
```

## Comparison

### Same signature, same result: one operation under two names

These are the duplicates. Each has a decision already.

| Signatures | Decision |
| --- | --- |
| `collect<T>(Seq<T>) : List<T>` (HQL-0004) and `list<T>(Seq<T>) : List<T>` (HQL-0007, queries) | HQL-0007 renames materialization to `list`; HQL-0004's text and `examples/mapreduce/README.md` still say `collect` |
| `size<T>(Collection<T>) : Int` and `count<T>(Collection<T>) : Int` (the no-argument form) | kept as a declared alias, the only one; `std/collections.hql` says so |
| `graph(x) : Graph` and `expand(x, depth = 0) : Graph` over the same input union | `graph(x)` is `expand` with no traversal; [issue 0017](../../issues/0017-graph-projection-and-view.md) makes `graph` the presentation and renames the projection |

### Same signature, different result: not duplicates

The type alone cannot separate these. The value does.

| Signatures | Why they stay apart |
| --- | --- |
| `lexical` and `semantic`, both `(Collection<T : Doc>, String) : Ranking<Card>` | two ranking functions over one type; the hits differ |
| `uplinks` and `downlinks`, both `(…) : List<Edge>` | the edges leaving a card and the edges entering it |
| `table`, `json`, `text`, all `Any : Presentation` | three renderings of one value |

### Same name, different signatures: one operation declared inconsistently

These are not duplicates but disagreements. One name is given more than one
contract depending on which document is read.

| Name | Declarations | Difference |
| --- | --- | --- |
| `take` over a stream | `: List<T>` (HQL-0006); `: Seq<T>` (queries) | the model promises positions, the proposal and the implementation promise a materialized list |
| `sort` | `: List<T>` (std, builtins); `: Seq<T>` (queries) | same |
| `map` over a finite input | `Collection<T> → List<R>` (builtins); `Seq<T> → Seq<U>` (queries) | the model preserves the input kind, the implementation always materializes, even from a `Set` |
| `filter` | `: Input` (builtins, HQL-0004); `Seq<T> → Seq<T>` (queries) | agree in spirit; the model has no word for `Input` |
| `count` | `(Collection<T>, value?: T)` (builtins); `(Collection<T>, x: T)` (queries) | the alias form appears in one and not the other |
| `collect` | `Seq<T> → List<T>` (HQL-0004); `(Stream<T>, Int) → Stream<List<T>>` (HQL-0007) | the same word for materialization and for batching; HQL-0007 retires the first |
| `take` on a concrete `Seq` | `Collection<T> → List<T>`, with "a concrete Seq keeps its type" | a general rule now, but still stated in prose, not in the signature |

The queries model states its rule plainly: a transformation should preserve
every guarantee it can prove, and `Seq` is the weakest promise that is still
true. The implementation returns `List`, which is stronger, and `List : Seq`,
so no program that checks against the model fails against the implementation.
The disagreement is in what is written down, and it has one visible effect:
if `take` and `sort` already return `List<T>`, then `take(n) | list` and
`sort | list` are identities and `list` has nothing left to materialize except
the output of an operation that returns a bare `Seq<T>`, of which there is
none in the implementation today.

### Different signatures: resemblance only

Each pair was raised as a possible duplicate on the strength of its names. The
signatures show different operations.

| Pair | Signatures | Difference |
| --- | --- | --- |
| `somes` and `emit` | `List<Option<T>> → List<T>`; `Seq<Seq<T>> → List<T>` | different element types; `Option<T>` is not a `Seq<T>`, so no value accepts both |
| `take(n)` and `collect(n)` | `Stream<T> → List<T>`; `Stream<T> → Stream<List<T>>` | one closes the stream with a prefix, the other keeps it open in batches |
| `first` and `take(1)` | `Stream<T> → Option<T>`; `Stream<T> → List<T>` | absence against a list of at most one |
| `uplinks` and `expand(depth = 1, direction = "out")` | `→ List<Edge>`; `→ Graph` | edges against a graph that keeps why each node is present |
| `filter(by)` and `typed(as)` | `(T -> Bool) → Input`; `Type<U> → Input<U>` | `typed` narrows the element type; `filter` cannot |
| `reduce` and `group` | `Seq<T> → R`; `Collection<T> → Set<Group<K, T>>` | a fold against a partition |

### One name over several input types: overloads, not duplicates

`map` and `filter` are declared over `Collection<T>`, `Seq<T>`, `Stream<T>`
and a single `Option<T>`; `drop` over `Seq<T>` and `Stream<T>`; `collect(n)`
and `emit` over a stream and a finite sequence. In each case the operation is
the same and the output type follows the input's guarantee. This is the rule
the queries model states, and one name is right.

## Open and unresolved

- **`Seq` or `List`, `Input`, `map` from a `Set`, `take` on a concrete
  `Seq`.** [Issue 0018](../../issues/0018-operator-synonymy-audit.md) settles
  all four with one rule: the result keeps the input's type. `filter`,
  `drop`, `take` and `sort` bind the input type, `S : Seq<T>` or a weaker
  bound, and return `S`; `map` and `typed` bind its constructor,
  `F<_> : Iterable<_>`, and return `F<U>`, so a `Set` maps to its image
  `Set<U>` and a `Ranking` maps as the `Seq<Hit<T>>` it is. The signatures
  are not yet written into the sources above, and constructor parameters are
  not yet in the type system.
- **HQL-0004's `collect`.** The proposal still declares
  `collect<T>(Seq<T>) : List<T>`; it reads `list` once 0013 reaches it.
- **`somes` has no home.** Declared on the type interpretation card and in
  `doc/memory/`, and listed on no card that owns option operations.
- **`drop` is model only.** The queries model declares it; no proposal and no
  issue carries it, and [issue 0016](../../issues/0016-sql-parity-analysis.md)
  needs it for `OFFSET`.
