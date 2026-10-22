// status: proposed
// feature: A slice of a String is a String, between two character positions
// implementation: pending
// proposal: doc/proposals/HQL-0026/README.md
// environment: pure
// expected-type: List<String>
// expected-json: ["hello","world","hello, world","world","","lo, world"]

// `s[a:b]` is the text from position `a` up to, not including, position `b`,
// counted in characters. A missing bound is the start or the end, a negative
// position counts from the end, and a bound past the end is the end: a slice
// is never a failure, only a shorter text.
s = "hello, world"

[
    s[0:5],
    s[7:],
    s[:],
    s[-5:],
    s[5:0],
    s[3:100],
]
