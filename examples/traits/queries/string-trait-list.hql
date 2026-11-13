// status: valid-now
// feature: The string module lists Index and Slice, so s[1], s.at(1) and string.at(s, 1) reach one function
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["e", "e", "e", "e", "el", "el", "el"]

// `module string for String : Index<String>, Slice` states that a `String`
// implements both traits through the module's `at` and `slice`. A bracket,
// a receiver call and a qualified call reach the same declaration, and a
// bound `E : Index<T>` takes a `String`.
fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]

s = "hello"
part = Range(start = 1, end = 3)

[s[1], s.at(1), string.at(s, 1), first_of("ello")]
    | map(x => x.unwrap_or(""))
    | concat([s[1:3], s.slice(part), string.slice(s, part)])
