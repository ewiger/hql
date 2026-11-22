// status: proposed
// feature: target writes one document to the child's standard input, closes it, and waits
// implementation: pending
// proposal: doc/proposals/HQL-0064/parts/02-child-processes.hmd
// environment: pure
// expected-type: Step
// expected: ()

from io.process import *

["wren", "kite"] | target(command("sh", ["-c", "cat > /dev/null"]), Lines)
