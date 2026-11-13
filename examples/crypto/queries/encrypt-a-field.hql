// status: proposed
// feature: encrypt_to writes an age message to an X25519 recipient, and its identity decrypts it
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/04-encryption.md
// environment: pure
// expected-type: Option<String>
// expected-json: "J45.909"

import crypto

pair = generate_key_pair(X25519)
record = {patient: "p-104", diagnosis: "J45.909"}
sealed = record.diagnosis | encrypt_to([pair.public])
sealed | decrypt_with(pair.private) | and_then(to_text)
