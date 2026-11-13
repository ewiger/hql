// status: proposed
// feature: an Ed25519 key read from PKCS #8 PEM signs as RFC 8032 test 1 says
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/03-signatures-macs-passwords.md
// environment: pure
// io: sandbox
// expected-type: String
// expected-json: "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"

import crypto
import encoding

key = read_private_key(file("../keys/rfc8032-test-1.pem"), Pem, Ed25519)
"" | sign(key) | encode(Hex)
