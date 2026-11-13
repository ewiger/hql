//! Keys (HQL-0053 part 2): reading the text other tools write, writing text
//! they read back, thumbprints, refusals, encrypted keys and generation.
//! The fixtures under `tests/fixtures/crypto/` were written by `ssh-keygen`
//! and `openssl`.

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/crypto/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn shown(source: &str) -> String {
    hql::eval(&format!("import crypto\n{source}"))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refused(source: &str) -> String {
    hql::eval(&format!("import crypto\n{source}"))
        .map(|value| panic!("{source} gave {value}"))
        .unwrap_err()
        .message
}

/// The public key a private key file holds equals the one its public file
/// holds, and a key's thumbprint does not depend on the format it was read
/// from.
#[test]
fn every_fixture_reads_and_matches_its_public_half() {
    for (private, private_format, public, public_format, algorithm) in [
        (
            "ed25519.openssh",
            "OpenSsh",
            "ed25519.openssh.pub",
            "OpenSsh",
            "Ed25519",
        ),
        (
            "p256.openssh",
            "OpenSsh",
            "p256.openssh.pub",
            "OpenSsh",
            "P256",
        ),
        (
            "rsa.openssh",
            "OpenSsh",
            "rsa.openssh.pub",
            "OpenSsh",
            "Rsa",
        ),
        ("p256.pkcs8.pem", "Pem", "p256.spki.pem", "Pem", "P256"),
        ("p256.sec1.pem", "Pem", "p256.spki.pem", "Pem", "P256"),
        ("rsa.pkcs8.pem", "Pem", "rsa.spki.pem", "Pem", "Rsa"),
        ("rsa.pkcs1.pem", "Pem", "rsa.spki.pem", "Pem", "Rsa"),
    ] {
        let source = format!(
            "private = read_private_key(file({:?}), {private_format}, {algorithm})\n\
             public = read_public_key(file({:?}), {public_format}, {algorithm})\n\
             [private.public() == public, \
              PublicKey(public.to_text(Jwk), Jwk, {algorithm}).thumbprint() == public.thumbprint(), \
              PublicKey(public.to_der(), {algorithm}) == public, \
              PrivateKey(private.export_private(Pem), Pem, {algorithm}).public() == public, \
              PrivateKey(private.export_private(Jwk), Jwk, {algorithm}).public() == public, \
              PrivateKey(private.export_private(OpenSsh), OpenSsh, {algorithm}).public() == public]",
            fixture(private),
            fixture(public)
        );
        assert_eq!(
            shown(&source),
            "[true, true, true, true, true, true]",
            "{private}"
        );
    }
}

#[test]
fn the_rfc_8037_key_has_its_published_thumbprint() {
    let source = format!(
        "read_public_key(file({:?}), Pem, Ed25519).thumbprint()",
        format!(
            "{}/examples/crypto/keys/rfc8032-test-1.pub.pem",
            env!("CARGO_MANIFEST_DIR")
        )
    );
    assert_eq!(
        shown(&source),
        "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k"
    );
}

#[test]
fn a_key_of_another_algorithm_is_refused_naming_both() {
    let message = refused(&format!(
        "read_public_key(file({:?}), Pem, Rsa)",
        fixture("p256.spki.pem")
    ));
    assert!(
        message.contains("P256") && message.contains("Rsa"),
        "{message}"
    );
}

#[test]
fn an_rsa_key_below_2048_bits_is_refused() {
    let message = refused(&format!(
        "read_private_key(file({:?}), Pem, Rsa)",
        fixture("rsa-1024.pkcs1.pem")
    ));
    assert!(message.contains("2048 to 8192"), "{message}");
}

#[test]
fn encrypted_keys_read_with_the_right_passphrase_and_say_so_for_a_wrong_one() {
    for (file, format, algorithm) in [
        ("rsa.encrypted.pem", "Pem", "Rsa"),
        ("ed25519-encrypted.openssh", "OpenSsh", "Ed25519"),
    ] {
        let path = fixture(file);
        shown(&format!(
            "read_private_key(file({path:?}), {format}, {algorithm}, passphrase = \"correct horse\").public()"
        ));
        let message = refused(&format!(
            "read_private_key(file({path:?}), {format}, {algorithm}, passphrase = \"wrong\")"
        ));
        assert!(message.contains("passphrase does not decrypt"), "{message}");
        assert_eq!(
            shown(&format!(
                "texts : Seq<String> = source(file({path:?}), Text)\n\
                 texts[0] | unwrap_or(\"\") | to_private_key({format}, {algorithm}, passphrase = \"wrong\")"
            )),
            "none"
        );
    }
}

#[test]
fn a_format_that_holds_no_key_of_the_algorithm_is_a_check_error() {
    for source in [
        "PublicKey(\"x\", Age, Ed25519)",
        "PrivateKey(\"x\", OpenSsh, X25519)",
    ] {
        let failed = hql::eval(&format!("import crypto\n{source}")).unwrap_err();
        assert!(failed.message.contains("holds no"), "{}", failed.message);
        assert_eq!(failed.phase, hql::diagnostics::Phase::Check, "{source}");
    }
}

#[test]
fn a_jwk_without_its_members_and_a_point_off_the_curve_are_refused() {
    let message = refused(
        "PublicKey(\"{\\\"kty\\\": \\\"OKP\\\", \\\"crv\\\": \\\"Ed25519\\\"}\", Jwk, Ed25519)",
    );
    assert!(message.contains("`x`"), "{message}");
    let off = "{\\\"kty\\\": \\\"EC\\\", \\\"crv\\\": \\\"P-256\\\", \
               \\\"x\\\": \\\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\\\", \
               \\\"y\\\": \\\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE\\\"}";
    let message = refused(&format!("PublicKey(\"{off}\", Jwk, P256)"));
    assert!(message.contains("curve"), "{message}");
}

#[test]
fn generated_keys_round_trip_through_every_format_they_have() {
    let source = "\
        e = generate_key_pair(Ed25519)\n\
        x = generate_key_pair(X25519)\n\
        p = generate_key_pair(P256)\n\
        [PrivateKey(e.private.export_private(OpenSsh, passphrase = \"pw\"), OpenSsh, Ed25519, passphrase = \"pw\").public() == e.public,\n\
         PrivateKey(p.private.export_private(Pem, passphrase = \"pw\"), Pem, P256, passphrase = \"pw\").public() == p.public,\n\
         PrivateKey(x.private.export_private(Age), Age, X25519).public() == x.public,\n\
         PublicKey(x.public.to_text(Age), Age, X25519) == x.public,\n\
         x.public.to_text(Age) | starts_with(\"age1\"),\n\
         x.private.export_private(Age) | starts_with(\"AGE-SECRET-KEY-1\")]";
    assert_eq!(shown(source), "[true, true, true, true, true, true]");
}

#[test]
fn a_private_key_and_a_pair_print_redacted() {
    let printed = shown(
        "pair = generate_key_pair(Ed25519)\n[str(pair), str(pair.private), str(generate_secret_key(Aes256Gcm))]",
    );
    assert!(printed.contains("\"KeyPair<Ed25519>("), "{printed}");
    assert!(printed.contains("\"PrivateKey<Ed25519>("), "{printed}");
    assert!(
        printed.contains("\"SecretKey<Aes256Gcm>(redacted)\""),
        "{printed}"
    );
}

#[test]
fn a_secret_key_holds_the_length_its_algorithm_takes() {
    assert!(refused("SecretKey(Bytes(\"short\"), Aes256Gcm)").contains("32 octets"));
    shown("SecretKey(Bytes(\"any length\"), HmacSha256)");
}

fn check_refused(source: &str) -> String {
    let failed = hql::check(&format!("import crypto\n{source}")).unwrap_err();
    assert_eq!(failed.phase, hql::diagnostics::Phase::Check, "{source}");
    failed.message
}

#[test]
fn a_secret_reaches_no_encoder() {
    let pair = "pair = generate_key_pair(Ed25519)\n";
    for source in [
        "pair | format(Json)",
        "pair.private | format(Yaml)",
        "[pair] | target(file(\"keys.json\"), Json)",
        "type R { k : PrivateKey<Ed25519> }\n[R(k = pair.private)] | format(Json)",
    ] {
        let message = check_refused(&format!("{pair}{source}"));
        assert!(message.contains("export_private"), "{source}: {message}");
    }
    hql::check(&format!("import crypto\n{pair}pair.public | format(Json)")).unwrap();
}

#[test]
fn a_secret_is_not_compared_with_eq() {
    let message = check_refused("pair = generate_key_pair(Ed25519)\npair.private == pair.private");
    assert!(message.contains("implements no `Eq`"), "{message}");
    shown("pair = generate_key_pair(Ed25519)\npair.public == pair.private.public()");
}

#[test]
fn a_private_key_written_into_the_source_is_refused() {
    for literal in [
        "\"-----BEGIN PRIVATE KEY-----\\nMC4CAQAwBQYDK2VwBCIEIJ1hsZ3v\\n-----END PRIVATE KEY-----\"",
        "\"-----BEGIN OPENSSH PRIVATE KEY-----\"",
        "\"AGE-SECRET-KEY-1QQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQPQ\"",
        "\"{\\\"kty\\\": \\\"OKP\\\", \\\"crv\\\": \\\"Ed25519\\\", \\\"x\\\": \\\"a\\\", \\\"d\\\": \\\"b\\\"}\"",
    ] {
        let message = check_refused(&format!("x = {literal}\nx"));
        assert!(
            message.contains("never lives in a program's source"),
            "{literal}: {message}"
        );
    }
    hql::check("x = \"-----BEGIN PUBLIC KEY-----\"\nx").unwrap();
    hql::check("x = \"AGE-SECRET-KEY-1\"\nx").unwrap();
}
