// Module `result`: the ways out of a Result<T, E>, in the prelude, each
// saying what a failure means. There is no unwrap. `map` over a result is the
// core's step over the success. Every declaration is external: the `result`
// extension implements it.
//
// See doc/proposals/HQL-0024/parts/11-type-system-completion.md.
module result for Result<T, E>
import core

fn is_ok(self) : Bool
fn is_err(self) : Bool
fn ok(self) : Option<T>
fn err(self) : Option<E>
fn unwrap_or(self, default: T) : T
fn unwrap_or_else(self, default: E -> T) : T
fn map_err<U>(self, by: E -> U) : Result<T, U>
fn and_then<U>(self, by: T -> Result<U, E>) : Result<U, E>
fn or_else<U>(self, by: E -> Result<T, U>) : Result<T, U>
