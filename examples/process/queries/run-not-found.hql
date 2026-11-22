// status: valid-now
// feature: a program that is not in PATH is err under run, with the reason not-found
// implementation: implemented
// environment: pure
// expected-type: String
// expected: not-found

from io.process import *

match command("hql-example-program-never-installed").run() {
    ok(out) => "ok",
    err(e)  => e.reason,
}
