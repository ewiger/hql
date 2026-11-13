//! Signatures, MACs and password hashes (HQL-0053 part 3).
//!
//! A signature is detached octets: Ed25519's 64 (R || S), P-256's 64
//! (r || s, IEEE P1363), RSA's modulus length (PKCS #1 v1.5, SHA-256).
//! Every scheme signs deterministically, and `verify` gives `false`, never
//! an error, for a signature that is malformed or of another key.

use super::{PrivateKey, PublicKey, SecretAlgorithm, SecretKey};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, String>;

/// The signature of `message` by `key`.
///
/// # Errors
/// An `X25519` key, which does not sign, or the signer's failure.
pub fn sign(message: &[u8], key: &PrivateKey) -> Result<Vec<u8>> {
    use signature::Signer as _;
    match key {
        PrivateKey::Ed25519(key) => Ok(key.sign(message).to_bytes().to_vec()),
        PrivateKey::P256(key) => {
            let signer = p256::ecdsa::SigningKey::from(key);
            let signature: p256::ecdsa::Signature = signer.sign(message);
            // The low-s form, as FIPS 186-5 allows either and verifiers
            // that insist take this one.
            let signature = signature.normalize_s();
            Ok(signature.to_bytes().to_vec())
        }
        PrivateKey::Rsa(key) => {
            let signer = rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(key.as_ref().clone());
            let signature: rsa::pkcs1v15::Signature = signer
                .try_sign(message)
                .map_err(|error| format!("the Rsa signer failed: {error}"))?;
            Ok(signature_octets(&signature))
        }
        PrivateKey::X25519(_) => Err("an X25519 key does not sign".to_owned()),
    }
}

fn signature_octets(signature: &rsa::pkcs1v15::Signature) -> Vec<u8> {
    use signature::SignatureEncoding as _;
    signature.to_vec()
}

/// Whether `signature` is `key`'s signature of `message`. Ed25519 checks
/// RFC 8032 strictly: non-canonical signatures and small-order keys fail.
#[must_use]
pub fn verify(message: &[u8], signature: &[u8], key: &PublicKey) -> bool {
    use signature::Verifier as _;
    match key {
        PublicKey::Ed25519(key) => ed25519_dalek::Signature::from_slice(signature)
            .is_ok_and(|signature| key.verify_strict(message, &signature).is_ok()),
        PublicKey::P256(key) => {
            p256::ecdsa::Signature::from_slice(signature).is_ok_and(|signature| {
                p256::ecdsa::VerifyingKey::from(key)
                    .verify(message, &signature)
                    .is_ok()
            })
        }
        PublicKey::Rsa(key) => {
            rsa::pkcs1v15::Signature::try_from(signature).is_ok_and(|signature| {
                rsa::pkcs1v15::VerifyingKey::<sha2::Sha256>::new(key.as_ref().clone())
                    .verify(message, &signature)
                    .is_ok()
            })
        }
        PublicKey::X25519(_) => false,
    }
}

/// The `r || s` form of an ASN.1 DER ECDSA P-256 signature.
#[must_use]
pub fn p256_from_der(der: &[u8]) -> Option<Vec<u8>> {
    p256::ecdsa::Signature::from_der(der)
        .ok()
        .map(|signature| signature.to_bytes().to_vec())
}

/// The HMAC tag of `message` under `key`.
///
/// # Errors
/// A key of a cipher, which is no MAC key.
pub fn mac(message: &[u8], key: &SecretKey) -> Result<Vec<u8>> {
    use hmac::{KeyInit as _, Mac as _};
    match key.algorithm {
        SecretAlgorithm::HmacSha256 => {
            let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&key.octets)
                .map_err(|error| error.to_string())?;
            mac.update(message);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        SecretAlgorithm::HmacSha512 => {
            let mut mac = hmac::Hmac::<sha2::Sha512>::new_from_slice(&key.octets)
                .map_err(|error| error.to_string())?;
            mac.update(message);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        other => Err(format!(
            "a {} key encrypts, and a MAC takes a Mac key",
            other.name()
        )),
    }
}

/// Whether `tag` is the HMAC tag of `message` under `key`, compared in time
/// independent of where it first differs.
#[must_use]
pub fn verify_mac(message: &[u8], tag: &[u8], key: &SecretKey) -> bool {
    mac(message, key).is_ok_and(|expected| super::constant_time_eq(&expected, tag))
}

/// The lowest costs `hash_password` takes: OWASP's for Argon2id.
pub const MINIMUM_MEMORY_KIB: i64 = 19_456;
const MAXIMUM_MEMORY_KIB: i64 = 1_048_576;

/// The Argon2id hash of `password` under a new 16-octet salt, as a PHC
/// string.
///
/// # Errors
/// Names the parameter outside its range, or the generator's failure.
pub fn hash_password(
    password: &str,
    memory_kib: i64,
    iterations: i64,
    parallelism: i64,
) -> Result<String> {
    if !(MINIMUM_MEMORY_KIB..=MAXIMUM_MEMORY_KIB).contains(&memory_kib) {
        return Err(format!(
            "`memory_kib` is {MINIMUM_MEMORY_KIB} to {MAXIMUM_MEMORY_KIB}, and {memory_kib} is outside: a cost below OWASP's minimum is refused"
        ));
    }
    if iterations < 2 {
        return Err(format!(
            "`iterations` is at least 2, and {iterations} is below"
        ));
    }
    if !(1..=16).contains(&parallelism) {
        return Err(format!(
            "`parallelism` is 1 to 16, and {parallelism} is outside"
        ));
    }
    let to_u32 = |value: i64| u32::try_from(value).unwrap_or(u32::MAX);
    let params = argon2::Params::new(
        to_u32(memory_kib),
        to_u32(iterations),
        to_u32(parallelism),
        Some(32),
    )
    .map_err(|error| error.to_string())?;
    let hasher = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut salt = [0_u8; 16];
    super::random(&mut salt)?;
    let mut hash = Zeroizing::new([0_u8; 32]);
    hasher
        .hash_password_into(password.as_bytes(), &salt, &mut hash[..])
        .map_err(|error| error.to_string())?;
    let b64 = |octets: &[u8]| {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(octets)
    };
    Ok(format!(
        "$argon2id$v=19$m={memory_kib},t={iterations},p={parallelism}${}${}",
        b64(&salt),
        b64(&hash[..])
    ))
}

/// Whether `password` is the one `hash` was made from, under the hash's own
/// parameters.
///
/// # Errors
/// A `hash` that is not a PHC string of an Argon2 variant, naming the
/// algorithm identifier it holds.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let identifier = hash
        .strip_prefix('$')
        .and_then(|rest| rest.split('$').next())
        .unwrap_or("");
    let algorithm = match identifier {
        "argon2id" => argon2::Algorithm::Argon2id,
        "argon2i" => argon2::Algorithm::Argon2i,
        "argon2d" => argon2::Algorithm::Argon2d,
        "" => return Err("the hash is not a PHC string".to_owned()),
        other => {
            return Err(format!(
                "the hash is a `{other}` PHC string, and `verify_password` reads Argon2id, Argon2i and Argon2d"
            ));
        }
    };
    let fields: Vec<&str> = hash.split('$').collect();
    // `$id$v=19$m=…,t=…,p=…$salt$hash`
    let [_, _, version, params, salt, expected] = fields.as_slice() else {
        return Err(format!(
            "the `{identifier}` hash is not a PHC string of five fields"
        ));
    };
    let version = match *version {
        "v=19" => argon2::Version::V0x13,
        "v=16" => argon2::Version::V0x10,
        other => {
            return Err(format!(
                "the `{identifier}` hash has version `{other}`, and Argon2 has v=16 and v=19"
            ));
        }
    };
    let mut memory = None;
    let mut time = None;
    let mut lanes = None;
    for pair in params.split(',') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value: Option<u32> = value.parse().ok();
        match name {
            "m" => memory = value,
            "t" => time = value,
            "p" => lanes = value,
            _ => {}
        }
    }
    let (Some(memory), Some(time), Some(lanes)) = (memory, time, lanes) else {
        return Err(format!("the `{identifier}` hash does not state m, t and p"));
    };
    if i64::from(memory) > MAXIMUM_MEMORY_KIB {
        return Err(format!(
            "the hash asks for {memory} KiB, above the 1 GiB `verify_password` spends"
        ));
    }
    let b64 = |text: &str| {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD_NO_PAD.decode(text.trim_end_matches('='))
    };
    let (Ok(salt), Ok(expected)) = (b64(salt), b64(expected)) else {
        return Err(format!(
            "the `{identifier}` hash's salt or output is not base64"
        ));
    };
    let params = argon2::Params::new(memory, time, lanes, Some(expected.len()))
        .map_err(|error| format!("the `{identifier}` hash's parameters are not valid: {error}"))?;
    let mut computed = Zeroizing::new(vec![0_u8; expected.len()]);
    argon2::Argon2::new(algorithm, version, params)
        .hash_password_into(password.as_bytes(), &salt, &mut computed)
        .map_err(|error| error.to_string())?;
    Ok(super::constant_time_eq(&computed, &expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    /// RFC 8032 section 7.1, test 1: the empty message.
    #[test]
    fn rfc_8032_test_1() {
        let seed: [u8; 32] =
            hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .try_into()
                .unwrap();
        let key = PrivateKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&seed));
        let signature = sign(b"", &key).unwrap();
        assert_eq!(
            signature,
            hex(
                "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
            )
        );
        assert!(verify(b"", &signature, &key.public()));
        assert!(!verify(b"x", &signature, &key.public()));
        assert!(!verify(b"", &signature[..63], &key.public()));
    }

    /// RFC 8032 section 7.1, test 2: one octet.
    #[test]
    fn rfc_8032_test_2() {
        let seed: [u8; 32] =
            hex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
                .try_into()
                .unwrap();
        let key = PrivateKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&seed));
        assert_eq!(
            sign(&[0x72], &key).unwrap(),
            hex(
                "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
            )
        );
    }

    /// RFC 6979 A.2.5: P-256 with SHA-256, the message "sample".
    #[test]
    fn rfc_6979_p256_sha256() {
        let scalar = hex("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721");
        let key = PrivateKey::P256(p256::SecretKey::from_slice(&scalar).unwrap());
        let signature = sign(b"sample", &key).unwrap();
        let r = "efd48b2aacb6a8fd1140dd9cd45e81d69d2c877b56aaf991c34d0ea84eaf3716";
        let s = "f7cb1c942d657c41d436c7a1b6e29f65f3e900dbb9aff4064dc4ab2f843acda8";
        // The RFC's s is above n/2; the signature is written in the low-s form.
        assert_eq!(&signature[..32], &hex(r)[..]);
        assert_ne!(&signature[32..], &hex(s)[..]);
        assert!(verify(b"sample", &signature, &key.public()));
        let high = [hex(r), hex(s)].concat();
        assert!(
            verify(b"sample", &high, &key.public()),
            "either form verifies"
        );
    }

    /// RFC 4231 test case 2.
    #[test]
    fn rfc_4231_case_2() {
        for (algorithm, tag) in [
            (
                SecretAlgorithm::HmacSha256,
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            ),
            (
                SecretAlgorithm::HmacSha512,
                "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
            ),
        ] {
            let key = SecretKey {
                algorithm,
                octets: Zeroizing::new(b"Jefe".to_vec()),
            };
            let message = b"what do ya want for nothing?";
            assert_eq!(mac(message, &key).unwrap(), hex(tag));
            assert!(verify_mac(message, &hex(tag), &key));
            assert!(!verify_mac(message, &hex(tag)[..31], &key));
        }
    }

    #[test]
    fn a_password_hash_verifies_its_password_alone() {
        let hash = hash_password("hunter2", MINIMUM_MEMORY_KIB, 2, 1).unwrap();
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        assert_eq!(verify_password("hunter2", &hash), Ok(true));
        assert_eq!(verify_password("hunter3", &hash), Ok(false));
        assert!(
            hash_password("x", 4096, 2, 1)
                .unwrap_err()
                .contains("memory_kib")
        );
        assert!(
            verify_password(
                "x",
                "$2b$12$abcdefghijklmnopqrstuuABCDEFGHIJKLMNOPQRSTUVWXYZ01234"
            )
            .unwrap_err()
            .contains("`2b`")
        );
    }

    /// The PHC string of the reference implementation's README:
    /// `echo -n "password" | ./argon2 somesalt -t 2 -m 16 -p 4 -l 24`.
    #[test]
    fn the_reference_implementations_argon2i_string_verifies() {
        let hash = "$argon2i$v=19$m=65536,t=2,p=4$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG";
        assert_eq!(verify_password("password", hash), Ok(true));
        assert_eq!(verify_password("passw0rd", hash), Ok(false));
    }
}
