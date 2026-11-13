// status: proposed
// feature: verify_data checks a signature over the canonical JSON of a tree, and a changed member fails it
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/03-signatures-macs-passwords.md
// environment: pure
// io: sandbox
// expected-type: Seq<Bool>
// expected: [true, false]

import crypto
import encoding

type Release {
    manifest  : Data
    signature : String
}

public = read_public_key(file("../keys/rfc8032-test-1.pub.pem"), Pem, Ed25519)
releases : Seq<Release> = source(file("../releases.jsonl"))
releases
| map(r => r.manifest.verify_data(r.signature.to_bytes(Base64) | unwrap_or(Bytes("")), public))
