// status: valid-now
// feature: target writes one document to the child's standard input, closes it, and waits
// implementation: implemented
// environment: pure
// expected-type: Step
// expected: ()

from io.process import *

["wren", "kite"] | target(command("sh", ["-c", "cat > /dev/null"]), Lines)
