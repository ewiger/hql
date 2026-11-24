// status: valid-now
// feature: a child that exits non-zero is err under run, with the reason exited and its status
// implementation: implemented
// environment: pure
// platform: unix
// expected-type: String
// expected: exited 3

from io.process import *

match command("sh", ["-c", "exit 3"]).run() {
    ok(out) => "ok",
    err(e)  => fmt("{} {}", e.reason, e.status | unwrap_or(0)),
}
