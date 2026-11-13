// Imported where it is used, since a pattern language is a domain of its
// own. Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

/// Text matched against a regular expression. A pattern is a `String`; one
/// that does not parse is an evaluation error at the call. The module is
/// imported where it is used: `import regex`.
module regex

import core

/// Whether `pattern` matches somewhere in `text`:
/// `"a1b22c" | regex.is_match("[0-9]+")` is `true`.
fn is_match(text: String, pattern: String) : Bool

/// The first match of `pattern` in `text`, or `none`:
/// `"a1b22c" | regex.find("[0-9]+")` is `"1"`.
fn find(text: String, pattern: String) : Option<String>

/// Every non-overlapping match of `pattern` in `text`, in order:
/// `"a1b22c" | regex.find_all("[0-9]+")` is `["1", "22"]`.
fn find_all(text: String, pattern: String) : List<String>

/// The first match of `pattern` in `text` and its groups, whole match first,
/// or `none`; a group that took no part in the match is `""`:
/// `"a" | regex.captures("(a)|(b)")` is `["a", "a", ""]`.
fn captures(text: String, pattern: String) : Option<List<String>>

/// `text` with every match of `pattern` replaced by `replacement`, in which
/// `$1` is the first group:
/// `"a1b22c" | regex.replace_all("([0-9]+)", "<$1>")` is `"a<1>b<22>c"`.
fn replace_all(text: String, pattern: String, replacement: String) : String

/// `text` with the first match of `pattern` replaced by `replacement`, in
/// which `$1` is the first group:
/// `"a1b22c" | regex.replace_first("[0-9]+", "#")` is `"a#b22c"`.
fn replace_first(text: String, pattern: String, replacement: String) : String

/// The pieces of `text` between matches of `pattern`, in order:
/// `"a1b22c" | regex.split("[0-9]+")` is `["a", "b", "c"]`.
fn split(text: String, pattern: String) : List<String>
