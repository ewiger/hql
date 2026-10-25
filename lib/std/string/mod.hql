// Every declaration here is external: the `std` extension implements it, and
// a test holds the two to each other.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md and
// 04-receiver-modules.md.

/// The functions of a `String`, in the prelude. A receiver module: each
/// function takes `self`, so `s.trim()`, `string.trim(s)` and `s | trim` are
/// one declaration. A `String` is a sequence of Unicode scalar values held
/// as UTF-8, whatever encoding its text was read from. Positions and size
/// count code points.
module string for String

import core

/// The text `self` prints as, for a value of any type: `str(some(2))` is
/// `"2"`.
fn str<T>(self: T) : String

/// The template `self` with each `{}` replaced by the text of the next of
/// `args`, in order: `fmt("{} of {}", 1, some(2))` is `"1 of 2"`. `...Any`
/// takes a value of any type in each position. A count of `args` other than
/// the count of `{}` is an evaluation error. Not `format`, which encodes a
/// value in a `Format`.
fn fmt(self, args: ...Any) : String

/// How many characters `self` has, counting code points: `"héllo" | length`
/// is `5`.
fn length(self) : Int

/// How many characters `self` has, counting code points; the same as
/// `length`: `"héllo" | size` is `5`.
fn size(self) : Int

/// `self` in upper case: `"Straße" | upper` is `"STRASSE"`.
fn upper(self) : String

/// `self` in lower case: `"ÀB" | lower` is `"àb"`.
fn lower(self) : String

/// `self` without leading and trailing whitespace: `"  a b " | trim` is
/// `"a b"`.
fn trim(self) : String

/// `self` without leading whitespace: `"  a " | trim_start` is `"a "`.
fn trim_start(self) : String

/// `self` without trailing whitespace: `"  a " | trim_end` is `"  a"`.
fn trim_end(self) : String

/// `self` with its first character in upper case: `"élan" | capitalize` is
/// `"Élan"`.
fn capitalize(self) : String

/// The pieces of `self` between occurrences of `separator`, in order:
/// `"a,b,,c" | split(",")` is `["a", "b", "", "c"]`. A `limit` of 1 or more
/// keeps at most that many pieces, the last holding the rest of the text:
/// `"a,b,c" | split(",", 2)` is `["a", "b,c"]`; below 1 there is no limit.
fn split(self, separator: String, limit: Int = 0) : List<String>

/// The texts of `self` in order, with `separator` between each two:
/// `["a", "b"].join(", ")` is `"a, b"`. The one function here whose
/// receiver is not a `String`, so the receiver is written.
fn join(self: Collection<String>, separator: String) : String

/// `self` with every occurrence of `from` replaced by `to`:
/// `"aaa" | replace_all("a", "b")` is `"bbb"`.
fn replace_all(self, from: String, to: String) : String

/// `self` with the first occurrence of `from` replaced by `to`:
/// `"aaa" | replace_first("a", "b")` is `"baa"`.
fn replace_first(self, from: String, to: String) : String

/// Whether `self` begins with `part`: `"hello" | starts_with("he")` is
/// `true`.
fn starts_with(self, part: String) : Bool

/// Whether `self` ends with `part`: `"hello" | ends_with("lo")` is `true`.
fn ends_with(self, part: String) : Bool

/// Whether `part` occurs in `self`: `"hello" | contains("ell")` is `true`.
fn contains(self, part: String) : Bool

/// The position of the first occurrence of `part` in `self`, counting
/// characters from 0, or `none`: `"héllo" | index_of("l")` is `2`.
fn index_of(self, part: String) : Option<Int>

/// The position of the last occurrence of `part` in `self`, counting
/// characters from 0, or `none`: `"héllo" | last_index_of("l")` is `3`.
fn last_index_of(self, part: String) : Option<Int>

/// How many non-overlapping times `part` occurs in `self`; 0 for an empty
/// `part`: `"aaaa" | occurrences("aa")` is `2`.
fn occurrences(self, part: String) : Int

/// `self` without `part` at its start, or `none` when it does not begin
/// with it: `"x.hql" | strip_prefix("x.")` is `"hql"`.
fn strip_prefix(self, part: String) : Option<String>

/// `self` without `part` at its end, or `none` when it does not end with
/// it: `"x.hql" | strip_suffix(".hql")` is `"x"`.
fn strip_suffix(self, part: String) : Option<String>

/// `self` without `part` at its start, or `self` as it is when it does not
/// begin with it: `"x" | trim_prefix("y")` is `"x"`.
fn trim_prefix(self, part: String) : String

/// `self` without `part` at its end, or `self` as it is when it does not
/// end with it: `"x.hql" | trim_suffix(".hql")` is `"x"`.
fn trim_suffix(self, part: String) : String

/// Whether `self` and `other` are equal in lower case:
/// `"ABC" | equals_ignore_case("abc")` is `true`.
fn equals_ignore_case(self, other: String) : Bool

/// Each character of `self` as its own text, in order: `"héy" | chars` is
/// `["h", "é", "y"]`.
fn chars(self) : List<String>

/// The lines of `self`, without their line breaks; a final line break
/// opens no empty line: `string.lines("a\nb\n")` is `["a", "b"]`.
fn lines(self) : List<String>

/// The whitespace-separated words of `self`: `"  a  b\tc " | words` is
/// `["a", "b", "c"]`.
fn words(self) : List<String>

/// The characters of `self` in the opposite order: `"héllo" | reverse` is
/// `"olléh"`.
fn reverse(self) : String

/// `self` written `times` times in a row; `""` for `times` below 1:
/// `"ab" | repeat(3)` is `"ababab"`.
fn repeat(self, times: Int) : String

/// `self` preceded by `fill`, repeated, until it is `width` characters long;
/// a longer `self` as it is: `"7" | pad_left(3, "0")` is `"007"`.
fn pad_left(self, width: Int, fill: String) : String

/// `self` followed by `fill`, repeated, until it is `width` characters long;
/// a longer `self` as it is: `"7" | pad_right(3, "0")` is `"700"`.
fn pad_right(self, width: Int, fill: String) : String

/// `self` between `fill`s until it is `width` characters long; an odd fill
/// character goes after it: `"ab" | center(5, "*")` is `"*ab**"`.
fn center(self, width: Int, fill: String) : String

/// At most the first `width` characters of `self`:
/// `"hello" | truncate(3)` is `"hel"`.
fn truncate(self, width: Int) : String

/// The characters of `self` from `start` up to, not including, `end`,
/// counting from 0; `s[start:end]`. The bounds are clamped to the text, and
/// a negative bound counts from the end: `"hello" | slice(-3, 10)` is
/// `"llo"`. `s.slice(range)`, with one argument, is the core trait `Slice`.
fn slice(self, start: Int, end: Int) : String

/// The character of `self` at `position`, counting from 0, or `none`
/// outside the text: `"hello" | char_at(1)` is `"e"`.
fn char_at(self, position: Int) : Option<String>

/// The integer `self` writes, surrounding whitespace aside, or `none` when
/// it writes none: `"-42" | to_int` is `-42`, and `"1.5" | to_int` is
/// `none`.
fn to_int(self) : Option<Int>

/// The number `self` writes, as a `Float`, or `none` when it writes none:
/// `"1e3" | to_float` is `1000.0`.
fn to_float(self) : Option<Float>

/// The exact decimal number `self` writes, at its written scale, or `none`
/// when it writes none: `"2.50" | to_decimal` is `2.50`.
fn to_decimal(self) : Option<Decimal>

/// The truth value `self` writes as `true` or `false`, or `none` for any
/// other text: `"True" | to_bool` is `none`.
fn to_bool(self) : Option<Bool>

/// Whether `self` has no characters: `"" | is_empty` is `true`.
fn is_empty(self) : Bool

/// Whether `self` has no characters other than whitespace: `" " | is_blank`
/// is `true`.
fn is_blank(self) : Bool

/// The Unicode code point of the one-character text `self`, or `none` for
/// any other text: `"a" | ord` is `97`.
fn ord(self) : Option<Int>

/// The one-character text of the Unicode code point `self`, or `none` for
/// an `Int` that is none: `97 | chr` is `"a"`. The receiver is an `Int`, so
/// it is written.
fn chr(self: Int) : Option<String>

/// The UTF-8 encoding of `self`, one `Int` per byte: `"é" | bytes` is
/// `[195, 169]`.
fn bytes(self) : List<Int>
