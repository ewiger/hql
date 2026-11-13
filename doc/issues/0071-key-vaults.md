# 0071 — Key vaults: `crypto.keyvault` and its backends

Status: backlog
Related: [HQL-0054](../proposals/HQL-0054/README.md) (the proposal),
[HQL-0053](../proposals/HQL-0053/README.md),
[0067](archive/0067-std-crypto.md)

A program that signs or decrypts holds the private key as a value, read from
a file or the environment. Keys teams already keep elsewhere, in an SSH
agent, the operating system's keychain or a cloud key manager, are out of
reach, so a program copies them into a file.

[HQL-0054](../proposals/HQL-0054/README.md), split from HQL-0053's part 5,
specifies module `crypto.keyvault`: a key named by a configured vault's id
and its own name, which signs and decrypts inside its vault and is never a
value of the program, and `hql keyvault`, which changes a vault's keys.

## Work

- `[keyvault.<id>]` tables read beside `[endpoint.<id>]`, with every
  refusal HQL-0054 part 1 names.
- `crypto.keyvault`: `key`, `keys`, `public_key`, `sign`, `sign_data`,
  `decrypt_with` and `decrypted_with`, with the check-time rules: a literal,
  configured vault id; a key of an algorithm its kind holds; no vault under
  `hql render`.
- Backends, in this order, each its own commit: `file`, `ssh_agent`,
  `aws_kms`, `keychain`.
- `hql keyvault list | generate | import | public | delete` for `file` and
  `keychain` vaults.

## Constraints

- A backend that reaches user state, the keychain or an SSH agent, is
  reached only where `hql.toml` configures it, and no test touches the
  user's own: the keychain's tests are ignored by default, and the agent's
  speak to an agent the test starts.
- A failure names the vault, never a passphrase, a credential or an octet of
  a store.

## Done when

- The three cases under `examples/crypto/` that HQL-0054 lists check and
  are promoted.
- HQL-0054's status is `implemented`.
