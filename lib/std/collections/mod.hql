// Module `collections`. Always visible: the `std` extension implements it
// with `core`. See doc/wiki/hql/types/collections/collection-types.md and
// collection.md.

/// The operations over the collection types `core` declares: membership,
/// filtering, mapping, order, grouping, joins and reductions. Each one is
/// callable directly or as a pipeline step, and carries the shape a planner
/// splits it by.
module collections

import core
import doc

/// One partition of a `group`: its `key`, and the occurrences with that key
/// in input order, as `items`. Two groups are equal iff their keys are.
type Group<K, T> {
    key   : K
    items : List<T>
}

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

/// The number of element occurrences in `self`, an `Int`:
/// `[1, 2, 2, 3] | size` is `4`, and `Set([1, 2, 2]) | size` is `2`.
reduction fn size<T>(self: Collection<T>) : Int

/// The number of element occurrences in `self`, as `size` gives it: `count`
/// is the kept alias of `size`. `[1, 2, 2, 3] | count` is `4`.
reduction fn count<T>(self: Collection<T>) : Int

/// The number of occurrences in `self` equal to `value`:
/// `[1, 2, 2, 3] | count(2)` is `2`. `value` is passed by position only.
reduction fn count<T>(self: Collection<T>, value: T, /) : Int

/// `true` when `value` occurs in `self`, and `false` otherwise:
/// `[1, 2, 3] | contains(2)` is `true`.
reduction fn contains<T>(self: Collection<T>, value: T, /) : Bool

/// The value a map holds for `key`, or `none` when the key is missing:
/// `{"a": 1} | get("a")` is `1`, and `{"a": 1} | get("b")` is `none`.
fn get<K, V>(self: Map<K, V>, key: K, /) : Option<V>

/// The document named `name` in a collection of documents, or `none` when
/// none has that name: `cards.get("alice")` is the card named alice.
fn get<T : Doc>(self: Collection<T>, name: String, /) : Option<T>

/// The elements for which a type test holds, each at the tested type. A
/// `by` of type `T -> Is<U>`, such as `x => x | is(Int)`, makes the result
/// an `F<U>`. Over an `Option<T>`, the result is the value when it is
/// present and the test holds, and `none` otherwise.
elementwise fn filter<F<_> : Iterable<_>, T, U>(self: F<T>, by: T -> Is<U>) : F<U>
elementwise fn filter<T, U>(self: Option<T>, by: T -> Is<U>) : Option<U>

/// The elements for which `by` is `true`, in their order, in a collection
/// of the input's type. Over an `Option<T>`, the result is the value when
/// it is present and `by` holds, and `none` otherwise.
elementwise fn filter<S : Iterable<T>, T>(self: S, by: T -> Bool) : S
elementwise fn filter<T>(self: Option<T>, by: T -> Bool) : Option<T>

/// The elements whose runtime type is `type` or one of its subtypes, each
/// at that type, in an `F<U>`: `filter(x => x | is(U))`.
/// `[1, "a", 2] | of_type(Int)` is `[1, 2]`.
elementwise fn of_type<F<_> : Collection<_>, T, U>(self: F<T>, type: Type<U>) : F<U>

/// `true` when the runtime type of `self` is `type` or one of its subtypes.
/// The checker refuses a test no `T` can pass, and warns about one every
/// `T` passes. Used as a `filter` predicate, it narrows the result.
fn is<T, U>(self: T, type: Type<U>) : Bool

/// The present values of a collection of options, in an `F<T>`; the absent
/// ones are dropped: `[some(1), none, some(3)] | somes` is `[1, 3]`.
elementwise fn somes<F<_> : Collection<_>, T>(self: F<Option<T>>) : F<T>

/// The success values of a collection of results, in an `F<T>`; the
/// failures are dropped: `[ok(1), err("x"), ok(3)] | oks` is `[1, 3]`.
elementwise fn oks<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<T>

/// The error values of a collection of results, in an `F<E>`; the
/// successes are dropped: `[ok(1), err("x"), ok(3)] | errs` is `["x"]`.
elementwise fn errs<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<E>

/// The success values of a stream of results, as a stream; the failures
/// are dropped.
elementwise fn oks<T, E>(self: Stream<Result<T, E>>) : Stream<T>

/// The error values of a stream of results, as a stream; the successes are
/// dropped.
elementwise fn errs<T, E>(self: Stream<Result<T, E>>) : Stream<E>

/// Each element through `by`, in a collection of the input's constructor:
/// `[1, 2, 3] | map(x => x * 10)` is `[10, 20, 30]`. A `Set` maps to its
/// image, which may hold fewer elements than the input. Over an `Option`,
/// `by` applies to a present value and `none` stays `none`; over a
/// `Result`, `by` applies to the success value and a failure is unchanged.
elementwise fn map<F<_> : Iterable<_>, T, R>(self: F<T>, by: T -> R) : F<R>
elementwise fn map<T, R>(self: Option<T>, by: T -> R) : Option<R>
elementwise fn map<T, R, E>(self: Result<T, E>, by: T -> R) : Result<R, E>

// HQL-0033.
/// Each element through the fallible `operation`, invoked again while it
/// fails with an error `when` accepts (every error, without `when`) and
/// fewer than `attempts` invocations have run. The result is each
/// element's last result: its success, or its last failure.
elementwise fn retry<F<_> : Iterable<_>, T, U, E>(self: F<T>, operation: T -> Result<U, E>, attempts: Int) : F<Result<U, E>>
elementwise fn retry<F<_> : Iterable<_>, T, U, E>(self: F<T>, operation: T -> Result<U, E>, attempts: Int, when: E -> Bool) : F<Result<U, E>>

/// The first `n` elements. A prefix of a sequence keeps its positions and
/// its type: `[5, 3, 4] | take(2)` is `[5, 3]`. A prefix of a stream is a
/// finite `List`; a prefix of any other collection is taken in its
/// elements' order: `Set([5, 3, 4]) | take(2)` is `[3, 4]`. A negative `n`
/// is an evaluation error.
fn take<S : Seq<T>, T>(self: S, n: Int) : S
fn take<T>(self: Stream<T>, n: Int) : List<T>
fn take<T : Orderable>(self: Collection<T>, n: Int) : List<T>

/// The elements after the first `n`, in the input's order and type:
/// `[5, 3, 4] | drop(1)` is `[3, 4]`. The checker refuses an input without
/// a stable order, such as a `Set`.
fn drop<S : Iterable<T>, T>(self: S, n: Int) : S

/// Consecutive elements in batches of `n`, in order; the last batch may be
/// shorter: `[1, 2, 3, 4, 5] | collect(2)` is `[[1, 2], [3, 4], [5]]`. `n`
/// is at least `1`. `xs | collect(n) | emit` is `xs`.
fn collect<T>(self: Collection<T>, n: Int) : List<List<T>>
fn collect<T>(self: Stream<T>, n: Int) : Stream<List<T>>

/// Every member of every sequence, in order, the inverse of `collect`:
/// `[[1, 2], [3]] | emit` is `[1, 2, 3]`.
elementwise fn emit<T>(self: Collection<Seq<T>>) : List<T>
elementwise fn emit<T>(self: Stream<Seq<T>>) : Stream<T>

/// Each element through `by` to a sequence, and the members of those
/// sequences in order: `map(by) | emit`.
/// `[1, 2] | flat_map(x => [x, x])` is `[1, 1, 2, 2]`.
elementwise fn flat_map<T, U>(self: Collection<T>, by: T -> Seq<U>) : List<U>
elementwise fn flat_map<T, U>(self: Stream<T>, by: T -> Seq<U>) : Stream<U>

// Sorting supplies positions; its input need not already be a sequence. A
// sequence is rebuilt in its own constructor, so a List stays a List, while
// a Ranking, whose element is Hit<T> and not T, becomes a Seq<Hit<T>>: the
// new order is not the retrieval's.
/// The elements in ascending order: by their own value order, by the key
/// `by` gives each, or by a comparison `by` of two elements. A sequence
/// keeps its constructor; any other collection becomes a `List`.
/// `[3, 1, 2] | sort` is `[1, 2, 3]`, and
/// `[3, 1, 2] | sort(by = (a, b) => compare(b, a))` is `[3, 2, 1]`.
fn sort<F<_> : Seq<_>, T : Orderable>(self: F<T>) : F<T>
fn sort<T : Orderable>(self: Collection<T>) : List<T>
fn sort<F<_> : Seq<_>, T, K : Orderable>(self: F<T>, by: T -> K) : F<T>
fn sort<T, K : Orderable>(self: Collection<T>, by: T -> K) : List<T>
fn sort<F<_> : Seq<_>, T>(self: F<T>, by: (T, T) -> Ordering) : F<T>
fn sort<T>(self: Collection<T>, by: (T, T) -> Ordering) : List<T>

/// The occurrences partitioned by the key `by` gives each, as a set of
/// `Group`s, each a key and its occurrences in input order.
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

/// The pairs of an element of the input and an element of `other` that
/// match, each built by `into`, ordered by the input element's position,
/// then `other`'s. A pair matches when `left(x) == right(y)`, when `on`
/// holds for it, or both; a join states at least one condition.
fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>
fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, U) -> R) : Seq<R>
fn join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>

/// A `join` that keeps every element of the input: one with no match in
/// `other` is built by `into` with `none` on the right.
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>

/// A `join` that keeps every element of `other`: one with no match in the
/// input is built by `into` with `none` on the left.
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>

/// A `join` that keeps every element of both sides, with `none` for the
/// missing side; the unmatched elements of `other` come after the matched
/// results.
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>

/// Every pair of an element of the input and an element of `other`, built
/// by `into`, in order, with no condition:
/// `[1, 2] | cross_join(["a", "b"], into = (x, y) => [x, y])` is
/// `[[1, "a"], [1, "b"], [2, "a"], [2, "b"]]`.
fn cross_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, into: (T, U) -> R) : Seq<R>

/// The elements of the input that match some element of `other`, by keys,
/// a predicate `on`, or both; each is kept at most once, in its position.
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn semi_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>

/// The elements of the input that match no element of `other`, by keys, a
/// predicate `on`, or both, in their positions.
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn anti_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>

/// The elements folded from the left by `by`, from `initial`, or without
/// it from the first element, the result then `none` over `[]`:
/// `[1, 2, 3] | reduce(0, (a, x) => a + x)` is `6`. The checker refuses a
/// `Set`, which has no positions to fold in.
fn reduce<T, R>(self: Collection<T>, initial: R, by: (R, T) -> R) : R
fn reduce<T>(self: Collection<T>, by: (T, T) -> T) : Option<T>

/// The sum of the occurrences, `reduce(0, +)`; `[] | sum` is `0`.
reduction fn sum(self: Collection<Int>) : Int
reduction fn sum(self: Collection<Float>) : Float
reduction fn sum(self: Collection<Decimal>) : Decimal

/// The product of the occurrences, `reduce(1, *)`; `[] | product` is `1`.
reduction fn product(self: Collection<Int>) : Int
reduction fn product(self: Collection<Float>) : Float
reduction fn product(self: Collection<Decimal>) : Decimal

/// The mean of the occurrences, or `none` over `[]`:
/// `[1, 2] | average` is `1.5`. `Int`s and `Float`s average to a `Float`;
/// a `Decimal` mean is the sum divided by the count, as `Decimal` `/`
/// divides.
reduction fn average(self: Collection<Int>) : Option<Float>
reduction fn average(self: Collection<Float>) : Option<Float>
reduction fn average(self: Collection<Decimal>) : Option<Decimal>

/// Every map merged in order, a later key winning: `reduce({:}, merge)`.
reduction fn merge_all<T>(self: Collection<T>) : T

/// Every member of every set: `reduce(Set([]), union)`.
reduction fn union_all<T>(self: Collection<T>) : T

/// Every member of every sequence, in order: `reduce([], concat)`.
/// `[[1], [2, 3]] | concat_all` is `[1, 2, 3]`.
reduction fn concat_all<U>(self: Collection<Seq<U>>) : List<U>

/// The least occurrence by value order, `reduce(min)`, or `none` over `[]`.
reduction fn min<T : Orderable>(self: Collection<T>) : Option<T>

/// The greatest occurrence by value order, `reduce(max)`, or `none` over
/// `[]`.
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
/// The population variance of the occurrences, as a `Float`, or `none` over
/// `[]`: `[1, 2, 3, 4] | variance` is `1.25`.
reduction fn variance(self: Collection<Int>) : Option<Float>
reduction fn variance(self: Collection<Float>) : Option<Float>

// count_by: a count per key, combined by `+` per key; {} over [].
//   reduce(initial = Map<K, Int>([], []),
//          by = (m, x) => m | with(by(x), (m | get(by(x)) | unwrap_or(0)) + 1))
/// The number of occurrences with each key `by` gives, as a map from key
/// to count; empty over `[]`.
/// `["a", "bb", "c"] | count_by(s => size(s))` is `Map {1: 2, 2: 1}`.
reduction fn count_by<T, K>(self: Collection<T>, by: T -> K) : Map<K, Int>

// histogram: count_by the bucket's lower bound.
//   reduce(initial = SortedMap<Int, Int>([], []),
//          by = (m, x) => m | with(x - (x % width + width) % width,
//              (m | get(x - (x % width + width) % width) | unwrap_or(0)) + 1))
//   over Floats the bound is floor(x / width) * width.
/// The number of occurrences in each bucket of `width`, keyed by the
/// bucket's lower bound, the greatest multiple of `width` not above the
/// value, in key order: `[1, 5, 12, -3] | histogram(5)` is
/// `SortedMap {-5: 1, 0: 1, 5: 1, 10: 1}`. A `width` of zero or less is an
/// evaluation error.
reduction fn histogram(self: Collection<Int>, width: Int) : SortedMap<Int, Int>
reduction fn histogram(self: Collection<Float>, width: Float) : SortedMap<Float, Int>

// top_k: two partial lists combine by keeping the k greatest of both.
//   reduce(initial = List<T>([]),
//          by = (top, x) => concat(top, [x]) | sort(by = (a, b) => compare(b, a)) | take(k))
//   with `by`, the comparison is compare(by(b), by(a)).
/// The `k` greatest occurrences, by value order or by the key `by` gives,
/// greatest first, equal keys in input order:
/// `[3, 1, 4, 1, 5] | top_k(2)` is `[5, 4]`.
reduction fn top_k<T : Orderable>(self: Collection<T>, k: Int) : List<T>
reduction fn top_k<T, K : Orderable>(self: Collection<T>, k: Int, by: T -> K) : List<T>

// approx_distinct: its state is a HyperLogLog sketch, 4096 registers
// combined by `max` per register, over a SHA-256 digest of each value with
// no seed. HQL exposes no hash, so this longhand is not writable in HQL.
/// An estimate of the number of distinct occurrences, `size(Set(xs))`;
/// `0` over `[]`. The estimate is deterministic: the same input gives the
/// same result.
reduction fn approx_distinct<T>(self: Collection<T>) : Int

// The combiners: min(a, b), max(a, b), merge(m, n), union(s, t), concat(xs, ys).
// merge combines two maps, a later key winning, or two streams, one value
// from each in turn; concat joins two sequences, or two streams, one after
// the other (HQL-0037). merge and concat are also steps, `m | merge(n)`, and
// merge named as a value takes the form its expected function type selects.
// Named as a whole argument, a combiner or `+` / `*` is the function that
// combines two values with it: `reduce(0, +)`. A fold whose combiner ignores
// order (+, *, min, max, union) accepts any collection; one that does not
// visits positions, and so refuses a Set.
