// Module `regex`: text matched against a pattern. Imported where it is used,
// since a pattern language is a domain of its own. A pattern is a String; one
// that does not parse is an evaluation error at the call. Every declaration
// is external: the `regex` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

fn is_match(text: String, pattern: String) : Bool
fn find(text: String, pattern: String) : Option<String>
fn find_all(text: String, pattern: String) : List<String>
// The first match and its groups, whole match first.
fn captures(text: String, pattern: String) : Option<List<String>>
// `$1` in the replacement is the first group.
fn replace_all(text: String, pattern: String, replacement: String) : String
fn replace_first(text: String, pattern: String, replacement: String) : String
fn split(text: String, pattern: String) : List<String>
