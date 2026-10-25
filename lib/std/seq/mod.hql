// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

/// The sequence, set and map operations beyond the core's membership, order
/// and lookup, in the prelude. A receiver module over `Seq<T>`, with `T`
/// bound per call; a function that takes any collection writes its receiver,
/// `self: Collection<T>`, and the functions over sets and maps take theirs as
/// a typed first parameter. A receiver `S : Seq<T>` comes back at its own
/// type, and `F<_> : Seq<_>` rebuilt in its own constructor.
module seq for Seq<T>

import core

/// The element at position 0, or `none` over an empty sequence:
/// `[3, 1, 2] | first` is `3`.
fn first(self) : Option<T>

/// The element at the last position, or `none` over an empty sequence:
/// `[3, 1, 2] | last` is `2`.
fn last(self) : Option<T>

/// The elements of `self` in the opposite order, in its own constructor:
/// `[1, 2, 3] | reverse` is `[3, 2, 1]`. A `Ranking` reversed is a
/// `Seq<Hit<T>>`: the new order is not the retrieval's.
fn reverse<F<_> : Seq<_>>(self: F<T>) : F<T>

/// Each value of `self` once, at its first position, at the type of `self`:
/// `[1, 2, 1, 3, 2] | distinct` is `[1, 2, 3]`.
fn distinct<S : Seq<T>>(self: S) : S

/// Pairs of the elements of `self` and `other` at equal positions, as long
/// as the shorter: `[1, 2, 3] | zip(["a", "b"])` is
/// `[Pair(first = 1, second = "a"), Pair(first = 2, second = "b")]`.
fn zip<U>(self, other: Seq<U>) : List<Pair<T, U>>

// Over streams: HQL-0037.

/// Pairs of the elements of `self` and `other` at equal positions, where
/// either is a `Stream`, until either ends: a `Stream` beside a `Seq` ends
/// with it, as a `Seq`, and two `Stream`s give a `Stream`.
fn zip<U>(self: Stream<T>, other: Seq<U>) : Seq<Pair<T, U>>
fn zip<U>(self, other: Stream<U>) : Seq<Pair<T, U>>
fn zip<U>(self: Stream<T>, other: Stream<U>) : Stream<Pair<T, U>>

/// Each element of `self` paired with its position, counting from 0:
/// `["a", "b"] | enumerate` is
/// `[Pair(first = 0, second = "a"), Pair(first = 1, second = "b")]`.
fn enumerate(self) : List<Pair<Int, T>>

/// Whether `by` holds of some value of `self`; `false` over an empty
/// collection: `[1, 2, 3] | any(x => x > 2)` is `true`.
reduction fn any(self: Collection<T>, by: T -> Bool) : Bool

/// Whether `by` holds of every value of `self`; `true` over an empty
/// collection: `[] | all(x => x > 2)` is `true`.
reduction fn all(self: Collection<T>, by: T -> Bool) : Bool

/// The first element of `self` that `by` accepts, or `none`:
/// `[1, 2, 3, 4] | find(x => x > 2)` is `3`.
fn find(self, by: T -> Bool) : Option<T>

/// The position of the first element of `self` that `by` accepts, counting
/// from 0, or `none`: `[1, 2, 3, 4] | position(x => x > 2)` is `2`.
fn position(self, by: T -> Bool) : Option<Int>

/// The leading values of `self` that `by` accepts, up to the first it
/// rejects: `[1, 2, 5, 1] | take_while(x => x < 3)` is `[1, 2]`. A sequence
/// comes back at its own type, and any other iterable as a `List`.
fn take_while<S : Seq<T>>(self: S, by: T -> Bool) : S
fn take_while(self: Iterable<T>, by: T -> Bool) : List<T>

/// The values of `self` after the leading ones `by` accepts, at the type of
/// `self`: `[1, 2, 5, 1] | drop_while(x => x < 3)` is `[5, 1]`.
fn drop_while<S : Seq<T>>(self: S, by: T -> Bool) : S

/// Every run of `n` consecutive elements of `self`, in order:
/// `[1, 2, 3, 4] | window(2)` is `[[1, 2], [2, 3], [3, 4]]`. An `n` below 1
/// is an evaluation error.
fn window(self, n: Int) : List<List<T>>

// Partitioning: HQL-0036.

/// The values of `self` redistributed by the key `by` gives: each value
/// once, at its own type, those of one key in one logical partition. The
/// step is a stage boundary. The local backend has one partition per key on
/// one worker and passes the values on in their order.
partition fn partition<S : Seq<T>, K>(self: S, by: T -> K) : S
partition fn partition<K>(self: Stream<T>, by: T -> K) : Stream<T>

/// Whether `self` has no values: `[] | is_empty` is `true`.
reduction fn is_empty(self: Collection<T>) : Bool

/// Every pair of a value of `self` and a value of `other` that `by` relates,
/// in the order of `self`, then of `other`:
/// `[1, 2] | join([1, 2, 3], (a, b) => a < b)` gives the pairs `(1, 2)`,
/// `(1, 3)` and `(2, 3)`.
fn join<U>(self: Collection<T>, other: Collection<U>, by: (T, U) -> Bool) : List<Pair<T, U>>

/// The value of `self` that is least, by the key `by` gives or by the
/// `Ordering` `by` gives two values; the first of equals, or `none` over an
/// empty collection: `["bb", "a", "ccc"] | min_by(s => size(s))` is `"a"`.
fn min_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn min_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>

/// The value of `self` that is greatest, by the key `by` gives or by the
/// `Ordering` `by` gives two values; the first of equals, or `none` over an
/// empty collection: `["bb", "a", "ccc"] | max_by(s => size(s))` is `"ccc"`.
fn max_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn max_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>

/// A map from the `key` of each value of `self` to its `value`; two values
/// with one key are an evaluation error:
/// `["bb", "a"] | to_map(s => s, s => size(s))` is `Map {"bb": 2, "a": 1}`.
fn to_map<K, V>(self: Collection<T>, key: T -> K, value: T -> V) : Map<K, V>

/// The `Int`s from `start` up to, not including, `end`, each `step` after
/// the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and
/// `range(10, 0, -3)` is `[10, 7, 4, 1]`.
fn range(start: Int, end: Int, step: Int = 1) : List<Int>

/// The map `self` with `key` set to `value`, added or replaced, at the type
/// of `self`: `Map(["a"], [1]) | with("b", 2)` is `Map {"a": 1, "b": 2}`.
fn with<M : Map<K, V>, K, V>(self: M, key: K, value: V) : M

/// The map `self` without `key`, whether or not it held it, at the type of
/// `self`: `Map(["a", "b"], [1, 2]) | without("a")` is `Map {"b": 2}`.
fn without<M : Map<K, V>, K, V>(self: M, key: K) : M

/// Each association of the map `self` as a `Pair` of its key and its value,
/// in the map's order.
fn entries<K, V>(self: Map<K, V>) : List<Pair<K, V>>

/// The values of the map `self`, in the map's order:
/// `Map(["b", "a"], [2, 1]) | values` is `[2, 1]`.
fn values<K, V>(self: Map<K, V>) : List<V>

/// The members of both `self` and `other`, at the type of `self`:
/// `{1, 2, 3} | intersection({2, 3, 4})` is `{2, 3}`. The union is the
/// core's `union`.
fn intersection<S : Set<T>, T>(self: S, other: Set<T>) : S

/// The members of `self` that `other` does not hold, at the type of `self`:
/// `{1, 2, 3} | difference({2, 3, 4})` is `{1}`.
fn difference<S : Set<T>, T>(self: S, other: Set<T>) : S
