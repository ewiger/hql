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

reduction fn size<T>(self: Collection<T>) : Int
// The kept alias of size.
reduction fn count<T>(self: Collection<T>) : Int
reduction fn count<T>(self: Collection<T>, value: T, /) : Int
reduction fn contains<T>(self: Collection<T>, value: T, /) : Bool
fn get<K, V>(self: Map<K, V>, key: K, /) : Option<V>
// Documents are keyed by name: `cards.get("alice")` is the card named alice.
fn get<T : Doc>(self: Collection<T>, name: String, /) : Option<T>

// A predicate that tests the element's type, `x => x | is(U)`, is a
// `T -> Is<U>`, and narrows the result to F<U>: of_type(U) is that filter. Over one optional value, filter
// keeps it or makes it absent, and map maps what is present; map over a
// result maps its success. An optional collection is a collection that may
// be absent, so filter and map over it reach its elements.
elementwise fn filter<F<_> : Iterable<_>, T, U>(self: F<T>, by: T -> Is<U>) : F<U>
elementwise fn filter<T, U>(self: Option<T>, by: T -> Is<U>) : Option<U>
elementwise fn filter<S : Iterable<T>, T>(self: S, by: T -> Bool) : S
elementwise fn filter<T>(self: Option<T>, by: T -> Bool) : Option<T>
elementwise fn of_type<F<_> : Collection<_>, T, U>(self: F<T>, type: Type<U>) : F<U>
// Refused when no T can be a U.
fn is<T, U>(self: T, type: Type<U>) : Bool
// Keeps the present values.
elementwise fn somes<F<_> : Collection<_>, T>(self: F<Option<T>>) : F<T>
elementwise fn map<F<_> : Iterable<_>, T, R>(self: F<T>, by: T -> R) : F<R>
elementwise fn map<T, R>(self: Option<T>, by: T -> R) : Option<R>
elementwise fn map<T, R, E>(self: Result<T, E>, by: T -> R) : Result<R, E>

// A prefix of a sequence keeps its positions; a bounded prefix of a stream is
// finite; a prefix of anything else is taken in its elements' order.
fn take<S : Seq<T>, T>(self: S, n: Int) : S
fn take<T>(self: Stream<T>, n: Int) : List<T>
fn take<T : Orderable>(self: Collection<T>, n: Int) : List<T>
// Refused over values without a stable order.
fn drop<S : Iterable<T>, T>(self: S, n: Int) : S
// Batches of n, in order. xs | collect(n) | emit = xs.
fn collect<T>(self: Collection<T>, n: Int) : List<List<T>>
fn collect<T>(self: Stream<T>, n: Int) : Stream<List<T>>
elementwise fn emit<T>(self: Collection<Seq<T>>) : List<T>
elementwise fn emit<T>(self: Stream<Seq<T>>) : Stream<T>
// map(by) | emit.
elementwise fn flat_map<T, U>(self: Collection<T>, by: T -> Seq<U>) : List<U>
elementwise fn flat_map<T, U>(self: Stream<T>, by: T -> Seq<U>) : Stream<U>

// Sorting supplies positions; its input need not already be a sequence. A
// sequence is rebuilt in its own constructor, so a List stays a List, while
// a Ranking, whose element is Hit<T> and not T, becomes a Seq<Hit<T>>: the
// new order is not the retrieval's.
fn sort<F<_> : Seq<_>, T : Orderable>(self: F<T>) : F<T>
fn sort<T : Orderable>(self: Collection<T>) : List<T>
fn sort<F<_> : Seq<_>, T, K : Orderable>(self: F<T>, by: T -> K) : F<T>
fn sort<T, K : Orderable>(self: Collection<T>, by: T -> K) : List<T>
fn sort<F<_> : Seq<_>, T>(self: F<T>, by: (T, T) -> Ordering) : F<T>
fn sort<T>(self: Collection<T>, by: (T, T) -> Ordering) : List<T>
// A Set maps to its image, which may be smaller than the input; a Ranking
// maps as the Seq<Hit<T>> it is, to a Seq<U>; take keeps it a Ranking.
partition fn group<T, K>(self: Collection<T>, by: T -> K) : Set<Group<K, T>>

// Joins of the pipeline value with one other sequence (HQL-0013, the binary
// form). The condition is the key functions `left` and `right`, matching x
// with y when left(x) == right(y), a predicate `on` over the pair, or both;
// `into` builds each result from a matching pair. Results are ordered by the
// left element's position, then the right's. left_join keeps every element
// of the input, with `none` for an unmatched right; right_join every element
// of `other`; full_join both, the unmatched elements of `other` after the
// matched results. A join with no condition is a cross_join, asked for by
// name.

fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>
fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, U) -> R) : Seq<R>
fn join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>
// Every pair, in order: no condition.
fn cross_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, into: (T, U) -> R) : Seq<R>
// The elements of the input that match some element of `other`, or none:
// each kept at most once, in its position.
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn semi_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn anti_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>

// Without an initial value the fold starts at the first element, and is
// absent over [].
fn reduce<T, R>(self: Collection<T>, initial: R, by: (R, T) -> R) : R
fn reduce<T>(self: Collection<T>, by: (T, T) -> T) : Option<T>
// reduce(0, +); [] | sum is 0.
reduction fn sum(self: Collection<Int>) : Int
reduction fn sum(self: Collection<Float>) : Float
// reduce(1, *).
reduction fn product(self: Collection<Int>) : Int
reduction fn product(self: Collection<Float>) : Float
// Absent over [].
reduction fn average(self: Collection<Int>) : Option<Float>
reduction fn average(self: Collection<Float>) : Option<Float>
// reduce({:}, merge), a later key winning; reduce(Set([]), union).
reduction fn merge_all<T>(self: Collection<T>) : T
reduction fn union_all<T>(self: Collection<T>) : T
// reduce([], concat).
reduction fn concat_all<U>(self: Collection<Seq<U>>) : List<U>
// reduce(min) and reduce(max).
reduction fn min<T : Orderable>(self: Collection<T>) : Option<T>
reduction fn max<T : Orderable>(self: Collection<T>) : Option<T>

// Aggregations whose state differs from their element: each is a host step
// whose state combines by a known combiner, and equals the longhand fold
// written beside it. Each name stays a host step because a declaration with
// a body is not external, and only an external declaration carries the
// `reduction` shape a planner splits by. `Mean` and `Moments` are records
// a longhand declares: `type Mean { sum : Float  count : Int }` and
// `type Moments { count : Int  mean : Float  m2 : Float }`.
//
// average: a sum and a count, combined by `+` on both, then their quotient.
//   s = xs | reduce(initial = Mean(sum = 0.0, count = 0),
//                   by = (s, x) => Mean(sum = s.sum + x, count = s.count + 1))
//   none given s.count == 0 otherwise some(s.sum / float(s.count))
//
// variance: the population variance; a count, a running mean and the sum of
// squared deviations from it, combined pairwise. Absent over [].
//   s = xs | reduce(initial = Moments(count = 0, mean = 0.0, m2 = 0.0),
//                   by = (s, x) => Moments(count = s.count + 1,
//                       mean = s.mean + (x - s.mean) / float(s.count + 1),
//                       m2 = s.m2 + (x - s.mean)
//                            * (x - (s.mean + (x - s.mean) / float(s.count + 1)))))
//   none given s.count == 0 otherwise some(s.m2 / float(s.count))
reduction fn variance(self: Collection<Int>) : Option<Float>
reduction fn variance(self: Collection<Float>) : Option<Float>
// count_by: a count per key, combined by `+` per key; {} over [].
//   reduce(initial = Map<K, Int>([], []),
//          by = (m, x) => m | with(by(x), (m | get(by(x)) | unwrap_or(0)) + 1))
reduction fn count_by<T, K>(self: Collection<T>, by: T -> K) : Map<K, Int>
// histogram: count_by the bucket's lower bound, the greatest multiple of
// `width` not above the value, in key order. A width of zero or less is an
// evaluation error.
//   reduce(initial = SortedMap<Int, Int>([], []),
//          by = (m, x) => m | with(x - (x % width + width) % width,
//              (m | get(x - (x % width + width) % width) | unwrap_or(0)) + 1))
//   over Floats the bound is floor(x / width) * width.
reduction fn histogram(self: Collection<Int>, width: Int) : SortedMap<Int, Int>
reduction fn histogram(self: Collection<Float>, width: Float) : SortedMap<Float, Int>
// top_k: the k greatest, greatest first, equal keys in input order; two
// partial lists combine by keeping the k greatest of both.
//   reduce(initial = List<T>([]),
//          by = (top, x) => concat(top, [x]) | sort(by = (a, b) => compare(b, a)) | take(k))
//   with `by`, the comparison is compare(by(b), by(a)).
reduction fn top_k<T : Orderable>(self: Collection<T>, k: Int) : List<T>
reduction fn top_k<T, K : Orderable>(self: Collection<T>, k: Int, by: T -> K) : List<T>
// approx_distinct: an estimate of size(Set(xs)). Its state is a HyperLogLog
// sketch, 4096 registers combined by `max` per register, over a SHA-256
// digest of each value with no seed, so it is deterministic. HQL exposes no
// hash, so this longhand is not writable in HQL; 0 over [].
reduction fn approx_distinct<T>(self: Collection<T>) : Int

// The combiners: min(a, b), max(a, b), merge(m, n), union(s, t), concat(xs, ys).
// Named as a whole argument, a combiner or `+` / `*` is the function that
// combines two values with it: `reduce(0, +)`. A fold whose combiner ignores
// order (+, *, min, max, union) accepts any collection; one that does not
// visits positions, and so refuses a Set.
