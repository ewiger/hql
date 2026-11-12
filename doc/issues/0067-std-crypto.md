# 0067 — `crypto`: keys, signatures, encryption and a key vault

Status: backlog
Related: [0029](0029-binary-formats-and-codecs.md),
[HQL-0041 part 3](../proposals/HQL-0041/parts/03-hash-and-encoding.md),
[HQL-0024 part 7](../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[HQL-0021](../proposals/HQL-0021/README.md)

A program cannot sign a value or verify a signature, encrypt a field or a
file for a recipient, or decrypt one it was sent. HQL-0041 gives digests and
text encodings and leaves cryptography out as a non-goal; this issue picks it
up. It ends in a proposal, not code.

Most of the work is format work. A key is read from and written to PEM, DER,
JWK or OpenSSH text; a ciphertext or a signature is bytes, and encrypted
output is a format a `target` writes and a `source` reads. So the proposal
builds on the `Bytes` type and codecs of [0029](0029-binary-formats-and-codecs.md),
and its order against that issue is part of the decision.

## The proposal decides

- The key types: a public key, a private key and a key pair, typed by
  algorithm (Ed25519, X25519, ECDSA P-256, RSA), and how a key is read from
  and written to PEM, DER, JWK and OpenSSH.
- Signing and verifying a `String`, a `Bytes` value and a `Data` tree, the
  last over the canonical JSON that HQL-0041 part 3 fixes for digests.
- Encryption: authenticated symmetric encryption (AES-GCM,
  ChaCha20-Poly1305) and encryption to a public key (hybrid, such as HPKE or
  age), and whether encrypted output is an operation, a codec, or both.
- Keyed MACs (HMAC) and password hashing (Argon2), which HQL-0041 also
  leaves out.
- Where private key material may live: a value in a program, a file, an
  environment variable, a configured source; and what diagnostics and logs
  must never print.
- Module `crypto.keyvault`: a store of named keys that a program looks up
  by name and never holds in its source. It decides how a key is named and
  listed, whether a private key ever leaves the store or only signs and
  decrypts inside it, how the store is unlocked, and which backends it
  reads: a local encrypted file, the operating system keychain, an SSH or
  GPG agent, and a cloud key manager as a configured source.
- The Rust crates that implement it, under [`doc/stack.md`](../stack.md).

## Done when

- A proposal under `doc/proposals/` specifies modules `crypto` and
  `crypto.keyvault` and their formats, with examples under `examples/` at `status: proposed`.
- Its implementation is one or more backlog issues, ordered against 0029.
