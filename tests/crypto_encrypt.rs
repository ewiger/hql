//! Encrypted endpoints (HQL-0053 part 4): `target` and `sink` through
//! `encrypted_to` write an age file `source` and `tap` read back through
//! `decrypted_with`; a truncated file and a changed chunk end a `tap` after
//! the documents of the chunks before it; a file's `.end` is refused.

use std::path::Path;
use std::process::Command;

/// What `hql run` gives for `source`, in `directory`: its output, error
/// stream and whether it succeeded.
fn run(directory: &Path, source: &str) -> (String, String, bool) {
    let program = directory.join("program.hql");
    std::fs::write(&program, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory)
        .args(["--bare", "run", "program.hql"])
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.success(),
    )
}

/// A directory holding a new X25519 identity and its recipient.
fn keys() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let (_, error, ok) = run(
        directory.path(),
        "import crypto\n\
         pair = generate_key_pair(X25519)\n\
         pair.private.export_private(Age) | target(file(\"identity.txt\"), Text)\n\
         pair.public.to_text(Age) | target(file(\"recipient.txt\"), Text)\n",
    );
    assert!(ok, "{error}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            directory.path().join("identity.txt"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    directory
}

const READ: &str = "import crypto\n\
    identity = read_private_key(file(\"identity.txt\"), Age, X25519)\n";

const WRITE: &str = "import crypto\n\
    recipient = read_public_key(file(\"recipient.txt\"), Age, X25519)\n";

#[test]
fn a_sink_of_200000_lines_reads_back_through_source_and_tap() {
    let directory = keys();
    let (_, error, ok) = run(
        directory.path(),
        &format!(
            "{WRITE}range(0, 200000) | map(n => {{n: n}}) \
             | sink(encrypted_to(file(\"lines.jsonl.age\"), [recipient]))\n"
        ),
    );
    assert!(ok, "{error}");
    let sealed = std::fs::read(directory.path().join("lines.jsonl.age")).unwrap();
    assert!(sealed.starts_with(b"age-encryption.org/v1\n-> X25519 "));
    assert!(
        !String::from_utf8_lossy(&sealed).contains("{\"n\": 1"),
        "the lines are encrypted"
    );
    let (output, error, ok) = run(
        directory.path(),
        &format!(
            "{READ}source(decrypted_with(file(\"lines.jsonl.age\"), identity)) \
             | map(o => o.n) | size\n"
        ),
    );
    assert!(ok, "source: {error}");
    assert_eq!(output.trim(), "200000");
    let (output, error, ok) = run(
        directory.path(),
        &format!(
            "{READ}tap(decrypted_with(file(\"lines.jsonl.age\"), identity)) \
             | map(o => o.n) | sink(stdout, Lines)\n"
        ),
    );
    assert!(ok, "tap: {error}");
    assert_eq!(output.lines().count(), 200_000);
    assert_eq!(output.lines().last(), Some("199999"));
}

#[test]
fn a_target_replaces_the_file_with_one_age_file() {
    let directory = keys();
    let (_, error, ok) = run(
        directory.path(),
        &format!(
            "{WRITE}[{{id: 1}}, {{id: 2}}] | target(encrypted_to(file(\"orders.jsonl.age\"), [recipient]))\n"
        ),
    );
    assert!(ok, "{error}");
    let (output, error, ok) = run(
        directory.path(),
        &format!(
            "{READ}source(decrypted_with(file(\"orders.jsonl.age\"), identity)) | map(o => o.id)\n"
        ),
    );
    assert!(ok, "{error}");
    assert_eq!(output.trim(), "[1, 2]");
}

/// A file of five chunks, cut at a chunk boundary and changed in its third
/// chunk: `tap` gives the documents of the chunks before the fault, then
/// fails naming the endpoint and the chunk's octet offset; `source` gives
/// none.
#[test]
fn a_changed_or_truncated_file_ends_tap_at_the_chunk_that_fails() {
    let directory = keys();
    // Each line is 32 octets, so a 64 KiB chunk holds 2048 of them.
    let (_, error, ok) = run(
        directory.path(),
        &format!(
            "{WRITE}range(0, 10000) | map(n => fmt(\"{{}}\", str(1000000000000000000 + n) + \"-padding-to-32\")) \
             | sink(encrypted_to(file(\"lines.txt.age\"), [recipient]), Lines)\n"
        ),
    );
    assert!(ok, "{error}");
    let path = directory.path().join("lines.txt.age");
    let sealed = std::fs::read(&path).unwrap();
    let header = sealed
        .windows(4)
        .position(|window| window == b"\n---")
        .unwrap();
    let header = header
        + sealed[header + 1..]
            .iter()
            .position(|octet| *octet == b'\n')
            .unwrap()
        + 2;
    let payload = header + 16;
    let chunk = 65_536 + 16;
    let tapped = |file: &str| {
        run(
            directory.path(),
            &format!(
                "{READ}tap(decrypted_with(file({file:?}), identity), Lines) | sink(stdout, Lines)\n"
            ),
        )
    };
    let mut changed = sealed.clone();
    changed[payload + 2 * chunk + 100] ^= 1;
    std::fs::write(directory.path().join("changed.txt.age"), &changed).unwrap();
    // Each line and its break are 34 octets: the lines wholly inside the
    // first two chunks are given, and none after.
    let before = (2 * 65_536) / 34;
    let (output, error, ok) = tapped("changed.txt.age");
    assert!(!ok);
    assert_eq!(output.lines().count(), before, "{error}");
    assert!(error.contains("changed.txt.age"), "{error}");
    assert!(
        error.contains(&format!("octet offset {}", payload + 2 * chunk)),
        "{error}"
    );
    std::fs::write(
        directory.path().join("cut.txt.age"),
        &sealed[..payload + 2 * chunk],
    )
    .unwrap();
    let (output, error, ok) = tapped("cut.txt.age");
    assert!(!ok);
    assert_eq!(output.lines().count(), before, "{error}");
    assert!(error.contains("final chunk"), "{error}");
    let (output, error, ok) = run(
        directory.path(),
        &format!(
            "{READ}source(decrypted_with(file(\"changed.txt.age\"), identity), Lines) | map(l => 1)\n"
        ),
    );
    assert!(
        !ok && output.is_empty(),
        "source gives no document: {output}"
    );
    assert!(error.contains("does not authenticate"), "{error}");
}

#[test]
fn an_identity_that_opens_no_stanza_names_the_stanza_types() {
    let directory = keys();
    let other = keys();
    let (_, error, ok) = run(
        directory.path(),
        &format!("{WRITE}[{{id: 1}}] | target(encrypted_to(file(\"a.jsonl.age\"), [recipient]))\n"),
    );
    assert!(ok, "{error}");
    std::fs::copy(
        other.path().join("identity.txt"),
        directory.path().join("other.txt"),
    )
    .unwrap();
    let (_, error, ok) = run(
        directory.path(),
        "import crypto\n\
         other = read_private_key(file(\"other.txt\"), Age, X25519)\n\
         source(decrypted_with(file(\"a.jsonl.age\"), other))\n",
    );
    assert!(!ok);
    assert!(
        error.contains("opens no stanza") && error.contains("X25519"),
        "{error}"
    );
    assert!(
        !error.contains("AGE-SECRET-KEY"),
        "no key in an error: {error}"
    );
}

#[test]
fn encrypting_to_a_files_end_is_a_check_error() {
    let failed = hql::check(
        "import crypto\n\
         pair = generate_key_pair(X25519)\n\
         [1] | sink(encrypted_to(file(\"x.jsonl.age\").end, [pair.public]))",
    )
    .unwrap_err();
    assert_eq!(failed.phase, hql::diagnostics::Phase::Check);
    assert!(
        failed.message.contains("nothing is appended"),
        "{}",
        failed.message
    );
}

#[test]
fn a_rendered_document_draws_nothing_from_the_host_generator() {
    let vault = tempfile::tempdir().unwrap();
    std::fs::create_dir(vault.path().join(".hmd")).unwrap();
    let document = vault.path().join("keys.md");
    std::fs::write(
        &document,
        "# Keys\n\n```hql\nimport crypto\ngenerate_key_pair(Ed25519).public.thumbprint()\n```\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["render", "--vault"])
        .arg(vault.path())
        .arg(&document)
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(error.contains("draws from the host's generator"), "{error}");
}
