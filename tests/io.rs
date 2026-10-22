//! Endpoints through the real process: `source`, `tap`, `target` and `sink`
//! over standard input, files and their ends, and what `hql render` refuses.
//! See doc/proposals/HQL-0021/README.md.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::tempdir;

/// Run `hql` in a directory, with standard input closed.
fn hql_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Run `hql` in a directory with `input` on standard input.
fn piped_in(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
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

fn ok(output: &Output) -> String {
    assert!(output.status.success(), "{}", stderr(output));
    stdout(output)
}

fn refused(output: &Output, says: &str) {
    assert!(!output.status.success(), "{}", stdout(output));
    assert!(stderr(output).contains(says), "{}", stderr(output));
}

#[test]
fn standard_input_reads_as_jq_writes_it() {
    let dir = tempdir().unwrap();
    let one = piped_in(
        dir.path(),
        &["source(stdin, Json)", "--to", "json"],
        r#"{"name":"owl","count":3}"#,
    );
    assert_eq!(ok(&one), "{\n  \"count\": 3,\n  \"name\": \"owl\"\n}\n");
    let compact = piped_in(
        dir.path(),
        &["source(stdin, Json)", "--to", "json", "--compact"],
        "{\"a\": 1}\n{\"a\": 2}\n",
    );
    assert_eq!(ok(&compact), "{\"a\":1}\n{\"a\":2}\n");
    // What `--to json` writes, `source(stdin, Json)` reads back.
    let pretty = piped_in(
        dir.path(),
        &["source(stdin, Json)", "--to", "json"],
        "{\"a\": 1}\n{\"a\": 2}\n",
    );
    let again = piped_in(
        dir.path(),
        &["source(stdin, Json)", "--to", "json", "--compact"],
        &ok(&pretty),
    );
    assert_eq!(ok(&again), "{\"a\":1}\n{\"a\":2}\n");
    let yaml = piped_in(
        dir.path(),
        &["source(stdin, Json)", "--to", "yaml"],
        "{\"a\": 1}",
    );
    assert_eq!(ok(&yaml), "---\na: 1\n");
}

#[test]
fn a_json_array_is_one_document() {
    let dir = tempdir().unwrap();
    let output = piped_in(
        dir.path(),
        &[
            "source(stdin, Json) | map(d => 1)",
            "--to",
            "json",
            "--compact",
        ],
        "[1, 2, 3]",
    );
    assert_eq!(ok(&output), "1\n");
}

#[test]
fn a_file_is_every_document_it_holds_in_its_format() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("one.json"), r#"{"species": "owl"}"#).unwrap();
    fs::write(dir.path().join("many.jsonl"), "{\"n\": 1}\n{\"n\": 2}\n").unwrap();
    fs::write(
        dir.path().join("rows.csv"),
        "species,count\nowl,4\n\"eagle, golden\",7\n",
    )
    .unwrap();
    fs::write(dir.path().join("lines.txt"), "a\nb\n").unwrap();
    fs::write(dir.path().join("docs.yaml"), "a: 1\n---\na: 2\n").unwrap();
    let json = |program: &str| ok(&hql_in(dir.path(), &[program, "--to", "json", "--compact"]));
    assert_eq!(
        json(r#"source(file("one.json"))"#),
        "[{\"species\":\"owl\"}]\n"
    );
    assert_eq!(json(r#"source(file("many.jsonl")) | size"#), "2\n");
    assert_eq!(
        json(r#"source(file("rows.csv"))"#),
        "[{\"count\":\"4\",\"species\":\"owl\"},{\"count\":\"7\",\"species\":\"eagle, golden\"}]\n"
    );
    assert_eq!(json(r#"source(file("lines.txt"))"#), "[\"a\",\"b\"]\n");
    assert_eq!(
        json(r#"source(file("docs.yaml"))"#),
        "[{\"a\":1},{\"a\":2}]\n"
    );
    assert_eq!(
        json(r#"tap(file("many.jsonl")) | take(1)"#),
        "[{\"n\":1}]\n"
    );
    // A named format wins over the extension.
    assert_eq!(
        json(r#"source(file("one.json"), Text)"#),
        "[\"{\\\"species\\\": \\\"owl\\\"}\"]\n"
    );
}

#[test]
fn a_format_is_named_unless_a_literal_path_implies_it() {
    let dir = tempdir().unwrap();
    refused(
        &hql_in(
            dir.path(),
            &[r#"p = "x.json"
source(file(p))"#],
        ),
        "needs a format",
    );
    refused(
        &hql_in(dir.path(), &[r#"source(file("x.data"))"#]),
        "implies no format",
    );
    refused(&hql_in(dir.path(), &["source(stdin)"]), "needs a format");
}

#[test]
fn an_endpoint_is_used_only_for_what_it_can_do() {
    let dir = tempdir().unwrap();
    refused(
        &hql_in(dir.path(), &["source(stdout, Json)"]),
        "needs (Readable | String), not Stdout",
    );
    refused(
        &hql_in(dir.path(), &["42 | target(stdin, Json)"]),
        "needs Writable",
    );
    refused(
        &hql_in(dir.path(), &[r#"source(file("log.txt").end)"#]),
        "with `tap`",
    );
    refused(
        &hql_in(dir.path(), &["tap(stdin, Json) | target(stdout, Json)"]),
        "with `sink`",
    );
    refused(
        &hql_in(dir.path(), &[r#"file("https://example.com/x.json")"#]),
        "`http`",
    );
}

#[test]
fn a_target_replaces_with_one_document_and_a_sink_writes_one_per_value() {
    let dir = tempdir().unwrap();
    let written = hql_in(
        dir.path(),
        &[r#"[{a: 1}, {a: 2}] | target(file("whole.json"))"#],
    );
    // A program that ends in a write prints no value.
    assert_eq!(ok(&written), "");
    assert_eq!(
        fs::read_to_string(dir.path().join("whole.json")).unwrap(),
        "[\n  {\n    \"a\": 1\n  },\n  {\n    \"a\": 2\n  }\n]\n"
    );
    ok(&hql_in(
        dir.path(),
        &[r#"[{a: 1}, {a: 2}] | sink(file("lines.jsonl"))"#],
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("lines.jsonl")).unwrap(),
        "{\"a\":1}\n{\"a\":2}\n"
    );
    // A sink reproduces the JSON Lines it read, byte for byte.
    let input = "{\"a\":1}\n{\"b\":[1,2]}\n";
    ok(&piped_in(
        dir.path(),
        &[r#"source(stdin, Json) | sink(file("copy.jsonl"))"#],
        input,
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("copy.jsonl")).unwrap(),
        input
    );
}

#[test]
fn the_end_of_a_file_is_appended_to() {
    let dir = tempdir().unwrap();
    for _ in 0..2 {
        ok(&hql_in(
            dir.path(),
            &[r#"[{a: 1}] | sink(file("log.jsonl").end)"#],
        ));
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("log.jsonl")).unwrap(),
        "{\"a\":1}\n{\"a\":1}\n"
    );
    ok(&hql_in(
        dir.path(),
        &[r#"[{a: 2}] | sink(file("log.jsonl").end)
[{a: 3}] | sink(file("log.jsonl").end)"#],
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("log.jsonl")).unwrap(),
        "{\"a\":1}\n{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n"
    );
    // A CSV header is written once, not again for appended rows.
    for count in [1, 2] {
        let program = format!(r#"[{{n: {count}}}] | sink(file("rows.csv").end)"#);
        ok(&hql_in(dir.path(), &[&program]));
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("rows.csv")).unwrap(),
        "n\n1\n2\n"
    );
}

#[test]
fn one_evaluation_replaces_a_file_once_and_never_also_appends() {
    let dir = tempdir().unwrap();
    refused(
        &hql_in(
            dir.path(),
            &[r#"1 | target(file("a.json"))
2 | target(file("a.json"))"#],
        ),
        "already replaced",
    );
    refused(
        &hql_in(
            dir.path(),
            &[r#"1 | target(file("b.json"))
[2] | sink(file("b.json").end)"#],
        ),
        "not both",
    );
    refused(
        &hql_in(
            dir.path(),
            &[r#"1 | target(file("c.json"))
source(file("c.json"))"#],
        ),
        "has replaced the file",
    );
    // Reading a file and then replacing it is how a program rewrites one.
    fs::write(dir.path().join("d.json"), "[1, 2]").unwrap();
    ok(&hql_in(
        dir.path(),
        &[r#"xs = source(file("d.json"))
xs | target(file("d.json"))"#],
    ));
}

#[test]
fn a_relative_path_resolves_against_the_program_that_names_it() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/data.json"), "{\"here\": true}").unwrap();
    fs::write(dir.path().join("sub/q.hql"), r#"source(file("data.json"))"#).unwrap();
    let output = hql_in(
        dir.path(),
        &["run", "sub/q.hql", "--to", "json", "--compact"],
    );
    assert_eq!(ok(&output), "[{\"here\":true}]\n");
}

#[test]
fn a_document_reads_beside_itself_and_writes_nothing() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("doc")).unwrap();
    fs::write(dir.path().join("doc/d.json"), "{\"k\": 1}").unwrap();
    fs::write(dir.path().join("outside.json"), "{\"k\": 2}").unwrap();
    let cell = |name: &str, program: &str| {
        let path = dir.path().join("doc").join(name);
        fs::write(&path, format!("```hql\n{program}\n```\n")).unwrap();
        hql_in(dir.path(), &["render", &format!("doc/{name}")])
    };
    assert!(ok(&cell("read.md", r#"source(file("d.json"))"#)).contains("{k: 1}"));
    refused(
        &cell("up.md", r#"source(file("../outside.json"))"#),
        "under its own directory",
    );
    refused(
        &cell("write.md", r#"[1] | target(file("o.json"))"#),
        "writes nothing outside itself",
    );
    refused(
        &cell("sink.md", r#"[1] | sink(stdout, Json)"#),
        "writes nothing outside itself",
    );
    refused(
        &cell("stdin.md", r#"source(stdin, Json)"#),
        "no standard input",
    );
    assert!(!dir.path().join("doc/o.json").exists());
}

#[test]
fn a_program_that_does_not_read_standard_input_never_waits_on_it() {
    let dir = tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir.path())
        .args(["1 + 2"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // Standard input stays open and silent.
    let _held = child.stdin.take();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "hql waited on stdin"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success());
}

#[test]
fn a_stream_from_an_open_pipe_answers_before_the_input_ends() {
    let dir = tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir.path())
        .args([
            "source(stdin, Json) | filter(d => d == {a: 2})",
            "--to",
            "json",
            "--compact",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{{\"a\": 1}}\n{{\"a\": 2}}").unwrap();
    input.flush().unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert_eq!(first, "{\"a\":2}\n");
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn tapping_the_end_of_a_file_follows_what_is_appended() {
    let dir = tempdir().unwrap();
    let log = dir.path().join("app.log");
    fs::write(&log, "old\n").unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir.path())
        .args([
            r#"tap(file("app.log").end, Lines) | take(1)"#,
            "--to",
            "json",
            "--compact",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // Give the follower time to start at the end, then append.
    std::thread::sleep(Duration::from_millis(500));
    fs::OpenOptions::new()
        .append(true)
        .open(&log)
        .unwrap()
        .write_all(b"new\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "[\"new\"]\n");
}

#[test]
fn a_channel_holds_values_in_the_order_they_were_written() {
    let dir = tempdir().unwrap();
    let held = hql_in(
        dir.path(),
        &["c = Channel<Int>(\"nums\")\n1 | target(c)\n[2, 3] | sink(c)\nsource(c)"],
    );
    assert_eq!(ok(&held), "[1, 2, 3]\n");
    // Reading a channel does not consume it.
    let twice = hql_in(
        dir.path(),
        &[
            "c = Channel<Int>(\"n\")\n[1] | sink(c)\na = source(c)\nb = source(c)\n[size(a), size(b)]",
        ],
    );
    assert_eq!(ok(&twice), "[1, 1]\n");
    // A tap sees what is written after it begins.
    let tapped = hql_in(
        dir.path(),
        &["c = Channel<Int>(\"n\")\n[1] | sink(c)\nt = tap(c)\n[2] | sink(c)\nt | take(5)"],
    );
    assert_eq!(ok(&tapped), "[2]\n");
    refused(
        &hql_in(dir.path(), &["c = Channel<Int>(\"n\")\nsource(c, Json)"]),
        "takes no format",
    );
    refused(
        &hql_in(dir.path(), &["c = Channel<Int>(\"n\")\n\"x\" | target(c)"]),
        "holds Int, not String",
    );
    refused(
        &hql_in(
            dir.path(),
            &["c = Channel<Int>(\"n\")\nd = Channel<String>(\"n\")\n1"],
        ),
        "not also a Channel<String>",
    );
}

#[test]
fn a_document_may_write_its_own_channels() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("doc.md"),
        "```hql\nc = Channel<Int>(\"cell\")\n[1, 2] | sink(c)\nsource(c) | size\n```\n",
    )
    .unwrap();
    assert!(ok(&hql_in(dir.path(), &["render", "doc.md"])).contains('2'));
}

#[test]
fn a_function_that_reads_or_logs_is_effectful() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("q.hql"),
        "import logging\n\
         fn load(p: String) : Seq<Data> = source(file(p), Json)\n\
         fn total(p: String) : Int = load(p) | size\n\
         fn next(x: Int) : Int = x + 1\n\
         note(log: Logger) := log | info(\"noted\")\n\
         1\n",
    )
    .unwrap();
    let checked = ok(&hql_in(dir.path(), &["check", "q.hql"]));
    assert!(
        checked.contains("load is Effectful: it uses `source`"),
        "{checked}"
    );
    assert!(
        checked.contains("total is Effectful: it uses `load`"),
        "{checked}"
    );
    assert!(
        checked.contains("note is Effectful: it uses `info`"),
        "{checked}"
    );
    assert!(!checked.contains("next is"), "{checked}");
}

#[test]
fn a_program_read_from_standard_input_reads_no_data_there() {
    let dir = tempdir().unwrap();
    let output = piped_in(dir.path(), &["run", "-"], "source(stdin, Json)\n");
    refused(&output, "read from standard input");
}

#[test]
fn each_tee_branch_writes_its_own_output() {
    let dir = tempdir().unwrap();
    let program = "source(stdin, Json)\n\
                   | tee archive = sink(file(\"events.jsonl\"))\n\
                   | filter(e => e.level == \"error\")\n\
                   | tee errors = sink(file(\"errors.jsonl\"))\n\
                   | map(e => e.message)\n\
                   | sink(stdout, Text)";
    let input = "{\"level\":\"error\",\"message\":\"a\"}\n{\"level\":\"info\",\"message\":\"b\"}\n";
    assert_eq!(ok(&piped_in(dir.path(), &[program], input)), "a\n");
    assert_eq!(
        fs::read_to_string(dir.path().join("events.jsonl")).unwrap(),
        input
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("errors.jsonl")).unwrap(),
        "{\"level\":\"error\",\"message\":\"a\"}\n"
    );
    let failed = piped_in(
        dir.path(),
        &["source(stdin, Json) | tee x = sink(file(\"no/x.jsonl\")) | sink(stdout, Json)"],
        input,
    );
    refused(&failed, "no/x.jsonl");
}

#[test]
fn lines_frame_text_by_line_and_text_is_one_document() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("notes.txt"), "a\nb\n").unwrap();
    let json = |program: &str| ok(&hql_in(dir.path(), &[program, "--to", "json", "--compact"]));
    // The terminator belongs to the representation.
    assert_eq!(
        json(r#"source(file("notes.txt"), Lines)"#),
        "[\"a\",\"b\"]\n"
    );
    // Text is the whole document, terminators and all.
    assert_eq!(
        json(r#"source(file("notes.txt"), Text)"#),
        "[\"a\\nb\\n\"]\n"
    );
    // A plain text file is lines unless a format says otherwise.
    assert_eq!(json(r#"source(file("notes.txt")) | size"#), "2\n");
    // Writing puts the terminators back.
    ok(&hql_in(
        dir.path(),
        &[r#"["x", "y"] | sink(file("out.txt"), Lines)"#],
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("out.txt")).unwrap(),
        "x\ny\n"
    );
    // A sink writes a stream, one record per value; one value is one whole
    // document, written by target.
    refused(
        &hql_in(dir.path(), &[r#""solo" | sink(file("one.txt"), Lines)"#]),
        "`sink` needs",
    );
    ok(&hql_in(
        dir.path(),
        &[r#""solo" | target(file("one.txt"), Lines)"#],
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("one.txt")).unwrap(),
        "solo\n"
    );
    // The shorthands encode in memory: one line per element.
    assert_eq!(
        ok(&hql_in(dir.path(), &[r#"["a", "b"] | lines"#])),
        "a\nb\n"
    );
    assert_eq!(
        ok(&hql_in(dir.path(), &[r#"[{a: 1}, {a: 2}] | jsonlines"#])),
        "{\"a\":1}\n{\"a\":2}\n"
    );
    // JsonLines reads one document per line, and a stream of them taps.
    fs::write(dir.path().join("events.jsonl"), "{\"n\":1}\n{\"n\":2}\n").unwrap();
    assert_eq!(
        json(r#"source(file("events.jsonl"), JsonLines) | map(e => e.n)"#),
        "[1,2]\n"
    );
}

#[test]
fn a_read_decodes_each_document_as_the_type_it_is_expected_to_give() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("sightings.jsonl"),
        "{\"species\": \"owl\", \"count\": 3, \"site\": \"barn\"}\n{\"species\": \"wren\", \"count\": 5}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("one.json"),
        r#"{"species": "owl", "count": 3}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("rows.csv"),
        "species,count\nowl,4\nwren,7\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("bad.csv"),
        "species,count\nowl,4\nwren,many\n",
    )
    .unwrap();
    fs::write(dir.path().join("notes.txt"), "a\n").unwrap();
    const SIGHTING: &str = "type Sighting : Data {\n    species : String\n    count   : Int\n}\n";
    let run = |program: &str| {
        hql_in(
            dir.path(),
            &[&format!("{SIGHTING}{program}"), "--to", "json", "--compact"],
        )
    };
    // A binding's declared element type is what each document decodes as;
    // an entry the record does not name is ignored.
    assert_eq!(
        ok(&run(
            "s : Seq<Sighting> = source(file(\"sightings.jsonl\"))\ns | map(x => x.count + 1)"
        )),
        "[4,6]\n"
    );
    assert_eq!(
        ok(&run(
            "s : Seq<Sighting> = file(\"one.json\") | source\ns | map(x => x.species)"
        )),
        "[\"owl\"]\n"
    );
    // A function's declared result is an expected type too, and `tap` decodes
    // one document at a time.
    assert_eq!(
        ok(&run(
            "fn load() : Stream<Sighting> = tap(file(\"sightings.jsonl\"))\nload() | map(x => x.species)"
        )),
        "\"owl\"\n\"wren\"\n"
    );
    // A CSV cell is text, read as the scalar its field wants.
    assert_eq!(
        ok(&run(
            "s : Seq<Sighting> = source(file(\"rows.csv\"))\ns | map(x => x.count) | sum"
        )),
        "11\n"
    );
    // A document that does not decode is refused as it is read, naming the
    // document and the field.
    refused(
        &run("s : Seq<Sighting> = source(file(\"bad.csv\"))\ns"),
        "document 2 does not decode as Sighting: field `count`: \"many\" is not an Int",
    );
    // A Json leaf keeps the type it was written with.
    fs::write(
        dir.path().join("text.json"),
        r#"{"species": "owl", "count": "3"}"#,
    )
    .unwrap();
    refused(
        &run("s : Seq<Sighting> = source(file(\"text.json\"))\ns"),
        "document 1 does not decode as Sighting: field `count`: \"3\" is not Int",
    );
    let missing = piped_in(
        dir.path(),
        &[&format!(
            "{SIGHTING}s : Stream<Sighting> = source(stdin, Json)\ns | map(x => x.species)"
        )],
        "{\"species\": \"owl\", \"count\": 1}\n{\"species\": \"wren\"}\n",
    );
    refused(
        &missing,
        "document 2 does not decode as Sighting: field `count`: it is missing",
    );
    assert!(stdout(&missing).contains("owl"), "{}", stdout(&missing));
    // A text document is a String, and no record is read from one.
    refused(
        &run("s : Seq<Sighting> = source(file(\"notes.txt\"))\ns"),
        "declared Seq<Sighting> but its value is Seq<String>",
    );
}

#[test]
fn from_names_the_format_of_standard_input() {
    let dir = tempdir().unwrap();
    assert_eq!(
        ok(&piped_in(
            dir.path(),
            &[
                "--from",
                "json",
                "source(stdin) | map(d => d.a) | take(2) | List"
            ],
            "{\"a\": 1}\n{\"a\": 2}\n"
        ))
        .trim(),
        "[1, 2]"
    );
    assert_eq!(
        ok(&piped_in(
            dir.path(),
            &[
                "--from",
                "csv",
                "tap(stdin) | map(r => r.b) | take(1) | List"
            ],
            "a,b\n1,2\n3,4\n"
        ))
        .trim(),
        "[\"2\"]"
    );
    // A format named in the program wins over `--from`.
    assert_eq!(
        ok(&piped_in(
            dir.path(),
            &["--from", "csv", "source(stdin, Lines) | take(1) | List"],
            "a,b\n"
        ))
        .trim(),
        "[\"a,b\"]"
    );
    refused(
        &piped_in(dir.path(), &["source(stdin)"], "{}"),
        "`--from json`",
    );
    refused(
        &piped_in(dir.path(), &["--from", "d2", "source(stdin)"], "x"),
        "written, not read",
    );
}
