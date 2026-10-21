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
fn str<T>(self) : String
fn fmt(self, args: ...Data) : String

fn length(self) : Int
fn upper(self) : String
fn lower(self) : String
fn trim(self) : String
fn trim_start(self) : String
fn trim_end(self) : String
fn capitalize(self) : String
fn split(self, separator: String) : List<String>
// Over a collection of texts, the one function here whose receiver is not
// a String: `["a", "b"].join(", ")`.
fn join(self, separator: String) : String
fn replace(self, from: String, to: String) : String
fn starts_with(self, part: String) : Bool
fn ends_with(self, part: String) : Bool
fn contains(self, part: String) : Bool
fn index_of(self, part: String) : Option<Int>
fn chars(self) : List<String>
fn lines(self) : List<String>
fn words(self) : List<String>
fn reverse(self) : String
fn repeat(self, times: Int) : String
fn pad_left(self, width: Int, fill: String) : String
fn pad_right(self, width: Int, fill: String) : String
fn slice(self, start: Int, end: Int) : String
fn char_at(self, position: Int) : Option<String>
fn to_int(self) : Option<Int>
fn to_float(self) : Option<Float>
fn is_empty(self) : Bool
