// status: design-question
// feature: Option combinators
// implementation: pending
// environment: pure
// expected-type: Int
// expected: 5
// note: Not every use of an optional value deserves a match, so the standard library carries map and unwrap_or. Whether these are members reached with a dot or ordinary functions taking an Option is open, and the second reading composes with pipes once pipelines are settled.

title : Option<String> = Some("Alice")

title.map(t => t.length()).unwrap_or(0)
