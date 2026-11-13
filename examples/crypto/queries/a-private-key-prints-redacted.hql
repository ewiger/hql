// status: valid-now
// feature: a private key prints as its public key's thumbprint, never as its material
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: List<String>
// expected-json: ["PrivateKey<Ed25519>(kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k)", "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k"]

import crypto

key = read_private_key(file("../keys/rfc8032-test-1.pem"), Pem, Ed25519)
[str(key), key.public().thumbprint()]
