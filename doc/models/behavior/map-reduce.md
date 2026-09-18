# Map Reduce for queries

> see queries 

 The cleanup actually makes the **map/reduce interpretation cleaner**, because it separates the *collection hierarchy* from the *execution model*.

The pipe can still be your **execution boundary**:

```hql
cards
| filter(.metadata.type == "concept")
| map(.title)
| take(100)
| count
```

Conceptually, the left/middle part is a chain of **stream transformations**:

```text
source
  → map
  → filter
  → flatMap
  → take
  → ...
```

These can remain lazy, fuse together, and potentially be parallelized.

Then certain operations are **reducers / barriers**:

```text
count
size
sum
collect
group
sort
materialize
```

They consume some or all of the upstream traversal and produce a different kind of value.

So I would keep the mental model:

```text
pipe = query/execution boundary

map-like steps
    preserve traversal
    can fuse
    usually lazy

reduce-like steps
    collapse/reorganize traversal
    may force evaluation
    establish an execution barrier
```

The nice part is that the new types tell you **which reductions are legal**.

For example:

```hql
s : Stream<Int>

s | map(f)       // fine
s | filter(p)    // fine
s | take(10)     // fine → Seq<Int>

s | size         // type error
s | sort         // type error
```

Because an arbitrary `Stream<T>` may be infinite.

But:

```hql
xs : Seq<Int>

xs | size
xs | sort
```

are valid because `Seq<T> : Collection<T>` guarantees finiteness. That follows directly from the hierarchy we just cleaned up. 

I would actually go one step further and **not describe HQL as literally implementing MapReduce** in the Hadoop sense. The better statement is:

> **The HQL pipe is a map/reduce-like execution boundary.**

It gives the executor permission to treat a pipeline as a graph of transformations and reductions rather than as ordinary nested function calls.

For example:

```hql
cards
| filter(is_concept)
| map(embed)
| filter(.score > 0.8)
| take(20)
```

could internally become something like:

```text
Vault source
    │
    ▼
filter ─┐
map    ─┼─ fused traversal
filter ─┘
    │
 take(20)
    │
   stop
```

No intermediate `List`s need to exist.

Whereas:

```hql
cards
| group(.metadata.type)
| sort(by = .count)
```

contains genuine barriers:

```text
source
  ↓
map/filter/fused traversal
  ↓
GROUP       ← reduction/materialization boundary
  ↓
SORT        ← another global operation
  ↓
result
```

And this is also why I think your earlier intuition that **`|` matters semantically** was right. Without the pipe, these are just ordinary functions:

```hql
map(cards, f)
```

With the pipe:

```hql
cards
| map(f)
| filter(p)
| take(10)
```

HQL can explicitly say:

> This chain forms a query execution region. The runtime may fuse, parallelize, short-circuit, push predicates toward the source, or otherwise optimize it as long as the typed semantics are preserved.

So yes: **keep map/reduce as one of the central HQL ideas.** The type hierarchy answers *what values are legal*. The pipe answers *where query execution and optimization happen*. Those are complementary, not competing concepts.
