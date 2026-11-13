//! Keys (HQL-0053 part 2): the algorithms `crypto` supports, the values of
//! their keys, and the formats a key is read from and written to.
//!
//! A private or symmetric key is held in the type its crate gives it, each
//! of which overwrites its memory when dropped; the symmetric key's octets
//! are a [`Zeroizing`] buffer. None of them implements `Display`, and
//! [`Key`]'s `Debug` writes the redacted form, so no formatting of a value
//! reaches key material. The only paths to the octets are the operations of
//! `crypto` that use them and [`keys::export_private`].

pub mod keys;
pub mod sign;

use std::fmt;
use zeroize::Zeroizing;

/// An asymmetric key algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Ed25519,
    P256,
    Rsa,
    X25519,
}

impl Algorithm {
    /// The algorithm a type of `crypto` names.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Ed25519" => Self::Ed25519,
            "P256" => Self::P256,
            "Rsa" => Self::Rsa,
            "X25519" => Self::X25519,
            _ => return None,
        })
    }

    /// The type name of the algorithm.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ed25519 => "Ed25519",
            Self::P256 => "P256",
            Self::Rsa => "Rsa",
            Self::X25519 => "X25519",
        }
    }

    /// Whether the algorithm signs.
    #[must_use]
    pub fn signs(self) -> bool {
        !matches!(self, Self::X25519)
    }
}

/// A symmetric algorithm: a cipher or a MAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
    HmacSha256,
    HmacSha512,
}

impl SecretAlgorithm {
    /// The algorithm a type of `crypto` names.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Aes256Gcm" => Self::Aes256Gcm,
            "ChaCha20Poly1305" => Self::ChaCha20Poly1305,
            "HmacSha256" => Self::HmacSha256,
            "HmacSha512" => Self::HmacSha512,
            _ => return None,
        })
    }

    /// The type name of the algorithm.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Aes256Gcm => "Aes256Gcm",
            Self::ChaCha20Poly1305 => "ChaCha20Poly1305",
            Self::HmacSha256 => "HmacSha256",
            Self::HmacSha512 => "HmacSha512",
        }
    }

    /// Whether the algorithm is a cipher, whose key is exactly 32 octets.
    #[must_use]
    pub fn is_cipher(self) -> bool {
        matches!(self, Self::Aes256Gcm | Self::ChaCha20Poly1305)
    }

    /// How many octets a generated key holds: 32, or 64 for `HmacSha512`.
    #[must_use]
    pub fn generated_length(self) -> usize {
        match self {
            Self::HmacSha512 => 64,
            _ => 32,
        }
    }
}

/// How one key is text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFormat {
    Pem,
    Jwk,
    OpenSsh,
    Age,
}

impl KeyFormat {
    /// The format a type of `crypto` names.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Pem" => Self::Pem,
            "Jwk" => Self::Jwk,
            "OpenSsh" => Self::OpenSsh,
            "Age" => Self::Age,
            _ => return None,
        })
    }

    /// The type name of the format.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Pem => "Pem",
            Self::Jwk => "Jwk",
            Self::OpenSsh => "OpenSsh",
            Self::Age => "Age",
        }
    }

    /// Whether the format holds keys of `algorithm`: `Age` holds `X25519`
    /// alone, and `X25519` has no `OpenSsh` text.
    #[must_use]
    pub fn holds(self, algorithm: Algorithm) -> bool {
        match self {
            Self::Pem | Self::Jwk => true,
            Self::OpenSsh => algorithm != Algorithm::X25519,
            Self::Age => algorithm == Algorithm::X25519,
        }
    }
}

/// A public key.
#[derive(Clone)]
pub enum PublicKey {
    Ed25519(ed25519_dalek::VerifyingKey),
    X25519([u8; 32]),
    P256(p256::PublicKey),
    Rsa(Box<rsa::PublicKey>),
}

/// A private key.
pub enum PrivateKey {
    Ed25519(ed25519_dalek::SigningKey),
    X25519(Zeroizing<[u8; 32]>),
    P256(p256::SecretKey),
    Rsa(Box<rsa::PrivateKey>),
}

/// A symmetric key.
pub struct SecretKey {
    pub algorithm: SecretAlgorithm,
    pub octets: Zeroizing<Vec<u8>>,
}

/// A key a value holds.
pub enum Key {
    Public(PublicKey),
    Private(PrivateKey),
    Secret(SecretKey),
}

impl PublicKey {
    /// The algorithm of the key.
    #[must_use]
    pub fn algorithm(&self) -> Algorithm {
        match self {
            Self::Ed25519(_) => Algorithm::Ed25519,
            Self::X25519(_) => Algorithm::X25519,
            Self::P256(_) => Algorithm::P256,
            Self::Rsa(_) => Algorithm::Rsa,
        }
    }
}

impl PartialEq for PublicKey {
    /// Public keys compare by their SPKI octets.
    fn eq(&self, other: &Self) -> bool {
        keys::to_der(self).ok() == keys::to_der(other).ok()
    }
}

impl PrivateKey {
    /// The algorithm of the key.
    #[must_use]
    pub fn algorithm(&self) -> Algorithm {
        match self {
            Self::Ed25519(_) => Algorithm::Ed25519,
            Self::X25519(_) => Algorithm::X25519,
            Self::P256(_) => Algorithm::P256,
            Self::Rsa(_) => Algorithm::Rsa,
        }
    }

    /// The public key of this private key.
    #[must_use]
    pub fn public(&self) -> PublicKey {
        match self {
            Self::Ed25519(key) => PublicKey::Ed25519(key.verifying_key()),
            Self::X25519(key) => PublicKey::X25519(
                x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(**key)).to_bytes(),
            ),
            Self::P256(key) => PublicKey::P256(key.public_key()),
            Self::Rsa(key) => PublicKey::Rsa(Box::new(key.to_public_key())),
        }
    }
}

impl Key {
    /// The type the value is of: `PublicKey<Ed25519>`.
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::Public(key) => format!("PublicKey<{}>", key.algorithm().name()),
            Self::Private(key) => format!("PrivateKey<{}>", key.algorithm().name()),
            Self::Secret(key) => format!("SecretKey<{}>", key.algorithm.name()),
        }
    }

    /// The text of the value wherever HQL produces text for one: a public
    /// or private key as its type and its public key's thumbprint, a
    /// symmetric key as its type and `redacted`.
    #[must_use]
    pub fn redacted(&self) -> String {
        match self {
            Self::Public(key) => format!("{}({})", self.type_name(), keys::thumbprint(key)),
            Self::Private(key) => {
                format!("{}({})", self.type_name(), keys::thumbprint(&key.public()))
            }
            Self::Secret(_) => format!("{}(redacted)", self.type_name()),
        }
    }

    /// Whether the value holds private key material.
    #[must_use]
    pub fn is_secret(&self) -> bool {
        !matches!(self, Self::Public(_))
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.redacted())
    }
}

impl PartialEq for Key {
    /// Public keys compare by their octets; a secret compares with nothing,
    /// which the checker refuses to write.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Public(left), Self::Public(right)) => left == right,
            _ => false,
        }
    }
}

/// Octets from the host's cryptographically secure generator.
///
/// # Errors
/// The operating system's refusal, as text.
pub fn random(octets: &mut [u8]) -> Result<(), String> {
    getrandom::fill(octets).map_err(|error| format!("the host generator failed: {error}"))
}

/// Compare two octet strings in time independent of where they differ.
#[must_use]
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    use subtle::ConstantTimeEq as _;
    left.len() == right.len() && bool::from(left.ct_eq(right))
}
