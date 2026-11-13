// status: valid-now
// feature: encrypt writes nonce, ciphertext and tag; decrypt gives the text back, and none for a truncated ciphertext
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected: [true, true, true]

import crypto

key = generate_secret_key(ChaCha20Poly1305)
sealed = "meet at the north gate" | encrypt(key)
[(sealed.decrypt(key) | and_then(to_text)) == some("meet at the north gate"),
 sealed[:-1].decrypt(key) | is_none,
 sealed.size() == 12 + 22 + 16]
