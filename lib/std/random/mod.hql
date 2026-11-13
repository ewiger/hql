// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/04-random.md.

/// Randomness: samples, shuffles, choices, numbers and UUIDs. Each draw has
/// two forms. With a `seed` argument it is pure: its result is a function of
/// the seed, its input and its call site. Without one it is effectful and
/// draws from the evaluation seed, fixed once per evaluation from the host's
/// entropy or `--seed`. The generator is ChaCha12 and is not fit for keys,
/// tokens or any other secret: a seeded draw is predictable by design. Not
/// in the prelude: a program writes `import random`.
module random

import core

// Sampling.

/// `n` elements of `self` drawn without replacement, in their order in
/// `self`; all of `self` where `n` is not below its size. A negative `n` is
/// an evaluation error. A `Collection` without an order is sorted by its
/// elements first, so the sample does not depend on how it was built.
fn sample<T>(self: Seq<T>, n: Int, *, seed: Int) : List<T>
fn sample<T : Orderable>(self: Collection<T>, n: Int, *, seed: Int) : List<T>
effectful fn sample<T>(self: Seq<T>, n: Int) : List<T>
effectful fn sample<T : Orderable>(self: Collection<T>, n: Int) : List<T>

/// `n` elements of a stream, drawn by reservoir sampling: the step holds
/// `n` elements, never the stream, and gives its sample when the stream
/// ends. A stream that does not end gives no sample.
effectful fn sample<T>(self: Stream<T>, n: Int) : List<T>

/// The elements of `self` in a uniformly random order (Fisher–Yates).
fn shuffle<T>(self: Seq<T>, *, seed: Int) : List<T>
effectful fn shuffle<T>(self: Seq<T>) : List<T>

/// One element of `self` drawn uniformly, or `none` where `self` is empty.
fn choice<T>(self: Seq<T>, *, seed: Int) : Option<T>
effectful fn choice<T>(self: Seq<T>) : Option<T>

// Numbers.

/// An `Int` drawn uniformly from `low` to `high`, both included. `high`
/// below `low` is an evaluation error.
effectful fn random_int(low: Int, high: Int) : Int

/// A `Float` drawn uniformly from `0.0` included to `1.0` excluded.
effectful fn random_float() : Float

// UUIDs, as lowercase hyphenated text.

/// A version 7 UUID: its first 48 bits are the evaluation instant's Unix
/// milliseconds and the next 42 a counter, so the UUIDs of one evaluation
/// sort in the order they were drawn.
effectful fn uuid() : String

/// The version 5 UUID of `name` in `namespace`: a UUID's text, or one of
/// the RFC 9562 namespaces `"dns"`, `"url"`, `"oid"` and `"x500"`. Any
/// other namespace is an evaluation error.
fn uuid(name: String, namespace: String) : String
