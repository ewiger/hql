// Membership, order and lookup: three different contracts, one per type.
//
// See doc/wiki/hql/collections.hmd for the argument, and types/orderable.hmd
// for value order, which is a property of an element and not of a collection.

// Unordered, distinct elements. No implicit first element and no stable
// iteration order: discovering values must not establish a language ordering
// guarantee.
type Set<T>

// Positional order. A position exists for every element — 0, 1, 2 … — and the
// positions come from the sequence, not from the values. Repeats are allowed.
// A Seq<T> is ordered whether or not T is Orderable: Seq<Function> has a first
// and a second element while no two functions have a meaningful order.
type Seq<T>

// A Seq whose elements are all present at once. Seq states that a position
// exists for every element; List states that they are held rather than
// produced, so a length is known and a second traversal costs nothing.
type List<T> <: Seq<T>

// Lookup by key. Keys are distinct; iteration order is not implied.
type Map<K, V>
