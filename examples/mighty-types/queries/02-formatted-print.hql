// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: fmt with several placeholders, and str for any value's text
// expected-type: List<String>
// expected-json: ["31 days","a b c","pi is 3.14159","7 squared is 49","2.5","true","[1, 2, 3]"]

// `str(x)` is the text a value prints as; `fmt` writes several into a
// template in order. Neither is `format`, which writes a value in a
// presentation format such as Json.
pi = 3.14159

[
    fmt("{} days", 31),
    fmt("{} {} {}", "a", "b", "c"),
    fmt("{} is {}", "pi", pi),
    fmt("{} squared is {}", 7, 7 * 7),
    str(2.5),
    str(true),
    str([1, 2, 3]),
]
