<!-- hql reference: std/crypto -->
# `std/crypto`

```hql
module crypto
```

Keys, signatures, MACs, password hashes and encryption. An algorithm and
a key format are each named by a type, as a format is:
`PublicKey(text, Pem, Ed25519)`. A key's type carries its algorithm, so a
key of one algorithm used where another is named is a check error.
`PrivateKey`, `KeyPair` and `SecretKey` are `Secret`: no format writes
one, `==` does not compare them, and each prints redacted.

- [`KeyAlgorithm`](types/KeyAlgorithm.md) — An asymmetric key algorithm.
- [`Signing`](types/Signing.md) — An asymmetric key algorithm.
- [`Agreement`](types/Agreement.md) — An asymmetric key algorithm.
- [`Ed25519`](types/Ed25519.md) — An asymmetric key algorithm.
- [`P256`](types/P256.md) — An asymmetric key algorithm.
- [`Rsa`](types/Rsa.md) — An asymmetric key algorithm.
- [`X25519`](types/X25519.md) — An asymmetric key algorithm.
- [`SecretAlgorithm`](types/SecretAlgorithm.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`Cipher`](types/Cipher.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`Mac`](types/Mac.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`Aes256Gcm`](types/Aes256Gcm.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`ChaCha20Poly1305`](types/ChaCha20Poly1305.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`HmacSha256`](types/HmacSha256.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`HmacSha512`](types/HmacSha512.md) — A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
- [`Secret`](types/Secret.md) — The values that hold private key material: no format encodes one, `==` does not compare them, and each prints redacted.
- [`PublicKey`](types/PublicKey.md) — A public key of algorithm `A`.
- [`PrivateKey`](types/PrivateKey.md) — A private key of algorithm `A`.
- [`KeyPair`](types/KeyPair.md) — A private key and its public key.
- [`SecretKey`](types/SecretKey.md) — A symmetric key of algorithm `A`.
- [`KeyFormat`](types/KeyFormat.md) — How one key is text.
- [`Pem`](types/Pem.md) — How one key is text.
- [`Jwk`](types/Jwk.md) — How one key is text.
- [`OpenSsh`](types/OpenSsh.md) — How one key is text.
- [`Age`](types/Age.md) — How one key is text.
- [`to_public_key`](to_public_key.md) — The public key `self` holds in `format`, or `none`.
- [`to_private_key`](to_private_key.md) — The private key `self` holds in `format`, or `none`.
- [`read_public_key`](read_public_key.md) — The public key a file holds in `format`.
- [`read_private_key`](read_private_key.md) — The private key a file holds in `format`.
- [`to_text`](to_text.md) — `self` written in `format`: `key.to_text(OpenSsh)` is an `ssh-ed25519 AAAA…` line.
- [`to_der`](to_der.md) — `self` as SPKI DER.
- [`export_private`](export_private.md) — The private key material of `self` written in `format`, encrypted under `passphrase` where one is given: PKCS #8 with scrypt and AES-256-CBC for `Pem`, bcrypt-pbkdf and AES-256-CTR for `OpenSsh`.
- [`public`](public.md) — The public key of a private key.
- [`thumbprint`](thumbprint.md) — The RFC 7638 JWK thumbprint of `self`: base64url, unpadded, of the SHA-256 of its canonical JWK.
- [`generate_key_pair`](generate_key_pair.md) — A new key pair from the host's cryptographically secure generator.
- [`generate_secret_key`](generate_secret_key.md) — A new symmetric key from the host generator: 32 octets for a cipher and `HmacSha256`, 64 for `HmacSha512`.
- [`sign`](sign.md) — The signature of the octets of `self` by `key`.
- [`sign_data`](sign_data.md) — The signature of `canonical(self)`, the RFC 8785 text of the tree, so two trees equal under HQL equality have one signature.
- [`verify`](verify.md) — Whether `signature` is `key`'s signature of `self`: `false`, never an error, for a signature of another key, of the wrong length or malformed.
- [`verify_data`](verify_data.md) — Whether `signature` is `key`'s signature of `canonical(self)`.
- [`p256_signature_from_der`](p256_signature_from_der.md) — The `r || s` form of an ASN.1 DER ECDSA P-256 signature, as OpenSSL writes one, or `none` where `self` is not one.
- [`mac`](mac.md) — The HMAC tag of `self` under `key`: 32 octets for `HmacSha256`, 64 for `HmacSha512`.
- [`verify_mac`](verify_mac.md) — Whether `tag` is the MAC tag of `self` under `key`, compared in time independent of where it first differs.
- [`hash_password`](hash_password.md) — The Argon2id hash of `self` under a new 16-octet salt from the host generator, as a PHC string: `$argon2id$v=19$m=19456,t=2,p=1$…$…`.
- [`verify_password`](verify_password.md) — Whether `self` is the password `hash` was made from, under the hash's own parameters.
- [`encrypt`](encrypt.md) — The authenticated encryption of `self` under `key`, with `aad` bound to it and not encrypted: a new 12-octet nonce from the host generator, then the ciphertext, then the 16-octet tag.
- [`decrypt`](decrypt.md) — The plaintext of `self` under `key` and `aad`, or `none` where `self` is not an encryption of anything under them: shorter than 28 octets, a changed octet anywhere, a wrong key or a wrong `aad`.
- [`encrypt_to`](encrypt_to.md) — The age encryption of `self` to every key of `recipients`: any one of their private keys decrypts it.
- [`decrypt_with`](decrypt_with.md) — The plaintext of the age message `self` for `identity`, or `none` where it is not a message to that key or has been changed.
- [`encrypt_with_passphrase`](encrypt_with_passphrase.md) — The age encryption of `self` under a passphrase, by age's scrypt recipient with work factor 18.
- [`decrypt_with_passphrase`](decrypt_with_passphrase.md) — The plaintext of the age message `self` under `passphrase`, or `none`.
- [`armor`](armor.md) — The ASCII armor of an age message: `-----BEGIN AGE ENCRYPTED FILE-----`.
- [`unarmor`](unarmor.md) — The age message an armored text holds, or `none`.
- [`EncryptedEnd`](types/EncryptedEnd.md) — An endpoint whose writes are age-encrypted to `recipients` and stored in `endpoint`.
- [`DecryptedEnd`](types/DecryptedEnd.md) — An endpoint whose contents are those of a replayable `endpoint`, age-decrypted.
- [`DecryptedLive`](types/DecryptedLive.md) — An endpoint whose contents are those of a live `endpoint`, age-decrypted.
- [`encrypted_to`](encrypted_to.md) — `endpoint`, its writes age-encrypted to `recipients`.
- [`decrypted_with`](decrypted_with.md) — `endpoint`, its contents age-decrypted with `identity`.
<!-- hql reference: end -->
