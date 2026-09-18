

# Collection Types

HQL distinguishes **abstract semantic types** from **concrete runtime types**.

An `abstract type` is defined by its semantics, laws, and required behavior. It has no independent runtime representation. The Rust implementation is responsible for ensuring that concrete subtypes satisfy those laws.

A plain `type` is concrete. Values of that type exist at runtime and must have an in-memory representation.

The subtype relation `<:` is semantic ancestry. If `A <: B`, every value of `A` satisfies the contract of `B`.

```hql
abstract type Collection<T>

abstract type Seq<T> <: Collection<T>

type List<T> <: Seq<T>

type Set<T> <: Collection<T>

type Map<K, V>

type OrderedMap<K, V> <: Map<K, V>

type SortedMap<K, V> <: OrderedMap<K, V>

abstract type Orderable
```

The hierarchy is intentionally small.

```text
                    Collection<T>
                    /           \
                   /             \
              Seq<T>             Set<T>
                |
             List<T>


Map<K, V>
    |
OrderedMap<K, V>
    |
SortedMap<K, V>


Orderable
```

## Collection

`Collection<T>` is the common abstract type for finite collections of values.

It defines collection semantics such as finite cardinality, membership, and traversal, but does not define positional ordering or uniqueness.

```hql
abstract type Collection<T>
    // Rust:
    // No independent runtime representation.
    // Concrete subtypes implement the collection contract.
```

## Sequence

`Seq<T>` is an abstract collection whose elements have stable positions.

```hql
abstract type Seq<T> <: Collection<T>
    // Rust:
    // Preserve positional order through sequence operations.
    // Seq<T> itself has no required storage representation.
```

A sequence is ordered by **position**:

```text
0 -> x
1 -> y
2 -> z
```

This says nothing about whether `x`, `y`, and `z` themselves can be compared.

Therefore:

```hql
Seq<Function>
```

is valid even if `Function` is not `Orderable`.

## List

`List<T>` is the concrete, materialized form of a sequence.

```hql
type List<T> <: Seq<T>
    // Rust:
    // Store all elements in memory.
    // Length is immediately available.
    // Repeated traversal observes the same materialized sequence.
```

`Seq<T>` describes sequence semantics. `List<T>` describes an actual runtime value satisfying those semantics.

The distinction is therefore:

```text
Seq<T>     abstract positional sequence
List<T>    concrete materialized sequence
```

## Set

`Set<T>` is a concrete collection of distinct values.

```hql
type Set<T> <: Collection<T>
    // Rust:
    // Materialize the set in memory.
    // Enforce uniqueness using HQL equality semantics.
    // Storage order must not become part of HQL semantics.
```

A set has no positional ordering.

Iteration may require the runtime to produce values in some order, but that order is not observable as part of the `Set<T>` contract.

## Map

`Map<K, V>` is a concrete finite mapping from distinct keys of type `K` to values of type `V`.

```hql
type Map<K, V>
    // Rust:
    // Materialize key/value associations in memory.
    // Enforce unique keys according to HQL key equality.
    // No key iteration order is guaranteed.
```

A map is not defined as a `Collection<(K, V)>`. Its defining operation is keyed lookup, and it has its own semantic structure.

The keys of an ordinary map form a set:

```hql
map.keys : Set<K>
```

## Ordered Map

`OrderedMap<K, V>` is a map whose keys additionally have stable positions.

```hql
type OrderedMap<K, V> <: Map<K, V>
    // Rust:
    // Preserve a stable key sequence.
    // Iteration over keys and entries follows that sequence.
```

Its keys therefore form a sequence:

```hql
map.keys : Seq<K>
```

This does **not** require `K` to be `Orderable`.

For example, an ordered map may preserve insertion order:

```text
alice
carol
bob
```

even though that order does not arise from comparing the keys themselves.

`OrderedMap` therefore means that the **map carries an order**.

It does not mean that the **keys possess an ordering relation**.

## Orderable

`Orderable` is an abstract law type.

```hql
abstract type Orderable
    // Rust:
    // Concrete subtypes provide a deterministic total ordering.
    // The ordering must satisfy the laws required by HQL comparison.
```

A type below `Orderable` has an intrinsic ordering relation.

Conceptually:

```hql
Int <: Orderable
String <: Orderable
```

`Orderable` is different from sequence ordering:

```text
Seq<T>         positions are ordered

T <: Orderable values of T can be ordered
```

The two properties are independent.

## Sorted Map

`SortedMap<K, V>` is an ordered map whose key sequence follows the intrinsic ordering of its keys.

```hql
type SortedMap<K, V> <: OrderedMap<K, V>
where K <: Orderable
    // Rust:
    // Preserve Map and OrderedMap semantics.
    // Key iteration must follow the intrinsic total order of K.
```

The progression is therefore precise:

```text
Map<K, V>
    distinct keys and keyed lookup

OrderedMap<K, V>
    + stable positional order of keys

SortedMap<K, V>
    + positional order agrees with the intrinsic order of K
```

An `OrderedMap` does not require:

```hql
K <: Orderable
```

A `SortedMap` does.

## Sorting

Sorting is an operation, not a type named `Sortable`.

```hql
fn sort<T>(xs: Seq<T>) : List<T>
where T <: Orderable
```

The operation consumes sequence semantics, uses the intrinsic ordering of `T`, and produces a concrete materialized sequence.

An explicit comparison may supply the ordering directly:

```hql
fn sort<T>(
    xs: Seq<T>,
    by: (T, T) -> Ordering
) : List<T>
```

Providing such a comparison does not make `T` a subtype of `Orderable`. The ordering came from the operation rather than from the type itself.

## Complete declaration

The collection model is therefore:

```hql
abstract type Orderable
    // Rust:
    // Law-only type. No runtime Value representation.
    // Concrete subtypes provide a deterministic total order.


abstract type Collection<T>
    // Rust:
    // Law-only collection abstraction.
    // No runtime Value representation.


abstract type Seq<T> <: Collection<T>
    // Rust:
    // Law-only positional collection abstraction.
    // Preserve sequence order through sequence operations.


type List<T> <: Seq<T>
    // Rust:
    // Concrete materialized sequence.
    // All elements exist in memory.


type Set<T> <: Collection<T>
    // Rust:
    // Concrete materialized collection.
    // Values are unique.
    // Iteration order has no semantic meaning.


type Map<K, V>
    // Rust:
    // Concrete materialized keyed collection.
    // Keys are unique.
    // No key order is guaranteed.


type OrderedMap<K, V> <: Map<K, V>
    // Rust:
    // Concrete map with stable key sequence.
    // map.keys : Seq<K>


type SortedMap<K, V> <: OrderedMap<K, V>
where K <: Orderable
    // Rust:
    // Concrete ordered map.
    // Key sequence follows K's intrinsic ordering.
```

This yields three distinct notions of order without conflating them:

```text
Seq<T>             order belongs to element positions

OrderedMap<K, V>   order belongs to key positions

K <: Orderable     order belongs intrinsically to values of K
```

`SortedMap<K, V>` is where the latter two meet.
