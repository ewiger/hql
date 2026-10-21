// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: match over literals, guards, types and nested options
// expected-type: List<String>
// expected-json: ["1 2 Fizz 4 Buzz Fizz 7 8 Fizz Buzz 11 Fizz 13 14 FizzBuzz","negative","zero","positive","int","text hi","other","starts with one"]

// The first arm whose pattern fits and whose guard holds produces the
// result; the unguarded arms must cover the subject. A type pattern over an
// open tree reads the leaf at that type.
fn fizzbuzz(n: Int) : String = match n % 15 {
    0 => "FizzBuzz",
    3 => "Fizz",
    6 => "Fizz",
    9 => "Fizz",
    12 => "Fizz",
    5 => "Buzz",
    10 => "Buzz",
    _ => str(n),
}

fn sign(n: Int) : String = match n {
    0 => "zero",
    k given k < 0 => "negative",
    _ => "positive",
}

fn classify(x: Data) : String = match x {
    Int _ => "int",
    String s => fmt("text {}", s),
    _ => "other",
}

[
    range(1, 16) | map(fizzbuzz) | join(" "),
    sign(-5),
    sign(0),
    sign(3),
    classify(1),
    classify("hi"),
    classify(true),
    match [1, 2, 3].first() {
        some(1) => "starts with one",
        some(_) => "starts otherwise",
        none => "empty",
    },
]
