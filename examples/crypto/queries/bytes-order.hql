// status: valid-now
// feature: Bytes order octet by octet as unsigned numbers, a shorter prefix first
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["0x00", "0x0000", "0x01", "0xff"]

import encoding

[Bytes("ff", Hex), Bytes("0000", Hex), Bytes("01", Hex), Bytes("00", Hex)]
| sort
| map(b => str(b))
