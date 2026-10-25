//! Exercise the actual CLI process: I/O, exit codes, formats and modes.

use std::io::Write;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const VAULT: &str = "tests/fixtures/vault";
/// A vault that declines the prelude and configures its own import.
const CONFIGURED: &str = "tests/fixtures/configured";

fn hql(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(args)
        .output()
        .unwrap()
}

/// Run `hql` from a working directory, so the vault it finds is that one's.
fn hql_in(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn piped(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn eval_prints_values() {
    for (expression, expected) in [
        ("40 + 2", "42\n"),
        ("true", "true\n"),
        ("-1 + 2", "1\n"),
        ("0.5 + 0.25", "0.75\n"),
        ("1.0 + 2.0", "3.0\n"),
        ("answer = 40 + 2\nanswer", "42\n"),
    ] {
        let output = hql(&["eval", expression]);
        assert!(output.status.success(), "{expression}: {}", stderr(&output));
        assert_eq!(stdout(&output), expected, "{expression}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn expression_failures_use_stderr_and_exit_one() {
    for (source, message) in [
        ("1 +", "parse failure[unexpected-token]"),
        ("true + 1", "addition requires"),
        ("9223372036854775807 + 1", "numeric overflow"),
        ("1 + 2.0f", "addition requires"),
        ("nosuchname", "is not bound"),
    ] {
        let output = hql(&["eval", source]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(output.stdout.is_empty(), "{source}");
        assert!(
            stderr(&output).contains(message),
            "{source}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn a_diagnostic_points_at_a_line_a_column_and_the_source() {
    let output = hql(&["eval", "1 +\n  true + 1"]);
    let reported = stderr(&output);
    assert!(reported.contains("<expression>:2:3"), "{reported}");
    assert!(reported.contains("^^^^"), "{reported}");
    assert!(reported.contains("  true + 1"), "{reported}");
}

#[test]
fn check_reads_files_without_evaluation_and_reports_errors() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("example.hql");
    let name = path.to_str().unwrap();
    std::fs::write(&path, "9223372036854775807 + 1\n").unwrap();
    let output = hql(&["check", name]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Int\n");

    for bytes in [b"true + 1".as_slice(), b"1 +", b"\xff"] {
        std::fs::write(&path, bytes).unwrap();
        let output = hql(&["check", name]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
    let missing = directory.path().join("missing.hql");
    let output = hql(&["check", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("missing.hql"));
}

#[test]
fn run_evaluates_a_file_and_dash_reads_standard_input() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("program.hql");
    std::fs::write(&path, "a = 2\nb = 40\na + b\n").unwrap();
    let output = hql(&["run", path.to_str().unwrap()]);
    assert_eq!(stdout(&output), "42\n");

    assert_eq!(stdout(&piped(&["run", "-"], "1 + 1\n")), "2\n");
    assert_eq!(stdout(&piped(&["check", "-"], "1 + 1\n")), "Int\n");
}

#[test]
fn json_carries_the_type_the_value_and_the_diagnostics() {
    let output = hql(&["--envelope", "json", "eval", "40 + 2"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(parsed["type"], "Int");
    assert_eq!(parsed["value"], 42);
    assert_eq!(parsed["diagnostics"].as_array().unwrap().len(), 0);

    let output = hql(&["--envelope", "json", "eval", "nosuchname"]);
    assert_eq!(output.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(parsed["value"].is_null());
    let diagnostics = parsed["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics[0]["severity"], "error");
    assert_eq!(diagnostics[0]["phase"], "check");
    assert_eq!(diagnostics[0]["kind"], "unknown-name");
    assert_eq!(diagnostics[0]["origin"], "<expression>");
}

#[test]
fn the_vault_is_supplied_from_outside_and_its_absence_is_said_out_loud() {
    let outside = tempdir().unwrap();
    let output = hql_in(outside.path(), &["eval", "cards | count"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("--vault"), "{}", stderr(&output));

    let output = hql_in(outside.path(), &["eval", "1 + 1"]);
    assert_eq!(stdout(&output), "2\n");

    let output = hql(&["--vault", VAULT, "eval", "cards | count"]);
    assert_eq!(stdout(&output), "6\n");
}

#[test]
fn without_the_flag_the_vault_is_the_nearest_marked_directory_above() {
    let root = tempdir().unwrap();
    std::fs::create_dir(root.path().join(".hmd")).unwrap();
    std::fs::write(root.path().join("one.md"), "# One\n").unwrap();
    let deep = root.path().join("notes/deeper");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("two.md"), "# Two\n").unwrap();

    let output = hql_in(&deep, &["eval", "cards | count"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "2\n");

    let fixture = std::env::current_dir().unwrap().join(VAULT);
    let output = hql_in(
        &deep,
        &[
            "--vault",
            fixture.to_str().unwrap(),
            "eval",
            "cards | count",
        ],
    );
    assert_eq!(stdout(&output), "6\n");
}

#[test]
fn the_whole_pipeline_runs_from_the_command_line() {
    let output = hql(&[
        "--vault",
        VAULT,
        "eval",
        "import lexical\ncards\n| lexical(\"bearer token authorization\")\n| take(2)\n| expand(depth = 1)\n| text",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let table = stdout(&output);
    assert!(table.contains("retrieved"), "{table}");
    assert!(table.contains("expanded from"), "{table}");
}

#[test]
fn collect_mode_reports_every_error_and_strict_reports_the_first() {
    let source = "a = nosuchthing\nb = alsomissing";
    let strict = hql(&["eval", source]);
    assert_eq!(stderr(&strict).matches("check error[").count(), 1);

    let collected = hql(&["--report", "collect", "eval", source]);
    assert_eq!(stderr(&collected).matches("check error[").count(), 2);
    assert!(
        stderr(&collected).contains("2 diagnostics"),
        "{}",
        stderr(&collected)
    );

    let unknown = hql(&["--report", "loud", "eval", "1"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(stderr(&unknown).contains("unknown report mode"));
}

#[test]
fn the_environment_supplies_a_default_mode_and_the_flag_outranks_it() {
    let source = "a = nosuchthing\nb = alsomissing";
    let from_environment = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["eval", source])
        .env("HQL_REPORT", "collect")
        .output()
        .unwrap();
    assert_eq!(stderr(&from_environment).matches("check error[").count(), 2);

    let overridden = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["--report", "strict", "eval", source])
        .env("HQL_REPORT", "collect")
        .output()
        .unwrap();
    assert_eq!(stderr(&overridden).matches("check error[").count(), 1);
}

#[test]
fn a_warning_does_not_change_the_exit_status() {
    let output = hql(&["--vault", VAULT, "eval", "@[[nowhere]]"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "none\n");
    assert!(stderr(&output).contains("execute warning[unresolved-link]:"));
}

#[test]
fn json_diagnostics_are_one_document_per_line_in_the_order_written() {
    let output = hql(&[
        "--diagnostics-format",
        "json",
        "eval",
        "fn totalOf(xs : List<Int>) : Int = xs | sum\ntotalOf([1]) / 0",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let lines: Vec<serde_json::Value> = stderr(&output)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["phase"], "check");
    assert_eq!(lines[0]["severity"], "warning");
    assert_eq!(lines[1]["phase"], "execute");
    assert_eq!(lines[1]["kind"], "division-by-zero");
}

#[test]
fn the_repl_gives_each_line_typed_its_own_origin() {
    let output = piped(&["repl"], "x = 1\nx = \"a\" + 1\n");
    assert!(
        stdout(&output).contains("<repl:2>:1:11") || stderr(&output).contains("<repl:2>:1:11"),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
}

#[test]
fn render_builds_a_document_and_writes_it_where_asked() {
    let output = hql(&["--vault", VAULT, "render", "tests/fixtures/board.hmd"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!stdout(&output).contains("```"), "{}", stdout(&output));
    assert!(stdout(&output).contains("- Session cookies"));

    let directory = tempdir().unwrap();
    let built = directory.path().join("built.md");
    let write = hql(&[
        "--vault",
        VAULT,
        "render",
        "tests/fixtures/board.hmd",
        "--write",
        built.to_str().unwrap(),
    ]);
    assert!(write.status.success(), "{}", stderr(&write));
    assert!(stdout(&write).contains("2 blocks rendered"));
    assert_eq!(
        std::fs::read_to_string(&built).unwrap(),
        stdout(&output),
        "--write writes what standard output shows"
    );
}

#[test]
fn render_in_place_after_is_idempotent() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("board.hmd");
    std::fs::copy("tests/fixtures/board.hmd", &path).unwrap();
    let name = path.to_str().unwrap();

    let first = hql(&[
        "--vault",
        VAULT,
        "render",
        name,
        "--in-place",
        "--emit",
        "after",
    ]);
    assert!(first.status.success(), "{}", stderr(&first));
    let once = std::fs::read_to_string(&path).unwrap();
    assert!(once.contains("```hql#eval"));
    assert!(once.contains("<!-- hql:output hash="));

    hql(&[
        "--vault",
        VAULT,
        "render",
        name,
        "--in-place",
        "--emit",
        "after",
    ]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), once);
}

#[test]
fn render_in_place_renders_several_documents() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.hmd");
    let second = directory.path().join("second.hmd");
    std::fs::copy("tests/fixtures/board.hmd", &first).unwrap();
    std::fs::copy("tests/fixtures/board.hmd", &second).unwrap();

    let output = hql(&[
        "--vault",
        VAULT,
        "render",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
        "--in-place",
        "--emit",
        "after",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).matches("blocks rendered").count(), 2);
    let rendered = std::fs::read_to_string(&first).unwrap();
    assert!(rendered.contains("<!-- hql:output hash="));
    assert_eq!(std::fs::read_to_string(&second).unwrap(), rendered);
}

#[test]
fn render_refuses_several_documents_without_in_place() {
    let output = hql(&[
        "--vault",
        VAULT,
        "render",
        "tests/fixtures/board.hmd",
        "tests/fixtures/board.hmd",
    ]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(stderr(&output).contains("several files need --in-place"));
}

#[test]
fn render_in_place_refuses_to_replace_the_cells() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("board.hmd");
    std::fs::copy("tests/fixtures/board.hmd", &path).unwrap();

    let output = hql(&[
        "--vault",
        VAULT,
        "render",
        path.to_str().unwrap(),
        "--in-place",
    ]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        std::fs::read_to_string("tests/fixtures/board.hmd").unwrap()
    );
}

#[test]
fn render_writes_nothing_when_a_replaced_cell_fails() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("broken.hmd");
    let built = directory.path().join("built.md");
    std::fs::write(&source, "```hql\ncards | nosuchstep\n```\n").unwrap();

    let output = hql(&[
        "--vault",
        VAULT,
        "render",
        source.to_str().unwrap(),
        "--write",
        built.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("error"));
    assert!(!built.exists());
}

#[test]
fn every_query_fixture_runs_against_the_fixture_vault() {
    // These are the files a person runs by hand to see the language work, so
    // they are pinned rather than left to rot.
    for (query, expected) in [
        ("todo.hql", "Session cookies"),
        ("nearest.hql", "expanded from bearer-tokens"),
        ("kinds.hql", "RelationCard"),
    ] {
        let path = format!("tests/fixtures/queries/{query}");
        let output = hql(&["--vault", VAULT, "run", &path]);
        assert!(output.status.success(), "{query}: {}", stderr(&output));
        assert!(
            stdout(&output).contains(expected),
            "{query}: {}",
            stdout(&output)
        );
    }
}

#[test]
fn the_repl_keeps_bindings_and_answers_type_questions() {
    let session = "a = 40\na + 2\n:type a + 2\n:quit\n";
    let output = piped(&["repl"], session);
    let printed = stdout(&output);
    assert!(printed.contains("42"), "{printed}");
    assert!(printed.contains("Int"), "{printed}");
    assert!(output.status.success());
}

#[test]
fn the_repl_types_a_function_name_by_its_declarations() {
    let output = piped(&["repl"], ":type range\n:type sort\n:quit\n");
    let printed = stdout(&output);
    assert!(
        printed.contains("fn range(start: Int, end: Int, step: Int = 1) : List<Int>    // std/seq"),
        "{printed}"
    );
    assert_eq!(printed.matches("fn sort").count(), 6, "{printed}");
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
    assert!(output.status.success());
}

#[test]
fn the_repl_leaves_on_exit_and_quit() {
    for word in ["exit", "quit"] {
        let session = format!("{word}\n1 + 1\n");
        let output = piped(&["repl"], &session);
        let printed = stdout(&output);
        assert!(!printed.contains('2'), "{word}: {printed}");
        assert!(stderr(&output).is_empty(), "{word}: {}", stderr(&output));
        assert!(output.status.success());
    }
}

#[test]
fn the_repl_continues_an_input_while_a_bracket_is_open() {
    let session = "xs = [\n    40,\n    2,\n]\nxs | sum\n";
    let output = piped(&["repl"], session);
    let printed = stdout(&output);
    assert!(printed.contains("42"), "{printed}");
    assert!(output.status.success());
}

#[test]
fn the_repl_continues_a_pipeline_written_one_step_per_line() {
    // As the book writes them: a line opening with `|` continues the input.
    let session =
        "shelf = [\n  {t: \"a\"},\n  {t: \"b\"},\n]\nr = shelf\n| map(b => b.t)\n| take(1)\nr\n";
    let output = piped(&["repl"], session);
    assert!(stdout(&output).contains("[\"a\"]"), "{}", stdout(&output));
    // Typed by hand, the `|` line arrives on its own and joins the one before.
    let typed = piped(&["repl"], "xs = [3, 1, 2]\n\n| sort\nxs\n");
    assert!(stdout(&typed).contains("[1, 2, 3]"), "{}", stdout(&typed));
}

#[test]
fn a_collection_prints_its_elements_as_literals() {
    // A result is written as HQL, so even a string standing alone is quoted;
    // as text it prints bare, and inside a collection it is quoted there too.
    assert_eq!(stdout(&hql(&["eval", "\"a, b\""])), "\"a, b\"\n");
    assert_eq!(stdout(&piped(&["run", "-"], "\"a, b\"\n")), "\"a, b\"\n");
    assert_eq!(
        stdout(&hql(&["--to", "text", "eval", "\"a, b\""])),
        "a, b\n"
    );
    assert_eq!(
        stdout(&hql(&["eval", "[\"foo \", \"a, b\"]"])),
        "[\"foo \", \"a, b\"]\n"
    );
    assert_eq!(stdout(&hql(&["eval", "{1, 3, 2, 3}"])), "{1, 3, 2}\n");
    // `{}` is an empty record, so an empty set names its type.
    assert_eq!(stdout(&hql(&["eval", "Set<Int>([])"])), "Set<Int>([])\n");
}

#[test]
fn the_repl_keeps_every_declaration_and_no_expression() {
    let session = "fn twice(n: Int) : Int = n * 2\ntop(n) := sort | take(n)\nxs : List<Int> = [3, 1, 2] | map(n => n)\nxs | top(2) | map(twice)\n";
    let output = piped(&["repl"], session);
    assert!(stdout(&output).contains("[2, 4]"), "{}", stdout(&output));
}

#[test]
fn builtins_and_config_explain_the_surface_and_the_settings() {
    let listed = hql(&["builtins"]);
    assert!(listed.status.success());
    for name in ["lexical", "expand", "graph", "take", "of_type"] {
        assert!(stdout(&listed).contains(name), "{name}");
    }
    // A flat list stops being true once a name can come from two places, so
    // help says who supplies each step and whether this vault has it.
    for provider in [
        "core",
        "graph",
        "format",
        "always available",
        "prelude",
        "imported where it is used",
    ] {
        assert!(stdout(&listed).contains(provider), "{provider}");
    }
    let configured = stdout(&hql(&["--vault", CONFIGURED, "builtins"]));
    let version = env!("CARGO_PKG_VERSION");
    assert!(configured.contains(&format!("lexical {version} (imported by this vault)")));
    assert!(configured.contains(&format!("format {version} (imported where it is used)")));

    let configured = hql(&["--vault", VAULT, "config"]);
    assert!(stdout(&configured).contains("strict"));
    assert!(stdout(&configured).contains("6 cards"));
}

#[test]
fn builtins_print_purity_and_shape_for_every_step_and_the_combiner_laws() {
    let listed = stdout(&hql(&["builtins"]));
    assert!(listed.contains("  map\n"), "{listed}");
    assert!(listed.contains("pure, elementwise"));
    assert!(listed.contains("pure, partition"));
    assert!(listed.contains("pure, reduction"));
    assert!(listed.contains("pure, barrier"));
    assert!(listed.contains("effectful, barrier"));
    // Each step's third line is its purity and shape.
    let steps = listed
        .lines()
        .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
        .count();
    let shaped = listed
        .lines()
        .filter(|line| {
            ["pure, ", "effectful, "]
                .iter()
                .any(|purity| line.trim_start().starts_with(purity))
        })
        .count();
    let combiners = hql::extensions::COMBINERS.len();
    assert_eq!(shaped + combiners, steps, "{listed}");
    assert!(listed.contains("combiners\n  +\n    over numbers, identity 0\n"));
    assert!(listed.contains("associative, order-sensitive; folds Seq"));
}

#[test]
fn config_reports_the_backend_and_an_unknown_one_fails_every_program() {
    let plain = stdout(&hql(&["config"]));
    assert!(
        plain.contains("backend: localhost (from the default)"),
        "{plain}"
    );
    let mapreduce = stdout(&hql(&["--vault", "examples/mapreduce/wiki", "config"]));
    assert!(
        mapreduce.contains("backend: localhost (from the vault's hql.toml)"),
        "{mapreduce}"
    );
    let chosen = hql(&["--backend", "localhost", "eval", "1 + 1"]);
    assert_eq!(stdout(&chosen), "2\n");
    let refused = hql(&["--backend", "threads", "eval", "1 + 1"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        stderr(&refused).contains("no backend is named `threads`, which --backend names"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn plan_prints_each_step_with_its_stage_on_the_backend_in_force() {
    let program = "doubled : Elementwise<Int, Int> = n => n * 2\n\
                   [3, 1, 2] | filter(n => n > 1) | doubled | sort\n";
    let planned = piped(&["plan", "-"], program);
    assert!(planned.status.success(), "{}", stderr(&planned));
    assert_eq!(
        stdout(&planned),
        "backend localhost\n\
         line 2: [3, 1, 2]\n  \
         | filter   stage 0  in process    elementwise, pure\n  \
         | doubled  stage 0  in process    elementwise, pure\n  \
         | sort     stage 0  in process    barrier, pure\n"
    );
    let json = piped(&["--envelope", "json", "--compact", "plan", "-"], program);
    let document: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(document["backend"], "localhost");
    assert_eq!(document["pipelines"][0]["steps"][1]["name"], "doubled");
    assert_eq!(
        document["pipelines"][0]["steps"][1]["strategy"],
        "in process"
    );
    // A program that does not check has no plan, and says why.
    let refused = piped(&["plan", "-"], "[1] | take(true)\n");
    assert_eq!(refused.status.code(), Some(1));
    assert!(stderr(&refused).contains("error"), "{}", stderr(&refused));
}

/// HQL-0037: a `fork` and a `tee` are fan-out nodes with their branches
/// beneath them, a combiner written as a step is a fan-in node, and a
/// `fork` followed by `partition` stays two graph operations.
#[test]
fn plan_shows_fan_out_and_fan_in_nodes() {
    let program = "parts = [5, 3, 8] | fork { odd = filter(x => x % 2 == 1), n = size() }\n\
                   parts.odd | partition(by = x => x) | concat([1]) | size\n";
    let planned = piped(&["plan", "-"], program);
    assert!(planned.status.success(), "{}", stderr(&planned));
    assert_eq!(
        stdout(&planned),
        "backend localhost\n\
         line 1: [5, 3, 8]\n  \
         | fork  stage 0  in process    barrier, pure, fan-out\n      \
         odd =\n        \
         | filter  stage 0  in process    elementwise, pure\n      \
         n =\n        \
         | size  stage 0  in process    reduction, pure\n\
         line 2: parts.odd\n  \
         | partition  stage 0  in process    partition, pure\n  \
         | concat     stage 0  in process    barrier, pure, fan-in\n  \
         | size       stage 0  in process    reduction, pure\n"
    );
    let json = piped(&["--envelope", "json", "--compact", "plan", "-"], program);
    let document: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    let fork = &document["pipelines"][0]["steps"][0];
    assert_eq!(fork["topology"], "fan-out");
    assert_eq!(fork["branches"][0]["name"], "odd");
    assert_eq!(fork["branches"][1]["steps"][0]["name"], "size");
    assert_eq!(document["pipelines"][1]["steps"][1]["topology"], "fan-in");
}

#[test]
fn cli_surface_identifies_hql_and_usage_errors_exit_two() {
    let help = hql(&["--help"]);
    assert!(help.status.success());
    assert!(stdout(&help).contains("Hyper Query Language"));
    assert_eq!(
        hql(&["--version"]).stdout,
        format!("hql {}\n", env!("CARGO_PKG_VERSION")).into_bytes()
    );
    for args in [vec![], vec!["eval"], vec!["check"]] {
        assert_eq!(hql(&args).status.code(), Some(2), "{args:?}");
    }
    // A first word that names no command is a program: `hql '<program>'`.
    let program = hql(&["unknown"]);
    assert_eq!(program.status.code(), Some(1));
    assert!(
        stderr(&program).contains("`unknown` is not bound"),
        "{}",
        stderr(&program)
    );
    assert_eq!(stdout(&hql(&["1 + 2"])), "3\n");
    let missing = hql(&["--vault", "tests/fixtures/nowhere", "eval", "1"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(stderr(&missing).contains("not a directory"));
}

#[test]
fn a_vault_may_decline_the_prelude_and_import_for_every_program() {
    // Declining the prelude is what makes `| markdown` need saying out loud.
    let declined = hql(&["--vault", CONFIGURED, "eval", "1 | markdown"]);
    assert!(!declined.status.success());
    assert!(
        stderr(&declined).contains("write `import format`"),
        "{}",
        stderr(&declined)
    );
    let written = hql(&["--vault", CONFIGURED, "eval", "import format\n1 | markdown"]);
    assert!(written.status.success(), "{}", stderr(&written));

    // What the vault imports, a query does not have to.
    let ranked = hql(&[
        "--vault",
        CONFIGURED,
        "eval",
        "cards | lexical(\"bearer token authorization\") | count",
    ]);
    assert!(ranked.status.success(), "{}", stderr(&ranked));
    assert_eq!(stdout(&ranked).trim(), "1");
}

#[test]
fn the_standard_library_checks_from_the_command_line() {
    let checked = hql(&["check", "lib/std/graph/mod.hql"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
    assert_eq!(stdout(&checked).trim(), "Unit");
    assert_eq!(stderr(&checked), "");
}

#[test]
fn a_logger_writes_beside_the_result_and_the_host_sets_its_threshold() {
    let source = "import logging\nlog = Logger(stderr, Text, name = \"xs\")\n\
                  log | debug(\"hidden\")\nlog | info(\"counted\", {n: 2})\n[1, 2] | size";
    let quiet = hql(&["eval", source]);
    assert_eq!(stdout(&quiet), "2\n");
    assert_eq!(stderr(&quiet), "INFO xs counted {n: 2}\n");

    let loud = hql(&["--log-level", "debug", "eval", source]);
    assert_eq!(stderr(&loud), "DEBUG xs hidden\nINFO xs counted {n: 2}\n");
    let quieter = hql(&["--log-level", "warn", "eval", source]);
    assert!(stderr(&quieter).is_empty(), "{}", stderr(&quieter));
    let unknown = hql(&["--log-level", "loud", "eval", "1"]);
    assert_eq!(unknown.status.code(), Some(1));
}

#[test]
fn the_report_level_is_named_from_a_known_set() {
    let refused = hql(&["--report-level", "loud", "eval", "1"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(stderr(&refused).contains("unknown report level"));
}

#[test]
fn config_names_the_level_and_its_origin() {
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["config"])
        .env("HQL_REPORT_LEVEL", "info")
        .output()
        .unwrap();
    assert!(stdout(&output).contains("report level: info (from $HQL_REPORT_LEVEL)"));
}

#[test]
fn to_names_the_format_a_result_is_written_in() {
    let graph = "import lexical\ncards | lexical(\"bearer token authorization\") | take(2) | expand(depth = 1)";
    let text = hql(&["--vault", VAULT, "eval", graph]);
    assert!(stdout(&text).contains("(retrieved "), "{}", stdout(&text));
    let json = hql(&["--vault", VAULT, "--to", "json", "eval", graph]);
    assert!(stdout(&json).contains("\"nodes\""), "{}", stdout(&json));
    let mermaid = hql(&["--vault", VAULT, "--to", "mermaid", "eval", graph]);
    assert!(
        stdout(&mermaid).starts_with("graph LR"),
        "{}",
        stdout(&mermaid)
    );
    // A result the program formatted is text; asked for in another format,
    // it is that String encoded again.
    let formatted = hql(&["--to", "json", "eval", "[1, 2] | markdown"]);
    assert_eq!(stdout(&formatted), "\"- 1\\n- 2\"\n");
    let kept = hql(&["eval", "[1, 2] | json"]);
    assert_eq!(stdout(&kept), "[\n  1,\n  2\n]\n");
}

#[test]
fn to_none_evaluates_and_writes_no_value() {
    let output = hql(&["--to", "none", "eval", "40 + 2"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
    let failed = hql(&["--to", "none", "eval", "nosuchname"]);
    assert_eq!(failed.status.code(), Some(1));
    let unknown = hql(&["--to", "pdf", "eval", "1"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(
        stderr(&unknown).contains("unknown format `pdf`"),
        "{}",
        stderr(&unknown)
    );
}

#[test]
fn the_envelope_carries_the_value_and_the_reports() {
    let output = hql(&["--envelope", "json", "--to", "markdown", "eval", "[1, 2]"]);
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(document["value"], "- 1\n- 2");
    assert_eq!(document["type"], "List<Int>");
    let renamed = hql(&["--format", "json", "eval", "1"]);
    assert_eq!(renamed.status.code(), Some(2), "--format names nothing");
}

#[test]
fn a_stream_is_written_line_by_line_as_it_arrives() {
    use std::io::{BufRead, BufReader};
    let program = "fn naturals(from: Int) : Stream<Int> {\n    yield from\n    for n in naturals(from + 1) {\n        yield n\n    }\n}\nnaturals(1) | json";
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["eval", program])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let lines: Vec<String> = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .take(3)
        .map(Result::unwrap)
        .collect();
    assert_eq!(lines, ["1", "2", "3"]);
    // Closing the pipe ends the stream rather than leaving it running.
    assert!(child.wait().unwrap().success());
    let json = hql(&["--envelope", "json", "eval", program]);
    assert_eq!(json.status.code(), Some(1));
    assert!(stderr(&json).contains("may not end"), "{}", stderr(&json));
}

#[test]
fn render_runs_one_cell_and_reports_every_cell_as_json() {
    let directory = tempdir().unwrap();
    // A document renders only in a vault.
    std::fs::write(directory.path().join("hql.toml"), "").unwrap();
    let path = directory.path().join("notes.hmd");
    std::fs::write(
        &path,
        "```hql id=base\n10\n```\n\n```hql id=double\nbase * 2\n```\n\n```hql id=other\n1 / 0\n```\n",
    )
    .unwrap();
    let name = path.to_str().unwrap();

    let replaced = hql(&["render", name, "--cell", "double"]);
    assert_eq!(replaced.status.code(), Some(2), "--cell needs --emit after");

    let one = hql(&[
        "render",
        name,
        "--cell",
        "double",
        "--emit",
        "after",
        "--in-place",
    ]);
    assert!(one.status.success(), "{}", stderr(&one));
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("id=double hash="), "{written}");
    assert!(written.contains("status=ok -->\n20\n"), "{written}");
    // The failing cell was not asked for, so it did not run.
    assert!(!written.contains("status=error"), "{written}");

    let json = hql(&["--envelope", "json", "render", name, "--emit", "after"]);
    let record: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    let cells = record["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[0]["id"], "base");
    assert_eq!(cells[0]["type"], "Int");
    assert_eq!(cells[1]["markdown"], "20");
    assert_eq!(cells[2]["status"], "error");
    assert_eq!(cells[0]["span"][0], 0);
    assert_eq!(json.status.code(), Some(1));
}

#[test]
fn a_program_outside_every_vault_imports_builtin_modules() {
    // No vault above the directory: the standard modules still resolve, by
    // import and through the prelude, and only host bindings are missing.
    let dir = tempdir().unwrap();
    let program = dir.path().join("q.hql");
    std::fs::write(
        &program,
        "import regex\nimport io.logging as log\n[regex.is_match(\"wren\", \"^w\"), \"  x \".trim() == \"x\"]\n",
    )
    .unwrap();
    let output = hql_in(dir.path(), &["run", "q.hql"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "[true, true]"
    );
    let unbound = hql_in(dir.path(), &["eval", "cards | size"]);
    assert!(!unbound.status.success());
    assert!(String::from_utf8_lossy(&unbound.stderr).contains("needs a vault"));
}

/// HQL-0035: `hql boundary` prints a source's effective boundary, each
/// property with the layer it comes from, and every boundary error.
#[test]
fn boundary_prints_a_sources_effective_boundary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ledger.csv"),
        "Species Name,count\nwren,2\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("hql.toml"),
        "[sources.ledger]\nkind = \"csv\"\npath = \"ledger.csv\"\nstrictness = \"open\"\n\
         [sources.ledger.fields.species]\nname = \"Species Name\"\n\
         [sources.ledger.fields.count]\nwrite = false\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("types.hql"),
        "type Tally {\n    species : String\n    count? : Int\n}\n",
    )
    .unwrap();
    let shown = hql_in(
        dir.path(),
        &["boundary", "types.hql", "ledger", "--as", "Tally"],
    );
    assert!(shown.status.success(), "{}", stderr(&shown));
    let text = stdout(&shown);
    assert!(
        text.starts_with("source ledger (csv), read as Tally; strictness open, mode replace\n"),
        "{text}"
    );
    assert!(text.contains("count? : Int\n"), "{text}");
    let line = |property: &str, value: &str| {
        text.lines()
            .find(|line| line.trim_start().starts_with(property) && line.contains(value))
            .map(|line| line.split_whitespace().last().unwrap_or("").to_owned())
    };
    assert_eq!(
        line("external", "Species Name").as_deref(),
        Some("configuration"),
        "{text}"
    );
    assert_eq!(
        line("write", "false").as_deref(),
        Some("configuration"),
        "{text}"
    );
    assert_eq!(line("type", "text").as_deref(), Some("format"), "{text}");
    let json = hql_in(
        dir.path(),
        &[
            "--envelope",
            "json",
            "boundary",
            "types.hql",
            "ledger",
            "--as",
            "Tally",
        ],
    );
    let document: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(document["fields"][1]["write"]["origin"], "configuration");
    assert_eq!(document["fields"][0]["external_type"]["value"], "text");
    // A required field the header does not hold is a boundary error.
    std::fs::write(
        dir.path().join("types.hql"),
        "type Tally {\n    species : String\n    total : Int\n}\n",
    )
    .unwrap();
    let refused = hql_in(
        dir.path(),
        &["boundary", "types.hql", "ledger", "--as", "Tally"],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        stderr(&refused).contains("field `total`") && stderr(&refused).contains("unmapped field"),
        "{}",
        stderr(&refused)
    );
}
