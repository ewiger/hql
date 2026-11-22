// status: proposed
// feature: a child that exits non-zero is err under run, with the reason exited and its status
// implementation: pending
// proposal: doc/proposals/HQL-0064/parts/02-child-processes.hmd
// environment: pure
// expected-type: String
// expected: exited 3

from io.process import *

match command("sh", ["-c", "exit 3"]).run() {
    ok(out) => "ok",
    err(e)  => fmt("{} {}", e.reason, e.status | unwrap_or(0)),
}
