// status: valid-now
// feature: digest gives the SHA-256 of a text's UTF-8 octets as lowercase hex
// implementation: implemented
// environment: pure
// expected-type: String
// expected: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

import crypto.digest

"abc".digest(Sha256)
