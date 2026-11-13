//! The module `io.fs` and `io`'s `Path` (HQL-0040): listing a folder,
//! walking a tree in sorted order, matching globs, testing paths, the grants
//! a listing is held to, and the run context's bound on what a listing
//! holds. Every case lists a tree built in a temporary directory.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::{TempDir, tempdir};

/// A tree whose sorted walk differs from the text order of its paths:
/// `a/b.md` before `a-b.md`, although `-` sorts before `/`.
fn tree() -> TempDir {
    let dir = tempdir().unwrap();
    for folder in ["t/a", "t/a/c", "t/z"] {
        fs::create_dir_all(dir.path().join(folder)).unwrap();
    }
    for (file, text) in [
        ("t/a/b.md", "# B\n"),
        ("t/a/c/d.csv", "x\n1\n"),
        ("t/a-b.md", "# AB\n"),
        ("t/top.json", "{}"),
        ("t/z/y.md", "# Y\n"),
        ("t/.hidden.md", "# H\n"),
    ] {
        fs::write(dir.path().join(file), text).unwrap();
    }
    dir
}

fn root(dir: &TempDir) -> String {
    dir.path().join("t").display().to_string()
}

fn empty() -> Vault {
    Vault::empty()
}

/// A program's value as its text, the module imported.
fn value(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(&format!("import io.fs\n{source}"), vault, Mode::Strict);
    assert!(!outcome.failed(), "{source}: {:?}", outcome.diagnostics);
    outcome
        .value
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn refusal(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(&format!("import io.fs\n{source}"), vault, Mode::Strict);
    assert!(outcome.failed(), "{source} ran");
    outcome
        .diagnostics
        .iter()
        .map(|report| report.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of each entry's path below the listed root.
fn below(root: &str, listing: &str) -> String {
    value(
        &format!(
            "folder({root:?}) | {listing} | map(e => e.path.text.trim_prefix({:?}))",
            format!("{root}/")
        ),
        &empty(),
    )
}

#[test]
fn walk_is_the_sorted_walk_and_the_vaults_order() {
    let dir = tree();
    let root = root(&dir);
    assert_eq!(
        below(&root, "walk"),
        "[\".hidden.md\", \"a\", \"a/b.md\", \"a/c\", \"a/c/d.csv\", \"a-b.md\", \"top.json\", \"z\", \"z/y.md\"]"
    );
    // The cards of a vault over the tree are its walk's documents, in the
    // same order; the vault skips hidden entries.
    let vault = vault::load(Path::new(&root)).unwrap();
    let cards = hql::run("vault.cards | map(c => c.path)", &vault, Mode::Strict)
        .value
        .unwrap()
        .to_string();
    assert_eq!(cards, "[\"a/b.md\", \"a-b.md\", \"z/y.md\"]");
    let documents = value(
        &format!(
            "folder({root:?}) | walk \
             | filter(e => e.kind == \"file\" and e.path.extension() == some(\"md\") and not e.name.starts_with(\".\")) \
             | map(e => e.path.text.trim_prefix({:?}))",
            format!("{root}/")
        ),
        &empty(),
    );
    assert_eq!(documents, cards);
}

#[test]
fn entries_lists_one_folder_by_name() {
    let dir = tree();
    let root = root(&dir);
    assert_eq!(
        below(&root, "entries"),
        "[\".hidden.md\", \"a\", \"a-b.md\", \"top.json\", \"z\"]"
    );
}

#[test]
fn an_entry_states_its_kind_and_a_files_size() {
    let dir = tree();
    let root = root(&dir);
    let listed = value(
        &format!("folder({root:?}) | entries | map(e => [e.name, e.kind, str(e.size)])"),
        &empty(),
    );
    assert_eq!(
        listed,
        "[[\".hidden.md\", \"file\", \"4\"], [\"a\", \"folder\", \"none\"], \
         [\"a-b.md\", \"file\", \"5\"], [\"top.json\", \"file\", \"2\"], [\"z\", \"folder\", \"none\"]]"
    );
    // An entry's path is the listed folder's joined with its own, and
    // `file` reads it as written.
    let read = value(
        &format!(
            "folder({root:?}) | glob(Glob(\"**/*.csv\")) | flat_map(e => source(file(e.path), Csv)) | map(r => r.x)"
        ),
        &empty(),
    );
    assert_eq!(read, "[\"1\"]");
}

#[cfg(unix)]
#[test]
fn a_link_is_reported_and_not_followed() {
    let dir = tree();
    let root = root(&dir);
    std::os::unix::fs::symlink(dir.path().join("t/a"), dir.path().join("t/loop")).unwrap();
    std::os::unix::fs::symlink(dir.path().join("t/top.json"), dir.path().join("t/top.link"))
        .unwrap();
    let walked = value(
        &format!(
            "folder({root:?}) | walk | filter(e => e.kind == \"link\") | map(e => [e.name, str(e.size)])"
        ),
        &empty(),
    );
    assert_eq!(walked, "[[\"loop\", \"none\"], [\"top.link\", \"none\"]]");
    let through = value(
        &format!("folder({root:?}) | walk | filter(e => e.path.text.contains(\"loop/\")) | size"),
        &empty(),
    );
    assert_eq!(through, "0");
    let entry = value(
        &format!(
            "Path({:?}).entry() | map(e => e.kind) | unwrap_or(\"?\")",
            format!("{root}/loop")
        ),
        &empty(),
    );
    assert_eq!(entry, "link");
}

#[test]
fn each_glob_form_matches_as_stated_and_glob_is_its_longhand() {
    let dir = tree();
    let root = root(&dir);
    for (pattern, expected) in [
        ("*.md", "[\".hidden.md\", \"a-b.md\"]"),
        (
            "**/*.md",
            "[\".hidden.md\", \"a/b.md\", \"a-b.md\", \"z/y.md\"]",
        ),
        ("a/**", "[\"a\", \"a/b.md\", \"a/c\", \"a/c/d.csv\"]"),
        ("?/*.md", "[\"a/b.md\", \"z/y.md\"]"),
        ("[az]", "[\"a\", \"z\"]"),
        ("[a-c]/[a-c].md", "[\"a/b.md\"]"),
    ] {
        let globbed = below(&root, &format!("glob(Glob({pattern:?}))"));
        assert_eq!(globbed, expected, "{pattern}");
        // glob(f, g) is walk(f) | filter(e => g.matches(relative)).
        let longhand = value(
            &format!(
                "g = Glob({pattern:?})\n\
                 folder({root:?}) | walk \
                 | filter(e => g.matches(Path(e.path.text.trim_prefix({prefix:?})))) \
                 | map(e => e.path.text.trim_prefix({prefix:?}))",
                prefix = format!("{root}/")
            ),
            &empty(),
        );
        assert_eq!(longhand, globbed, "{pattern}");
    }
    // `matches` reads text only: no file needs to be there.
    assert_eq!(
        value(
            "[Glob(\"**/*.csv\").matches(Path(\"x/y/z.csv\")), Glob(\"*.csv\").matches(Path(\"x/z.csv\"))]",
            &empty()
        ),
        "[true, false]"
    );
}

#[test]
fn a_pattern_that_does_not_parse_is_refused_at_its_construction() {
    for (pattern, says) in [
        ("[ab", "a `[` is not closed"),
        ("a**", "`**` stands for whole segments"),
        ("x/[]", "holds no character"),
    ] {
        let refused = refusal(&format!("Glob({pattern:?})"), &empty());
        assert!(refused.contains(says), "{pattern}: {refused}");
        assert!(refused.contains(pattern), "{pattern}: {refused}");
    }
}

#[test]
fn a_missing_path_is_answered_and_a_missing_folder_refused() {
    let dir = tree();
    let root = root(&dir);
    let missing = format!("{root}/nothing");
    assert_eq!(
        value(
            &format!(
                "[str(Path({missing:?}).exists()), str(Path({missing:?}).entry()), str(Path({root:?}).exists())]"
            ),
            &empty()
        ),
        "[\"false\", \"none\", \"true\"]"
    );
    let refused = refusal(&format!("folder({missing:?}) | entries"), &empty());
    assert!(refused.contains(&missing), "{refused}");
    assert!(refused.contains("no folder is there"), "{refused}");
    let file = format!("{root}/top.json");
    let refused = refusal(&format!("folder({file:?}) | walk"), &empty());
    assert!(refused.contains("not a folder"), "{refused}");
    assert!(refused.contains(&file), "{refused}");
}

#[test]
fn a_listing_is_held_once_whatever_changes_on_disk() {
    let dir = tree();
    let root = root(&dir);
    let added = format!("{root}/added.json");
    let program = format!(
        "xs = folder({root:?}) | entries\n\
         before = xs | map(e => e.name)\n\
         written = {{a: 1}} | target(file({added:?}))\n\
         after = xs | map(e => e.name)\n\
         [before == after, (folder({root:?}) | entries | size) == (xs | size) + 1]"
    );
    assert_eq!(value(&program, &empty()), "[true, true]");
}

#[test]
fn a_listing_is_bounded_by_the_run_context() {
    let dir = tree();
    fs::write(
        dir.path().join("hql.toml"),
        "[execution]\nmax_elements = 3\n",
    )
    .unwrap();
    let vault = vault::load(dir.path()).unwrap();
    let root = root(&dir);
    let refused = refusal(&format!("folder({root:?}) | walk"), &vault);
    assert!(refused.contains("max_elements = 3"), "{refused}");
    assert_eq!(
        value(
            &format!("folder({:?}) | entries | size", format!("{root}/a")),
            &vault
        ),
        "2"
    );
}

#[test]
fn path_operations_read_the_text_and_paths_sort_by_segments() {
    let vault = empty();
    assert_eq!(
        value(
            "p = Path(\"data/birds.csv\")\n\
             [p.name(), p.stem(), p.extension() | unwrap_or(\"\"), \
              p.parent() | map(q => q.text) | unwrap_or(\"\"), p.join(\"x\").text, \
              str(Path(\"a\").parent()), str(Path(\"a/.env\").extension())]",
            &vault
        ),
        "[\"birds.csv\", \"birds\", \"csv\", \"data\", \"data/birds.csv/x\", \"none\", \"none\"]"
    );
    assert_eq!(
        value(
            "[Path(\"a-b\"), Path(\"a/b\"), Path(\"a\")] | sort | map(p => p.text)",
            &vault
        ),
        "[\"a\", \"a/b\", \"a-b\"]"
    );
    // `file` of a Path is `file` of its text.
    assert_eq!(
        value(
            "file(Path(\"data\").join(\"b.csv\")).path == file(\"data/b.csv\").path",
            &vault
        ),
        "true"
    );
}

/// Run `hql` in a directory, with standard input closed.
fn hql_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn a_document_lists_only_under_its_own_directory() {
    let dir = tree();
    fs::write(dir.path().join("hql.toml"), "").unwrap();
    fs::write(dir.path().join("t/a/secret.txt"), "s").unwrap();
    let cell = |name: &str, program: &str| {
        fs::write(
            dir.path().join("t/z").join(name),
            format!("```hql\nimport io.fs\n{program}\n```\n"),
        )
        .unwrap();
        hql_in(dir.path(), &["render", &format!("t/z/{name}")])
    };
    let inside = cell("in.md", r#"folder(".") | entries | map(e => e.name)"#);
    assert!(
        inside.status.success(),
        "{}",
        String::from_utf8_lossy(&inside.stderr)
    );
    assert!(String::from_utf8_lossy(&inside.stdout).contains("y.md"));
    for (name, program) in [
        ("up.md", r#"folder("../a") | entries"#),
        ("exists.md", r#"Path("../a/secret.txt").exists()"#),
    ] {
        let output = cell(name, program);
        assert!(!output.status.success(), "{program}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("under its own directory"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("t/a"), dir.path().join("t/z/away")).unwrap();
        let output = cell("link.md", r#"folder("away") | walk"#);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("under its own directory"));
        // Walked from inside, the link is an entry and is not followed.
        let output = cell(
            "walk.md",
            r#"folder(".") | walk | map(e => [e.name, e.kind])"#,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let shown = String::from_utf8_lossy(&output.stdout);
        assert!(shown.contains("[\"away\", \"link\"]"), "{shown}");
        assert!(!shown.contains("secret"), "{shown}");
    }
}

#[test]
fn listing_is_effectful_and_a_path_is_not() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("q.hql"),
        "import io.fs\n\
         fn listing(p: String) : Seq<Entry> = folder(p) | walk\n\
         fn here(p: String) : Bool = Path(p).exists()\n\
         fn named(p: Path) : String = p.name()\n\
         fn pattern(p: String) : Glob = Glob(p)\n\
         1\n",
    )
    .unwrap();
    let output = hql_in(dir.path(), &["check", "q.hql"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let checked = String::from_utf8_lossy(&output.stdout);
    assert!(
        checked.contains("listing is Effectful: it uses `walk`"),
        "{checked}"
    );
    assert!(
        checked.contains("here is Effectful: it uses `exists`"),
        "{checked}"
    );
    assert!(!checked.contains("named is"), "{checked}");
    assert!(!checked.contains("pattern is"), "{checked}");
}

#[test]
fn io_fs_is_imported_and_not_in_the_prelude() {
    let outcome = hql::run("folder(\".\") | entries", &empty(), Mode::Strict);
    assert!(outcome.failed());
    let message = &outcome.diagnostics[0].message;
    assert!(message.contains("import io.fs"), "{message}");
    // Imported under another name, it is reached by that name.
    let outcome = hql::run(
        "import io.fs as files\nfiles.Glob(\"*.md\").pattern",
        &empty(),
        Mode::Strict,
    );
    assert_eq!(
        outcome.value.map(|value| value.to_string()).as_deref(),
        Some("*.md"),
        "{:?}",
        outcome.diagnostics
    );
    // `Path` is `io`'s, in the prelude.
    let outcome = hql::run("Path(\"a/b\").name()", &empty(), Mode::Strict);
    assert_eq!(
        outcome.value.map(|value| value.to_string()).as_deref(),
        Some("b")
    );
}
