// Module `seq`: the sequence, set and map operations beyond the core's
// membership, order and lookup, in the prelude. A receiver module over
// `Seq<T>`, with `T` bound per call; a function that takes any collection
// writes its receiver, `self: Collection<T>`, and the functions over sets and
// maps take theirs as a typed first parameter. A receiver `S : Seq<T>` comes
// back at its own type, and `F<_> : Seq<_>` rebuilt in its own constructor.
// Every declaration is external: the `seq` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.
module for Seq<T>

fn first(self) : Option<T>
fn last(self) : Option<T>
// A Ranking reversed is a Seq<Hit<T>>: the new order is not the retrieval's.
fn reverse<F<_> : Seq<_>>(self: F<T>) : F<T>
fn distinct<S : Seq<T>>(self: S) : S
fn zip<U>(self, other: Seq<U>) : List<Pair<T, U>>
fn enumerate(self) : List<Pair<Int, T>>
reduction fn any(self: Collection<T>, by: T -> Bool) : Bool
reduction fn all(self: Collection<T>, by: T -> Bool) : Bool
fn find(self, by: T -> Bool) : Option<T>
fn position(self, by: T -> Bool) : Option<Int>
fn take_while<S : Seq<T>>(self: S, by: T -> Bool) : S
fn take_while(self: Iterable<T>, by: T -> Bool) : List<T>
fn drop_while<S : Seq<T>>(self: S, by: T -> Bool) : S
fn window(self, n: Int) : List<List<T>>
// The values redistributed by a key (HQL-0036): each value once, at its own
// type, those of one key in one logical partition. The step is a stage
// boundary. The local backend has one partition per key on one worker, and
// passes the values on in their order, which keeps each partition's.
partition fn partition<S : Seq<T>, K>(self: S, by: T -> K) : S
partition fn partition<K>(self: Stream<T>, by: T -> K) : Stream<T>
reduction fn is_empty(self: Collection<T>) : Bool
fn join<U>(self: Collection<T>, other: Collection<U>, by: (T, U) -> Bool) : List<Pair<T, U>>
fn min_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn min_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>
fn max_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn max_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>
fn to_map<K, V>(self: Collection<T>, key: T -> K, value: T -> V) : Map<K, V>

// The Ints from a start up to, not including, an end: `range(1, 6)`, and
// `range(10, 0, -3)` with a step.
fn range(start: Int, end: Int, step: Int = 1) : List<Int>

// Over a map, which keeps its own type.
fn with<M : Map<K, V>, K, V>(self: M, key: K, value: V) : M
fn without<M : Map<K, V>, K, V>(self: M, key: K) : M
fn entries<K, V>(self: Map<K, V>) : List<Pair<K, V>>
fn values<K, V>(self: Map<K, V>) : List<V>

// Over a set; `union` is a combiner of the core.
fn intersection<S : Set<T>, T>(self: S, other: Set<T>) : S
fn difference<S : Set<T>, T>(self: S, other: Set<T>) : S
