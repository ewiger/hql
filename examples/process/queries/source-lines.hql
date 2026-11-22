// status: proposed
// feature: source of a command waits for the child and gives every line of its standard output
// implementation: pending
// proposal: doc/proposals/HQL-0064/parts/02-child-processes.hmd
// environment: pure
// expected-type: Seq<String>
// expected-json: ["wren", "kite"]

from io.process import *

source(command("printf", ["wren\nkite\n"]), Lines)
