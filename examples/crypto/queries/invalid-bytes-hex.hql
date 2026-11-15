// status: invalid
// feature: Bytes(text, Hex) refuses text that is not hex, naming the first invalid position
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Bytes

from encoding import *

Bytes("6g", Hex)
