// status: proposed
// feature: run gives ok with what a child that exited 0 wrote
// implementation: pending
// proposal: doc/proposals/HQL-0064/parts/02-child-processes.hmd
// environment: pure
// expected-type: String
// expected: 2

from io.process import *

match command("wc", ["-l"]).run(input = "wren\nkite\n") {
    ok(out) => out.stdout | trim,
    err(e)  => e.reason,
}
