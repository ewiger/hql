> So what laws does collection has?

Very few, actually. I would keep `Collection<T>` deliberately weak so that `Seq<T>` and `Set<T>` can both refine it cleanly.

The key insight is that a collection contains **occurrences**, not merely distinct values. Otherwise `Seq<T>` with duplicates cannot naturally be a subtype of `Collection<T>`.

So I would define the laws around **finiteness, cardinality, membership, and multiplicity**:

```hql
abstract type Collection<T>
```

with the conceptual operations:

```hql
size     : Collection<T> -> Int
contains : (Collection<T>, T) -> Bool
count    : (Collection<T>, T) -> Int
```

and these laws:

```text
size(c) is finite

contains(c, x) <=> count(c, x) > 0

size(c) = Σ count(c, x)
          over all distinct x occurring in c
```

That is almost enough.

A collection **does not** establish any of these:

```text
order
uniqueness
indexing
sorting
key lookup
storage representation
```

Those come from subtypes.

So:

```text
Collection<T>
    finite occurrences
    membership
    multiplicity
    cardinality
```

Then `Set<T>` strengthens it with:

```text
count(s, x) <= 1
```

while `Seq<T>` strengthens it with positions:

```text
positions(s) = 0 .. size(s)-1

each position identifies exactly one occurrence

every occurrence occupies exactly one position
```

That makes the hierarchy mathematically clean:

```text
                    Collection<T>
                  occurrences + size
                    /          \
                   /            \
              Seq<T>            Set<T>
          + positions        + uniqueness
```

I would **not** make iteration order a `Collection` law. A Rust implementation obviously needs some way of traversing the values, but the traversal order must not become part of the HQL semantics at this level.

Likewise, I would not define collection equality yet. Equality gets surprisingly tricky because:

```hql
[1, 2]        // Seq
[2, 1]        // Seq
```

are different sequences, whereas two sets containing `1` and `2` are equal regardless of traversal order. Equality therefore belongs to the more specific type semantics, not `Collection<T>` itself.

So the actual declaration comment could be extremely compact:

```hql
abstract type Collection<T>
    // Laws:
    // - finite number of element occurrences
    // - size(c) counts all occurrences
    // - count(c, x) counts occurrences of x
    // - contains(c, x) iff count(c, x) > 0
    //
    // No ordering, uniqueness, indexing, or storage representation is implied.
```

I think **multiplicity is the missing concept** that makes `Collection<T>` rigorous instead of just saying vaguely “a bunch of things.”
