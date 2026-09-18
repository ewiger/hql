// Finite occurrences, positional order, and keyed lookup.
// See doc/wiki/hql/types/collections/collection-types.md and collection.md.

// No runtime representation and no promise of order or uniqueness.
// size(c) counts occurrences; count(c, x) counts occurrences equal to x.
// contains(c, x) iff count(c, x) > 0.
// size(c) is the sum of count(c, x) over all distinct values occurring in c.
abstract type Collection<T>

// Every occurrence occupies exactly one position in 0 .. size(c)-1.
// Positional order does not require T : Orderable.
abstract type Seq<T> : Collection<T>

// Materialized occurrences; repeated traversal preserves positions.
type List<T> : Seq<T>

// Materialized distinct values, using HQL equality: count(s, x) <= 1.
// Traversal order is not a semantic guarantee.
type Set<T> : Collection<T>

// Materialized associations with distinct keys; unrelated to Collection<T>.
// Construct from equally long key and value sequences: Map(keys, values).
// Repeated keys are rejected. Missing lookup results are Option<V>.
// The key projection of this static view has type Set<K>.
type Map<K, V>

// Keys preserve their construction positions; keys have type Seq<K>.
// K need not be Orderable. Viewing this as Map exposes set key semantics.
type OrderedMap<K, V> : Map<K, V>

// Key positions follow K's intrinsic total ordering.
type SortedMap<K, V> : OrderedMap<K, V>
where K : Orderable

// Core operations, callable directly or through a pipe:
// size<T>(xs: Collection<T>) : Int
// count<T>(xs: Collection<T>, value: T) : Int
// contains<T>(xs: Collection<T>, value: T) : Bool
// get<K, V>(xs: Map<K, V>, key: K) : Option<V>
// sort<T>(xs: Collection<T>) : List<T> where T : Orderable
// sort<T>(xs: Collection<T>, by: (T, T) -> Ordering) : List<T>
// sort<T, K>(xs: Collection<T>, by: T -> K) : List<T> where K : Orderable
// Sorting supplies positions; its input need not already be a sequence.
// count(xs) remains a compatibility spelling of size(xs).
