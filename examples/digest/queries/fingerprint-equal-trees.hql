// status: valid-now
// feature: two trees equal under HQL equality have one fingerprint, the SHA-256 of their canonical JSON
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["5b07d3c10bbccec19fd995c80fb3393b8fd39691dbcc412ac55c0d08262ed5af", "5b07d3c10bbccec19fd995c80fb3393b8fd39691dbcc412ac55c0d08262ed5af"]

import crypto.digest

a = Data("{\"b\": 1, \"a\": [1.10, \"x\"]}", Json)
b = Data("{\"a\": [1.1, \"x\"], \"b\": 1.0}", Json)
[fingerprint(a), fingerprint(b)]
