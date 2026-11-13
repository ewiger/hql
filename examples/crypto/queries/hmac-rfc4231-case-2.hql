// status: proposed
// feature: mac gives the HMAC-SHA-256 tag of RFC 4231 test case 2
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/03-signatures-macs-passwords.md
// environment: pure
// expected-type: String
// expected-json: "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"

import crypto
import encoding

key = SecretKey(Bytes("Jefe"), HmacSha256)
"what do ya want for nothing?" | mac(key) | encode(Hex)
