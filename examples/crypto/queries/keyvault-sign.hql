// status: proposed
// feature: a key in a vault signs a tree, and the vault's public key verifies the signature
// implementation: pending
// proposal: doc/proposals/HQL-0053/parts/05-keyvault.md
// environment: knowledge-v1
// vault: crypto/vault
// expected-type: Bool
// expected: true

import crypto
import crypto.keyvault

signer = keyvault.key("team", "release-2026", Ed25519)
manifest = {name: "hql", version: "0.4.0"}
signature = keyvault.sign_data(manifest, signer)
manifest.verify_data(signature, keyvault.public_key(signer))
