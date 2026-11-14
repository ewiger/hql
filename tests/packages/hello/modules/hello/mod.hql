// The contract of module `hello`: every function is declared without a
// body, and the package's extension implements it.

/// Greetings, provided by the `hello` package.
module hello

import core

/// A greeting and the number of characters in it.
type Greeting { text : String, length : Int }

/// The greeting of a name: `Greeting("Ada")` holds `Hello, Ada!`.
/// `punctuation` replaces the `!`.
fn Greeting(name: String, *, punctuation: String = "!") : Greeting

/// The text greeting a name: `greet("Ada")` is `Hello, Ada!`.
fn greet(name: String) : String

/// `self` in capitals: `"hi".shout()` is `HI`.
fn shout(self: String) : String

/// The text of a greeting in capitals.
fn shout(self: Greeting) : String

/// The sum of `weight` over the elements of `self`.
fn total<T>(self: List<T>, weight: T -> Int) : Int

/// A function that panics, which the call reports as a failure.
fn fail(message: String) : String
