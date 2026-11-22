// status: proposed
// feature: tap of a command gives each line as the child writes it, and take ends the read and the child
// implementation: pending
// proposal: doc/proposals/HQL-0064/parts/02-child-processes.hmd
// environment: pure
// expected-type: Int
// expected: 3

from io.process import *

tap(command("yes", ["wren"]), Lines) | take(3) | size
