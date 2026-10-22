// Module `string`: the functions of a String, in the prelude. A receiver
// module: each function takes `self`, so `s.trim()`, `string.trim(s)` and
// `s | trim` are one declaration. Every declaration here is external: the
// `string` extension implements it, and a test holds the two to each other.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md and
// 04-receiver-modules.md.
module for String

// The text a value prints as, and a template filled with the text of each
// argument in order: `fmt("{} of {}", 1, 2)`. Neither is `format`, which
// encodes a value in a Format.
fn str<T>(self: T) : String
fn fmt(self, args: ...Data) : String

fn length(self) : Int
fn upper(self) : String
fn lower(self) : String
fn trim(self) : String
fn trim_start(self) : String
fn trim_end(self) : String
fn capitalize(self) : String
// A limit of 1 or more keeps at most that many pieces, the last holding
// the rest of the text; below 1 there is no limit.
fn split(self, separator: String, limit: Int = 0) : List<String>
// Over a collection of texts, the one function here whose receiver is not
// a String, so the receiver is written: `["a", "b"].join(", ")`.
fn join(self: Collection<String>, separator: String) : String
fn replace_all(self, from: String, to: String) : String
fn replace_first(self, from: String, to: String) : String
fn starts_with(self, part: String) : Bool
fn ends_with(self, part: String) : Bool
fn contains(self, part: String) : Bool
fn index_of(self, part: String) : Option<Int>
fn last_index_of(self, part: String) : Option<Int>
fn occurrences(self, part: String) : Int
fn strip_prefix(self, part: String) : Option<String>
fn strip_suffix(self, part: String) : Option<String>
fn trim_prefix(self, part: String) : String
fn trim_suffix(self, part: String) : String
fn equals_ignore_case(self, other: String) : Bool
fn chars(self) : List<String>
fn lines(self) : List<String>
fn words(self) : List<String>
fn reverse(self) : String
fn repeat(self, times: Int) : String
fn pad_left(self, width: Int, fill: String) : String
fn pad_right(self, width: Int, fill: String) : String
fn center(self, width: Int, fill: String) : String
fn truncate(self, width: Int) : String
fn slice(self, start: Int, end: Int) : String
fn char_at(self, position: Int) : Option<String>
fn to_int(self) : Option<Int>
fn to_float(self) : Option<Float>
fn to_bool(self) : Option<Bool>
fn is_empty(self) : Bool
fn is_blank(self) : Bool
// The Unicode code point of a one-character text, and the one-character
// text of a code point. `chr` is over an Int, so its receiver is written.
fn ord(self) : Option<Int>
fn chr(self: Int) : Option<String>
// The UTF-8 encoding of the text, one Int per byte.
fn bytes(self) : List<Int>
