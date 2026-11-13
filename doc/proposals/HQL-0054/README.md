# HQL-0054: Key vaults: stored keys, SSH agents, keychains and cloud key managers

**Status**: accepted
**Created**: 2026-11-13
**Source**: [HQL-0053](../HQL-0053/README.md), whose part 5 this record was
split from; [issue 0071](../../issues/0071-key-vaults.md);
[HQL-0046](../HQL-0046/README.md) and [HQL-0049](../HQL-0049/README.md)
(configured endpoints and their secrets)

## Companion notes

- [`parts/01-keyvault.md`](parts/01-keyvault.md) — module
  `crypto.keyvault`: configured key stores, key names, signing and
  decrypting inside a store, unlocking, its backends, and `hql keyvault`.

## Abstract

Module `crypto.keyvault` reaches keys held in a store that `hql.toml`
configures under `[keyvault.<id>]`, by the vault's id and the key's name. A
stored private key signs and decrypts inside its vault and never becomes a
value of the program: `keyvault.sign`, `keyvault.sign_data` and
`keyvault.decrypt_with` give what HQL-0053's `crypto.sign`, `sign_data` and
`decrypt_with` give for the same key. Four kinds of vault are specified: an
age-encrypted `file` of private JWKs, the operating system's `keychain`, an
`ssh_agent`, and `aws_kms`. `hql keyvault` lists, generates, imports, prints
and deletes the keys of a `file` or `keychain` vault. The `std` extension
implements the module, which is not in the prelude.

## Motivation

- A private key read from a file is held by the evaluation for as long as
  the program holds the value. A key an SSH agent or a cloud key manager
  holds signs without ever entering the process.
- Teams already keep signing keys in an SSH agent (1Password's, Bitwarden's,
  `ssh-agent` itself), in the operating system's keychain, and in cloud key
  managers. A program that cannot reach them copies the key into a file.
- Naming a key by a vault id and a name, as an endpoint is named by an id,
  keeps every place a key comes from in configuration, where `hql check`
  sees it before anything runs.

## Non-goals

- A GPG agent as a backend, and OpenPGP keys. A key a GPG agent holds for
  SSH is reached through the `ssh_agent` kind, which it serves.
- `gcp_kms`, `azure_key_vault` and `vault_transit`: reserved kinds, each
  specified by the issue that implements it, under the rules of part 1.
- Creating, rotating or deleting keys from a program. A program reads a
  vault; `hql keyvault` changes one.

## Specification

### 1. Ownership

| Module | File | Holds |
| --- | --- | --- |
| `crypto.keyvault` | `lib/std/crypto/keyvault/mod.hql` | `KeyRef`, `KeyInfo`, `key`, `keys`, and the operations over a key in a store |

- `crypto.keyvault` imports `core`, `crypto` and `io`, and MUST NOT be added
  to the prelude. A program writes `import crypto.keyvault`, and reaches it
  as `keyvault`.
- The module MUST be listed in `library::MODULES` after `crypto`. The `std`
  extension's descriptor MUST provide it.

### 2. Effects

| Operation | Effect | Reads |
| --- | --- | --- |
| `key` | `fn` | nothing; a use of the reference reads the vault |
| every other operation of `crypto.keyvault` | `effectful` | a configured key vault |
| `decrypted_with` | `fn`, an endpoint constructor | nothing; reading the endpoint is the effect |

### 3. Configuration and grants

- `hql.toml` holds no key material. A `[keyvault.<id>]` table with a secret
  written into it MUST refuse the vault, naming the `*_env` key that
  replaces it, as `dsn` does in HQL-0046; part 1 lists the keys.
- `hql check` MUST open no vault, and MUST check every use against the
  `[keyvault.<id>]` tables.
- `hql render` MUST grant no vault: every use is refused at check time.

## Backwards Compatibility

- Additive. `keyvault` becomes a local name in a unit that imports
  `crypto.keyvault`.
- A `[keyvault.<id>]` table in `hql.toml` was ignored and is now read and
  checked.

## Security Considerations

- **Key vaults are visible.** Every use of a vault is `effectful` and names
  a configured id, so `hql check` reports every vault a program reaches
  before anything runs, and opens none of them.
- **The private key stays in its vault.** An `ssh_agent` and an `aws_kms`
  vault compute each signature outside the process. A `file` or `keychain`
  vault reads a private key for one operation into a buffer overwritten
  when the operation returns; it is never a value.
- **Nothing is printed.** A failure to unlock names the vault id and the
  variable, file or socket involved, never a passphrase, a credential or an
  octet of a store. A vault operation MAY log at `debug` the vault id, the
  key name, the operation and the key's thumbprint, and nothing else.
- **User state is reached only by configuration.** A `keychain` vault reads
  the user's keychain and an `ssh_agent` vault the user's agent, and only
  where `hql.toml` names one. No test reaches either on a host it runs on:
  the keychain's tests are ignored by default, and the agent's speak to an
  agent the test starts.

## Deployment / Activation

1. Add `[keyvault.<id>]` tables, `crypto.keyvault` and the `file` backend,
   with `hql keyvault` for it.
2. Add the `ssh_agent` backend.
3. Add the `aws_kms` backend.
4. Add the `keychain` backend.
5. Promote each corpus case under `examples/crypto/` as it checks.

## Reference Implementation

- `lib/std/crypto/keyvault/mod.hql`: the declarations of part 1.
- `src/extensions/keyvault.rs`: the natives; `src/crypto/keyvault.rs` for
  configuration, the store and unlocking, and one file per backend under
  `src/crypto/`.
- `src/endpoints.rs`: `[keyvault.<id>]` tables, read and checked as
  `[endpoint.<id>]` tables are.
- `src/cli.rs`: `hql keyvault`.
- Crates, each added with `cargo add` per [the stack](../../stack.md), pure
  Rust, none needing a C toolchain:

  | Need | Crate |
  | --- | --- |
  | OS keychains | `keyring` (`apple-native`, `windows-native`, `async-secret-service`, `crypto-rust`, `tokio`), on a one-thread `tokio` runtime as Kafka and NATS are |
  | a passphrase from the terminal | `rpassword` |

  The SSH agent protocol (list identities, sign) is hand-written over a Unix
  socket in `src/crypto/ssh_agent.rs`, as the Thrift calls of
  `src/spark.rs` are; AWS KMS's calls and their Signature Version 4 signing
  are hand-written over `ureq` in `src/crypto/aws_kms.rs`.
- `doc/stack.md` gains a row for key vaults.

## Test Plan

Example cases under [`examples/crypto/`](../../../examples/crypto/README.md),
at `status: proposed` with `proposal:` naming this record, promoted as each
checks:

| Case | Pins |
| --- | --- |
| [sign in a vault](../../../examples/crypto/queries/keyvault-sign.hql) | a signature from a store, verified by its public key |
| [decrypt in a vault](../../../examples/crypto/queries/keyvault-decrypt.hql) | age to a stored identity |
| [list a vault](../../../examples/crypto/queries/keyvault-keys.hql) | names and algorithms |

Part 1 names its unit tests. Across them:

- Refusals, each a `status: invalid` corpus case once implemented:
  `keyvault.decrypt_with` on an `ssh_agent` vault; a vault id `hql.toml`
  does not configure; a `passphrase` key in `[keyvault.<id>]`; and a vault
  use under `hql render`.
- AWS Signature Version 4 against the `get-vanilla` case of AWS's test
  suite.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Changelog

- 2026-11-13: split from HQL-0053 part 5; accepted
