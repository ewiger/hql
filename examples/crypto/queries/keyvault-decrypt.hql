// status: proposed
// feature: an age message to a stored identity is decrypted inside the vault
// implementation: pending
// proposal: doc/proposals/HQL-0054/parts/01-keyvault.md
// environment: knowledge-v1
// vault: crypto/vault
// expected-type: Option<String>
// expected-json: "quarterly numbers"

import crypto
import crypto.keyvault

backup = keyvault.key("team", "backup-2026", X25519)
sealed = "quarterly numbers" | encrypt_to([keyvault.public_key(backup)])
keyvault.decrypt_with(sealed, backup) | and_then(to_text)
