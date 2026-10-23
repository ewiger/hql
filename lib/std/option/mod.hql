// Module `option`: the ways out of an Option<T>, in the prelude, each saying
// what absence means. There is no unwrap. `map` and `filter` over an option
// are the core's steps lifted over the present value. Every declaration is
// external: the `option` extension implements it.
//
// See doc/wiki/hql/types/option-type.hmd.
module for Option<T>
import core

fn is_some(self) : Bool
fn is_none(self) : Bool
fn unwrap_or(self, default: T) : T
fn unwrap_or_else(self, default: () -> T) : T
fn and_then<U>(self, by: T -> Option<U>) : Option<U>
fn or_else(self, default: () -> Option<T>) : Option<T>
fn alternative(self, other: Option<T>) : Option<T>
