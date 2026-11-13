//! Reading and writing keys: DER (SPKI and PKCS #8), PEM around it, JWK,
//! OpenSSH, and age's Bech32 text; RFC 7638 thumbprints; generation.
//!
//! Every error is text naming the format and the algorithm, and never
//! quotes the key text it was reading.

use super::{Algorithm, KeyFormat, PrivateKey, PublicKey};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rsa::traits::{PrivateKeyParts as _, PublicKeyParts as _};
use zeroize::Zeroizing;

/// Why a private key was not read.
#[derive(Debug)]
pub enum ReadError {
    /// The text is not such a key.
    Invalid(String),
    /// The key is encrypted and the passphrase does not decrypt it.
    Passphrase(String),
}

impl ReadError {
    /// The error's text.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Invalid(message) | Self::Passphrase(message) => message,
        }
    }
}

type Result<T> = std::result::Result<T, String>;

const ED25519_OID: &str = "1.3.101.112";
const X25519_OID: &str = "1.3.101.110";
const EC_PUBLIC_KEY_OID: &str = "1.2.840.10045.2.1";
const P256_OID: &str = "1.2.840.10045.3.1.7";
const RSA_OID: &str = "1.2.840.113549.1.1.1";

/// The fixed DER prefix of an Ed25519 or X25519 SPKI, before its 32 octets.
fn okp_spki_prefix(algorithm: Algorithm) -> [u8; 12] {
    let last = if algorithm == Algorithm::X25519 {
        0x6e
    } else {
        0x70
    };
    [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, last, 0x03, 0x21, 0x00,
    ]
}

/// The fixed DER prefix of an Ed25519 or X25519 PKCS #8 v1 key.
fn okp_pkcs8_prefix(algorithm: Algorithm) -> [u8; 16] {
    let last = if algorithm == Algorithm::X25519 {
        0x6e
    } else {
        0x70
    };
    [
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, last, 0x04, 0x22, 0x04,
        0x20,
    ]
}

/// The algorithm an SPKI or PKCS #8 algorithm identifier names.
fn algorithm_of(oid: &str, parameter: Option<&str>) -> Option<Algorithm> {
    match (oid, parameter) {
        (ED25519_OID, _) => Some(Algorithm::Ed25519),
        (X25519_OID, _) => Some(Algorithm::X25519),
        (EC_PUBLIC_KEY_OID, Some(P256_OID)) => Some(Algorithm::P256),
        (RSA_OID, _) => Some(Algorithm::Rsa),
        _ => None,
    }
}

/// Refuse a key of another algorithm than the one named.
fn same(found: Algorithm, wanted: Algorithm, format: &str) -> Result<()> {
    if found == wanted {
        Ok(())
    } else {
        Err(format!(
            "the {format} text holds an {} key, and it is read as {}",
            found.name(),
            wanted.name()
        ))
    }
}

/// Hold an RSA key to the sizes and exponent `crypto` takes.
fn rsa_allowed(modulus_bits: usize, exponent: &rsa::BoxedUint) -> Result<()> {
    if !(2048..=8192).contains(&modulus_bits) {
        return Err(format!(
            "an Rsa key has a modulus of 2048 to 8192 bits, and this one has {modulus_bits}"
        ));
    }
    if *exponent != rsa::BoxedUint::from(65_537_u64) {
        return Err("an Rsa key has the public exponent 65537".to_owned());
    }
    Ok(())
}

fn rsa_public_allowed(key: &rsa::PublicKey) -> Result<()> {
    rsa_allowed(key.n().bit_width() as usize, key.e())
}

// DER.

/// The public key an SPKI DER holds.
///
/// # Errors
/// Names the algorithm the DER holds where it is another, or why it is
/// not a key.
pub fn public_from_der(der: &[u8], wanted: Algorithm) -> Result<PublicKey> {
    use pkcs8::der::Decode as _;
    let info = spki::SubjectPublicKeyInfoRef::from_der(der)
        .map_err(|_| format!("the DER is not a public key (SPKI) of {}", wanted.name()))?;
    let parameter = info
        .algorithm
        .parameters
        .as_ref()
        .and_then(|any| any.decode_as::<pkcs8::der::asn1::ObjectIdentifier>().ok())
        .map(|oid| oid.to_string());
    let found =
        algorithm_of(&info.algorithm.oid.to_string(), parameter.as_deref()).ok_or_else(|| {
            "the DER holds a public key of an algorithm crypto does not read".to_owned()
        })?;
    same(found, wanted, "DER")?;
    let bits = info.subject_public_key.raw_bytes();
    let invalid = |what: &str| {
        format!(
            "the DER is not a valid {} public key: {what}",
            wanted.name()
        )
    };
    match wanted {
        Algorithm::Ed25519 => {
            let octets: [u8; 32] = bits.try_into().map_err(|_| invalid("not 32 octets"))?;
            ed25519_dalek::VerifyingKey::from_bytes(&octets)
                .map(PublicKey::Ed25519)
                .map_err(|_| invalid("not a point of the curve"))
        }
        Algorithm::X25519 => bits
            .try_into()
            .map(PublicKey::X25519)
            .map_err(|_| invalid("not 32 octets")),
        Algorithm::P256 => p256::PublicKey::from_sec1_bytes(bits)
            .map(PublicKey::P256)
            .map_err(|_| invalid("not a point on the curve")),
        Algorithm::Rsa => {
            use rsa::pkcs8::DecodePublicKey as _;
            let key = rsa::PublicKey::from_public_key_der(der)
                .map_err(|error| invalid(&error.to_string()))?;
            rsa_public_allowed(&key)?;
            Ok(PublicKey::Rsa(Box::new(key)))
        }
    }
}

/// A public key as SPKI DER.
///
/// # Errors
/// The encoder's failure, which a valid key does not have.
pub fn to_der(key: &PublicKey) -> Result<Vec<u8>> {
    match key {
        PublicKey::Ed25519(key) => {
            Ok([&okp_spki_prefix(Algorithm::Ed25519)[..], key.as_bytes()].concat())
        }
        PublicKey::X25519(key) => Ok([&okp_spki_prefix(Algorithm::X25519)[..], key].concat()),
        PublicKey::P256(key) => {
            use p256::pkcs8::EncodePublicKey as _;
            key.to_public_key_der()
                .map(|der| der.as_bytes().to_vec())
                .map_err(|error| error.to_string())
        }
        PublicKey::Rsa(key) => {
            use rsa::pkcs8::EncodePublicKey as _;
            key.to_public_key_der()
                .map(|der| der.as_bytes().to_vec())
                .map_err(|error| error.to_string())
        }
    }
}

/// The private key a PKCS #8 DER holds.
///
/// # Errors
/// Names the algorithm the DER holds where it is another, or why it is
/// not a key.
pub fn private_from_der(der: &[u8], wanted: Algorithm) -> Result<PrivateKey> {
    use pkcs8::der::Decode as _;
    let info = pkcs8::PrivateKeyInfoRef::from_der(der).map_err(|_| {
        format!(
            "the DER is not a private key (PKCS #8) of {}",
            wanted.name()
        )
    })?;
    let parameter = info
        .algorithm
        .parameters
        .as_ref()
        .and_then(|any| any.decode_as::<pkcs8::der::asn1::ObjectIdentifier>().ok())
        .map(|oid| oid.to_string());
    let found =
        algorithm_of(&info.algorithm.oid.to_string(), parameter.as_deref()).ok_or_else(|| {
            "the DER holds a private key of an algorithm crypto does not read".to_owned()
        })?;
    same(found, wanted, "DER")?;
    let invalid = |what: &str| {
        format!(
            "the DER is not a valid {} private key: {what}",
            wanted.name()
        )
    };
    match wanted {
        Algorithm::Ed25519 | Algorithm::X25519 => {
            // The private key is an OCTET STRING of 32 octets inside the
            // PKCS #8 OCTET STRING.
            let inner = info.private_key.as_bytes();
            let seed = match inner {
                [0x04, 0x20, rest @ ..] if rest.len() == 32 => rest,
                _ => return Err(invalid("not 32 octets")),
            };
            let mut octets = Zeroizing::new([0_u8; 32]);
            octets.copy_from_slice(seed);
            Ok(if wanted == Algorithm::Ed25519 {
                PrivateKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&octets))
            } else {
                PrivateKey::X25519(octets)
            })
        }
        Algorithm::P256 => {
            use p256::pkcs8::DecodePrivateKey as _;
            p256::SecretKey::from_pkcs8_der(der)
                .map(PrivateKey::P256)
                .map_err(|error| invalid(&error.to_string()))
        }
        Algorithm::Rsa => {
            use rsa::pkcs8::DecodePrivateKey as _;
            let key = rsa::PrivateKey::from_pkcs8_der(der)
                .map_err(|error| invalid(&error.to_string()))?;
            rsa_allowed(key.n().bit_width() as usize, key.e())?;
            Ok(PrivateKey::Rsa(Box::new(key)))
        }
    }
}

/// A private key as PKCS #8 DER.
///
/// # Errors
/// The encoder's failure, which a valid key does not have.
pub fn private_to_der(key: &PrivateKey) -> Result<Zeroizing<Vec<u8>>> {
    match key {
        PrivateKey::Ed25519(key) => Ok(Zeroizing::new(
            [
                &okp_pkcs8_prefix(Algorithm::Ed25519)[..],
                &key.to_bytes()[..],
            ]
            .concat(),
        )),
        PrivateKey::X25519(key) => Ok(Zeroizing::new(
            [&okp_pkcs8_prefix(Algorithm::X25519)[..], &key[..]].concat(),
        )),
        PrivateKey::P256(key) => {
            use p256::pkcs8::EncodePrivateKey as _;
            key.to_pkcs8_der()
                .map(|der| Zeroizing::new(der.as_bytes().to_vec()))
                .map_err(|error| error.to_string())
        }
        PrivateKey::Rsa(key) => {
            use rsa::pkcs8::EncodePrivateKey as _;
            key.to_pkcs8_der()
                .map(|der| Zeroizing::new(der.as_bytes().to_vec()))
                .map_err(|error| error.to_string())
        }
    }
}

// PEM.

/// The label and DER of the one PEM block `text` holds; text before and
/// after it is ignored, as RFC 7468 allows.
fn pem_block(text: &str, format_error: &str) -> Result<(String, Zeroizing<Vec<u8>>)> {
    let begins = text.matches("-----BEGIN ").count();
    if begins != 1 {
        return Err(format!(
            "{format_error}: Pem text holds exactly one PEM block, and this holds {begins}"
        ));
    }
    let start = text.find("-----BEGIN ").unwrap_or(0);
    let end_marker = text[start..]
        .find("-----END ")
        .and_then(|at| {
            text[start + at..]
                .find("-----\n")
                .or_else(|| text[start + at + 9..].find("-----").map(|end| end + 9))
                .map(|end| start + at + end + 5)
        })
        .ok_or_else(|| format!("{format_error}: the PEM block has no END line"))?;
    let block = &text[start..end_marker.min(text.len())];
    let (label, der) = pem_rfc7468::decode_vec(block.trim().as_bytes())
        .map_err(|_| format!("{format_error}: the PEM block is not valid RFC 7468 text"))?;
    Ok((label.to_owned(), Zeroizing::new(der)))
}

/// Text in PEM: 64-column lines and a final line break.
fn pem(label: &str, der: &[u8]) -> Result<String> {
    pem_rfc7468::encode_string(label, pem_rfc7468::LineEnding::LF, der)
        .map_err(|error| error.to_string())
}

// JWK.

fn b64url(octets: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(octets)
}

fn member<'a>(
    jwk: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
    wanted: Algorithm,
) -> Result<&'a str> {
    jwk.get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            format!(
                "the Jwk object of a {} key has no `{name}` member",
                wanted.name()
            )
        })
}

fn member_octets(
    jwk: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    wanted: Algorithm,
) -> Result<Zeroizing<Vec<u8>>> {
    let text = member(jwk, name, wanted)?;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text.trim_end_matches('='))
        .map(Zeroizing::new)
        .map_err(|_| format!("the `{name}` member of the Jwk object is not base64url"))
}

/// The algorithm a JWK object's `kty` and `crv` name.
fn jwk_algorithm(jwk: &serde_json::Map<String, serde_json::Value>) -> Option<Algorithm> {
    let crv = jwk.get("crv").and_then(serde_json::Value::as_str);
    match (jwk.get("kty").and_then(serde_json::Value::as_str)?, crv) {
        ("OKP", Some("Ed25519")) => Some(Algorithm::Ed25519),
        ("OKP", Some("X25519")) => Some(Algorithm::X25519),
        ("EC", Some("P-256")) => Some(Algorithm::P256),
        ("RSA", _) => Some(Algorithm::Rsa),
        _ => None,
    }
}

fn jwk_object(text: &str, wanted: Algorithm) -> Result<serde_json::Map<String, serde_json::Value>> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| {
        format!(
            "the Jwk text of a {} key is not a JSON object",
            wanted.name()
        )
    })?;
    let serde_json::Value::Object(jwk) = value else {
        return Err(format!(
            "the Jwk text of a {} key is not a JSON object",
            wanted.name()
        ));
    };
    let found = jwk_algorithm(&jwk)
        .ok_or_else(|| "the Jwk object names no key type crypto reads".to_owned())?;
    same(found, wanted, "Jwk")?;
    Ok(jwk)
}

fn uint(octets: &[u8]) -> rsa::BoxedUint {
    let bits = u32::try_from(octets.len() * 8).unwrap_or(u32::MAX).max(64);
    rsa::BoxedUint::from_be_slice(octets, bits.next_multiple_of(64))
        .unwrap_or_else(|_| rsa::BoxedUint::zero())
}

fn uint_octets(value: &rsa::BoxedUint) -> Vec<u8> {
    let octets = value.to_be_bytes();
    let start = octets
        .iter()
        .position(|octet| *octet != 0)
        .unwrap_or(octets.len());
    octets[start..].to_vec()
}

fn public_from_jwk(
    jwk: &serde_json::Map<String, serde_json::Value>,
    wanted: Algorithm,
) -> Result<PublicKey> {
    match wanted {
        Algorithm::Ed25519 | Algorithm::X25519 => {
            let x = member_octets(jwk, "x", wanted)?;
            let der = [&okp_spki_prefix(wanted)[..], &x[..]].concat();
            public_from_der(&der, wanted)
        }
        Algorithm::P256 => {
            let (x, y) = (
                member_octets(jwk, "x", wanted)?,
                member_octets(jwk, "y", wanted)?,
            );
            if x.len() != 32 || y.len() != 32 {
                return Err("the `x` and `y` members of a P256 Jwk are 32 octets each".to_owned());
            }
            let point = [&[0x04][..], &x[..], &y[..]].concat();
            p256::PublicKey::from_sec1_bytes(&point)
                .map(PublicKey::P256)
                .map_err(|_| "the Jwk point is not on the P-256 curve".to_owned())
        }
        Algorithm::Rsa => {
            let (n, e) = (
                member_octets(jwk, "n", wanted)?,
                member_octets(jwk, "e", wanted)?,
            );
            let key = rsa::PublicKey::new(uint(&n), uint(&e))
                .map_err(|error| format!("the Jwk is not a valid Rsa key: {error}"))?;
            rsa_public_allowed(&key)?;
            Ok(PublicKey::Rsa(Box::new(key)))
        }
    }
}

fn private_from_jwk(
    jwk: &serde_json::Map<String, serde_json::Value>,
    wanted: Algorithm,
) -> Result<PrivateKey> {
    let d = member_octets(jwk, "d", wanted)?;
    let key = match wanted {
        Algorithm::Ed25519 | Algorithm::X25519 => {
            if d.len() != 32 {
                return Err(format!(
                    "the `d` member of a {} Jwk is 32 octets",
                    wanted.name()
                ));
            }
            let der = Zeroizing::new([&okp_pkcs8_prefix(wanted)[..], &d[..]].concat());
            private_from_der(&der, wanted)?
        }
        Algorithm::P256 => PrivateKey::P256(
            p256::SecretKey::from_slice(&d)
                .map_err(|_| "the `d` member is not a P-256 scalar".to_owned())?,
        ),
        Algorithm::Rsa => {
            let (n, e) = (
                member_octets(jwk, "n", wanted)?,
                member_octets(jwk, "e", wanted)?,
            );
            let (p, q) = (
                member_octets(jwk, "p", wanted)?,
                member_octets(jwk, "q", wanted)?,
            );
            let key = rsa::PrivateKey::from_components(
                uint(&n),
                uint(&e),
                uint(&d),
                vec![uint(&p), uint(&q)],
            )
            .map_err(|error| format!("the Jwk is not a valid Rsa key: {error}"))?;
            rsa_allowed(key.n().bit_width() as usize, key.e())?;
            PrivateKey::Rsa(Box::new(key))
        }
    };
    // A private JWK that carries its public members must agree with them.
    if jwk.contains_key("x") || jwk.contains_key("n") {
        let public = public_from_jwk(jwk, wanted)?;
        if public != key.public() {
            return Err("the Jwk's public members are not the public key of its `d`".to_owned());
        }
    }
    Ok(key)
}

/// The members of a public JWK, sorted, as RFC 7638 and RFC 8785 order
/// them.
fn public_jwk(key: &PublicKey) -> Vec<(&'static str, String)> {
    match key {
        PublicKey::Ed25519(key) => vec![
            ("crv", "Ed25519".to_owned()),
            ("kty", "OKP".to_owned()),
            ("x", b64url(key.as_bytes())),
        ],
        PublicKey::X25519(key) => vec![
            ("crv", "X25519".to_owned()),
            ("kty", "OKP".to_owned()),
            ("x", b64url(key)),
        ],
        PublicKey::P256(key) => {
            use p256::elliptic_curve::sec1::ToSec1Point as _;
            let point = key.to_sec1_point(false);
            vec![
                ("crv", "P-256".to_owned()),
                ("kty", "EC".to_owned()),
                ("x", b64url(point.x().map_or(&[][..], |x| &x[..]))),
                ("y", b64url(point.y().map_or(&[][..], |y| &y[..]))),
            ]
        }
        PublicKey::Rsa(key) => vec![
            ("e", b64url(&uint_octets(key.e()))),
            ("kty", "RSA".to_owned()),
            ("n", b64url(&uint_octets(key.n().as_ref()))),
        ],
    }
}

/// Members as canonical JSON (RFC 8785): every value here is a string, and
/// the members are already sorted.
fn canonical_object(members: &[(&str, String)]) -> String {
    let body: Vec<String> = members
        .iter()
        .map(|(name, value)| {
            format!(
                "{}:{}",
                serde_json::Value::from(*name),
                serde_json::Value::from(value.as_str())
            )
        })
        .collect();
    format!("{{{}}}", body.join(","))
}

/// The RFC 7638 JWK thumbprint of a public key: base64url, unpadded, of the
/// SHA-256 of its canonical JWK.
#[must_use]
pub fn thumbprint(key: &PublicKey) -> String {
    use sha2::Digest as _;
    b64url(&sha2::Sha256::digest(
        canonical_object(&public_jwk(key)).as_bytes(),
    ))
}

// OpenSSH.

fn mpint(octets: &[u8]) -> Result<ssh_key::Mpint> {
    ssh_key::Mpint::from_positive_bytes(octets).map_err(|error| error.to_string())
}

fn public_from_openssh(text: &str, wanted: Algorithm) -> Result<PublicKey> {
    let key = ssh_key::PublicKey::from_openssh(text.trim()).map_err(|_| {
        format!(
            "the OpenSsh text is not a public key line of {}",
            wanted.name()
        )
    })?;
    public_from_ssh_data(key.key_data(), wanted)
}

fn ssh_algorithm(data: &ssh_key::public::KeyData) -> Option<Algorithm> {
    match data {
        ssh_key::public::KeyData::Ed25519(_) => Some(Algorithm::Ed25519),
        ssh_key::public::KeyData::Ecdsa(ssh_key::public::EcdsaPublicKey::NistP256(_)) => {
            Some(Algorithm::P256)
        }
        ssh_key::public::KeyData::Rsa(_) => Some(Algorithm::Rsa),
        _ => None,
    }
}

fn public_from_ssh_data(data: &ssh_key::public::KeyData, wanted: Algorithm) -> Result<PublicKey> {
    let found = ssh_algorithm(data).ok_or_else(|| {
        format!(
            "the OpenSsh key is a {}, which crypto does not read",
            data.algorithm()
        )
    })?;
    same(found, wanted, "OpenSsh")?;
    match data {
        ssh_key::public::KeyData::Ed25519(key) => ed25519_dalek::VerifyingKey::from_bytes(&key.0)
            .map(PublicKey::Ed25519)
            .map_err(|_| "the OpenSsh Ed25519 key is not a point of the curve".to_owned()),
        ssh_key::public::KeyData::Ecdsa(key) => {
            p256::PublicKey::from_sec1_bytes(key.as_sec1_bytes())
                .map(PublicKey::P256)
                .map_err(|_| "the OpenSsh P-256 key is not a point on the curve".to_owned())
        }
        ssh_key::public::KeyData::Rsa(key) => {
            let rsa = rsa::PublicKey::new(
                uint(key.n.as_positive_bytes().unwrap_or_default()),
                uint(key.e.as_positive_bytes().unwrap_or_default()),
            )
            .map_err(|error| format!("the OpenSsh Rsa key is not valid: {error}"))?;
            rsa_public_allowed(&rsa)?;
            Ok(PublicKey::Rsa(Box::new(rsa)))
        }
        _ => Err("the OpenSsh key is of an algorithm crypto does not read".to_owned()),
    }
}

fn ssh_public_data(key: &PublicKey) -> Result<ssh_key::public::KeyData> {
    Ok(match key {
        PublicKey::Ed25519(key) => {
            ssh_key::public::KeyData::Ed25519(ssh_key::public::Ed25519PublicKey(key.to_bytes()))
        }
        PublicKey::P256(key) => {
            use p256::elliptic_curve::sec1::ToSec1Point as _;
            ssh_key::public::KeyData::Ecdsa(
                ssh_key::public::EcdsaPublicKey::from_sec1_bytes(
                    key.to_sec1_point(false).as_bytes(),
                )
                .map_err(|error| error.to_string())?,
            )
        }
        PublicKey::Rsa(key) => ssh_key::public::KeyData::Rsa(ssh_key::public::RsaPublicKey {
            e: mpint(&uint_octets(key.e()))?,
            n: mpint(&uint_octets(key.n().as_ref()))?,
        }),
        PublicKey::X25519(_) => return Err("an X25519 key has no OpenSsh text".to_owned()),
    })
}

fn private_from_openssh(
    text: &str,
    wanted: Algorithm,
    passphrase: &str,
) -> std::result::Result<PrivateKey, ReadError> {
    let key = ssh_key::PrivateKey::from_openssh(text.trim()).map_err(|_| {
        ReadError::Invalid(format!(
            "the OpenSsh text is not a private key of {}",
            wanted.name()
        ))
    })?;
    let key = if key.is_encrypted() {
        key.decrypt(passphrase.as_bytes()).map_err(|_| {
            ReadError::Passphrase(if passphrase.is_empty() {
                "the OpenSsh private key is encrypted, and no passphrase is given".to_owned()
            } else {
                "the passphrase does not decrypt the OpenSsh private key".to_owned()
            })
        })?
    } else {
        key
    };
    let found = ssh_algorithm(key.public_key().key_data());
    let invalid = |message: String| ReadError::Invalid(message);
    let found = found.ok_or_else(|| {
        invalid(format!(
            "the OpenSsh key is a {}, which crypto does not read",
            key.algorithm()
        ))
    })?;
    same(found, wanted, "OpenSsh").map_err(invalid)?;
    let read = match key.key_data() {
        ssh_key::private::KeypairData::Ed25519(pair) => PrivateKey::Ed25519(
            ed25519_dalek::SigningKey::from_bytes(&pair.private.to_bytes()),
        ),
        ssh_key::private::KeypairData::Ecdsa(ssh_key::private::EcdsaKeypair::NistP256 {
            private,
            ..
        }) => PrivateKey::P256(
            p256::SecretKey::from_slice(private.as_slice())
                .map_err(|_| invalid("the OpenSsh P-256 scalar is not valid".to_owned()))?,
        ),
        ssh_key::private::KeypairData::Rsa(pair) => {
            let octets =
                |value: &ssh_key::Mpint| uint(value.as_positive_bytes().unwrap_or_default());
            let key = rsa::PrivateKey::from_components(
                octets(&pair.public.n),
                octets(&pair.public.e),
                octets(&pair.private.d),
                vec![octets(&pair.private.p), octets(&pair.private.q)],
            )
            .map_err(|error| invalid(format!("the OpenSsh Rsa key is not valid: {error}")))?;
            rsa_allowed(key.n().bit_width() as usize, key.e()).map_err(invalid)?;
            PrivateKey::Rsa(Box::new(key))
        }
        _ => {
            return Err(invalid(
                "the OpenSsh key is of an algorithm crypto does not read".to_owned(),
            ));
        }
    };
    Ok(read)
}

fn openssh_private(key: &PrivateKey, passphrase: &str) -> Result<String> {
    let data = match key {
        PrivateKey::Ed25519(key) => {
            ssh_key::private::KeypairData::Ed25519(ssh_key::private::Ed25519Keypair {
                public: ssh_key::public::Ed25519PublicKey(key.verifying_key().to_bytes()),
                private: ssh_key::private::Ed25519PrivateKey::from_bytes(&key.to_bytes()),
            })
        }
        PrivateKey::P256(secret) => {
            let ssh_key::public::KeyData::Ecdsa(ssh_key::public::EcdsaPublicKey::NistP256(public)) =
                ssh_public_data(&key.public())?
            else {
                return Err("a P-256 key's public point did not encode".to_owned());
            };
            // The scalar as the SSH wire writes it: a length, then the
            // octets, which is the one way the type is built.
            let scalar = Zeroizing::new(secret.to_bytes().to_vec());
            let mut wire = Zeroizing::new(32_u32.to_be_bytes().to_vec());
            wire.extend_from_slice(&scalar);
            let private = <ssh_key::private::EcdsaPrivateKey<32> as ssh_encoding::Decode>::decode(
                &mut &wire[..],
            )
            .map_err(|error| error.to_string())?;
            ssh_key::private::KeypairData::Ecdsa(ssh_key::private::EcdsaKeypair::NistP256 {
                public,
                private,
            })
        }
        PrivateKey::Rsa(rsa) => {
            let primes = rsa.primes();
            let (p, q) = (primes[0].as_ref(), primes[1].as_ref());
            let iqmp = rsa
                .crt_coefficient()
                .ok_or("an Rsa key has no CRT coefficient")?;
            ssh_key::private::KeypairData::Rsa(ssh_key::private::RsaKeypair {
                public: ssh_key::public::RsaPublicKey {
                    e: mpint(&uint_octets(rsa.e()))?,
                    n: mpint(&uint_octets(rsa.n().as_ref()))?,
                },
                private: ssh_key::private::RsaPrivateKey {
                    d: mpint(&uint_octets(rsa.d()))?,
                    iqmp: mpint(&uint_octets(&iqmp))?,
                    p: mpint(&uint_octets(p))?,
                    q: mpint(&uint_octets(q))?,
                },
            })
        }
        PrivateKey::X25519(_) => return Err("an X25519 key has no OpenSsh text".to_owned()),
    };
    let key = ssh_key::PrivateKey::new(data, "").map_err(|error| error.to_string())?;
    let key = if passphrase.is_empty() {
        key
    } else {
        key.encrypt(&mut rand_core::OsRng, passphrase.as_bytes())
            .map_err(|error| error.to_string())?
    };
    key.to_openssh(ssh_key::LineEnding::LF)
        .map(|text| text.to_string())
        .map_err(|error| error.to_string())
}

// Age.

const AGE_RECIPIENT: &str = "age";
const AGE_IDENTITY: &str = "age-secret-key-";

fn bech32_octets(text: &str, hrp: &str, what: &str) -> Result<Zeroizing<Vec<u8>>> {
    let (found, octets) =
        bech32::decode(text.trim()).map_err(|_| format!("the Age text is not an age {what}"))?;
    if found.as_str().to_ascii_lowercase() != hrp || octets.len() != 32 {
        return Err(format!("the Age text is not an age {what}"));
    }
    Ok(Zeroizing::new(octets))
}

fn bech32_text(hrp: &str, octets: &[u8]) -> Result<String> {
    let hrp = bech32::Hrp::parse(hrp).map_err(|error| error.to_string())?;
    bech32::encode::<bech32::Bech32>(hrp, octets).map_err(|error| error.to_string())
}

/// An age recipient's text: `age1…`.
///
/// # Errors
/// The encoder's failure.
pub fn age_recipient(key: &[u8; 32]) -> Result<String> {
    bech32_text(AGE_RECIPIENT, key)
}

/// An age identity's text: `AGE-SECRET-KEY-1…`.
///
/// # Errors
/// The encoder's failure.
pub fn age_identity(key: &[u8; 32]) -> Result<Zeroizing<String>> {
    bech32_text(AGE_IDENTITY, key).map(|text| Zeroizing::new(text.to_ascii_uppercase()))
}

// Reading and writing text.

/// The public key `text` holds in `format`.
///
/// # Errors
/// Names the format and the algorithm, and what was wrong.
pub fn read_public(text: &str, format: KeyFormat, wanted: Algorithm) -> Result<PublicKey> {
    if !format.holds(wanted) {
        return Err(format!(
            "{} text holds no {} key",
            format.name(),
            wanted.name()
        ));
    }
    match format {
        KeyFormat::Pem => {
            let (label, der) = pem_block(text, "the Pem text is not a public key")?;
            if label != "PUBLIC KEY" {
                return Err(format!(
                    "the Pem block is a {label}, and a public key is a PUBLIC KEY"
                ));
            }
            public_from_der(&der, wanted)
        }
        KeyFormat::Jwk => public_from_jwk(&jwk_object(text, wanted)?, wanted),
        KeyFormat::OpenSsh => public_from_openssh(text, wanted),
        KeyFormat::Age => {
            let octets = bech32_octets(text, AGE_RECIPIENT, "recipient")?;
            let mut key = [0_u8; 32];
            key.copy_from_slice(&octets);
            Ok(PublicKey::X25519(key))
        }
    }
}

/// The private key `text` holds in `format`, decrypted with `passphrase`
/// where it is encrypted.
///
/// # Errors
/// Names the format and the algorithm, and says so where the passphrase
/// does not decrypt the key.
pub fn read_private(
    text: &str,
    format: KeyFormat,
    wanted: Algorithm,
    passphrase: &str,
) -> std::result::Result<PrivateKey, ReadError> {
    let invalid = ReadError::Invalid;
    if !format.holds(wanted) {
        return Err(invalid(format!(
            "{} text holds no {} key",
            format.name(),
            wanted.name()
        )));
    }
    match format {
        KeyFormat::Pem => {
            let (label, der) =
                pem_block(text, "the Pem text is not a private key").map_err(invalid)?;
            match label.as_str() {
                "PRIVATE KEY" => private_from_der(&der, wanted).map_err(invalid),
                "ENCRYPTED PRIVATE KEY" => {
                    use pkcs8::der::Decode as _;
                    let encrypted =
                        pkcs8::EncryptedPrivateKeyInfoRef::from_der(&der).map_err(|_| {
                            invalid("the Pem block is not an encrypted PKCS #8 key".to_owned())
                        })?;
                    let plain = encrypted.decrypt(passphrase.as_bytes()).map_err(|_| {
                        ReadError::Passphrase(if passphrase.is_empty() {
                            "the Pem private key is encrypted, and no passphrase is given"
                                .to_owned()
                        } else {
                            "the passphrase does not decrypt the Pem private key".to_owned()
                        })
                    })?;
                    private_from_der(plain.as_bytes(), wanted).map_err(invalid)
                }
                "EC PRIVATE KEY" if wanted == Algorithm::P256 => {
                    p256::SecretKey::from_sec1_der(&der)
                        .map(PrivateKey::P256)
                        .map_err(|_| {
                            invalid("the EC PRIVATE KEY block is not a P-256 key".to_owned())
                        })
                }
                "RSA PRIVATE KEY" if wanted == Algorithm::Rsa => {
                    use rsa::pkcs1::DecodeRsaPrivateKey as _;
                    let key = rsa::PrivateKey::from_pkcs1_der(&der).map_err(|_| {
                        invalid("the RSA PRIVATE KEY block is not an Rsa key".to_owned())
                    })?;
                    rsa_allowed(key.n().bit_width() as usize, key.e()).map_err(invalid)?;
                    Ok(PrivateKey::Rsa(Box::new(key)))
                }
                other => Err(invalid(format!(
                    "the Pem block is a {other}, and a {} private key is a PRIVATE KEY",
                    wanted.name()
                ))),
            }
        }
        KeyFormat::Jwk => jwk_object(text, wanted)
            .and_then(|jwk| private_from_jwk(&jwk, wanted))
            .map_err(invalid),
        KeyFormat::OpenSsh => private_from_openssh(text, wanted, passphrase),
        KeyFormat::Age => {
            let octets = bech32_octets(text, AGE_IDENTITY, "identity").map_err(invalid)?;
            let mut key = Zeroizing::new([0_u8; 32]);
            key.copy_from_slice(&octets);
            Ok(PrivateKey::X25519(key))
        }
    }
}

/// A public key written in `format`.
///
/// # Errors
/// Says so where the format holds no key of the key's algorithm.
pub fn write_public(key: &PublicKey, format: KeyFormat) -> Result<String> {
    let algorithm = key.algorithm();
    if !format.holds(algorithm) {
        return Err(format!(
            "{} text holds no {} key",
            format.name(),
            algorithm.name()
        ));
    }
    match format {
        KeyFormat::Pem => pem("PUBLIC KEY", &to_der(key)?),
        KeyFormat::Jwk => Ok(canonical_object(&public_jwk(key))),
        KeyFormat::OpenSsh => {
            let data = ssh_public_data(key)?;
            ssh_key::PublicKey::new(data, "")
                .to_openssh()
                .map_err(|error| error.to_string())
        }
        KeyFormat::Age => match key {
            PublicKey::X25519(key) => age_recipient(key),
            _ => Err("Age text holds X25519 keys alone".to_owned()),
        },
    }
}

/// The private key material of `key` written in `format`, encrypted under
/// `passphrase` where one is given.
///
/// # Errors
/// Says so where the format holds no key of the key's algorithm, or does
/// not encrypt.
pub fn export_private(key: &PrivateKey, format: KeyFormat, passphrase: &str) -> Result<String> {
    let algorithm = key.algorithm();
    if !format.holds(algorithm) {
        return Err(format!(
            "{} text holds no {} key",
            format.name(),
            algorithm.name()
        ));
    }
    match format {
        KeyFormat::Pem => {
            let der = private_to_der(key)?;
            if passphrase.is_empty() {
                return pem("PRIVATE KEY", &der);
            }
            use pkcs8::der::Decode as _;
            let info =
                pkcs8::PrivateKeyInfoRef::from_der(&der).map_err(|error| error.to_string())?;
            let encrypted = info
                .encrypt_with_rng(
                    &mut rand_core_10::UnwrapErr(getrandom::SysRng),
                    passphrase.as_bytes(),
                )
                .map_err(|error| error.to_string())?;
            pem("ENCRYPTED PRIVATE KEY", encrypted.as_bytes())
        }
        KeyFormat::Jwk => {
            if !passphrase.is_empty() {
                return Err("Jwk text is not encrypted: write it without a passphrase".to_owned());
            }
            let mut members = public_jwk(&key.public());
            match key {
                PrivateKey::Ed25519(key) => members.push(("d", b64url(&key.to_bytes()))),
                PrivateKey::X25519(key) => members.push(("d", b64url(&key[..]))),
                PrivateKey::P256(key) => members.push(("d", b64url(&key.to_bytes()))),
                PrivateKey::Rsa(rsa) => {
                    let primes = rsa.primes();
                    let iqmp = rsa
                        .crt_coefficient()
                        .ok_or("an Rsa key has no CRT coefficient")?;
                    let (p, q) = (primes[0].as_ref(), primes[1].as_ref());
                    let dp = rsa.dp().ok_or("an Rsa key has no CRT exponent")?;
                    let dq = rsa.dq().ok_or("an Rsa key has no CRT exponent")?;
                    members.extend([
                        ("d", b64url(&uint_octets(rsa.d()))),
                        ("dp", b64url(&uint_octets(dp))),
                        ("dq", b64url(&uint_octets(dq))),
                        ("p", b64url(&uint_octets(p))),
                        ("q", b64url(&uint_octets(q))),
                        ("qi", b64url(&uint_octets(&iqmp))),
                    ]);
                }
            }
            members.sort_by_key(|(name, _)| *name);
            Ok(canonical_object(&members))
        }
        KeyFormat::OpenSsh => openssh_private(key, passphrase),
        KeyFormat::Age => {
            if !passphrase.is_empty() {
                return Err(
                    "Age identity text is not encrypted: write it without a passphrase".to_owned(),
                );
            }
            match key {
                PrivateKey::X25519(key) => age_identity(key).map(|text| text.to_string()),
                _ => Err("Age text holds X25519 keys alone".to_owned()),
            }
        }
    }
}

/// A new private key of `algorithm` from the host generator; `bits` for
/// `Rsa`.
///
/// # Errors
/// The generator's failure, or an RSA size outside 2048 to 8192.
pub fn generate(algorithm: Algorithm, bits: i64) -> Result<PrivateKey> {
    let mut seed = Zeroizing::new([0_u8; 32]);
    match algorithm {
        Algorithm::Ed25519 => {
            super::random(&mut seed[..])?;
            Ok(PrivateKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(
                &seed,
            )))
        }
        Algorithm::X25519 => {
            super::random(&mut seed[..])?;
            Ok(PrivateKey::X25519(seed))
        }
        Algorithm::P256 => loop {
            super::random(&mut seed[..])?;
            if let Ok(key) = p256::SecretKey::from_slice(&seed[..]) {
                return Ok(PrivateKey::P256(key));
            }
        },
        Algorithm::Rsa => {
            let bits = usize::try_from(bits).unwrap_or(0);
            if !(2048..=8192).contains(&bits) {
                return Err(format!(
                    "an Rsa key has 2048 to 8192 bits, and {bits} is asked for"
                ));
            }
            rsa::PrivateKey::new(&mut rand_core_10::UnwrapErr(getrandom::SysRng), bits)
                .map(|key| PrivateKey::Rsa(Box::new(key)))
                .map_err(|error| error.to_string())
        }
    }
}

/// Whether a base64 text decodes: a helper for diagnostics that must not
/// quote what they read.
#[must_use]
pub fn is_base64(text: &str) -> bool {
    STANDARD.decode(text).is_ok()
}
