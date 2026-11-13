// status: valid-now
// feature: each algorithm of crypto.digest against its published vector for "abc"
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["900150983cd24fb0d6963f7d28e17f72", "a9993e364706816aba3e25717850c26c9cd0d89d", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f", "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"]

import crypto.digest

s = "abc"
[s.digest(Md5), s.digest(Sha1), s.digest(Sha256), s.digest(Sha512), s.digest(Blake3)]
