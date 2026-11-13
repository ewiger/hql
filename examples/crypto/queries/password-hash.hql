// status: proposed
// feature: hash_password writes an Argon2id PHC string, and verify_password accepts only the password it was made from
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/03-signatures-macs-passwords.md
// environment: pure
// expected-type: List<Bool>
// expected: [true, false, true]

import crypto

stored = "correct horse battery staple" | hash_password
["correct horse battery staple".verify_password(stored),
 "Tr0ub4dor&3".verify_password(stored),
 stored.starts_with("$argon2id$v=19$m=19456,t=2,p=1$")]
