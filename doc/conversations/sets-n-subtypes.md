# Sets, subtypes, and the pipe boundary

A design conversation on `<:` as the single subtype relation, set-of-supertypes
syntax, edges as ordinary data, generic contracts such as `Renderable[T]`, and
what typing the pipe operator does and does not carry.

## Converging surface

This is converging into a fairly coherent language surface, and it also exposes
an important distinction between type inheritance and capability/behavior.

### 1. `<:` plus collection syntax

Keep `<:` exactly as the subtype/refinement operator.

```hql
type Doc {
    title: String
    header: Data
    body: Data
}

type Card <: Doc {
    id: CardId
}

type HmdCard <: Card {
    frontmatter: Data
}
```

And allow multiple direct supertypes as:

```hql
type X <: {A, B, C}
```

This reads very naturally as: *X is a subtype of every type in this set.* So it
is effectively the type-system meaning people often write as `X <: A & B & C`,
without consuming `&`.

The example `HmdCard <: {Card, Doc}` is valid but redundant if `Card <: Doc`,
because the type graph already implies `HmdCard <: Card <: Doc`. The compiler
can simply normalize the supertype set and warn about redundant ancestry if
desirable.

And the collection syntax can be pleasantly conventional:

```hql
[a, b, c]            // List<T> / Seq<T>
(a, b, c)            // tuple
{a, b, c}            // Set<T>
{name: "alice",
 age: 42}            // map / record
```

There is no real ambiguity between set and map: `{A, B}` is a set, while
`{a: A, b: B}` is a keyed structure. And inside `type X <: {A, B}` the grammar
knows that the set is a supertype set. That feels much nicer than inventing a
separate intersection punctuation.

### 2. `[[foo]] { ... }` is really an edge expression

```hql
[[foo]] {
    color: red
    relation: parent
}
```

with the semantic expansion:

```hql
[[this]] -> [[foo]] {
    color: red
    relation: parent
}
```

That gives `[[foo]]` two related meanings depending on context. As a value,
`x = [[foo]]` denotes/resolves a reference. But as an authored link in a card,
`[[foo]]` has an implicit source: `[[this]] -> [[foo]]`.

Then `{...}` is not special graph syntax at all. It is simply the data attached
to the edge. Conceptually:

```hql
[[alice]] -> [[bob]] {
    relation: parent
    confidence: 0.9
}
```

constructs something approximately like:

```hql
Edge {
    source: [[alice]]
    target: [[bob]]
    data: {
        relation: parent
        confidence: 0.9
    }
}
```

So `relation`, `color`, etc. should not go into the language grammar. The graph
machinery only knows that an edge has data. That preserves the extension
boundary extremely well.

### 3. This suggests a very small graph model

The graph extension could introduce something along these lines:

```hql
type Edge[S, T] {
    source: S
    target: T
    data: Data
}

type Graph[N, E]
```

Then HMD specializes it:

```hql
type Link[S, T] <: Edge[S, T]

type HmdGraph <: Graph[Card, Link]
```

while the knowledge extension can independently have things such as:

```hql
type Relation[S, T] <: Edge[S, T]

type KnowledgeGraph <: Graph[Concept, Relation]
```

That distinction is useful. An HMD link `[[alice]] -> [[project-x]]` is an edge
in the document/card graph. A semantic statement `WorksOn(alice, project_x)` can
become an edge in a knowledge graph. They can be rendered using the same graph
infrastructure without pretending they are the same semantic object.

### Where `<:` should stop doing all the work

There is one important consequence of "Graph is Renderable".

For `Card <: Doc`, subtyping is obvious: a Card genuinely is a more specific Doc.
But `Graph <: Renderable` is different. "Renderable" is primarily a capability,
not an ontological/data relationship.

If `<:` is used for that too, traits/interfaces get reinvented through nominal
subtyping:

```hql
Graph <: Renderable
Table <: Renderable
Markdown <: Renderable
Image <: Renderable
```

and then the compiler has to ask: where does the implementation of `render` come
from? That is precisely the trait problem discussed earlier.

The cleaner HQL architecture is:

```hql
type Graph { ... }
```

with the graph extension implemented in Rust supplying `render(Graph) -> Html`,
or registering the appropriate rendering capability. Then HQL itself knows the
type, while the extension knows how operations on that type work.

So the boundary currently falls as:

```text
<:              structural / semantic type refinement
{A, B}          multiple supertypes

->              graph edge construction operator
{...}           ordinary edge data
Graph           ordinary HQL type

render(Graph)   extension-provided behavior
```

This is attractive because the core HQL language stays very small. Graph, Edge,
Link, HTML rendering, semantic relations, PostgreSQL, vector search, etc. can
mostly be types + functions + operator implementations supplied by extensions,
rather than progressively becoming HQL keywords.

And that gives a particularly nice interpretation of:

```hql
knowledge
| graph
```

`graph` is not a magical presentation keyword. It is an ordinary
function/operator transforming a knowledge value into a `Graph`; the resulting
`Graph` happens to have a renderer registered by the graph extension.

## Generic contracts

> I forgot the generic typing as in `Renderable[T] {..}`

That actually makes the model cleaner. Instead of a bare `Renderable`, the
rendering contract can be parameterized by the rendered result: `Renderable[T]`.

```hql
Graph <: Renderable[Html]
Table <: Renderable[Html]
Json  <: Renderable[Text]
```

or, if one type can support several renderings:

```hql
Graph <: {
    Renderable[Html],
    Renderable[Svg],
    Renderable[Json]
}
```

That is much more expressive than a non-generic `Renderable`, because
"renderable" always implicitly means *renderable into what?*

Conceptually it would be defined as something like:

```hql
type Renderable[T] {
    render: () -> T
}
```

or, depending on how HQL handles behavior:

```hql
type Renderable[T]
```

with the actual `render(Self) -> T` implementation supplied by the
extension/runtime.

This also means generic constraints become useful:

```hql
fn show[T, R](x: T)
where T <: Renderable[R]
```

or perhaps more compactly:

```hql
fn show[T <: Renderable[R], R](x: T)
```

But rushing into Rust-style `T: Renderable[R]` syntax is worth avoiding if `<:`
is already the canonical subtype relation. Keeping one relation syntax is
aesthetically stronger.

And then the set-of-supertypes idea becomes particularly nice:

```hql
type Graph <: {
    Collection[Edge],
    Renderable[Html],
    Renderable[Svg]
}
```

Now `{...}` isn't just multiple nominal parents; it naturally expresses the set
of generic contracts the type refines/satisfies.

The key question then becomes whether `Renderable[T]` is meant to be a true
subtype with structure, or a behavior-only contract backed by Rust
implementations. For HQL the latter is probably more useful.

## The pipe operator

> How about the pipe operator in map/reduce, at the boundary of the query — does
> it have to carry some typing and generics?

`|` itself should not carry explicit generic syntax. The cleanest model is: the
pipe is a typed query boundary in the AST, but its typing falls out of ordinary
function typing.

```hql
cards
| filter(fn(c) => c.type == "person")
| map(fn(c) => c.title)
| collect
```

can be understood approximately as:

```text
cards         : Set<HmdCard>
filter(...)   : Set<HmdCard> -> Set<HmdCard>
map(...)      : Set<HmdCard> -> Set<String>
collect       : Set<String>  -> List<String>
```

Therefore every pipe boundary simply checks:

```text
output type of left stage <: input type of right stage
```

Conceptually the operator is polymorphic:

```text
(|)[A, B] : A -> (A -> B) -> B
```

but users would never be required to write those generics. They are inferred.

The really nice part is that syntactically `x | f(a, b)` can essentially desugar
to `f(x, a, b)` for typing purposes, while remaining a pipe node in the query
AST.

That distinction is important. Fully desugaring it away too early loses the
semantic information that:

```hql
cards
| filter(...)
| map(...)
| group(...)
| reduce(...)
```

contains explicit execution boundaries where the query engine can reason about
parallelism, partitioning, fusion, distribution, SQL pushdown, etc.

So there are two layers:

```text
Language semantics:
    x | f(y)  ≈  f(x, y)

Query semantics:
    | creates a typed query-stage boundary
```

### Map/reduce naturally becomes generic

The standard operations have ordinary generic types:

```hql
map[A, B](
    f: A -> B
) : Collection[A] -> Collection[B]

filter[T](
    predicate: T -> Bool
) : Collection[T] -> Collection[T]

reduce[A, B](
    initial: B,
    f: (B, A) -> B
) : Collection[A] -> B
```

Possibly:

```hql
group[A, K](
    key: A -> K
) : Collection[A] -> Groups[K, A]
```

Now the compiler can type-check a whole pipeline:

```hql
cards
| map(fn(c) => c.title)
| reduce("", concat)
```

as:

```text
Set<HmdCard>
    |
Set<String>
    |
String
```

No special pipe type annotation is necessary.

### Where the MapReduce idea gets powerful

`|` should mean more than ergonomic function application, specifically because
HQL is a query language. Consider:

```hql
cards
| filter(relevant)
| map(extract_concept)
| group(.kind)
| reduce(summarize)
```

Internally the compiler gets something like:

```text
QueryStage<Set<HmdCard>, Set<HmdCard>>
QueryStage<Set<HmdCard>, Set<Concept>>
QueryStage<Set<Concept>, Groups<Kind, Concept>>
QueryStage<Groups<Kind, Concept>, Summary>
```

It can then determine that the first two stages are trivially parallelizable,
`group` introduces repartitioning, and `reduce` is an aggregation boundary.

But exposing something such as `Pipe[A, B]` as a fundamental HQL concept is
premature.

A Rust extension implementing a function may internally advertise execution
properties like:

```text
map
  input: Collection[A]
  output: Collection[B]
  parallel: elementwise

filter
  input: Collection[A]
  output: Collection[A]
  parallel: elementwise

group
  input: Collection[A]
  output: Groups[K, A]
  parallel: partition

reduce
  input: Collection[A]
  output: B
  parallel: reduction
```

Those are execution properties, not really part of the HQL source-level type.

That also helps with the PostgreSQL extension. Something like:

```hql
postgres.table("people")
| filter(.age > 18)
| map({name: .name, age: .age})
| limit(100)
```

could have ordinary inferred types throughout, while the PostgreSQL extension
recognizes the pipeline and pushes the compatible stages into SQL.

### Current model

```text
|       = language-level typed composition
        + preserved query execution boundary

A -> B  = what makes adjacent stages type-compatible

generics = belong to functions/types being piped,
           not explicitly to |

MapReduce / SQL / parallelism
        = execution semantics attached to stages
```

That gives `|` substantial semantic importance without making it another
miniature type system.
