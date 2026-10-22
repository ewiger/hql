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

// Values produced in a stable order: x0, x1, x2, ... Production need not end,
// so a stream has no size, no membership and no global order: those need the
// end. See doc/wiki/hql/types/stream-type.hmd.
abstract type Stream<T> : Iterable<T>

// Every occurrence occupies exactly one position in 0 .. size(c)-1.
// Positional order does not require T : Orderable. A sequence is a stream
// that ends.
abstract type Seq<T> : {Collection<T>, Stream<T>}

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

// Constructors: a function named after its type builds a value of it, and
// `List(xs)` and `xs | List` are one call, the piped value its first
// argument. A type's forms of one arity differ in their first parameter, so
// the first argument selects one. The binary implements each, as it does the
// operations below.
fn List<T>(xs: Seq<T>) : List<T>
fn Set<T>(xs: Collection<T>) : Set<T>
fn Map<K, V>(keys: Seq<K>, values: Seq<V>) : Map<K, V>
fn OrderedMap<K, V>(keys: Seq<K>, values: Seq<V>) : OrderedMap<K, V>
fn SortedMap<K : Orderable, V>(keys: Seq<K>, values: Seq<V>) : SortedMap<K, V>

// Core operations, callable directly or through a pipe. Each one returns the
// type it was given: `S : Seq<T>` is the input at its own type, and
// `F<_> : Iterable<_>` its constructor, which `F<U>` rebuilds around another
// element. An operation that gains a guarantee its input lacks returns the
// type that states it.
//
// Every declaration is external: the core implements it, and binding holds
// the two to each other. A name declared more than once is overloaded: a call
// takes the first declaration its input and its arguments fit, so the more
// specific forms come first. A `/` ends the parameters a call supplies by
// position only: `count(1)`, never `count(value = 1)`.

fn size<T>(xs: Collection<T>) : Int
// The kept alias of size.
fn count<T>(xs: Collection<T>) : Int
fn count<T>(xs: Collection<T>, value: T, /) : Int
fn contains<T>(xs: Collection<T>, value: T, /) : Bool
fn get<K, V>(xs: Map<K, V>, key: K, /) : Option<V>

// A predicate that tests the element's type, `x => x | is(U)`, is a
// `T -> Is<U>`, and narrows the result to F<U>: of_type(U) is that filter. Over one optional value, filter
// keeps it or makes it absent, and map maps what is present; map over a
// result maps its success. An optional collection is a collection that may
// be absent, so filter and map over it reach its elements.
fn filter<F<_> : Iterable<_>, T, U>(xs: F<T>, by: T -> Is<U>) : F<U>
fn filter<T, U>(xs: Option<T>, by: T -> Is<U>) : Option<U>
fn filter<S : Iterable<T>, T>(xs: S, by: T -> Bool) : S
fn filter<T>(xs: Option<T>, by: T -> Bool) : Option<T>
fn of_type<F<_> : Collection<_>, T, U>(xs: F<T>, type: Type<U>) : F<U>
// Refused when no T can be a U.
fn is<T, U>(x: T, type: Type<U>) : Bool
// Keeps the present values.
fn somes<F<_> : Collection<_>, T>(xs: F<Option<T>>) : F<T>
fn map<F<_> : Iterable<_>, T, R>(xs: F<T>, by: T -> R) : F<R>
fn map<T, R>(xs: Option<T>, by: T -> R) : Option<R>
fn map<T, R, E>(xs: Result<T, E>, by: T -> R) : Result<R, E>

// A prefix of a sequence keeps its positions; a bounded prefix of a stream is
// finite; a prefix of anything else is taken in its elements' order.
fn take<S : Seq<T>, T>(xs: S, n: Int) : S
fn take<T>(xs: Stream<T>, n: Int) : List<T>
fn take<T : Orderable>(xs: Collection<T>, n: Int) : List<T>
// Refused over values without a stable order.
fn drop<S : Iterable<T>, T>(xs: S, n: Int) : S
// Batches of n, in order. xs | collect(n) | emit = xs.
fn collect<T>(xs: Collection<T>, n: Int) : List<List<T>>
fn collect<T>(xs: Stream<T>, n: Int) : Stream<List<T>>
fn emit<T>(xs: Collection<Seq<T>>) : List<T>
fn emit<T>(xs: Stream<Seq<T>>) : Stream<T>
// map(by) | emit.
fn flat_map<T, U>(xs: Collection<T>, by: T -> Seq<U>) : List<U>
fn flat_map<T, U>(xs: Stream<T>, by: T -> Seq<U>) : Stream<U>

// Sorting supplies positions; its input need not already be a sequence. A
// sequence is rebuilt in its own constructor, so a List stays a List, while
// a Ranking, whose element is Hit<T> and not T, becomes a Seq<Hit<T>>: the
// new order is not the retrieval's.
fn sort<F<_> : Seq<_>, T : Orderable>(xs: F<T>) : F<T>
fn sort<T : Orderable>(xs: Collection<T>) : List<T>
fn sort<F<_> : Seq<_>, T, K : Orderable>(xs: F<T>, by: T -> K) : F<T>
fn sort<T, K : Orderable>(xs: Collection<T>, by: T -> K) : List<T>
fn sort<F<_> : Seq<_>, T>(xs: F<T>, by: (T, T) -> Ordering) : F<T>
fn sort<T>(xs: Collection<T>, by: (T, T) -> Ordering) : List<T>
// A Set maps to its image, which may be smaller than the input; a Ranking
// maps as the Seq<Hit<T>> it is, to a Seq<U>; take keeps it a Ranking.
fn group<T, K>(xs: Collection<T>, by: T -> K) : Set<Group<K, T>>

// Without an initial value the fold starts at the first element, and is
// absent over [].
fn reduce<T, R>(xs: Collection<T>, initial: R, by: (R, T) -> R) : R
fn reduce<T>(xs: Collection<T>, by: (T, T) -> T) : Option<T>
// reduce(0, +); [] | sum is 0.
fn sum(xs: Collection<Int>) : Int
fn sum(xs: Collection<Float>) : Float
// reduce(1, *).
fn product(xs: Collection<Int>) : Int
fn product(xs: Collection<Float>) : Float
// Absent over [].
fn average(xs: Collection<Int>) : Option<Float>
fn average(xs: Collection<Float>) : Option<Float>
// reduce({}, merge), a later key winning; reduce(Set([]), union).
fn merge_all<T>(xs: Collection<T>) : T
fn union_all<T>(xs: Collection<T>) : T
// reduce([], concat).
fn concat_all<U>(xs: Collection<Seq<U>>) : List<U>
// reduce(min) and reduce(max).
fn min<T : Orderable>(xs: Collection<T>) : Option<T>
fn max<T : Orderable>(xs: Collection<T>) : Option<T>
// The combiners: min(a, b), max(a, b), merge(m, n), union(s, t), concat(xs, ys).
// Named as a whole argument, a combiner or `+` / `*` is the function that
// combines two values with it: `reduce(0, +)`. A fold whose combiner ignores
// order (+, *, min, max, union) accepts any collection; one that does not
// visits positions, and so refuses a Set.
