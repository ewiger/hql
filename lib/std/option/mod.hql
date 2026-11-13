// Every declaration is external: the `std` extension implements it.
//
// See doc/wiki/hql/types/option-type.hmd.

/// The ways out of an `Option<T>`, in the prelude, each stating what
/// absence gives. There is no `unwrap`. `map` and `filter` over an option
/// are the core's steps applied to the present value. See
/// [[/learn/02-data-types/option|Option: a value that may be missing]].
module option for Option<T>

import core

/// Whether `self` holds a value: `some(3) | is_some` is `true`.
fn is_some(self) : Bool

/// Whether `self` is `none`: `[1, 2][5] | is_none` is `true`.
fn is_none(self) : Bool

/// The value of `self` when present, and `default` otherwise:
/// `[1, 2][5] | unwrap_or(0)` is `0`.
fn unwrap_or(self, default: T) : T

/// The value of `self` when present, and what `default` returns otherwise;
/// `default` is called only when `self` is `none`:
/// `[1, 2][5] | unwrap_or_else(() => 7)` is `7`.
fn unwrap_or_else(self, default: () -> T) : T

/// `by` applied to the value of `self` when present, its own `none` kept,
/// and `none` otherwise: `[4][0] | and_then(x => [10][x])` is `none`.
fn and_then<U>(self, by: T -> Option<U>) : Option<U>

/// `self` when present, and the option `default` returns otherwise:
/// `[1][5] | or_else(() => [9][0])` is `9`.
fn or_else(self, default: () -> Option<T>) : Option<T>

/// `self` when present, and `other` otherwise:
/// `[1][5] | alternative([9][0])` is `9`.
fn alternative(self, other: Option<T>) : Option<T>
