// status: proposed
// feature: keys lists a vault's keys by name and algorithm, and no listing holds key material
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/05-keyvault.md
// environment: knowledge-v1
// vault: crypto/vault
// expected-type: List<Data>
// expected-json: [{"name": "backup-2026", "algorithm": "X25519", "decrypts": true}, {"name": "release-2026", "algorithm": "Ed25519", "decrypts": false}]

import crypto.keyvault

keyvault.keys("team") | map(k => {name: k.name, algorithm: k.algorithm, decrypts: k.decrypts})
