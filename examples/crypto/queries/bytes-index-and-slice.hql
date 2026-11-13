// status: valid-now
// feature: a Bytes value's positions are its octets as Ints, a slice is Bytes, and + concatenates
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["0x68c3a9", "104", "169", "none", "0x68c3", "0x68c3a921"]

b = Bytes("hé")
[str(b), str(b[0]), str(b[-1]), str(b.at(3)), str(b[0:2]), str(b + Bytes("!"))]
