// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: A formatted greeting, the first program of every textbook
// expected-type: String
// expected: Hello, world!

// `fmt` fills each `{}` with the text of an argument. The program's value
// is its last expression, so the greeting is what `hql run` prints.
greeting = fmt("Hello, {}!", "world")

greeting
