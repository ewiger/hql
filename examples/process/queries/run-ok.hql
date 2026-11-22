// status: valid-now
// feature: run gives ok with what a child that exited 0 wrote
// implementation: implemented
// environment: pure
// expected-type: String
// expected: 2

from io.process import *

match command("wc", ["-l"]).run(input = "wren\nkite\n") {
    ok(out) => out.stdout | trim,
    err(e)  => e.reason,
}
