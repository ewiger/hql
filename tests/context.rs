//! Execution contexts through the CLI process (HQL-0025): the mode, the
//! workspace and the anchor discovery starts from. Every run is isolated
//! from the user's own HQL home and the machine's configuration.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::{TempDir, tempdir};

/// A sandbox: a working tree, an HQL home and a system configuration file,
/// none of them the real ones.
struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            dir: tempdir().unwrap(),
        };
        fs::create_dir_all(sandbox.home()).unwrap();
        fs::create_dir_all(sandbox.tree()).unwrap();
        sandbox
    }

    /// The canonical root, so paths compare as the binary prints them.
    fn root(&self) -> PathBuf {
        fs::canonicalize(self.dir.path()).unwrap()
    }

    fn tree(&self) -> PathBuf {
        self.root().join("tree")
    }

    fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    fn system_config(&self) -> PathBuf {
        self.root().join("etc/hql/config.toml")
    }

    fn write(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn command(&self, cwd: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hql"));
        command
            .current_dir(cwd)
            .args(args)
            .env("HOME", self.home())
            .env("USERPROFILE", self.home())
            .env_remove("HQL_HOME")
            .env("HQL_SYSTEM_CONFIG", self.system_config())
            .env_remove("HQL_PATH")
            .env_remove("HQL_CONTEXT")
            .env_remove("HQL_REPORT")
            .env_remove("HQL_REPORT_LEVEL")
            .env_remove("HQL_BACKEND");
        command
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, args).output().unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// Run and require success, returning standard output.
fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        stdout(output),
        stderr(output)
    );
    stdout(output)
}

/// The value of one `label:` line of `hql context`.
fn field(report: &str, label: &str) -> String {
    report
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{label}:")))
        .unwrap_or_else(|| panic!("no {label} in {report}"))
        .trim()
        .to_owned()
}

#[test]
fn a_directory_outside_every_workspace_runs_bare() {
    let sandbox = Sandbox::new();
    let report = ok(&sandbox.run(&sandbox.tree(), &["context"]));
    assert_eq!(field(&report, "mode"), "bare");
    assert_eq!(field(&report, "workspace"), "none");
    assert_eq!(field(&report, "vault"), "none");
    assert_eq!(
        field(&report, "anchor"),
        sandbox.tree().display().to_string()
    );
    assert_eq!(field(&report, "cwd"), sandbox.tree().display().to_string());
}

#[test]
fn a_program_file_anchors_discovery_wherever_hql_is_run_from() {
    let sandbox = Sandbox::new();
    let vault = sandbox.tree().join("a");
    fs::create_dir_all(vault.join(".hmd")).unwrap();
    let script = vault.join("b/c/script.hql");
    sandbox.write(&script, "cards | count\n");
    sandbox.write(&vault.join("note.hmd"), "# Note\n");
    let elsewhere = sandbox.tree().join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();

    let report = ok(&sandbox.run(&elsewhere, &["context", script.to_str().unwrap()]));
    assert_eq!(field(&report, "mode"), "vault");
    assert_eq!(
        field(&report, "workspace"),
        format!("{} (discovered from the anchor)", vault.display())
    );
    assert_eq!(field(&report, "vault"), vault.display().to_string());
    assert_eq!(
        field(&report, "anchor"),
        vault.join("b/c").display().to_string()
    );
    assert_eq!(field(&report, "cwd"), elsewhere.display().to_string());

    // The program runs in the vault it was found in, not the working
    // directory's.
    assert_eq!(
        ok(&sandbox.run(&elsewhere, &["run", script.to_str().unwrap()])),
        "1\n"
    );
}

#[test]
fn hql_toml_alone_is_a_workspace_and_the_nearest_boundary_wins() {
    let sandbox = Sandbox::new();
    let outer = sandbox.tree();
    fs::create_dir_all(outer.join(".hmd")).unwrap();
    let inner = outer.join("inner");
    sandbox.write(&inner.join("hql.toml"), "");
    sandbox.write(&inner.join("src/program.hql"), "1\n");
    let report = ok(&sandbox.run(&outer, &["context", "inner/src/program.hql"]));
    assert_eq!(field(&report, "mode"), "workspace");
    assert!(
        field(&report, "workspace").starts_with(&inner.display().to_string()),
        "{report}"
    );
    assert_eq!(field(&report, "vault"), "none");
}

#[test]
fn bare_suppresses_discovery_and_conflicts_with_a_named_workspace() {
    let sandbox = Sandbox::new();
    let vault = sandbox.tree();
    fs::create_dir_all(vault.join(".hmd")).unwrap();
    sandbox.write(&vault.join("note.hmd"), "# Note\n");
    let report = ok(&sandbox.run(&vault, &["--bare", "context"]));
    assert_eq!(field(&report, "mode"), "bare");
    // A workspace-dependent operation fails in bare mode rather than reading
    // the working directory as a vault.
    let bare = sandbox.run(&vault, &["--bare", "cards | count"]);
    assert_eq!(bare.status.code(), Some(1));
    assert!(stderr(&bare).contains("needs a vault"), "{}", stderr(&bare));
    assert_eq!(ok(&sandbox.run(&vault, &["cards | count"])), "1\n");

    let conflict = sandbox.run(&vault, &["--bare", "--workspace", ".", "context"]);
    assert_eq!(conflict.status.code(), Some(2), "{}", stderr(&conflict));
}

#[test]
fn an_explicit_workspace_keeps_the_anchor_of_the_file() {
    let sandbox = Sandbox::new();
    let repo = sandbox.tree().join("repo");
    sandbox.write(&repo.join("hql.toml"), "");
    sandbox.write(&repo.join("src/jobs/hql.toml"), "");
    sandbox.write(&repo.join("src/jobs/report.hql"), "1\n");
    let report = ok(&sandbox.run(
        &sandbox.tree(),
        &[
            "--workspace",
            repo.to_str().unwrap(),
            "context",
            repo.join("src/jobs/report.hql").to_str().unwrap(),
        ],
    ));
    assert_eq!(
        field(&report, "workspace"),
        format!("{} (--workspace)", repo.display())
    );
    assert_eq!(
        field(&report, "anchor"),
        repo.join("src/jobs").display().to_string()
    );

    let unmarked = sandbox.run(&sandbox.tree(), &["--workspace", ".", "context"]);
    assert_eq!(unmarked.status.code(), Some(1));
    assert!(
        stderr(&unmarked).contains("not a workspace"),
        "{}",
        stderr(&unmarked)
    );
}

#[test]
fn the_context_is_written_as_json() {
    let sandbox = Sandbox::new();
    let report = ok(&sandbox.run(
        &sandbox.tree(),
        &["context", "--envelope", "json", "--compact"],
    ));
    let document: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(document["mode"], "bare");
    assert_eq!(document["workspace"], serde_json::Value::Null);
    assert_eq!(document["cwd"], sandbox.tree().display().to_string());
    assert_eq!(
        document["hql_home"]["path"],
        sandbox.home().join(".hql").display().to_string()
    );
    assert_eq!(document["config"][0]["layer"], "system configuration");
    assert_eq!(document["config"][0]["present"], false);
}

/// A module file that opens with its declaration.
fn module(name: &str, body: &str) -> String {
    format!("module {name}\n{body}\n")
}

#[test]
fn bare_hql_imports_modules_from_the_hql_home_and_hql_path() {
    let sandbox = Sandbox::new();
    let vault = sandbox.tree();
    fs::create_dir_all(vault.join(".hmd")).unwrap();
    sandbox.write(
        &sandbox.home().join(".hql/modules/greet.hql"),
        &module("greet", "fn one() : Int = 1"),
    );
    let shared = sandbox.root().join("shared");
    sandbox.write(
        &shared.join("twice.hql"),
        &module("twice", "fn of(n: Int) : Int = n * 2"),
    );
    let output = sandbox
        .command(
            &vault,
            &[
                "--bare",
                "import greet\nimport twice\n[greet.one(), twice.of(21)]",
            ],
        )
        .env("HQL_PATH", &shared)
        .output()
        .unwrap();
    assert_eq!(ok(&output), "[1, 42]\n");

    let report = ok(&sandbox
        .command(&vault, &["--bare", "context"])
        .env("HQL_PATH", &shared)
        .output()
        .unwrap());
    assert!(
        report.contains(&format!(
            "  {} (the HQL home)",
            sandbox.home().join(".hql/modules").display()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!("  {} ($HQL_PATH)", shared.display())),
        "{report}"
    );
    assert_eq!(
        field(&report, "hql-home"),
        format!("{} (the default)", sandbox.home().join(".hql").display())
    );
}

#[test]
fn user_configuration_contributes_in_every_mode_and_names_its_files() {
    let sandbox = Sandbox::new();
    let hql_home = sandbox.root().join("custom-home");
    sandbox.write(
        &hql_home.join("config.toml"),
        "[modules]\nroots = [\"lib\"]\n",
    );
    sandbox.write(
        &hql_home.join("lib/units.hql"),
        &module("units", "fn km(m: Int) : Int = m / 1000"),
    );
    let project = sandbox.tree().join("project");
    sandbox.write(&project.join("hql.toml"), "[modules]\nroots = [\"src\"]\n");
    sandbox.write(
        &project.join("src/local.hql"),
        &module("local", "fn base() : Int = 5000"),
    );
    sandbox.write(
        &project.join("jobs/job.hql"),
        "import units\nimport local\nunits.km(local.base())\n",
    );
    let run = |args: &[&str]| {
        sandbox
            .command(&sandbox.tree(), args)
            .env("HQL_HOME", &hql_home)
            .output()
            .unwrap()
    };
    let job = project.join("jobs/job.hql");
    assert_eq!(ok(&run(&["run", job.to_str().unwrap()])), "5\n");

    let report = ok(&run(&["context", job.to_str().unwrap()]));
    assert_eq!(
        field(&report, "hql-home"),
        format!("{} ($HQL_HOME)", hql_home.display())
    );
    for line in [
        format!(
            "  {} (system configuration, absent)",
            sandbox.system_config().display()
        ),
        format!(
            "  {} (user configuration)",
            hql_home.join("config.toml").display()
        ),
        format!(
            "  {} (the workspace's hql.toml)",
            project.join("hql.toml").display()
        ),
        format!("  {} (user configuration)", hql_home.join("lib").display()),
        format!(
            "  {} (the workspace's hql.toml)",
            project.join("src").display()
        ),
    ] {
        assert!(report.contains(&line), "{line}\n{report}");
    }
}

#[test]
fn a_malformed_user_configuration_fails_every_run_naming_the_file() {
    let sandbox = Sandbox::new();
    sandbox.write(&sandbox.home().join(".hql/config.toml"), "[modules\n");
    let output = sandbox.run(&sandbox.tree(), &["1"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains(".hql/config.toml"),
        "{}",
        stderr(&output)
    );
}
