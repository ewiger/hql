//! Signatures, MACs and password hashes (HQL-0053 part 3): signatures that
//! `openssl dgst -sha256 -sign` wrote verify here, `sign_data` reads the
//! canonical JSON of a tree, and `verify` gives `false` rather than failing.

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/crypto/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn shown(source: &str) -> String {
    hql::eval(&format!("import crypto\nimport encoding\n{source}"))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refused(source: &str) -> String {
    hql::eval(&format!("import crypto\nimport encoding\n{source}"))
        .map(|value| panic!("{source} gave {value}"))
        .unwrap_err()
        .message
}

#[test]
fn signatures_openssl_wrote_verify() {
    let hex = |name: &str| std::fs::read_to_string(fixture(name)).unwrap();
    let rsa = format!(
        "key = read_public_key(file({:?}), Pem, Rsa)\n\
         ms : Seq<String> = source(file({:?}), Text)\n\
         m = ms[0] | unwrap_or(\"\")\n\
         [m.verify(Bytes({:?}, Hex), key), (m + \"!\").verify(Bytes({:?}, Hex), key)]",
        fixture("rsa.spki.pem"),
        fixture("message.txt"),
        hex("message.rsa.sig.hex"),
        hex("message.rsa.sig.hex"),
    );
    assert_eq!(shown(&rsa), "[true, false]");
    let p256 = format!(
        "key = read_public_key(file({:?}), Pem, P256)\n\
         ms : Seq<String> = source(file({:?}), Text)\n\
         m = ms[0] | unwrap_or(\"\")\n\
         s = Bytes({:?}, Hex).p256_signature_from_der() | unwrap_or(Bytes(\"\"))\n\
         [s.size() == 64, m.verify(s, key), m.verify(Bytes({:?}, Hex), key)]",
        fixture("p256.spki.pem"),
        fixture("message.txt"),
        hex("message.p256.der.sig.hex"),
        hex("message.p256.der.sig.hex"),
    );
    assert_eq!(
        shown(&p256),
        "[true, true, false]",
        "DER is converted first"
    );
}

#[test]
fn each_algorithm_signs_and_verifies_its_own_signature_alone() {
    for (file, format, algorithm) in [
        ("ed25519.openssh", "OpenSsh", "Ed25519"),
        ("p256.pkcs8.pem", "Pem", "P256"),
        ("rsa.pkcs8.pem", "Pem", "Rsa"),
    ] {
        let source = format!(
            "k = read_private_key(file({:?}), {format}, {algorithm})\n\
             other = generate_key_pair({algorithm}, bits = 2048).public\n\
             s = \"m\".sign(k)\n\
             [\"m\".verify(s, k.public()), \"n\".verify(s, k.public()), \"m\".verify(s, other), \
              \"m\".verify(s[0:10], k.public()), \"m\".sign(k) == s]",
            fixture(file)
        );
        assert_eq!(
            shown(&source),
            "[true, false, false, false, true]",
            "{algorithm}"
        );
    }
}

#[test]
fn sign_data_reads_the_canonical_json_of_a_tree() {
    let source = "\
        k = generate_key_pair(Ed25519)\n\
        a = Data(\"{\\\"b\\\": 1.10, \\\"a\\\": [1, \\\"x\\\"]}\", Json)\n\
        b = Data(\"{\\\"a\\\": [1.0, \\\"x\\\"], \\\"b\\\": 1.1}\", Json)\n\
        c = Data(\"{\\\"a\\\": [2, \\\"x\\\"], \\\"b\\\": 1.1}\", Json)\n\
        s = a.sign_data(k.private)\n\
        [s == b.sign_data(k.private), b.verify_data(s, k.public), c.verify_data(s, k.public),\n\
         canonical_text(a).verify(s, k.public)]";
    let source = source.replace(
        "canonical_text(a)",
        "\"{\\\"a\\\":[1,\\\"x\\\"],\\\"b\\\":1.1}\"",
    );
    assert_eq!(shown(&source), "[true, true, false, true]");
}

#[test]
fn a_mac_key_of_any_length_and_a_cipher_key_refused() {
    assert_eq!(
        shown(
            "k = SecretKey(Bytes(\"a webhook secret of the sender's length\"), HmacSha512)\n\"body\".mac(k).size()"
        ),
        "64"
    );
    assert!(refused("hash_password(\"x\", memory_kib = 4096)").contains("memory_kib"));
}
