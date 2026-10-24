// status: valid-now
// implementation: implemented
// environment: pure
// feature: Result<T, E>: a computation that says how it failed
// expected-type: List<String>
// expected-json: ["age 42; cannot read '-3': negative; cannot read 'forty': not a number","1","14","not a number"]

// `ok(v)` and `err(e)` build a Result, `ok(p)` and `err(p)` match one,
// and the combinators mirror Option's. There is no unwrap.
type ParseError { input : String, reason : String }

fn parse_age(text: String) : Result<Int, ParseError> = match text.to_int() {
    some(n) given n >= 0 => ok(n),
    some(_) => err(ParseError(input = text, reason = "negative")),
    none => err(ParseError(input = text, reason = "not a number")),
}

fn describe(r: Result<Int, ParseError>) : String = match r {
    ok(n) => fmt("age {}", n),
    err(e) => fmt("cannot read '{}': {}", e.input, e.reason),
}

inputs = ["42", "-3", "forty"]

[
    inputs | map(parse_age) | map(describe) | join("; "),
    str(inputs.map(parse_age).filter(r => r.is_ok()).size()),
    str(parse_age("7").map(n => n * 2).unwrap_or(0)),
    parse_age("x").map_err(e => e.reason).err().unwrap_or("fine"),
]
