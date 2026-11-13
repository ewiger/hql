//! The module `io.env` (HQL-0041 part 5): variables read through the host's
//! grant, the `--arg` pairs of the invocation, and the refusals before
//! evaluation.
//!
//! Every case runs the binary with its environment cleared and the variables
//! it names set, so none depends on the shell that runs the tests. The
//! platform may add a variable of its own to a cleared environment, so a
//! case that lists `vars()` runs under a grant that excludes it.

use std::path::Path;
use std::process::{Command, Output};

/// `hql` run in `directory` with only `variables` in its environment.
fn hql_in(directory: &Path, variables: &[(&str, &str)], arguments: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hql"));
    command
        .current_dir(directory)
        .env_clear()
        .env("HQL_HOME", directory.join(".hql-home"))
        .env("HQL_SYSTEM_CONFIG", directory.join("no-system.toml"))
        .args(arguments);
    for (name, value) in variables {
        command.env(name, value);
    }
    command.output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// The value `program` gives, run on its own with `variables` set.
fn standalone(variables: &[(&str, &str)], extra: &[&str], program: &str) -> String {
    let directory = tempfile::tempdir().unwrap();
    let mut arguments = vec!["--bare"];
    arguments.extend_from_slice(extra);
    let source = format!("import io.env\n{program}");
    arguments.extend(["eval", source.as_str()]);
    let output = hql_in(directory.path(), variables, &arguments);
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output).trim_end().to_owned()
}

/// The value `program` gives in a workspace whose `hql.toml` is `config`.
fn in_workspace(config: &str, variables: &[(&str, &str)], program: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("hql.toml"), config).unwrap();
    let path = directory.path().join("p.hql");
    std::fs::write(&path, format!("import io.env\n{program}\n")).unwrap();
    hql_in(
        directory.path(),
        variables,
        &["run", path.to_str().unwrap()],
    )
}

const VARIABLES: &[(&str, &str)] = &[("APP_TOKEN", "t0k"), ("APP_REGION", "eu"), ("OTHER", "o")];

#[test]
fn standalone_grants_every_variable() {
    assert_eq!(
        standalone(
            VARIABLES,
            &[],
            "[env.var(\"APP_TOKEN\"), env.var(\"OTHER\")]"
        ),
        "[\"t0k\", \"o\"]"
    );
    assert_eq!(standalone(VARIABLES, &[], "env.var(\"MISSING\")"), "none");
    assert_eq!(
        standalone(VARIABLES, &[], "env.vars() | get(\"OTHER\")"),
        "\"o\""
    );
}

#[test]
fn a_withheld_variable_reads_as_an_unset_one() {
    let output = in_workspace(
        "env = [\"APP_*\"]\n",
        VARIABLES,
        "[env.var(\"APP_TOKEN\"), env.var(\"OTHER\"), env.var(\"MISSING\")]",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "[\"t0k\", none, none]\n");
    let output = in_workspace(
        "env = [\"APP_*\"]\n",
        VARIABLES,
        "env.var(\"OTHER\") == env.var(\"MISSING\")",
    );
    assert_eq!(stdout(&output), "true\n");
}

#[test]
fn vars_under_a_narrowed_grant_holds_only_the_matching_names() {
    let output = in_workspace("env = [\"APP_*\", \"OTHER\"]\n", VARIABLES, "vars()");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Map {\"APP_REGION\": \"eu\", \"APP_TOKEN\": \"t0k\", \"OTHER\": \"o\"}\n"
    );
    let exact = in_workspace("env = [\"APP_REGION\"]\n", VARIABLES, "env.vars()");
    assert_eq!(stdout(&exact), "Map {\"APP_REGION\": \"eu\"}\n");
    let none = in_workspace("env = []\n", VARIABLES, "env.vars() | entries | size");
    assert_eq!(stdout(&none), "0\n");
    // A workspace that states no grant grants every variable.
    let open = in_workspace("", VARIABLES, "env.var(\"OTHER\")");
    assert_eq!(stdout(&open), "\"o\"\n");
}

#[test]
fn a_grant_that_is_not_a_list_of_strings_is_refused() {
    let output = in_workspace("env = \"APP_*\"\n", VARIABLES, "1");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("`env` is a list of variable names"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn render_reads_no_variable_and_every_argument() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("hql.toml"), "").unwrap();
    let path = directory.path().join("notes.hmd");
    std::fs::write(
        &path,
        "```hql\nimport io.env\n[env.var(\"OTHER\"), env.arg(\"who\")]\n```\n\n\
         ```hql\nimport io.env\nvars() | entries | size\n```\n",
    )
    .unwrap();
    let output = hql_in(
        directory.path(),
        VARIABLES,
        &["--arg", "who=owl", "render", path.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let rendered = stdout(&output);
    assert!(rendered.contains("- none\n- owl\n"), "{rendered}");
    assert!(rendered.contains("\n0\n"), "{rendered}");
}

#[test]
fn arg_and_args_read_the_invocations_pairs() {
    let extra = [
        "--arg",
        "species=owl",
        "--arg",
        "limit=20",
        "--arg",
        "empty=",
    ];
    assert_eq!(
        standalone(
            &[],
            &extra,
            "[env.arg(\"species\"), env.arg(\"missing\"), env.arg(\"empty\")]"
        ),
        "[\"owl\", none, \"\"]"
    );
    assert_eq!(
        standalone(
            &[],
            &extra,
            "env.arg(\"limit\") | and_then(s => s.to_int()) | unwrap_or(10)"
        ),
        "20"
    );
    assert_eq!(
        standalone(&[], &extra, "env.args()"),
        "Map {\"empty\": \"\", \"limit\": \"20\", \"species\": \"owl\"}"
    );
    assert_eq!(standalone(&[], &[], "args() | entries | size"), "0");
    // An argument is granted where no variable is.
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("hql.toml"), "env = []\n").unwrap();
    let path = directory.path().join("p.hql");
    std::fs::write(&path, "import io.env\nenv.arg(\"who\")\n").unwrap();
    let output = hql_in(
        directory.path(),
        &[],
        &["--arg", "who=me", "run", path.to_str().unwrap()],
    );
    assert_eq!(stdout(&output), "\"me\"\n", "{}", stderr(&output));
}

#[test]
fn a_repeated_or_malformed_argument_is_refused_before_evaluation() {
    let directory = tempfile::tempdir().unwrap();
    // The program would fail if it ran; the invocation is refused first.
    let program = "1 / 0";
    for (arguments, said) in [
        (
            vec!["--arg", "a=1", "--arg", "a=2"],
            "--arg a is given twice",
        ),
        (
            vec!["--arg", "Species=owl"],
            "\"Species\" is not an argument name",
        ),
        (vec!["--arg", "1st=a"], "\"1st\" is not an argument name"),
        (
            vec!["--arg", "no-dash=a"],
            "\"no-dash\" is not an argument name",
        ),
        (vec!["--arg", "nothing"], "\"nothing\" is not name=value"),
    ] {
        let mut arguments = arguments;
        arguments.extend(["eval", program]);
        let output = hql_in(directory.path(), &[], &arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        let error = stderr(&output);
        assert!(error.contains(said), "{arguments:?}: {error}");
        assert!(!error.contains("division"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn a_variable_that_is_not_utf8_reads_as_none() {
    use std::os::unix::ffi::OsStrExt;
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_hql"));
    command
        .current_dir(directory.path())
        .env_clear()
        .env("HQL_HOME", directory.path().join(".hql-home"))
        .env("APP_TEXT", "fine")
        .env("APP_BYTES", std::ffi::OsStr::from_bytes(b"caf\xe9"))
        .args([
            "--bare",
            "eval",
            "import io.env\n[[env.var(\"APP_BYTES\"), env.var(\"APP_TEXT\")], \
             vars() | entries | map(e => some(e.first)) \
             | filter(k => k | unwrap_or(\"\") | starts_with(\"APP_\"))]",
        ]);
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "[[none, \"fine\"], [\"APP_TEXT\"]]\n");
}

#[test]
fn every_operation_is_effectful() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("p.hql");
    std::fs::write(
        &path,
        "import io.env\n\
         fn token(name: String) : Option<String> = env.var(name)\n\
         fn passed(limit: Int) : Map<String, String> = env.args()\n\
         token(\"TOKEN\")\n",
    )
    .unwrap();
    let output = hql_in(directory.path(), &[], &["check", path.to_str().unwrap()]);
    let report = stdout(&output);
    assert!(
        report.contains("token is Effectful: it uses `var`"),
        "{report}"
    );
    assert!(
        report.contains("passed is Effectful: it uses `args`"),
        "{report}"
    );
}

#[test]
fn io_env_is_imported_not_in_the_prelude() {
    let refused = hql::eval("var(\"HOME\")").unwrap_err();
    assert!(
        refused.message.contains("import io.env"),
        "{}",
        refused.message
    );
}
