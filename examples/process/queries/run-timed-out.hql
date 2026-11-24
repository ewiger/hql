// status: valid-now
// feature: a child still running at timeout_ms is terminated, and run gives err with the reason timed-out
// implementation: implemented
// environment: pure
// platform: unix
// expected-type: String
// expected: timed-out

from io.process import *

match command("sleep", ["30"], timeout_ms = 200).run() {
    ok(out) => "ok",
    err(e)  => e.reason,
}
