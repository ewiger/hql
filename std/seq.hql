// Module `seq`: the sequence, set and map operations beyond the core's
// membership, order and lookup, in the prelude. A receiver module over
// `Seq<T>`, with `T` bound per call; the functions over sets and maps take
// their receiver as a typed first parameter. Every declaration is external:
// the `seq` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.
module for Seq<T>

fn first(self) : Option<T>
fn last(self) : Option<T>
fn reverse(self) : Seq<T>
fn distinct(self) : Seq<T>
fn zip<U>(self, other: Seq<U>) : List<Pair<T, U>>
fn enumerate(self) : List<Pair<Int, T>>
fn any(self, by: T -> Bool) : Bool
fn all(self, by: T -> Bool) : Bool
fn find(self, by: T -> Bool) : Option<T>
fn position(self, by: T -> Bool) : Option<Int>
fn take_while(self, by: T -> Bool) : Seq<T>
fn drop_while(self, by: T -> Bool) : Seq<T>
fn window(self, n: Int) : List<List<T>>
fn partition(self, by: T -> Bool) : Pair<List<T>, List<T>>
fn is_empty(self) : Bool
fn join<U>(self, other: Collection<U>, by: (T, U) -> Bool) : List<Pair<T, U>>
fn min_by<K : Orderable>(self, by: T -> K) : Option<T>
fn max_by<K : Orderable>(self, by: T -> K) : Option<T>
fn to_map<K, V>(self, key: T -> K, value: T -> V) : Map<K, V>

// The Ints from a start up to, not including, an end: `range(1, 6)`, and
// `range(10, 0, -3)` with a step.
fn range(start: Int, end: Int, step: Int) : List<Int>

// Over a map.
fn with<K, V>(m: Map<K, V>, key: K, value: V) : Map<K, V>
fn without<K, V>(m: Map<K, V>, key: K) : Map<K, V>
fn entries<K, V>(m: Map<K, V>) : List<Pair<K, V>>
fn values<K, V>(m: Map<K, V>) : List<V>

// Over a set; `union` is a combiner of the core.
fn intersection<U>(s: Set<U>, other: Set<U>) : Set<U>
fn difference<U>(s: Set<U>, other: Set<U>) : Set<U>
