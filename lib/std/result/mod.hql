// Every declaration is external: the `result` extension implements it.
//
// See doc/proposals/HQL-0024/parts/11-type-system-completion.md.

/// The ways out of a `Result<T, E>`, in the prelude, each stating what a
/// failure gives. A result is `ok(v)` or `err(e)`. There is no `unwrap`.
/// `map` over a result is the core's step applied to the value of a success:
/// `ok(3) | map(n => n + 1)` is `ok(4)`.
module result for Result<T, E>

import core

/// Whether `self` is a success: `ok(4) | is_ok` is `true`.
fn is_ok(self) : Bool

/// Whether `self` is a failure: `err("x") | is_err` is `true`.
fn is_err(self) : Bool

/// The value of `self` when a success, and `none` when a failure:
/// `err("x") | ok` is `none`.
fn ok(self) : Option<T>

/// The error of `self` when a failure, and `none` when a success:
/// `ok(4) | err` is `none`.
fn err(self) : Option<E>

/// The value of `self` when a success, and `default` when a failure:
/// `err("x") | unwrap_or(0)` is `0`.
fn unwrap_or(self, default: T) : T

/// The value of `self` when a success, and `default` applied to the error
/// when a failure: `err("ab") | unwrap_or_else(e => e.size())` is `2`.
fn unwrap_or_else(self, default: E -> T) : T

/// A failure with `by` applied to its error; a success is kept as it is:
/// `err("x") | map_err(e => e.upper())` is `err("X")`.
fn map_err<U>(self, by: E -> U) : Result<T, U>

/// `by` applied to the value of a success, its own failure kept; a failure
/// is kept as it is: `ok(4) | and_then(n => ok(n * 2))` is `ok(8)`.
fn and_then<U>(self, by: T -> Result<U, E>) : Result<U, E>

/// `by` applied to the error of a failure, giving another result; a success
/// is kept as it is: `err("x") | or_else(e => ok(e.size()))` is `ok(1)`.
fn or_else<U>(self, by: E -> Result<T, U>) : Result<T, U>
