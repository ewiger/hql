// status: valid-now
// feature: source of a command waits for the child and gives every line of its standard output
// implementation: implemented
// environment: pure
// platform: unix
// expected-type: Seq<String>
// expected-json: ["wren", "kite"]

from io.process import *

source(command("printf", ["wren\nkite\n"]), Lines)
