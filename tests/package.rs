//! Packages: a crate built into a library, installed under the HQL home and
//! loaded when a program imports one of its modules (HQL-0057).
//!
//! The fixture is `tests/packages/hello`. It is built once, by `hql package
//! install`, and each test runs the binary against a copy of what that
//! installed, since a process's packages are activated once.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const HQL: &str = env!("CARGO_BIN_EXE_hql");

/// The fixture package's crate.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/packages/hello")
}

/// `hql` under an HQL home, in a directory that is no workspace.
fn hql(home: &Path) -> Command {
    let mut command = Command::new(HQL);
    command
        .env("HQL_HOME", home)
        .env_remove("HQL_PATH")
        .env_remove("HQL_CONTEXT")
        .current_dir(home);
    command
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_owned()
}

/// A home holding the installed fixture, built on the first call.
fn installed() -> &'static Path {
    static HOME: OnceLock<tempfile::TempDir> = OnceLock::new();
    HOME.get_or_init(|| {
        let home = tempfile::tempdir().unwrap();
        let output = hql(home.path())
            .args(["package", "install", "--path"])
            .arg(fixture())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(
            stdout(&output).starts_with("installed hello 0.1.0 (hello) in "),
            "{}",
            stdout(&output)
        );
        home
    })
    .path()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A home of its own holding a copy of the installed fixture.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    copy_tree(installed(), home.path());
    home
}

/// What a program prints under a home.
fn shown(home: &Path, program: &str) -> String {
    let output = hql(home).args(["eval", program]).output().unwrap();
    assert!(output.status.success(), "{program}: {}", stderr(&output));
    stdout(&output)
}

/// What a program is refused with under a home.
fn refused(home: &Path, program: &str) -> String {
    let output = hql(home).args(["eval", program]).output().unwrap();
    assert!(!output.status.success(), "{program}: {}", stdout(&output));
    stderr(&output)
}

/// Rewrite a file of the installed package.
fn edit(home: &Path, file: &str, change: impl FnOnce(String) -> String) {
    let path = home.join("lib/hello").join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, change(text)).unwrap();
}

#[test]
fn an_installed_package_is_a_record_a_module_root_and_a_library() {
    let directory = installed().join("lib/hello");
    assert!(directory.join("package.toml").is_file());
    assert!(directory.join("modules/hello/mod.hql").is_file());
    let record = std::fs::read_to_string(directory.join("package.toml")).unwrap();
    for stated in [
        "name = \"hello\"",
        "crate = \"hql-ext-hello\"",
        "provides = [\"hello\"]",
        "kind = \"path\"",
        concat!("hql = \"", env!("CARGO_PKG_VERSION"), "\""),
        "rustc = \"",
        "library = \"",
    ] {
        assert!(record.contains(stated), "{stated}: {record}");
    }
    let library = std::fs::read_dir(&directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.contains("hql_ext_hello"));
    assert!(library.is_some(), "no library in {}", directory.display());
}

#[test]
fn an_import_reaches_the_functions_the_library_implements() {
    let home = home();
    assert_eq!(
        shown(home.path(), "import hello\nhello.greet(\"Ada\")"),
        "\"Hello, Ada!\""
    );
    // A receiver call, over two forms of one name.
    assert_eq!(
        shown(
            home.path(),
            "import hello\n[\"hi\".shout(), Greeting(\"Ada\").shout()]"
        ),
        "[\"HI\", \"HELLO, ADA!\"]"
    );
    // A function of the program, applied by the running hql.
    assert_eq!(
        shown(home.path(), "import hello\n[1, 2, 3].total(x => x * 10)"),
        "60"
    );
}

#[test]
fn a_type_of_the_package_is_constructed_by_its_own_constructor() {
    let home = home();
    assert_eq!(
        shown(home.path(), "import hello\nGreeting(\"Ada\")"),
        "Greeting(text = \"Hello, Ada!\", length = 11)"
    );
    assert_eq!(
        shown(
            home.path(),
            "import hello\ng = Greeting(\"Ada\", punctuation = \"?\")\n[g.text, str(g.length)]"
        ),
        "[\"Hello, Ada?\", \"11\"]"
    );
}

#[test]
fn a_call_is_checked_against_the_declaration() {
    let home = home();
    let wrong = refused(home.path(), "import hello\nhello.greet(3)");
    assert!(wrong.contains("`greet` needs String, not Int"), "{wrong}");
    let unimported = refused(home.path(), "Greeting(\"Ada\")");
    assert!(
        unimported.contains("`Greeting` is declared in the module `hello`: write `import hello`"),
        "{unimported}"
    );
}

#[test]
fn a_panic_in_the_library_is_a_failure_of_the_call() {
    let home = home();
    let failed = refused(home.path(), "import hello\nhello.fail(\"boom\")");
    assert!(
        failed.contains("the extension function `fail` panicked: boom"),
        "{failed}"
    );
}

#[test]
fn the_commands_list_describe_and_remove_a_package() {
    let home = home();
    let list = hql(home.path()).args(["package", "list"]).output().unwrap();
    assert_eq!(stdout(&list), "hello 0.1.0 (hello)");
    let info = hql(home.path())
        .args(["package", "info", "hello"])
        .output()
        .unwrap();
    let info = stdout(&info);
    assert!(info.starts_with("package: hello\ncrate: hql-ext-hello 0.1.0\nprovides: hello\n"));
    assert!(info.contains(concat!("against hql ", env!("CARGO_PKG_VERSION"))));
    let builtins = hql(home.path()).arg("builtins").output().unwrap();
    assert!(
        stdout(&builtins).contains("hello (hello 0.1.0, imported where it is used)"),
        "builtins does not name the package"
    );
    let removed = hql(home.path())
        .args(["package", "remove", "hello"])
        .output()
        .unwrap();
    assert_eq!(stdout(&removed), "removed hello 0.1.0");
    let gone = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(gone.contains("no module named `hello`"), "{gone}");
    let absent = hql(home.path())
        .args(["package", "info", "hello"])
        .output()
        .unwrap();
    assert!(stderr(&absent).contains("no package `hello` is installed"));
}

#[test]
fn a_package_built_against_another_hql_is_refused_until_rebuilt() {
    let home = home();
    edit(home.path(), "package.toml", |record| {
        record.replace(
            concat!("hql = \"", env!("CARGO_PKG_VERSION"), "\""),
            "hql = \"0.0.1\"",
        )
    });
    let stale = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(
        stale.contains("no module named `hello`: package `hello` was built against hql 0.0.1"),
        "{stale}"
    );
    assert!(stale.contains("run `hql package rebuild hello`"), "{stale}");
    // The rest of the language is as it was.
    assert_eq!(shown(home.path(), "1 + 1"), "2");
    let list = hql(home.path()).args(["package", "list"]).output().unwrap();
    assert!(stdout(&list).contains("run `hql package rebuild hello`"));
    let rebuilt = hql(home.path())
        .args(["package", "rebuild", "hello"])
        .output()
        .unwrap();
    assert!(rebuilt.status.success(), "{}", stderr(&rebuilt));
    assert_eq!(
        shown(home.path(), "import hello\nhello.greet(\"Ada\")"),
        "\"Hello, Ada!\""
    );
}

#[test]
fn a_contract_and_an_extension_that_disagree_refuse_the_package() {
    // A declaration no function implements.
    let home = home();
    edit(home.path(), "modules/hello/mod.hql", |contract| {
        format!("{contract}\nfn missing(name: String) : String\n")
    });
    let undeclared = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(
        undeclared.contains(
            "package `hello` is refused: `hello.missing` is declared, and the extension \
             `hello` implements no such function"
        ),
        "{undeclared}"
    );
    assert_eq!(shown(home.path(), "[1, 2].count()"), "2");
    // A function no declaration states.
    let home = self::home();
    edit(home.path(), "modules/hello/mod.hql", |contract| {
        contract.replace(
            "/// The text greeting a name: `greet(\"Ada\")` is `Hello, Ada!`.\nfn greet(name: \
             String) : String\n",
            "",
        )
    });
    let unimplemented = refused(home.path(), "import hello\nhello.shout(\"a\")");
    assert!(
        unimplemented.contains("package `hello` is refused: ") && unimplemented.contains("greet"),
        "{unimplemented}"
    );
    // A record naming a module the library does not provide.
    let home = self::home();
    edit(home.path(), "package.toml", |record| {
        record.replace(
            "provides = [\"hello\"]",
            "provides = [\"hello\", \"other\"]",
        )
    });
    let other = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(
        other.contains("package `hello` provides `other`, which has no file under"),
        "{other}"
    );
}

#[test]
fn a_module_two_packages_provide_has_no_provider() {
    let home = home();
    let twin = home.path().join("lib/twin");
    copy_tree(&home.path().join("lib/hello"), &twin);
    let record = std::fs::read_to_string(twin.join("package.toml")).unwrap();
    std::fs::write(
        twin.join("package.toml"),
        record.replace("name = \"hello\"", "name = \"twin\""),
    )
    .unwrap();
    let both = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(
        both.contains("both provide `hello`: a module has one provider, so neither is loaded"),
        "{both}"
    );
    let doctor = hql(home.path()).arg("doctor").output().unwrap();
    assert!(
        stdout(&doctor).contains("package  refused packages `hello` and `twin` both provide"),
        "{}",
        stdout(&doctor)
    );
    // The same package found twice is one package.
    std::fs::remove_dir_all(&twin).unwrap();
    let healthy = hql(home.path()).arg("doctor").output().unwrap();
    assert!(
        stdout(&healthy).contains("package  ok      hello 0.1.0 (hello)"),
        "{}",
        stdout(&healthy)
    );
}

#[test]
fn a_file_that_is_no_package_library_is_refused() {
    let home = home();
    let directory = home.path().join("lib/hello");
    let library = std::fs::read_dir(&directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains("hql_ext_hello"))
        })
        .unwrap();
    std::fs::write(&library, b"not a library").unwrap();
    let broken = refused(home.path(), "import hello\nhello.greet(\"Ada\")");
    assert!(broken.contains("no module named `hello`: "), "{broken}");
    assert_eq!(shown(home.path(), "1 + 1"), "2");
}

#[test]
fn a_workspace_lists_the_crate_of_a_package_it_develops() {
    // Built first, so the two builds of the fixture do not run at once.
    let _ = installed();
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("hql.toml"),
        format!(
            "[packages]\npaths = [{:?}]\n",
            fixture().display().to_string()
        ),
    )
    .unwrap();
    let output = hql(home.path())
        .current_dir(workspace.path())
        .args(["eval", "import hello\nhello.greet(\"Ada\")"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "\"Hello, Ada!\"");
    assert!(fixture().join("target/hql/hello/package.toml").is_file());
    // Nothing was installed under the home.
    assert!(!home.path().join("lib").exists());
}

#[test]
fn installing_needs_cargo_and_a_package_crate() {
    let home = tempfile::tempdir().unwrap();
    let without = hql(home.path())
        .env("PATH", "")
        .args(["package", "install", "--path"])
        .arg(fixture())
        .output()
        .unwrap();
    assert!(!without.status.success());
    assert!(
        stderr(&without).contains("needs `cargo`") && stderr(&without).contains("rustup"),
        "{}",
        stderr(&without)
    );
    let plain = tempfile::tempdir().unwrap();
    std::fs::write(
        plain.path().join("Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let no_package = hql(home.path())
        .args(["package", "install", "--path"])
        .arg(plain.path())
        .output()
        .unwrap();
    assert!(
        stderr(&no_package).contains("no `[package.metadata.hql]`"),
        "{}",
        stderr(&no_package)
    );
}

/// A repository of official packages on this machine: the fixture as a
/// crate under `crates/`, the registry at the root, and the tag an
/// official entry pins.
fn repository(requirement: &str) -> tempfile::TempDir {
    let repository = tempfile::tempdir().unwrap();
    let krate = repository.path().join("crates/hql-ext-hello");
    std::fs::create_dir_all(&krate).unwrap();
    for part in ["Cargo.toml", "src", "modules"] {
        let (from, to) = (fixture().join(part), krate.join(part));
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            std::fs::copy(from, to).unwrap();
        }
    }
    std::fs::write(
        repository.path().join("hql-packages.yaml"),
        format!(
            "registry: 1\npackages:\n  - name: hello\n    crate: hql-ext-hello\n    kind: \
             official\n    version: 0.1.0\n    hql: \"{requirement}\"\n    provides: \
             [hello]\n    description: Greetings, as the fixture of the package tests\n"
        ),
    )
    .unwrap();
    let git = |arguments: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository.path())
            .args(["-c", "user.name=hql", "-c", "user.email=hql@example.org"])
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", stderr(&output));
    };
    git(&["init", "--quiet", "--initial-branch", "main"]);
    git(&["add", "."]);
    git(&["commit", "--quiet", "--message", "the hello package"]);
    git(&["tag", "hql-ext-hello-v0.1.0"]);
    repository
}

/// `hql package` under a home, reading a repository's registry.
fn registered(home: &Path, repository: &Path, arguments: &[&str]) -> Output {
    hql(home)
        .env("HQL_REGISTRY", repository.join("hql-packages.yaml"))
        .env("HQL_REGISTRY_REPOSITORY", repository)
        .arg("package")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn a_package_the_registry_lists_installs_by_name() {
    let repository = repository(">=0.7.0, <0.8.0");
    let home = tempfile::tempdir().unwrap();
    let found = registered(home.path(), repository.path(), &["search", "greet"]);
    assert_eq!(
        stdout(&found),
        "hello 0.1.0 (official, hello)\n  Greetings, as the fixture of the package tests"
    );
    let listed = registered(home.path(), repository.path(), &["registry"]);
    assert!(
        stdout(&listed).ends_with("packages: 1"),
        "{}",
        stdout(&listed)
    );
    assert!(home.path().join("registry/hql-packages.yaml").is_file());
    let absent = registered(home.path(), repository.path(), &["install", "absent"]);
    assert!(
        stderr(&absent).contains("the registry lists no package `absent`"),
        "{}",
        stderr(&absent)
    );
    let installed = registered(home.path(), repository.path(), &["install", "hello"]);
    assert!(installed.status.success(), "{}", stderr(&installed));
    assert!(
        stdout(&installed).starts_with("installed hello 0.1.0 (hello) in "),
        "{}",
        stdout(&installed)
    );
    // The source is the tag's, kept under the home for a rebuild.
    assert!(
        home.path()
            .join("src/hello/crates/hql-ext-hello/Cargo.toml")
            .is_file()
    );
    let info = registered(home.path(), repository.path(), &["info", "hello"]);
    assert!(
        stdout(&info).contains("source: hql-ext-hello-v0.1.0 of "),
        "{}",
        stdout(&info)
    );
    assert_eq!(
        shown(home.path(), "import hello\nhello.greet(\"Ada\")"),
        "\"Hello, Ada!\""
    );
    let current = registered(home.path(), repository.path(), &["update"]);
    assert_eq!(stdout(&current), "hello 0.1.0 is the registry's version");
    let rebuilt = registered(home.path(), repository.path(), &["rebuild", "hello"]);
    assert!(rebuilt.status.success(), "{}", stderr(&rebuilt));
    let removed = registered(home.path(), repository.path(), &["remove", "hello"]);
    assert_eq!(stdout(&removed), "removed hello 0.1.0");
    assert!(!home.path().join("src/hello").exists());
}

#[test]
fn an_entry_that_excludes_this_hql_is_refused_before_anything_is_fetched() {
    let repository = repository(">=9.0.0");
    let home = tempfile::tempdir().unwrap();
    let refused = registered(home.path(), repository.path(), &["install", "hello"]);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains(concat!(
            "package `hello` requires hql >=9.0.0; this is hql ",
            env!("CARGO_PKG_VERSION")
        )),
        "{}",
        stderr(&refused)
    );
    assert!(!home.path().join("src").exists());
    assert!(!home.path().join("lib").exists());
}
