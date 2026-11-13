// status: valid-now
// feature: source through decrypted_with reads the documents of an age file, its format implied by the path without .age
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Seq<Option<Data>>
// expected-json: [1, 2]

import crypto

identity = read_private_key(file("../keys/x25519-identity.txt"), Age, X25519)
source(decrypted_with(file("../orders.jsonl.age"), identity)) | map(o => o.id)
