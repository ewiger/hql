# Part 3: signatures, MACs and password hashes

- **Depends on**: [the proposal](../README.md) for effects and secret
  values; [part 1](01-bytes.md) for `Bytes` and `canonical`;
  [part 2](02-keys.md) for the key types
- **Needed by**: [HQL-0054](../../HQL-0054/README.md), whose `sign` gives what `sign`
  here gives

## Scope

Detached signatures over text, octets and `Data` trees; keyed MACs; and
password hashes. Python's `cryptography` signing interfaces, `hmac`, and the
`argon2-cffi` package cover the same ground.

## Signatures

A signature is `Bytes`, detached from what it signs.

```hql
// lib/std/crypto/mod.hql
/// The signature of the octets of `self` by `key`.
fn sign<A : Signing>(self: Bytes, key: PrivateKey<A>) : Bytes
/// The signature of the UTF-8 octets of `self`.
fn sign<A : Signing>(self: String, key: PrivateKey<A>) : Bytes
/// The signature of `canonical(self)`, the RFC 8785 text of the tree.
fn sign_data<A : Signing>(self: Data, key: PrivateKey<A>) : Bytes

/// Whether `signature` is `key`'s signature of `self`.
fn verify<A : Signing>(self: Bytes, signature: Bytes, key: PublicKey<A>) : Bool
fn verify<A : Signing>(self: String, signature: Bytes, key: PublicKey<A>) : Bool
fn verify_data<A : Signing>(self: Data, signature: Bytes, key: PublicKey<A>) : Bool

/// The `r || s` form of an ASN.1 DER ECDSA P-256 signature, as OpenSSL
/// writes one, or `none` where `self` is not one.
fn p256_signature_from_der(self: Bytes) : Option<Bytes>
```

The octets a signature holds, per algorithm:

```text
Ed25519   64 octets: R || S, RFC 8032 section 5.1.6
P256      64 octets: r || s, each 32 octets big-endian (IEEE P1363, as JWS ES256)
Rsa       k octets, k the modulus length: RSASSA-PKCS1-v1_5, SHA-256 (as JWS RS256)
```

- `sign_data(x, k)` MUST equal `sign(canonical(x), k)`, and
  `verify_data(x, s, p)` MUST equal `verify(canonical(x), s, p)`. Two trees
  equal under HQL equality then have one signature, whatever their key
  order, and a verifier in another language checks it with an RFC 8785
  implementation.
- A `String` and a `Data` tree are signed by different names. A `Data` tree
  whose value is a string is signed as the JSON text of that string, with
  its quotes, and `sign` and `sign_data` never select each other.
- `verify` MUST give `false` for a signature of the wrong length, a
  malformed one, and one by another key, and MUST NOT fail.
- `verify` of `P256` reads `r || s` only. A DER signature is converted
  first with `p256_signature_from_der`.

## MACs

```hql
/// The MAC tag of `self` under `key`: 32 octets for `HmacSha256`, 64 for
/// `HmacSha512`.
fn mac<A : Mac>(self: Bytes, key: SecretKey<A>) : Bytes
fn mac<A : Mac>(self: String, key: SecretKey<A>) : Bytes
/// Whether `tag` is the MAC tag of `self` under `key`, compared in
/// constant time.
fn verify_mac<A : Mac>(self: Bytes, tag: Bytes, key: SecretKey<A>) : Bool
fn verify_mac<A : Mac>(self: String, tag: Bytes, key: SecretKey<A>) : Bool
```

- `verify_mac` MUST compare the whole tag in time independent of where it
  first differs. A program that writes `mac(m, k) == tag` compares `Bytes`
  with `==`, which is not constant-time; the reference page of `mac` says
  so and names `verify_mac`.
- A `SecretKey` of a `Mac` algorithm MUST hold at least one octet, and
  takes any length RFC 2104 takes: a webhook's secret is the length its
  sender chose. `generate_secret_key` gives 32 octets for `HmacSha256` and
  64 for `HmacSha512`, the output length of each.

## Password hashes

```hql
/// The Argon2id hash of `self` under a new 16-octet salt from the host
/// generator, as a PHC string: `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`.
effectful fn hash_password(self: String, *, memory_kib: Int = 19456, iterations: Int = 2, parallelism: Int = 1) : String
/// Whether `self` is the password `hash` was made from. `hash` is a PHC
/// string of Argon2id, Argon2i or Argon2d, and its own parameters apply.
fn verify_password(self: String, hash: String) : Bool
```

- The hash MUST be 32 octets, and the PHC string MUST write salt and hash
  in unpadded standard base64, as the PHC string format specifies.
- `memory_kib` MUST be at least 19456 and at most 1048576 (1 GiB),
  `iterations` at least 2, `parallelism` 1 to 16. A value outside is an
  evaluation error naming the parameter: a cost below OWASP's minimum is
  refused, not weakened silently.
- `verify_password` MUST give `false` for a wrong password and MUST be an
  evaluation error for a `hash` that is not a PHC string of an Argon2
  variant, naming the algorithm identifier it found. A stored hash of
  another scheme is a data error, not a wrong password.

## Examples

```hql
import crypto
import encoding
import io.env

// Verify a webhook: the body's HMAC-SHA256, hex, in a header.
key = env.var("WEBHOOK_SECRET") | map(s => SecretKey(Bytes(s), HmacSha256))
ok = key | map(k => request.body.verify_mac(request.headers["x-signature"]
                                            | and_then(h => h.to_bytes(Hex))
                                            | unwrap_or(Bytes("")), k))
         | unwrap_or(false)

// Sign each record of an export, and ship the signature beside it.
signer = read_private_key(file("export.pem"), Pem, Ed25519)
records | map(r => {record: r, signature: r.sign_data(signer).encode(Base64)})

// Check a login against stored hashes.
users | filter(u => u.name == name) | map(u => password.verify_password(u.hash))
```

Cases: [RFC 8032 signature](../../../../examples/crypto/queries/sign-rfc8032-test-1.hql),
[verify a release](../../../../examples/crypto/queries/verify-a-release.hql),
[HMAC](../../../../examples/crypto/queries/hmac-rfc4231-case-2.hql),
[password hashes](../../../../examples/crypto/queries/password-hash.hql).

## Test plan

- RFC 8032 section 7.1 tests 1 to 3 and the 1023-octet test 4 for
  Ed25519; RFC 6979 A.2.5 for P-256 with SHA-256, compared after the
  conversion to `r || s`; an RS256 vector from RFC 7515 appendix A.2.
- `sign_data` of two trees with the same members in different key order
  gives one signature; a changed member gives `verify_data` `false`.
- A signature written by `openssl pkeyutl` and one by `ssh-keygen -Y sign`
  for the same key and message verify.
- RFC 4231 test cases 1 to 7 for both MACs, test case 5's truncated tag
  excluded: `mac` gives the whole tag.
- RFC 9106's Argon2id vector through the Rust crate; a PHC string written
  by the `argon2` command line verifies; an `$2b$` bcrypt string is an
  evaluation error.
- `hash_password` with `memory_kib = 4096` is refused.
