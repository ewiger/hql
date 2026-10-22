// status: proposed
// feature: A position in a String is the one-character String there, or absence
// implementation: pending
// proposal: doc/proposals/HQL-0026/README.md
// environment: pure
// expected-type: List<Option<String>>
// expected-json: ["h","d",null]

// A String is indexed as a sequence of characters is: `s[i]` is the text of
// the character at `i`, `s[-1]` is the last one, and a position past the
// end is absence.
s = "hello, world"

[s[0], s[-1], s[20]]
