//! Encryption (HQL-0053 part 4): authenticated encryption under a shared
//! key, and age v1 to X25519 recipients or a passphrase.
//!
//! An AEAD ciphertext is `nonce (12) || ciphertext || tag (16)`, its nonce
//! drawn from the host generator on every call. An age message is the
//! octets the `age` command line reads and writes.

use super::{SecretAlgorithm, SecretKey};
use std::io::{Read, Write};
use std::str::FromStr as _;
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, String>;

const NONCE: usize = 12;
const TAG: usize = 16;

/// The age scrypt work factor `encrypt_with_passphrase` writes.
pub const WORK_FACTOR: u8 = 18;
/// The highest scrypt work factor a decryption spends.
pub const MAX_WORK_FACTOR: u8 = 22;

/// The authenticated encryption of `plaintext` under `key`, `aad` bound to
/// it: a new nonce, the ciphertext, the tag.
///
/// # Errors
/// A MAC key, or the generator's failure.
pub fn encrypt(plaintext: &[u8], key: &SecretKey, aad: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0_u8; NONCE];
    super::random(&mut nonce)?;
    encrypt_with_nonce(plaintext, key, aad, &nonce)
}

/// [`encrypt`] under a nonce the caller gives: for the published vectors,
/// and never for a program.
///
/// # Errors
/// A MAC key, or the cipher's failure.
pub fn encrypt_with_nonce(
    plaintext: &[u8],
    key: &SecretKey,
    aad: &[u8],
    nonce: &[u8; NONCE],
) -> Result<Vec<u8>> {
    use aes_gcm::aead::{Aead as _, KeyInit as _, Payload};
    let payload = Payload {
        msg: plaintext,
        aad,
    };
    let sealed = match key.algorithm {
        SecretAlgorithm::Aes256Gcm => aes_gcm::Aes256Gcm::new_from_slice(&key.octets)
            .map_err(|error| error.to_string())?
            .encrypt(nonce.into(), payload),
        SecretAlgorithm::ChaCha20Poly1305 => {
            chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key.octets)
                .map_err(|error| error.to_string())?
                .encrypt(nonce.into(), payload)
        }
        other => {
            return Err(format!(
                "a {} key authenticates, and `encrypt` takes a Cipher key",
                other.name()
            ));
        }
    }
    .map_err(|_| "the cipher refused the plaintext".to_owned())?;
    Ok([&nonce[..], &sealed].concat())
}

/// The plaintext of `sealed` under `key` and `aad`, or `None` where it is
/// not an encryption of anything under them. No plaintext is given before
/// the tag is checked.
///
/// # Errors
/// A MAC key.
pub fn decrypt(sealed: &[u8], key: &SecretKey, aad: &[u8]) -> Result<Option<Zeroizing<Vec<u8>>>> {
    use aes_gcm::aead::{Aead as _, KeyInit as _, Payload};
    if sealed.len() < NONCE + TAG {
        return Ok(None);
    }
    let (nonce, body) = sealed.split_at(NONCE);
    let nonce: &[u8; NONCE] = nonce
        .try_into()
        .map_err(|_| "a nonce is 12 octets".to_owned())?;
    let payload = Payload { msg: body, aad };
    let opened = match key.algorithm {
        SecretAlgorithm::Aes256Gcm => aes_gcm::Aes256Gcm::new_from_slice(&key.octets)
            .map_err(|error| error.to_string())?
            .decrypt(nonce.into(), payload),
        SecretAlgorithm::ChaCha20Poly1305 => {
            chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key.octets)
                .map_err(|error| error.to_string())?
                .decrypt(nonce.into(), payload)
        }
        other => {
            return Err(format!(
                "a {} key authenticates, and `decrypt` takes a Cipher key",
                other.name()
            ));
        }
    };
    Ok(opened.ok().map(Zeroizing::new))
}

// age.

/// The age recipients of X25519 public keys.
fn recipients(keys: &[[u8; 32]]) -> Result<Vec<age::x25519::Recipient>> {
    keys.iter()
        .map(|key| {
            age::x25519::Recipient::from_str(&super::keys::age_recipient(key)?)
                .map_err(|error| error.to_string())
        })
        .collect()
}

/// The age identity of an X25519 private key.
pub fn identity(key: &[u8; 32]) -> Result<age::x25519::Identity> {
    let text = super::keys::age_identity(key)?;
    age::x25519::Identity::from_str(&text).map_err(|error| error.to_string())
}

/// A writer that age-encrypts what is written to `output` for `keys`; its
/// `finish` writes the last chunk.
///
/// # Errors
/// No recipient, or the output's failure.
pub fn writer_to<W: Write>(keys: &[[u8; 32]], output: W) -> Result<age::stream::StreamWriter<W>> {
    if keys.is_empty() {
        return Err("an age message has at least one recipient, and the list is empty".to_owned());
    }
    let recipients = recipients(keys)?;
    let encryptor = age::Encryptor::with_recipients(
        recipients
            .iter()
            .map(|recipient| recipient as &dyn age::Recipient),
    )
    .map_err(|error| error.to_string())?;
    encryptor
        .wrap_output(output)
        .map_err(|error| error.to_string())
}

/// The age encryption of `plaintext` to `keys`.
///
/// # Errors
/// No recipient.
pub fn encrypt_to(plaintext: &[u8], keys: &[[u8; 32]]) -> Result<Vec<u8>> {
    let mut writer = writer_to(keys, Vec::new())?;
    writer
        .write_all(plaintext)
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())
}

/// Why an age message was not opened.
#[derive(Debug)]
pub enum Refusal {
    /// Not a message to the key, or changed: a program sees `none`.
    Unopened,
    /// The stanza types the header holds, none of which the identity opens.
    NoStanza(Vec<String>),
    /// A scrypt work factor above the bound: an evaluation error.
    Costly(u8),
}

/// A reader of the plaintext of the age message `input` holds for `key`.
/// A chunk is given only once it is authenticated.
///
/// # Errors
/// The refusal, with the stanza types of a header the key does not open.
pub fn reader_with<R: Read>(
    key: &[u8; 32],
    input: R,
) -> std::result::Result<age::stream::StreamReader<R>, Refusal> {
    let identity = identity(key).map_err(|_| Refusal::Unopened)?;
    let decryptor = age::Decryptor::new(input).map_err(|_| Refusal::Unopened)?;
    decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|error| match error {
            age::DecryptError::NoMatchingKeys => Refusal::NoStanza(Vec::new()),
            _ => Refusal::Unopened,
        })
}

/// The plaintext of the age message `sealed` for `key`, or `None`.
#[must_use]
pub fn decrypt_with(sealed: &[u8], key: &[u8; 32]) -> Option<Zeroizing<Vec<u8>>> {
    let mut reader = reader_with(key, sealed).ok()?;
    let mut plaintext = Zeroizing::new(Vec::new());
    reader.read_to_end(&mut plaintext).ok()?;
    Some(plaintext)
}

/// The age encryption of `plaintext` under `passphrase`, by age's scrypt
/// recipient with work factor 18.
///
/// # Errors
/// The writer's failure.
pub fn encrypt_with_passphrase(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    encrypt_with_passphrase_at(plaintext, passphrase, WORK_FACTOR)
}

/// [`encrypt_with_passphrase`] at a work factor the caller gives: for
/// tests, and never for a program.
///
/// # Errors
/// The writer's failure.
pub fn encrypt_with_passphrase_at(
    plaintext: &[u8],
    passphrase: &str,
    work_factor: u8,
) -> Result<Vec<u8>> {
    let mut recipient =
        age::scrypt::Recipient::new(age::secrecy::SecretString::from(passphrase.to_owned()));
    recipient.set_work_factor(work_factor);
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|error| error.to_string())?;
    let mut writer = encryptor
        .wrap_output(Vec::new())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(plaintext)
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())
}

/// The plaintext of the age message `sealed` under `passphrase`, or `None`.
///
/// # Errors
/// A scrypt work factor above 22, which a reader does not spend.
pub fn decrypt_with_passphrase(
    sealed: &[u8],
    passphrase: &str,
) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let mut identity =
        age::scrypt::Identity::new(age::secrecy::SecretString::from(passphrase.to_owned()));
    identity.set_max_work_factor(MAX_WORK_FACTOR);
    let Ok(decryptor) = age::Decryptor::new(sealed) else {
        return Ok(None);
    };
    match decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity)) {
        Ok(mut reader) => {
            let mut plaintext = Zeroizing::new(Vec::new());
            Ok(reader.read_to_end(&mut plaintext).ok().map(|_| plaintext))
        }
        Err(age::DecryptError::ExcessiveWork { required, .. }) => Err(format!(
            "the message asks for scrypt work factor {required}, above the {MAX_WORK_FACTOR} a decryption spends"
        )),
        Err(_) => Ok(None),
    }
}

/// The ASCII armor of an age message.
///
/// # Errors
/// The writer's failure.
pub fn armor(message: &[u8]) -> Result<String> {
    let mut writer =
        age::armor::ArmoredWriter::wrap_output(Vec::new(), age::armor::Format::AsciiArmor)
            .map_err(|error| error.to_string())?;
    writer
        .write_all(message)
        .map_err(|error| error.to_string())?;
    let octets = writer.finish().map_err(|error| error.to_string())?;
    String::from_utf8(octets).map_err(|error| error.to_string())
}

/// The age message an armored text holds, or `None`.
#[must_use]
pub fn unarmor(text: &str) -> Option<Vec<u8>> {
    if !text
        .trim_start()
        .starts_with("-----BEGIN AGE ENCRYPTED FILE-----")
    {
        return None;
    }
    let mut reader = age::armor::ArmoredReader::new(text.as_bytes());
    let mut message = Vec::new();
    reader.read_to_end(&mut message).ok()?;
    Some(message)
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

    /// RFC 8439 section 2.8.2.
    #[test]
    fn rfc_8439_aead_vector() {
        let key = SecretKey {
            algorithm: SecretAlgorithm::ChaCha20Poly1305,
            octets: Zeroizing::new(hex(
                "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
            )),
        };
        let nonce: [u8; 12] = hex("070000004041424344454647").try_into().unwrap();
        let aad = hex("50515253c0c1c2c3c4c5c6c7");
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let sealed = encrypt_with_nonce(plaintext, &key, &aad, &nonce).unwrap();
        assert_eq!(
            &sealed[sealed.len() - 16..],
            &hex("1ae10b594f09e26a7e902ecbd0600691")[..]
        );
        assert_eq!(&sealed[12..16], &hex("d31a8d34")[..]);
        assert_eq!(
            decrypt(&sealed, &key, &aad)
                .unwrap()
                .as_deref()
                .map(Vec::as_slice),
            Some(&plaintext[..])
        );
    }

    /// The NIST GCM test case 14: a zero key, nonce and block.
    #[test]
    fn aes_256_gcm_vector() {
        let key = SecretKey {
            algorithm: SecretAlgorithm::Aes256Gcm,
            octets: Zeroizing::new(vec![0; 32]),
        };
        let sealed = encrypt_with_nonce(&[0; 16], &key, &[], &[0; 12]).unwrap();
        assert_eq!(
            &sealed[12..],
            &hex("cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919")[..]
        );
    }

    #[test]
    fn decrypt_refuses_every_change() {
        let key = SecretKey {
            algorithm: SecretAlgorithm::Aes256Gcm,
            octets: Zeroizing::new(vec![7; 32]),
        };
        let other = SecretKey {
            algorithm: SecretAlgorithm::Aes256Gcm,
            octets: Zeroizing::new(vec![8; 32]),
        };
        let sealed = encrypt(b"secret", &key, b"aad").unwrap();
        assert_ne!(
            sealed,
            encrypt(b"secret", &key, b"aad").unwrap(),
            "a fresh nonce each time"
        );
        for at in [0, 12, sealed.len() - 1] {
            let mut changed = sealed.clone();
            changed[at] ^= 1;
            assert!(
                decrypt(&changed, &key, b"aad").unwrap().is_none(),
                "octet {at}"
            );
        }
        assert!(decrypt(&sealed[..27], &key, b"aad").unwrap().is_none());
        assert!(decrypt(&sealed, &other, b"aad").unwrap().is_none());
        assert!(decrypt(&sealed, &key, b"other").unwrap().is_none());
    }

    #[test]
    fn age_round_trips_to_recipients_and_a_passphrase() {
        let (a, b, c) = ([1_u8; 32], [2_u8; 32], [3_u8; 32]);
        let public = |key: &[u8; 32]| {
            x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*key)).to_bytes()
        };
        let sealed = encrypt_to(b"hello", &[public(&a), public(&b)]).unwrap();
        assert_eq!(
            decrypt_with(&sealed, &a).as_deref().map(Vec::as_slice),
            Some(&b"hello"[..])
        );
        assert_eq!(
            decrypt_with(&sealed, &b).as_deref().map(Vec::as_slice),
            Some(&b"hello"[..])
        );
        assert!(decrypt_with(&sealed, &c).is_none());
        assert!(encrypt_to(b"x", &[]).is_err());
        let text = armor(&sealed).unwrap();
        assert!(text.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"));
        assert_eq!(unarmor(&text), Some(sealed));
        let locked = encrypt_with_passphrase_at(b"hi", "pw", 4).unwrap();
        assert_eq!(
            decrypt_with_passphrase(&locked, "pw")
                .unwrap()
                .as_deref()
                .map(Vec::as_slice),
            Some(&b"hi"[..])
        );
        assert!(decrypt_with_passphrase(&locked, "wrong").unwrap().is_none());
    }

    #[test]
    fn a_work_factor_above_22_is_refused() {
        // The header states the factor; a reader refuses it before it
        // derives anything, so a cheap message with its factor edited is
        // the message a costly one would be.
        let cheap = encrypt_with_passphrase_at(b"x", "pw", 4).unwrap();
        let text = String::from_utf8_lossy(&cheap).into_owned();
        let line = text
            .lines()
            .find(|line| line.starts_with("-> scrypt "))
            .unwrap()
            .to_owned();
        let costly = line.replacen(" 4", " 23", 1);
        let sealed = {
            let at = cheap
                .windows(line.len())
                .position(|window| window == line.as_bytes())
                .unwrap();
            [&cheap[..at], costly.as_bytes(), &cheap[at + line.len()..]].concat()
        };
        assert!(
            decrypt_with_passphrase(&sealed, "pw")
                .unwrap_err()
                .contains("work factor 23")
        );
    }
}
