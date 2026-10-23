// status: valid-now
// feature: `retry` reinvokes a fallible operation while it fails, up to `attempts` invocations; exhausted, the last `err` stands, and an error `when` rejects is not retried
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["[ok(10)] after 3", "[err(\"down\")] after 3", "[err(\"fatal\")] after 1"]

// Each operation writes every invocation to a channel of its own, so the
// number of invocations can be read back. `flaky` succeeds from its third
// invocation on, as a remote call may; `down` never does; `refused` fails in
// a way no later invocation changes.
flaky_calls = Channel<Int>("retry-flaky")
down_calls = Channel<Int>("retry-down")
refused_calls = Channel<Int>("retry-refused")

fn flaky(x: Int) : Result<Int, String> = match [x] | sink(flaky_calls) {
    _ => ok(x * 10) given (source(flaky_calls) | size()) >= 3 otherwise err("busy")
}

fn down(x: Int) : Result<Int, String> = match [x] | sink(down_calls) {
    _ => err("down")
}

fn refused(x: Int) : Result<Int, String> = match [x] | sink(refused_calls) {
    _ => err("fatal")
}

recovered = [1] | retry(flaky, attempts = 5)
exhausted = [1] | retry(down, attempts = 3)
not_retried = [1] | retry(refused, attempts = 5, when = e => e == "busy")
[
    fmt("{} after {}", recovered, source(flaky_calls) | size()),
    fmt("{} after {}", exhausted, source(down_calls) | size()),
    fmt("{} after {}", not_retried, source(refused_calls) | size())
]
