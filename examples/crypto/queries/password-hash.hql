// status: valid-now
// feature: hash_password writes an Argon2id PHC string, and verify_password accepts only the password it was made from
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected: [true, false, true]

from crypto import hash_password, verify_password

stored = "correct horse battery staple" | hash_password
["correct horse battery staple".verify_password(stored),
 "Tr0ub4dor&3".verify_password(stored),
 stored.starts_with("$argon2id$v=19$m=19456,t=2,p=1$")]
