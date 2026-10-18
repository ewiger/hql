// Finite occurrences, positional order, and keyed lookup.
// See doc/wiki/hql/types/collections/collection-types.md and collection.md.

// Values that can be traversed, and nothing more: no promise of finiteness,
// order or uniqueness. The bound an operation states when it needs only to
// visit each value. See doc/models/behavior/queries.md.
abstract type Iterable<T>

// No runtime representation and no promise of order or uniqueness.
// size(c) counts occurrences; count(c, x) counts occurrences equal to x.
// contains(c, x) iff count(c, x) > 0.
// size(c) is the sum of count(c, x) over all distinct values occurring in c.
abstract type Collection<T> : Iterable<T>

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

// One partition of a group: its key, and the occurrences with that key in
// input order. Two groups are equal iff their keys are.
type Group<K, T> {
    key   : K
    items : List<T>
}

// Core operations, callable directly or through a pipe. Each one returns the
// type it was given: `S` is the input's own type and `F<_>` its constructor,
// where `F<X> : Iterable<X>` for every `X`. An operation that gains a
// guarantee its input lacks returns the type that states it.
// size<T>(xs: Collection<T>) : Int
// count<T>(xs: Collection<T>) : Int                  the kept alias of size
// count<T>(xs: Collection<T>, value: T) : Int
// contains<T>(xs: Collection<T>, value: T) : Bool
// get<K, V>(xs: Map<K, V>, key: K) : Option<V>
// filter<S : Collection<T>, T>(xs: S, by: T -> Bool) : S
// filter<F<_> : Iterable<_>, T, U>(xs: F<T>, by: x => x | is(U)) : F<U>   narrows
// of_type<F<_> : Iterable<_>, T, U>(xs: F<T>, type: Type<U>) : F<U>   filter(x => x | is(U))
// is<T, U>(x: T, type: Type<U>) : Bool          refused when no T can be a U
// map<F<_> : Iterable<_>, T, U>(xs: F<T>, by: T -> U) : F<U>
// take<S : Seq<T>, T>(xs: S, n: Int) : S
// take<T : Orderable>(xs: Collection<T>, n: Int) : List<T>
// sort<T>(xs: Ranking<T>, by: …) : Seq<Hit<T>>
// sort<S : Seq<T>, T : Orderable>(xs: S) : S
// sort<T : Orderable>(xs: Collection<T>) : List<T>
// sort<S : Seq<T>, T>(xs: S, by: (T, T) -> Ordering) : S
// sort<T>(xs: Collection<T>, by: (T, T) -> Ordering) : List<T>
// sort<S : Seq<T>, T, K : Orderable>(xs: S, by: T -> K) : S
// sort<T, K : Orderable>(xs: Collection<T>, by: T -> K) : List<T>
// A Set maps to its image, which may be smaller than the input. A Ranking's
// element is Hit<T>, not T, so `map` takes it as the Seq<Hit<T>> it is and
// returns a Seq<U>; `take` keeps it a Ranking; `sort` gives it a new order
// that is not the retrieval's, so the result is a Seq<Hit<T>>.
// Sorting supplies positions; its input need not already be a sequence.
// group<T, K>(xs: Collection<T>, by: T -> K) : Set<Group<K, T>>
// reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R) : R
// reduce<T>(xs: Seq<T>, by: (T, T) -> T) : Option<T>       none over []
// sum(xs: Collection<Int>) : Int                           reduce(0, +)
// sum(xs: Collection<Float>) : Float                       reduce(0.0, +)
// min<T : Orderable>(xs: Collection<T>) : Option<T>        reduce(min)
// max<T : Orderable>(xs: Collection<T>) : Option<T>        reduce(max)
// A fold visits positions, so reduce refuses a Set; sum, min and max combine
// in an order-independent way and accept any collection.
