# HQL-0053: Bytes, keys, signatures, encryption and a key vault

**Status**: accepted
**Created**: 2026-11-13
**Source**: [issue 0067](../../issues/0067-std-crypto.md);
[HQL-0041 part 3](../HQL-0041/parts/03-digest-and-encoding.md) (digests,
canonical JSON, and cryptography as a non-goal);
[issue 0029](../../issues/0029-binary-formats-and-codecs.md) (binary formats
and codecs); [HQL-0024 part 7](../HQL-0024/parts/07-formats-and-codecs.md)
(formats and codecs); [HQL-0021](../HQL-0021/README.md) (endpoints, effects
and host grants); [HQL-0046](../HQL-0046/README.md) and
[HQL-0049](../HQL-0049/README.md) (configured endpoints and their secrets)

## Companion notes

- [`parts/01-bytes.md`](parts/01-bytes.md) — the scalar `Bytes`, the
  receiver module `bytes`, its conversions to and from text, how formats
  write it, `Bytes` against "binary", and the order of this work against
  issue 0029.
- [`parts/02-keys.md`](parts/02-keys.md) — module `crypto`: algorithms, the
  key types, reading and writing keys in PEM, DER, JWK, OpenSSH and age
  text, generation, and secret values.
- [`parts/03-signatures-macs-passwords.md`](parts/03-signatures-macs-passwords.md)
  — signing and verifying `String`, `Bytes` and `Data`, keyed MACs, and
  password hashing.
- [`parts/04-encryption.md`](parts/04-encryption.md) — authenticated
  symmetric encryption, encryption to public keys in the age format, and
  encrypted endpoints.
- [`parts/05-keyvault.md`](parts/05-keyvault.md) — module `crypto.keyvault`:
  configured key stores, key names, signing and decrypting inside a store,
  and its backends.

## Abstract

`core` gains the scalar `Bytes`, a finite sequence of octets, and a receiver
module `bytes` gives its operations. Module `crypto` declares four key
algorithms, `Ed25519`, `X25519`, `P256` and `Rsa`, and the key types
`PublicKey<A>`, `PrivateKey<A>`, `KeyPair<A>` and `SecretKey<A>`, read from
and written to PEM, DER, JWK, OpenSSH and age text. It signs and verifies
`String`, `Bytes` and `Data`, the last over the canonical JSON of HQL-0041
part 3; computes and verifies HMAC tags; hashes and verifies passwords with
Argon2id; encrypts with AES-256-GCM and ChaCha20-Poly1305; and encrypts to
X25519 public keys in the age v1 format, as an operation over `Bytes` and as
an endpoint that encrypts what a format writes. `PrivateKey`, `SecretKey` and
`KeyPair` are secret types: no format encodes one, and every printed form of
one is redacted. Module `crypto.keyvault` reaches keys held in a store that
`hql.toml` configures, by vault id and key name; a private key in a store
signs and decrypts inside it and never becomes a value. The `std` extension
implements both modules, and neither is in the prelude.

## Motivation

- A program cannot sign a value it publishes or verify one it receives. A
  release manifest, a webhook payload, an exported dataset and a JWT's
  signing input all need a detached signature that another tool checks.
- A program cannot encrypt a field before it writes it, or a file for a
  recipient, or decrypt either. A pipeline that moves personal data through
  a file or a topic writes it in clear text.
- A webhook's HMAC header and a stored password hash are ordinary inputs to
  a data program, and HQL-0041 leaves both out.
- A key is text in one of five formats, and the work of reading and writing
  them is most of what a user meets. Every one of them is format work over
  bytes, and HQL has no value that holds bytes.
- A private key written into a program's source, a configuration file or a
  log is a leak. The language states where key material may live and
  guarantees what is never printed, rather than leaving it to each program.

## Non-goals

- TLS configuration, X.509 certificates and certificate chains, and
  certificate signing requests.
- OpenPGP: its keys, signatures and messages, and a GPG agent as a key
  vault backend. A key a GPG agent holds for SSH is reached through the
  `ssh_agent` backend, which it serves.
- JOSE and COSE envelopes: JWS, JWE, JWT and their compact serialisations. A
  program builds a JWS signing input from `encoding` and signs it with
  `sign`; an envelope module is a later record.
- RSA encryption, RSA-PSS, ECDSA over curves other than P-256, and
  post-quantum algorithms.
- Key derivation functions other than Argon2id for passwords and age's
  scrypt for passphrases, and a key agreement operation of its own: `X25519`
  keys are used through age.
- A `Bytes` literal. `Bytes` values are built by constructors and
  conversions (part 1).
- Binary formats and codecs from outside the binary: issue 0029.

## Specification

### 1. Ownership

| Module | File | Holds |
| --- | --- | --- |
| `core` | `lib/std/core/mod.hql` | `Bytes`, `Bytes(text)` and its `Eq`, `Ord` and `Add` |
| `bytes` | `lib/std/bytes/mod.hql` | `at`, `slice`, `size`, `to_text` |
| `encoding` | `lib/std/encoding/mod.hql` | the `Bytes` forms of `encode`, `to_bytes` and `Bytes(text, as)` |
| `crypto.digest` | `lib/std/crypto/digest/mod.hql` | the `Bytes` form of `digest`, and `canonical` |
| `crypto` | `lib/std/crypto/mod.hql` | algorithms, key types, key formats, signatures, MACs, passwords, encryption, encrypted endpoints |
| `crypto.keyvault` | `lib/std/crypto/keyvault/mod.hql` | `KeyRef`, `KeyInfo`, `key`, `keys`, and the operations over a key in a store |

- `Bytes` MUST be declared in `core` as `{Scalar, Orderable}`, beside
  `String`, so that a `Data` tree holds one and a format encodes one.
- `bytes` is a receiver module, `module bytes for Bytes`, and MUST be in the
  prelude, as `string` is.
- `crypto` imports `core`, `encoding`, `crypto.digest` and `io`;
  `crypto.keyvault` imports `core`, `crypto` and `io`. Neither MUST be added
  to the prelude. A program writes `import crypto`, and `import
  crypto.keyvault`, which it reaches as `keyvault`.
- The modules MUST be listed in `library::MODULES` after the modules they
  import. The `std` extension's descriptor MUST provide each.

### 2. Effects and the entropy source

Every operation that draws random bytes reads the host's cryptographically
secure generator.

| Operation | Effect | Reads |
| --- | --- | --- |
| `generate_key_pair`, `generate_secret_key` | `effectful` | the host generator |
| `encrypt`, `encrypt_to`, `encrypt_with_passphrase`, `hash_password` | `effectful` | the host generator |
| `encrypted_to`, `decrypted_with` | `fn`, an endpoint constructor | nothing; reading and writing the endpoint are the effects |
| `read_public_key`, `read_private_key` | `effectful` | a file |
| every operation of `crypto.keyvault` but `key` | `effectful` | a configured key vault |
| key parsing and writing, `sign`, `verify`, `mac`, `verify_mac`, `verify_password`, `decrypt`, `decrypt_with`, `decrypt_with_passphrase` | `fn` | nothing |

- The host generator MUST be the operating system's, read through
  `getrandom`. It MUST NOT be the evaluation seed of HQL-0041's `random`,
  and `--seed` MUST NOT change what it gives. A nonce or a key that a seed
  reproduces is a nonce or a key an attacker reproduces.
- HQL-0041's determinism rule, that fixed `--now`, `--zone`, `--seed`,
  arguments and inputs give the same value, MUST hold for every program
  that does not call an operation of the first two rows. A program that
  encrypts produces different ciphertext on every run; its decryption is
  deterministic.
- `sign` is deterministic for every algorithm: Ed25519 by its definition,
  ECDSA P-256 by RFC 6979 nonces, RSA by PKCS #1 v1.5. A signature is a
  function of the key and the message.

### 3. Secret values

A secret type is one whose values hold private key material.

- `crypto` declares `abstract type Secret`. `PrivateKey<A>`, `SecretKey<A>`
  and `KeyPair<A>` MUST be `Secret`, and no other standard type is.
- A `Secret` is neither a `Scalar` nor a `Data` alternative, so no `Data`
  tree holds one.
- The checker MUST refuse every program in which a type containing a
  `Secret` reaches an encoder: the value of `format`, `target` or `sink`,
  the `data` of a log call, and a field of a declared record so written.
  The diagnostic names the type and `export_private` (part 2).
- `Secret` types implement no `Eq`; `==` over two of them is a check error.
  `verify_mac` and `verify_password` compare in constant time.
- The text of a secret value, wherever HQL produces text for a value, MUST be
  its redacted form:

  ```text
  PrivateKey<Ed25519>(kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k)
  KeyPair<Ed25519>(kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k)
  SecretKey<ChaCha20Poly1305>(redacted)
  ```

  The text inside the parentheses of a private key or key pair is the RFC
  7638 thumbprint of its public key, which identifies the key and discloses
  nothing of it. This applies to `str`, `fmt`, the REPL, `--to`, `:type`,
  and every field of a diagnostic: its message, `found`, `element`,
  `bindings` and `trace`.
- A diagnostic raised while reading key text MUST name the format, the line
  and what was expected there, and MUST NOT quote the text.

The mechanism is in the representation. A secret value is held in Rust as a
`SecretBox` of the `secrecy` crate over a `zeroize`-on-drop buffer: its
`Debug` prints `[REDACTED]`, it implements neither `Display` nor
`serde::Serialize`, and the memory is overwritten when the value is dropped.
The runtime's value type renders a secret through one function, which writes
the redacted form above, so a diagnostic, a log line or a `tracing` event
that formats a value cannot reach the bytes. The only paths to the bytes are
the `crypto` operations that use them and `export_private`.

### 4. Where key material lives

Private key material enters an evaluation from one of four places, and a
program's source is not one of them.

| Place | How a program reaches it |
| --- | --- |
| a file | `read_private_key(file(p), Pem, Ed25519)` |
| an environment variable | `env.var("NAME")` of [HQL-0041 part 5](../HQL-0041/parts/05-env.md), then a key constructor |
| a key vault | `keyvault.key("vault", "name", Ed25519)`, which never yields the private key |
| the evaluation itself | `generate_key_pair`, `generate_secret_key` |

- The checker MUST refuse a string literal that holds the beginning of a
  private key in any format part 2 reads: `-----BEGIN` followed by a label
  ending in `PRIVATE KEY-----`, `AGE-SECRET-KEY-1`, or a JWK object with a
  `"d"` member. The diagnostic names the four places above.
- `hql.toml` holds no key material. A `[keyvault.<id>]` table with a
  secret written into it MUST refuse the vault, naming the `*_env` key that
  replaces it, as `dsn` does in HQL-0046; [part 5](parts/05-keyvault.md)
  lists the keys.
- `read_private_key` MUST warn, on a Unix host, when the file is readable
  by its group or by others, naming the path and the mode, as OpenSSH does.
- `hql render` MUST grant no key vault, and MUST refuse at check time the
  operations that read the host generator: a rendered document is shared,
  and its output is a function of its inputs.

### 5. Order of work

`Bytes` is the dependency every other part has, and issue 0029's binary
formats need it as much as `crypto` does.

1. HQL-0041 part 3, `crypto.digest` and `encoding` over `String`. `crypto` writes a
   key, a signature and a ciphertext as text through `encoding`.
2. Part 1 of this proposal, `Bytes`. Issue 0029 adopts it as the in-memory
   form of a binary format's encoded value, and is ordered after this step.
3. Parts 2 and 3: keys, signatures, MACs and passwords.
4. Part 4: symmetric and age encryption over `Bytes`, then encrypted
   endpoints. An encrypted endpoint transforms the bytes a format writes,
   so it needs no binary format kind, and does not wait for issue 0029.
5. Part 5: `crypto.keyvault`, the `file` and `ssh_agent` backends first,
   then `keychain`, then `aws_kms`.

Issue 0029 and steps 3 to 5 are independent of each other once step 2 is
done.

## Backwards Compatibility

- Additive. `Bytes` is a new name in `core`, and a program that declares a
  type named `Bytes` renames its own. `bytes` becomes a module in the
  prelude; a program binding `bytes` shadows it.
- `encoding.decode` keeps its signature and its `none` for decoded bytes
  that are not UTF-8; `to_bytes` is the form that keeps them.
- A JSON, YAML or CSV decoder never produces a `Bytes`: a declared field of
  type `Bytes` reads its base64 text, as a declared `Date` reads ISO 8601
  text. No existing program's decoded values change.
- `crypto` and `keyvault` become local names in a unit that imports them.
- A `[keyvault.<id>]` table in `hql.toml` was ignored and is now read and
  checked.

## Security Considerations

- **No secret is printed.** Section "Secret values" states the rule and its
  mechanism. `export_private` is the one operation that turns private key
  material into a `String`, and its name says so; once a `String`, the text
  is the program's to protect.
- **Nonces are random and never reused by construction.** `encrypt` draws a
  fresh 96-bit nonce per call from the host generator. AES-256-GCM's bound
  of 2^32 messages per key under random nonces is documented on `encrypt`;
  `ChaCha20Poly1305` has the same nonce size and bound.
- **Verification is strict.** Ed25519 verification is RFC 8032's with the
  strict checks: it rejects non-canonical signatures and small-order public
  keys. ECDSA P-256 `sign` writes the low-`s` form, and `verify` accepts
  either form, as FIPS 186-5 does. Every algorithm gives `false` for a
  malformed signature, never an error.
- **Weak keys are refused.** An RSA key under 2048 bits is refused when it
  is read or generated. A P-256 public key not on the curve is refused when
  it is read.
- **Passwords cost what OWASP recommends.** `hash_password` defaults to
  Argon2id with 19 MiB of memory, 2 iterations and 1 lane, and accepts
  higher costs.
- **Timing.** MAC and password verification compare in constant time. RSA
  operations use the `rsa` crate, whose timing advisory RUSTSEC-2023-0071
  concerns decryption; this proposal uses RSA to sign and verify only.
- **Resource bounds.** `Bytes` values are bounded by the run-context limits of
  [HQL-0012](../HQL-0012/README.md) as any held value is. Age decryption
  reads its input in 64 KiB chunks and holds one at a time; `hash_password`'s
  memory parameter MUST NOT exceed 1 GiB.
- **Key vaults are visible.** Every use of a vault is `effectful` and names
  a configured id, so `hql check` reports every vault a program reaches
  before anything runs, and opens none of them.

## Deployment / Activation

1. Implement HQL-0041 part 3 if it has not landed.
2. Add `Bytes` and the module `bytes`, and the `Bytes` forms of `encoding`
   and `crypto.digest`, with the corpus cases of part 1.
3. Add `crypto` with keys, signatures, MACs and passwords; the secret-type
   check and the source-literal check land with the first key type.
4. Add symmetric and age encryption, then the encrypted endpoints.
5. Add `crypto.keyvault` and `hql keyvault`, one backend at a time.
6. Promote each corpus case under `examples/crypto/` as it checks.

## Reference Implementation

- `lib/std/core/mod.hql`: `Bytes`, `Bytes(text)` and its trait
  implementations.
- `lib/std/bytes/mod.hql`, `lib/std/crypto/mod.hql` and
  `lib/std/crypto/keyvault/mod.hql`: the declarations of parts 1 to 5,
  written as external declarations; `lib/std/encoding/mod.hql` and
  `lib/std/crypto/digest/mod.hql` gain the `Bytes` forms.
- `src/extensions/std/`: one file each for `bytes`, `crypto` and
  `keyvault`; `src/crypto/` for the key formats, the age endpoint layer and
  the backends of the key vault, each backend in its own file.
- `src/checker.rs`: the refusal of a secret type at an encoder and of a
  private key literal in source.
- `src/endpoints.rs`: `[keyvault.<id>]` tables, read and checked as
  `[endpoint.<id>]` tables are.
- `src/cli.rs`: `hql keyvault`.
- Crates, each added with `cargo add` per [the stack](../../stack.md), all
  pure Rust, none needing a C toolchain:

  | Need | Crate |
  | --- | --- |
  | Ed25519 | `ed25519-dalek` (`pkcs8`, `pem`, `zeroize`) |
  | X25519 | `x25519-dalek` (`static_secrets`, `zeroize`) |
  | ECDSA P-256 | `p256` (`ecdsa`, `pkcs8`, `pem`, `jwk`) |
  | RSA signatures | `rsa` (`sha2`) |
  | AES-256-GCM, ChaCha20-Poly1305 | `aes-gcm`, `chacha20poly1305` |
  | HMAC | `hmac` over the `sha2` the stack has |
  | Argon2id | `argon2` |
  | age v1 | `age` (default features off; `armor`) |
  | OpenSSH keys and the agent's wire encoding | `ssh-key` (`ed25519`, `p256`, `rsa`, `encryption`) |
  | the host generator | `getrandom` |
  | secret buffers | `secrecy`, `zeroize` |
  | OS keychains | `keyring` (`apple-native`, `windows-native`, `async-secret-service`, `crypto-rust`, `tokio`), on a one-thread `tokio` runtime as Kafka and NATS are |

  JWK is read and written with `serde_json`, which the crate has. The
  SSH agent protocol (list identities, sign) is hand-written over a Unix
  socket in `src/crypto/ssh_agent.rs`, as the Thrift calls of
  `src/spark.rs` are; AWS KMS's two calls and their SigV4 signing are
  hand-written over `ureq` in `src/crypto/aws_kms.rs`.
- `doc/stack.md` gains rows for cryptography and key vaults.

## Test Plan

Example cases under [`examples/crypto/`](../../../examples/crypto/README.md),
written at `status: proposed` with `proposal:` naming this record, and
promoted as each checks:

| Case | Part | Pins |
| --- | --- | --- |
| [bytes and text](../../../examples/crypto/queries/bytes-and-text.hql) | 1 | `Bytes("hello")` in hex and base64, and back |
| [RFC 8032 signature](../../../examples/crypto/queries/sign-rfc8032-test-1.hql) | 2, 3 | Ed25519 test 1 from a PEM file |
| [verify a release](../../../examples/crypto/queries/verify-a-release.hql) | 3 | `verify_data` over canonical JSON: `[true, false]` |
| [a key prints redacted](../../../examples/crypto/queries/a-private-key-prints-redacted.hql) | 2 | the redacted text and the RFC 8037 thumbprint |
| [HMAC](../../../examples/crypto/queries/hmac-rfc4231-case-2.hql) | 3 | RFC 4231 test case 2 |
| [password hashes](../../../examples/crypto/queries/password-hash.hql) | 3 | Argon2id PHC text and both verdicts |
| [AEAD round trip](../../../examples/crypto/queries/aead-round-trip.hql) | 4 | layout, round trip, and a truncated ciphertext |
| [a field for a recipient](../../../examples/crypto/queries/encrypt-a-field.hql) | 4 | age to a generated X25519 key |
| [an encrypted file](../../../examples/crypto/queries/encrypted-file.hql) | 4 | `target` through `encrypted_to`, read back |
| [sign in a vault](../../../examples/crypto/queries/keyvault-sign.hql) | 5 | a signature from a store, verified by its public key |
| [decrypt in a vault](../../../examples/crypto/queries/keyvault-decrypt.hql) | 5 | age to a stored identity |
| [list a vault](../../../examples/crypto/queries/keyvault-keys.hql) | 5 | names and algorithms |

Each part names its unit tests. Across all of them:

- Every algorithm against its published vectors: RFC 8032 for Ed25519, RFC
  7748 for X25519, RFC 6979 A.2.5 for ECDSA P-256 with SHA-256, the NIST
  CAVP vectors for AES-GCM, RFC 8439 for ChaCha20-Poly1305, RFC 4231 for
  HMAC, RFC 9106 for Argon2id, and the age project's test vectors.
- Interoperability files written by other tools and committed as fixtures:
  keys from `ssh-keygen` and `openssl genpkey` in each format, a signature
  from `openssl pkeyutl`, and a file from the `age` command line.
- Redaction: a program that formats, logs, interpolates and fails on a
  private key, run with `--log-level trace` and
  `--diagnostics-format json`; the test asserts that neither stream holds
  the key's hex, base64 or base64url text.
- Refusals, each a `status: invalid` corpus case once implemented: a
  private key PEM literal in source; `target` of a `KeyPair`; `==` over two
  `SecretKey`s; `keyvault.decrypt_with` on an `ssh_agent` vault; a vault id
  `hql.toml` does not configure; a `passphrase` key in `[keyvault.<id>]`;
  and `encrypt` under `hql render`.
- Determinism: a program that signs and verifies gives the same value twice;
  `encrypt` under one `--seed` gives two different ciphertexts.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Changelog

- 2026-11-13: drafted
- 2026-11-13: digests are module `crypto.digest`; part 1 states `Bytes`
  as the value and "binary" as a property of a format
- 2026-11-13: accepted
- 2026-11-13: part 1, `Bytes`, implemented; `Index<Int>` and `Slice` are
  the `bytes` module's `at` and `slice`, and `b[i]` is `none` outside
