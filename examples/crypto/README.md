# Crypto: bytes, keys, signatures, encryption and key vaults

The cases follow [HQL-0053](../../doc/proposals/HQL-0053/README.md). A case
at `status: proposed` is written ahead of its part's implementation, and the
harness asserts that it does not check yet.

`Bytes` is a finite sequence of octets. Module `crypto` reads and writes
keys, signs and verifies `String`, `Bytes` and `Data`, computes HMAC tags,
hashes passwords with Argon2id, and encrypts under a shared key or to an
X25519 recipient in the age format. A private key is a secret value: it
prints as its public key's thumbprint, and no format writes it. Module
`crypto.keyvault` reaches keys a [vault](vault/hql.toml) configures by id
and name; a stored private key signs and decrypts through the vault and is
never a value of the program.

The key in [`keys/`](keys/) is RFC 8032's published Ed25519 test key 1,
and its signatures are the RFC's, so the expected values are the standard's.
A private key file is read with mode `0600`; the harness sets that mode on
its sandbox copy of `keys/`. [`orders.jsonl.age`](orders.jsonl.age) was
written by `target` through `encrypted_to` to the X25519 recipient in
`keys/`, in an evaluation of its own: one evaluation does not read a file
it has replaced. The [releases](releases.jsonl) are one manifest
signed by that key and the same manifest with its version changed.

| Case | What it shows | Result |
| --- | --- | --- |
| [bytes and text](queries/bytes-and-text.hql) | `Bytes("hello")` in hex and base64, and back | its size, two texts, `true` |
| [index and slice](queries/bytes-index-and-slice.hql) | `b[0]`, `b[-1]`, `b[0:2]` and `+` over `Bytes("hé")` | octets as `Int`s, `Bytes` |
| [order](queries/bytes-order.hql) | four values sorted | `0x00 < 0x0000 < 0x01 < 0xff` |
| [invalid hex](queries/invalid-bytes-hex.hql) | `Bytes("6g", Hex)` | an evaluation error at position 1 |
| [RFC 8032 signature](queries/sign-rfc8032-test-1.hql) | a PKCS #8 PEM key signs the empty message | RFC 8032 test 1's signature |
| [verify a release](queries/verify-a-release.hql) | `verify_data` over canonical JSON; a changed member fails | `[true, false]` |
| [a key prints redacted](queries/a-private-key-prints-redacted.hql) | `str` of a private key, and its thumbprint | RFC 8037's thumbprint, twice |
| [HMAC](queries/hmac-rfc4231-case-2.hql) | `mac` under `HmacSha256` | RFC 4231 test case 2's tag |
| [password hashes](queries/password-hash.hql) | `hash_password` and `verify_password` | `[true, false, true]` |
| [AEAD round trip](queries/aead-round-trip.hql) | nonce, ciphertext and tag; a truncated ciphertext | `[true, true, true]` |
| [a field for a recipient](queries/encrypt-a-field.hql) | `encrypt_to` an X25519 key, `decrypt_with` its identity | the field's text |
| [an encrypted file](queries/encrypted-file.hql) | `source` through `decrypted_with` of [an age file](orders.jsonl.age), JSON Lines by its path | `[1, 2]` |
| [sign in a vault](queries/keyvault-sign.hql) | a stored key signs; its public key verifies | `true` |
| [decrypt in a vault](queries/keyvault-decrypt.hql) | an age message to a stored identity | the message's text |
| [list a vault](queries/keyvault-keys.hql) | names, algorithms and what each key does | two keys |

The vault cases read `team`, a `file` vault whose store `keys.age` is
written by `hql keyvault generate` when the cases are promoted, and whose
passphrase the harness sets in `HQL_TEAM_VAULT_PASSPHRASE`.

From the repository root, once implemented:

```sh
cargo build --locked
target/debug/hql run examples/crypto/queries/hmac-rfc4231-case-2.hql
```
