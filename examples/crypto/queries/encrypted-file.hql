// status: proposed
// feature: target through encrypted_to writes an age file, and source through decrypted_with reads its documents back
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/04-encryption.md
// environment: pure
// io: sandbox
// expected-type: Seq<Data>
// expected-json: [1, 2]

import crypto

pair = generate_key_pair(X25519)
orders = [{id: 1, total: 12.50}, {id: 2, total: 7.25}]
orders | target(encrypted_to(file("orders.jsonl.age"), [pair.public]))
source(decrypted_with(file("orders.jsonl.age"), pair.private)) | map(o => o.id)
