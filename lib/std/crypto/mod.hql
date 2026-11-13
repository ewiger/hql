// Not in the prelude: a program writes `import crypto`. Every declaration is
// external: the `std` extension implements it.
//
// See doc/proposals/HQL-0053/index.hmd and its parts 2 to 4.

/// Keys, signatures, MACs, password hashes and encryption. An algorithm and
/// a key format are each named by a type, as a format is:
/// `PublicKey(text, Pem, Ed25519)`. A key's type carries its algorithm, so a
/// key of one algorithm used where another is named is a check error.
/// `PrivateKey`, `KeyPair` and `SecretKey` are `Secret`: no format writes
/// one, `==` does not compare them, and each prints redacted.
module crypto

import core
import io

/// An asymmetric key algorithm. A `Signing` one's private key signs and
/// its public key verifies; an `Agreement` one's public key receives
/// encrypted messages.
abstract type KeyAlgorithm
abstract type Signing : KeyAlgorithm
abstract type Agreement : KeyAlgorithm
type Ed25519 : Signing     // RFC 8032, pure Ed25519
type P256    : Signing     // ECDSA over NIST P-256 with SHA-256
type Rsa     : Signing     // RSASSA-PKCS1-v1_5 with SHA-256, 2048 to 8192 bits
type X25519  : Agreement   // RFC 7748; receives age messages

/// A symmetric algorithm: a `Cipher` encrypts, a `Mac` authenticates.
abstract type SecretAlgorithm
abstract type Cipher : SecretAlgorithm
abstract type Mac    : SecretAlgorithm
type Aes256Gcm        : Cipher   // 256-bit key
type ChaCha20Poly1305 : Cipher   // 256-bit key, RFC 8439
type HmacSha256       : Mac      // RFC 2104 over SHA-256
type HmacSha512       : Mac      // RFC 2104 over SHA-512

/// The values that hold private key material: no format encodes one, `==`
/// does not compare them, and each prints redacted.
abstract type Secret

/// A public key of algorithm `A`. Public keys compare by their octets, and
/// one prints as its RFC 7638 thumbprint.
type PublicKey<A>

/// A private key of algorithm `A`. It prints as its public key's
/// thumbprint, and no format writes it.
type PrivateKey<A> : Secret

/// A private key and its public key.
type KeyPair<A> : Secret {
    public  : PublicKey<A>
    private : PrivateKey<A>
}

/// A symmetric key of algorithm `A`. It prints as `SecretKey<A>(redacted)`.
type SecretKey<A> : Secret

/// How one key is text. `Pem` is RFC 7468 text around DER: SPKI for a
/// public key, PKCS #8 for a private key. `Jwk` is the RFC 7517 JSON text of
/// one key. `OpenSsh` is an authorized_keys line or an OpenSSH private key
/// file. `Age` is age's Bech32 recipient and identity text, for `X25519`
/// only; `X25519` has no `OpenSsh` text.
abstract type KeyFormat
type Pem     : KeyFormat
type Jwk     : KeyFormat
type OpenSsh : KeyFormat
type Age     : KeyFormat

// Keys.

/// The public key `text` holds in `format`, for algorithm `algorithm`:
/// `PublicKey(text, Pem, Ed25519)`. Text that is not such a key is an
/// evaluation error naming the format and the algorithm; `to_public_key`
/// gives `none` instead.
fn PublicKey<F : KeyFormat, A : KeyAlgorithm>(text: String, format: Type<F>, algorithm: Type<A>) : PublicKey<A>

/// The public key the SPKI DER `der` holds.
fn PublicKey<A : KeyAlgorithm>(der: Bytes, algorithm: Type<A>) : PublicKey<A>

/// The public key `self` holds in `format`, or `none`.
fn to_public_key<F : KeyFormat, A : KeyAlgorithm>(self: String, format: Type<F>, algorithm: Type<A>) : Option<PublicKey<A>>

/// The private key `text` holds in `format`. `passphrase` decrypts an
/// encrypted PKCS #8 or OpenSSH private key; an empty one reads an
/// unencrypted key. A wrong passphrase is an evaluation error that says so.
fn PrivateKey<F : KeyFormat, A : KeyAlgorithm>(text: String, format: Type<F>, algorithm: Type<A>, *, passphrase: String = "") : PrivateKey<A>

/// The private key the PKCS #8 DER `der` holds.
fn PrivateKey<A : KeyAlgorithm>(der: Bytes, algorithm: Type<A>) : PrivateKey<A>

/// The private key `self` holds in `format`, or `none`.
fn to_private_key<F : KeyFormat, A : KeyAlgorithm>(self: String, format: Type<F>, algorithm: Type<A>, *, passphrase: String = "") : Option<PrivateKey<A>>

/// The symmetric key of algorithm `algorithm` that `key` holds: exactly 32
/// octets for a cipher, and at least one for a MAC. Another length is an
/// evaluation error naming the length.
fn SecretKey<A : SecretAlgorithm>(key: Bytes, algorithm: Type<A>) : SecretKey<A>

/// The public key a file holds in `format`.
effectful fn read_public_key<F : KeyFormat, A : KeyAlgorithm>(file: File, format: Type<F>, algorithm: Type<A>) : PublicKey<A>

/// The private key a file holds in `format`. On a Unix host, a file its
/// group or others may read gives a warning naming the path and its mode.
effectful fn read_private_key<F : KeyFormat, A : KeyAlgorithm>(file: File, format: Type<F>, algorithm: Type<A>, *, passphrase: String = "") : PrivateKey<A>

/// `self` written in `format`: `key.to_text(OpenSsh)` is an
/// `ssh-ed25519 AAAA…` line. `Pem` has 64-column lines and a final line
/// break, `Jwk` is the canonical JSON of the required members, and
/// `OpenSsh` has no comment.
fn to_text<F : KeyFormat, A : KeyAlgorithm>(self: PublicKey<A>, format: Type<F>) : String

/// `self` as SPKI DER.
fn to_der<A : KeyAlgorithm>(self: PublicKey<A>) : Bytes

/// The private key material of `self` written in `format`, encrypted under
/// `passphrase` where one is given: PKCS #8 with scrypt and AES-256-CBC for
/// `Pem`, bcrypt-pbkdf and AES-256-CTR for `OpenSsh`. The result is a
/// `String`, and no longer redacted.
fn export_private<F : KeyFormat, A : KeyAlgorithm>(self: PrivateKey<A>, format: Type<F>, *, passphrase: String = "") : String

/// The public key of a private key.
fn public<A : KeyAlgorithm>(self: PrivateKey<A>) : PublicKey<A>

/// The RFC 7638 JWK thumbprint of `self`: base64url, unpadded, of the
/// SHA-256 of its canonical JWK. It names a key without disclosing it, and
/// is the same whichever format the key was read from.
fn thumbprint<A : KeyAlgorithm>(self: PublicKey<A>) : String

/// A new key pair from the host's cryptographically secure generator.
/// `bits` is read for `Rsa` only, 2048 to 8192.
effectful fn generate_key_pair<A : KeyAlgorithm>(algorithm: Type<A>, *, bits: Int = 3072) : KeyPair<A>

/// A new symmetric key from the host generator: 32 octets for a cipher and
/// `HmacSha256`, 64 for `HmacSha512`.
effectful fn generate_secret_key<A : SecretAlgorithm>(algorithm: Type<A>) : SecretKey<A>

// Signatures (part 3). A signature is `Bytes`, detached from what it signs:
// Ed25519's 64 octets (R || S), P256's 64 (r || s, as JWS ES256), Rsa's
// modulus length (PKCS #1 v1.5 over SHA-256, as JWS RS256). Every scheme
// signs deterministically.

/// The signature of the octets of `self` by `key`.
fn sign<A : Signing>(self: Bytes, key: PrivateKey<A>) : Bytes

/// The signature of the UTF-8 octets of `self`.
fn sign<A : Signing>(self: String, key: PrivateKey<A>) : Bytes

/// The signature of `canonical(self)`, the RFC 8785 text of the tree, so
/// two trees equal under HQL equality have one signature.
fn sign_data<A : Signing>(self: Data, key: PrivateKey<A>) : Bytes

/// Whether `signature` is `key`'s signature of `self`: `false`, never an
/// error, for a signature of another key, of the wrong length or malformed.
fn verify<A : Signing>(self: Bytes, signature: Bytes, key: PublicKey<A>) : Bool

/// Whether `signature` is `key`'s signature of the UTF-8 octets of `self`.
fn verify<A : Signing>(self: String, signature: Bytes, key: PublicKey<A>) : Bool

/// Whether `signature` is `key`'s signature of `canonical(self)`.
fn verify_data<A : Signing>(self: Data, signature: Bytes, key: PublicKey<A>) : Bool

/// The `r || s` form of an ASN.1 DER ECDSA P-256 signature, as OpenSSL
/// writes one, or `none` where `self` is not one.
fn p256_signature_from_der(self: Bytes) : Option<Bytes>

// MACs.

/// The HMAC tag of `self` under `key`: 32 octets for `HmacSha256`, 64 for
/// `HmacSha512`. `mac(m, k) == tag` compares with `==`, which is not
/// constant-time; `verify_mac` is.
fn mac<A : Mac>(self: Bytes, key: SecretKey<A>) : Bytes

/// The HMAC tag of the UTF-8 octets of `self` under `key`.
fn mac<A : Mac>(self: String, key: SecretKey<A>) : Bytes

/// Whether `tag` is the MAC tag of `self` under `key`, compared in time
/// independent of where it first differs.
fn verify_mac<A : Mac>(self: Bytes, tag: Bytes, key: SecretKey<A>) : Bool

/// Whether `tag` is the MAC tag of the UTF-8 octets of `self`.
fn verify_mac<A : Mac>(self: String, tag: Bytes, key: SecretKey<A>) : Bool

// Password hashes.

/// The Argon2id hash of `self` under a new 16-octet salt from the host
/// generator, as a PHC string: `$argon2id$v=19$m=19456,t=2,p=1$…$…`. A cost
/// below OWASP's minimum is refused: `memory_kib` is 19456 to 1048576,
/// `iterations` at least 2, `parallelism` 1 to 16.
effectful fn hash_password(self: String, *, memory_kib: Int = 19456, iterations: Int = 2, parallelism: Int = 1) : String

/// Whether `self` is the password `hash` was made from, under the hash's
/// own parameters. `hash` is a PHC string of Argon2id, Argon2i or Argon2d;
/// any other is an evaluation error naming its algorithm identifier.
fn verify_password(self: String, hash: String) : Bool

// Encryption (part 4). Encryption is not a `Format`: over a value it is an
// operation from `Bytes` to `Bytes`, and over an endpoint a layer between
// the format and the transport.

/// The authenticated encryption of `self` under `key`, with `aad` bound to
/// it and not encrypted: a new 12-octet nonce from the host generator, then
/// the ciphertext, then the 16-octet tag. Under random nonces a key takes
/// at most 2^32 encryptions, for either cipher.
effectful fn encrypt<C : Cipher>(self: Bytes, key: SecretKey<C>, *, aad: Bytes = Bytes("")) : Bytes

/// The authenticated encryption of the UTF-8 octets of `self`.
effectful fn encrypt<C : Cipher>(self: String, key: SecretKey<C>, *, aad: Bytes = Bytes("")) : Bytes

/// The plaintext of `self` under `key` and `aad`, or `none` where `self` is
/// not an encryption of anything under them: shorter than 28 octets, a
/// changed octet anywhere, a wrong key or a wrong `aad`.
fn decrypt<C : Cipher>(self: Bytes, key: SecretKey<C>, *, aad: Bytes = Bytes("")) : Option<Bytes>

/// The age encryption of `self` to every key of `recipients`: any one of
/// their private keys decrypts it. The `age` command line reads it.
effectful fn encrypt_to(self: Bytes, recipients: List<PublicKey<X25519>>) : Bytes

/// The age encryption of the UTF-8 octets of `self` to `recipients`.
effectful fn encrypt_to(self: String, recipients: List<PublicKey<X25519>>) : Bytes

/// The plaintext of the age message `self` for `identity`, or `none` where
/// it is not a message to that key or has been changed.
fn decrypt_with(self: Bytes, identity: PrivateKey<X25519>) : Option<Bytes>

/// The age encryption of `self` under a passphrase, by age's scrypt
/// recipient with work factor 18.
effectful fn encrypt_with_passphrase(self: Bytes, passphrase: String) : Bytes

/// The plaintext of the age message `self` under `passphrase`, or `none`.
/// A work factor above 22 is an evaluation error: a message cannot make a
/// reader spend unbounded memory and time.
fn decrypt_with_passphrase(self: Bytes, passphrase: String) : Option<Bytes>

/// The ASCII armor of an age message: `-----BEGIN AGE ENCRYPTED FILE-----`.
fn armor(self: Bytes) : String

/// The age message an armored text holds, or `none`.
fn unarmor(self: String) : Option<Bytes>

// Encrypted endpoints. The wrappers apply to byte endpoints: files,
// standard input and output, and HTTP. Where the format is omitted, a
// literal path ending in `.age` implies the format of the path without it:
// `file("orders.jsonl.age")` under either wrapper is `JsonLines`.

/// An endpoint whose writes are age-encrypted to `recipients` and stored in
/// `endpoint`. `target` writes one age file and `sink` encrypts each chunk
/// as it fills, each replacing the file only once its last chunk is
/// written. A file's `.end` is refused: nothing is appended to an age file.
type EncryptedEnd : Writable

/// An endpoint whose contents are those of a replayable `endpoint`,
/// age-decrypted. `source` authenticates every chunk before it gives any
/// document; `tap` gives a chunk's documents once that chunk is
/// authenticated.
type DecryptedEnd : Replayable

/// An endpoint whose contents are those of a live `endpoint`, age-decrypted.
type DecryptedLive : Readable

/// `endpoint`, its writes age-encrypted to `recipients`.
fn encrypted_to(endpoint: Writable, recipients: List<PublicKey<X25519>>) : EncryptedEnd

/// `endpoint`, its contents age-decrypted with `identity`.
fn decrypted_with(endpoint: Replayable, identity: PrivateKey<X25519>) : DecryptedEnd

/// `endpoint`, its contents age-decrypted with `identity`.
fn decrypted_with(endpoint: Readable, identity: PrivateKey<X25519>) : DecryptedLive
