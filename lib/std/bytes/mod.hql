// Every declaration here is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0053/parts/01-bytes.hmd.

/// The functions of a `Bytes` value, in the prelude. A receiver module:
/// `b.size()`, `bytes.size(b)` and `b | size` are one declaration. `Bytes`
/// implements `Index<Int>` and `Slice` through `at` and `slice`: `b[0]` is
/// `b.at(0)`, the octet as an `Int` in `0..255`, and `b[0:12]` is a `Bytes`.
module bytes for Bytes : Index<Int>, Slice

import core

/// The octet of `self` at `position`, as an `Int` in `0..255`, or `none`
/// outside it; a negative position counts from the end:
/// `Bytes("hi").at(0)` and `Bytes("hi")[0]` are `104`.
fn at(self, position: Int) : Option<Int>

/// The octets of `self` from `start` to `end`, clamped to its size; a
/// negative bound counts from the end. With one argument, a `Range`, it is
/// `Slice`'s: `b[0:12]` is `b.slice(Range(start = 0, end = 12))`.
fn slice(self, start: Int, end: Int) : Bytes
fn slice(self, range: Range) : Bytes

/// How many octets `self` holds: `Bytes("hé").size()` is `3`.
fn size(self) : Int

/// `self` read as UTF-8 text, or `none` where it is not valid UTF-8:
/// `Bytes("hé").to_text()` is `some("hé")`.
fn to_text(self) : Option<String>
