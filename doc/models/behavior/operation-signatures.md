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
first parameter. `S` binds the input's own type and `F<_>` its constructor, as
the [type system](../../wiki/hql/type-system.hmd) card states.

### Implemented (`hql builtins`)

```hql
size<T>(xs: Collection<T>)                              : Int
count<T>(xs: Collection<T>)                             : Int   // the alias of size
count<T>(xs: Collection<T>, value: T)                   : Int
contains<T>(xs: Collection<T>, value: T)                : Bool
get<K, V>(xs: Map<K, V>, key: K)                        : Option<V>
sort<T, U>(xs: Ranking<U>, by?: …)                      : Seq<T>    // T = Hit<U>
sort<S : Seq<T>, T>(xs: S, by?: …)                      : S
sort<T>(xs: Collection<T>, by?: (T -> Orderable) | ((T, T) -> Ordering)) : List<T>
take<S : Seq<T>, T>(xs: S, n: Int)                      : S
take<T : Orderable>(xs: Collection<T>, n: Int)          : List<T>
filter<S : Collection<T>, T>(xs: S, by: T -> Bool)      : S
map<F<_> : Iterable<_>, T, R>(xs: F<T>, by: T -> R)     : F<R>
of_type<F<_> : Iterable<_>, T, U>(xs: F<T>, type: Type<U>) : F<U>
is<T, U>(x: T, type: Type<U>)                           : Bool
filter<F<_> : Iterable<_>, T, U>(xs: F<T>, by: x => x | is(U)) : F<U>   // narrowing
group<T, K>(xs: Collection<T>, by: T -> K)              : Set<Group<K, T>>
reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R)   : R
reduce<T>(xs: Seq<T>, by: (T, T) -> T)                  : Option<T>
sum(xs: Collection<Int>)                                : Int
sum(xs: Collection<Float>)                              : Float
sum(xs: Collection<Decimal>)                            : Decimal
min<T : Orderable>(xs: Collection<T>)                   : Option<T>
max<T : Orderable>(xs: Collection<T>)                   : Option<T>

uplinks(x: Doc | Hit<Card> | Collection<Doc | Hit<Card>>)              : List<Edge>
downlinks(x: Doc | Hit<Card> | Collection<Doc | Hit<Card>>)            : List<Edge>
expand(x: Graph | Doc | Collection<…>, depth?: Int, direction?: String) : Graph

format<F : String>(x: Any, as: Type<F>, limit?: Int, renderer?: String) : F
text(x: Any)     : Text
markdown(x: Any) : Markdown
hmd(x: Any)      : Hmd
json(x: Any)     : Json
yaml(x: Any)     : Yaml
csv(x: Any)      : Csv
d2(x: Any)       : D2
mermaid(x: Any)  : Mermaid

lexical<T : Doc>(xs: Collection<T>, query: String)  : Ranking<Card>
semantic<T : Doc>(xs: Collection<T>, query: String) : Ranking<Card>
```

### Proposed

```hql
// HQL-0004 (reduce and group implemented, above)
collect<T>(xs: Seq<T>)                                : List<T>   // renamed by HQL-0007

// HQL-0006
map<F<_> : Iterable<_>, T, U>(xs: F<T>, by: T -> U) : F<U>
filter<S : Iterable<T>, T>(xs: S, by: T -> Bool)    : S
drop<S : Stream<T>, T>(xs: S, n: Int)               : S
take<S : Seq<T>, T>(xs: S, n: Int)                  : S
take<T>(xs: Stream<T>, n: Int)                      : List<T>

// HQL-0007
collect<T>(xs: Stream<T>, n: Int) : Stream<List<T>>
collect<T>(xs: Seq<T>,    n: Int) : List<List<T>>
emit<T>(xs: Stream<Seq<T>>)       : Stream<T>
emit<T>(xs: Seq<Seq<T>>)          : List<T>
List<T>(xs: Seq<T>)               : List<T>   // HQL-0016 retires `list`

// queries model
first<T>(xs: Stream<T>)                         : Option<T>
take<T>(xs: Stream<T>, n: Int)                  : List<T>
take<S : Seq<T>, T>(xs: S, n: Int)              : S
drop<S : Stream<T>, T>(xs: S, n: Int)           : S
map<F<_> : Iterable<_>, T, U>(xs: F<T>, f: T -> U) : F<U>
filter<S : Iterable<T>, T>(xs: S, f: T -> Bool) : S
size<T>(xs: Collection<T>)                      : Int
count<T>(xs: Collection<T>, x: T)               : Int
sort<S : Seq<T>, T : Orderable>(xs: S)          : S
sort<T : Orderable>(xs: Collection<T>)          : List<T>
List<T>(xs: Seq<T>)                             : List<T>
next<T>(xs: Stream<T>)                          : Option<(T, Stream<T>)>

// option-type card
somes<F<_> : Iterable<_>, T>(xs: F<Option<T>>) : F<T>

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
| `collect<T>(Seq<T>) : List<T>` (HQL-0004), `list<T>(Seq<T>) : List<T>` (HQL-0007) and the `List(xs)` constructor | the `List` constructor ([HQL-0016](../../proposals/HQL-0016/README.md)); HQL-0004's text still says `collect` |
| `size<T>(Collection<T>) : Int` and `count<T>(Collection<T>) : Int` (the no-argument form) | kept as a declared alias, the only one; `std/collections.hql` says so |
| `graph(x) : Graph` and `expand(x, depth = 0) : Graph` over the same input union | the `Graph` constructor ([HQL-0017](../../proposals/HQL-0017/README.md)); `expand` refuses depth `0` |

### Same signature, different result: not duplicates

The type alone cannot separate these. The value does.

| Signatures | Why they stay apart |
| --- | --- |
| `lexical` and `semantic`, both `(Collection<T : Doc>, String) : Ranking<Card>` | two ranking functions over one type; the hits differ |
| `uplinks` and `downlinks`, both `(…) : List<Edge>` | the edges leaving a card and the edges entering it |

### Same name, different signatures: one operation declared inconsistently

None. Every source above writes each name with one signature, in the form
[issue 0018](../../issues/0018-operator-synonymy-audit.md) settles: an
operation returns the type it was given. `filter`, `take`, `drop` and `sort`
bind the input's type `S` and return it; `map` binds its constructor `F<_>`
and returns `F<U>`. `take` over a stream and `take` or `sort` over a
`Collection<T>` that is not a `Seq` gain a guarantee the input lacked, and
return the `List<T>` that states it. A `Ranking<T>` sorted by another key
becomes a `Seq<Hit<T>>`, since the new order is not the retrieval's. `List`
after `take` or `sort` is an identity on a `List` and materializes a `Seq`.

### Different signatures: resemblance only

Each pair was raised as a possible duplicate on the strength of its names. The
signatures show different operations.

| Pair | Signatures | Difference |
| --- | --- | --- |
| `somes` and `emit` | `List<Option<T>> → List<T>`; `Seq<Seq<T>> → List<T>` | different element types; `Option<T>` is not a `Seq<T>`, so no value accepts both |
| `take(n)` and `collect(n)` | `Stream<T> → List<T>`; `Stream<T> → Stream<List<T>>` | one closes the stream with a prefix, the other keeps it open in batches |
| `first` and `take(1)` | `Stream<T> → Option<T>`; `Stream<T> → List<T>` | absence against a list of at most one |
| `uplinks` and `expand(depth = 1, direction = "out")` | `→ List<Edge>`; `→ Graph` | edges against a graph that keeps why each node is present |
| `filter(by)` and `of_type(U)` | `(T -> Bool) → S`; `Type<U> → F<U>` | `of_type(U)` is the shorthand for the narrowing `filter(x => x \| is(U))`; any other predicate keeps the element type |
| `reduce` and `group` | `Seq<T> → R`; `Collection<T> → Set<Group<K, T>>` | a fold against a partition |

### One name over several input types: overloads, not duplicates

`map` and `filter` are declared over `Collection<T>`, `Seq<T>`, `Stream<T>`
and a single `Option<T>`; `drop` over `Seq<T>` and `Stream<T>`; `collect(n)`
and `emit` over a stream and a finite sequence. In each case the operation is
the same and the output type follows the input's guarantee. This is the rule
the queries model states, and one name is right.

## Open and unresolved

- **HQL-0004's `collect`.** The proposal still declares
  `collect<T>(Seq<T>) : List<T>`; it reads `List` once
  [0013](../../issues/0013-typed-pipeline-operations.md) reaches it.
