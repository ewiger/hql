// status: valid-now
// feature: tap of a command gives each line as the child writes it, and take ends the read and the child
// implementation: implemented
// environment: pure
// platform: unix
// expected-type: Int
// expected: 3

from io.process import *

tap(command("yes", ["wren"]), Lines) | take(3) | size
