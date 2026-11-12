//! The time scalars and the `time` module (HQL-0041 part 1): construction
//! from ISO 8601 text and from parts, order, calendar arithmetic, zones,
//! truncation, rendering, the evaluation clock, and the formats and tables
//! a time value crosses.

use hql::reporting::Mode;
use hql::vault;
use std::fs;
use std::process::Command;

fn shown(source: &str) -> String {
    hql::eval(&format!("import time\n{source}"))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refused(source: &str) -> String {
    hql::eval(&format!("import time\n{source}"))
        .map(|value| panic!("{source} gave {value}"))
        .unwrap_err()
        .message
}

/// What `hql` writes for `arguments`, run in a temporary directory.
fn hql(arguments: &[&str]) -> String {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .env_remove("TZ")
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn each_type_is_built_from_its_text_and_written_as_it() {
    for (source, expected) in [
        ("Date(\"2026-10-24\")", "2026-10-24"),
        ("DateTime(\"2026-10-24T09:30:00\")", "2026-10-24T09:30:00"),
        (
            "DateTime(\"2026-10-24T09:30:00.123456789\")",
            "2026-10-24T09:30:00.123456789",
        ),
        (
            "Instant(\"2026-10-24T09:30:00+02:00\")",
            "2026-10-24T07:30:00Z",
        ),
        ("Duration(\"P2DT3H\")", "PT51H"),
        ("Duration(\"-PT1.5S\")", "-PT1.5S"),
        ("str(Date(\"2026-10-24\"))", "2026-10-24"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn each_type_is_built_from_its_parts() {
    for (source, expected) in [
        ("Date(year = 2026, month = 10, day = 24)", "2026-10-24"),
        (
            "DateTime(Date(\"2026-10-24\"), hour = 9, minute = 30)",
            "2026-10-24T09:30:00",
        ),
        ("Instant(unix_seconds = 0)", "1970-01-01T00:00:00Z"),
        ("Instant(unix_millis = 1500)", "1970-01-01T00:00:01.5Z"),
        ("Duration(hours = 1, minutes = 30)", "PT1H30M"),
        ("Duration(days = 1, seconds = -1)", "PT23H59M59S"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn a_constructor_refuses_other_text_and_a_conversion_gives_none() {
    for (source, named) in [
        ("Date(\"2026-02-30\")", "\"2026-02-30\" is not a Date"),
        ("Date(\"24.10.2026\")", "is not a Date"),
        ("DateTime(\"2026-10-24T09:30:00Z\")", "is not a DateTime"),
        ("Instant(\"2026-10-24T09:30:00\")", "is not an Instant"),
        ("Duration(\"P1M\")", "is not a Duration"),
        ("Duration(\"P1Y\")", "is not a Duration"),
        ("Date(year = 2026, month = 13, day = 1)", "is not a date"),
        ("Date(year = 2026, month = 2, day = 30)", "is not a date"),
        (
            "DateTime(Date(\"2026-10-24\"), hour = 24)",
            "is not a time of day",
        ),
        ("Zone(\"Mars/Olympus\")", "is not a zone"),
    ] {
        let message = refused(source);
        assert!(message.contains(named), "{source}: {message}");
    }
    assert_eq!(
        shown("[\"2026-02-30\".to_date(), \"2026-10-24\".to_date()]"),
        "[none, 2026-10-24]"
    );
    assert_eq!(
        shown("[\"P1M\".to_duration(), \"PT5M\".to_duration()]"),
        "[none, PT5M]"
    );
}

#[test]
fn time_values_compare_sort_and_deduplicate_by_time() {
    assert_eq!(
        shown("[Date(\"2026-10-24\"), Date(\"2025-12-31\"), Date(\"2026-01-01\")] | sort"),
        "[2025-12-31, 2026-01-01, 2026-10-24]"
    );
    assert_eq!(
        shown("[Instant(\"2026-10-24T09:00:00+02:00\"), Instant(\"2026-10-24T06:59:59Z\")] | sort"),
        "[2026-10-24T06:59:59Z, 2026-10-24T07:00:00Z]"
    );
    assert_eq!(
        shown("[Duration(\"PT90M\"), Duration(\"PT1H\"), Duration(\"-PT1S\")] | sort"),
        "[-PT1S, PT1H, PT1H30M]"
    );
    assert_eq!(
        shown(
            "[Date(\"2026-10-24\") < Date(\"2026-10-25\"), Instant(\"2026-10-24T09:30:00+02:00\") == Instant(\"2026-10-24T07:30:00Z\"), Duration(\"PT60M\") == Duration(\"PT1H\")]"
        ),
        "[true, true, true]"
    );
    assert_eq!(
        shown("[Date(\"2026-10-24\"), Date(\"2026-10-24\"), Date(\"2026-10-25\")] | Set | size"),
        "2"
    );
    assert_eq!(
        shown("[Date(\"2026-10-24\"), Date(\"2025-01-01\")] | max"),
        "2026-10-24"
    );
}

#[test]
fn durations_add_and_subtract_and_nothing_else_does() {
    assert_eq!(shown("Duration(\"PT1H\") + Duration(\"PT30M\")"), "PT1H30M");
    assert_eq!(shown("Duration(\"PT1H\") - Duration(\"PT2H\")"), "-PT1H");
    let error = hql::check("Date(\"2026-10-24\") + Date(\"2026-10-24\")").unwrap_err();
    assert!(error.message.contains("Add"), "{}", error.message);
}

#[test]
fn calendar_arithmetic_clamps_to_the_end_of_a_month() {
    for (source, expected) in [
        ("Date(\"2026-01-31\").add_months(1)", "2026-02-28"),
        ("Date(\"2024-01-31\").add_months(1)", "2024-02-29"),
        ("Date(\"2024-02-29\").add_years(1)", "2025-02-28"),
        ("Date(\"2026-12-31\").add_days(1)", "2027-01-01"),
        (
            "DateTime(\"2026-01-31T09:30:00\").add_months(1)",
            "2026-02-28T09:30:00",
        ),
        (
            "Date(\"2026-10-24\").days_since(Date(\"2026-01-01\"))",
            "296",
        ),
        (
            "Date(\"2026-01-01\").days_since(Date(\"2026-10-24\"))",
            "-296",
        ),
        (
            "dates(Date(\"2026-10-30\"), Date(\"2026-11-02\"))",
            "[2026-10-30, 2026-10-31, 2026-11-01]",
        ),
        ("dates(Date(\"2026-11-02\"), Date(\"2026-10-30\"))", "[]"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn parts_read_the_iso_calendar() {
    // 2026-10-24 is a Saturday; 2026-12-31 is the Thursday of week 53, and
    // 2024-12-30 the Monday of the first week of 2025.
    assert_eq!(
        shown(
            "d = Date(\"2026-10-24\")\n[d.year(), d.month(), d.day(), d.weekday(), d.day_of_year()]"
        ),
        "[2026, 10, 24, 6, 297]"
    );
    assert_eq!(
        shown(
            "[Date(\"2026-12-31\").iso_week(), Date(\"2027-01-01\").iso_week(), Date(\"2024-12-30\").iso_week(), Date(\"2024-12-30\").iso_year()]"
        ),
        "[53, 53, 1, 2025]"
    );
    assert_eq!(
        shown(
            "t = DateTime(\"2026-10-24T09:30:15.5\")\n[t.hour(), t.minute(), t.second(), t.nanosecond(), t.year()]"
        ),
        "[9, 30, 15, 500000000, 2026]"
    );
}

#[test]
fn an_instant_reads_unix_time_toward_negative_infinity() {
    assert_eq!(
        shown(
            "i = Instant(unix_millis = -1)\n[i.unix_seconds(), i.unix_millis(), Instant(\"2026-10-24T07:30:00Z\").unix_seconds()]"
        ),
        "[-1, -1, 1792827000]"
    );
    assert_eq!(
        shown("[Duration(\"PT1.5S\").total_seconds(), Duration(\"-PT1.0015S\").total_seconds()]"),
        "[1.5, -1.0015]"
    );
    assert_eq!(shown("Duration(\"-PT1.0015S\").total_millis()"), "-1001");
}

#[test]
fn a_zone_resolves_a_gap_forward_and_an_overlap_to_the_earlier_instant() {
    let berlin = "Zone(\"Europe/Berlin\")";
    for (source, expected) in [
        // 02:30 does not happen on 2026-03-29 in Berlin: it is 03:30 CEST.
        (
            format!("DateTime(\"2026-03-29T02:30:00\").in_zone({berlin})"),
            "2026-03-29T01:30:00Z",
        ),
        // 02:30 happens twice on 2026-10-25: the first, in CEST.
        (
            format!("DateTime(\"2026-10-25T02:30:00\").in_zone({berlin})"),
            "2026-10-25T00:30:00Z",
        ),
        (
            format!("Instant(\"2026-10-24T07:30:00Z\").at({berlin})"),
            "2026-10-24T09:30:00",
        ),
        (
            format!("i = Instant(\"2026-10-24T07:30:00Z\")\ni.at({berlin}).in_zone({berlin}) == i"),
            "true",
        ),
        (
            format!("{berlin}.offset(Instant(\"2026-07-01T00:00:00Z\"))"),
            "PT2H",
        ),
        (
            "Zone(\"+05:30\").offset(Instant(\"2026-07-01T00:00:00Z\"))".to_owned(),
            "PT5H30M",
        ),
        (
            "DateTime(\"2026-03-29T01:30:00\").plus(Duration(hours = 1))".to_owned(),
            "2026-03-29T02:30:00",
        ),
    ] {
        assert_eq!(shown(&source), expected, "{source}");
    }
}

#[test]
fn truncation_gives_the_start_of_the_unit() {
    for (source, expected) in [
        ("Date(\"2026-11-12\").truncate(Week)", "2026-11-09"),
        ("Date(\"2026-11-12\").truncate(Month)", "2026-11-01"),
        ("Date(\"2026-11-12\").truncate(Quarter)", "2026-10-01"),
        ("Date(\"2026-11-12\").truncate(Year)", "2026-01-01"),
        ("Date(\"2026-11-12\").truncate(Hour)", "2026-11-12"),
        (
            "DateTime(\"2026-11-12T09:47:13.25\").truncate(Minute)",
            "2026-11-12T09:47:00",
        ),
        (
            "DateTime(\"2026-11-12T09:47:13.25\").truncate(Day)",
            "2026-11-12T00:00:00",
        ),
        (
            "Instant(\"2026-10-25T12:00:00Z\").truncate(Day)",
            "2026-10-25T00:00:00Z",
        ),
        // Berlin's midnight on the day its clocks go back is 22:00 UTC
        // the day before, in CEST.
        (
            "Instant(\"2026-10-25T12:00:00Z\").truncate(Day, Zone(\"Europe/Berlin\"))",
            "2026-10-24T22:00:00Z",
        ),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn a_pattern_writes_and_reads_back_what_it_writes() {
    assert_eq!(
        shown("Date(\"2026-10-24\").fmt(\"%A, %d %B %Y\")"),
        "Saturday, 24 October 2026"
    );
    assert_eq!(
        shown(
            "Instant(\"2026-10-24T07:30:00Z\").fmt(\"%Y-%m-%d %H:%M %z\", Zone(\"Europe/Berlin\"))"
        ),
        "2026-10-24 09:30 +0200"
    );
    assert_eq!(
        shown("d = Date(\"2026-10-24\")\nd.fmt(\"%d.%m.%Y\").parse_date(\"%d.%m.%Y\") == some(d)"),
        "true"
    );
    assert_eq!(
        shown("\"24/10/2026 09:30\".parse_datetime(\"%d/%m/%Y %H:%M\")"),
        "2026-10-24T09:30:00"
    );
    assert_eq!(
        shown("\"2026-10-24 09:30 +0200\".parse_instant(\"%Y-%m-%d %H:%M %z\")"),
        "2026-10-24T07:30:00Z"
    );
    assert_eq!(shown("\"yesterday\".parse_date(\"%Y-%m-%d\")"), "none");
    assert!(refused("Date(\"2026-10-24\").fmt(\"%q\")").contains("`%q`"));
}

#[test]
fn the_clock_is_read_once_per_evaluation() {
    assert_eq!(
        shown("[now() == now(), today() == today()]"),
        "[true, true]"
    );
}

#[test]
fn now_and_zone_fix_the_clock() {
    let program = "import time\n[str(now()), str(today()), local_zone().name, str(today(Zone(\"America/New_York\")))]";
    assert_eq!(
        hql(&[
            "--now",
            "2026-10-24T23:30:00Z",
            "--zone",
            "Europe/Berlin",
            "eval",
            program
        ]),
        "[\"2026-10-24T23:30:00Z\", \"2026-10-25\", \"Europe/Berlin\", \"2026-10-24\"]\n"
    );
}

/// A declared time field reads its ISO 8601 text from JSON, YAML and CSV;
/// a field not declared stays the text it is.
#[test]
fn a_declared_field_reads_its_text_and_is_written_as_it() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(
        root.join("events.jsonl"),
        "{\"name\": \"launch\", \"on\": \"2026-10-24\", \"at\": \"2026-10-24T09:30:00+02:00\", \"took\": \"PT1H30M\"}\n\
         {\"name\": \"review\", \"on\": \"2026-03-02\", \"at\": \"2026-03-02T12:00:00Z\", \"took\": \"PT45M\"}\n",
    )
    .unwrap();
    fs::write(
        root.join("events.csv"),
        "name,on\nlaunch,2026-10-24\nreview,2026-03-02\n",
    )
    .unwrap();
    hql::io::set_host(hql::io::Policy::Open, root.to_path_buf());
    let vault = vault::Vault::empty();
    let run = |source: &str| {
        let outcome = hql::run(source, &vault, Mode::Strict);
        outcome
            .value
            .unwrap_or_else(|| panic!("{source}: {:?}", outcome.diagnostics))
    };
    let declared = "type Event {\n    name : String\n    on : Date\n    at : Instant\n    took : Duration\n}\n";
    let sorted = run(&format!(
        "{declared}events : Seq<Event> = source(file(\"events.jsonl\"))\n\
         events | sort(by = e => e.on) | map(e => {{name: e.name, on: e.on, at: e.at, took: e.took}})"
    ));
    assert_eq!(
        sorted.to_json(),
        serde_json::json!([
            {"name": "review", "on": "2026-03-02", "at": "2026-03-02T12:00:00Z", "took": "PT45M"},
            {"name": "launch", "on": "2026-10-24", "at": "2026-10-24T07:30:00Z", "took": "PT1H30M"},
        ])
    );
    let csv = run("type Row {\n    name : String\n    on : Date\n}\n\
         rows : Seq<Row> = source(file(\"events.csv\"))\n\
         rows | map(r => r.on) | max");
    assert_eq!(csv.to_string(), "2026-10-24");
    // Undeclared, the same text is a String.
    let untyped = run("source(file(\"events.jsonl\")) | map(e => e.on)");
    assert_eq!(untyped.to_string(), "[\"2026-10-24\", \"2026-03-02\"]");
    let refusal = hql::run(
        "type Row {\n    on : Date\n}\nrows : Seq<Row> = source(file(\"events.jsonl\"))\nrows | map(r => r.on)",
        &vault,
        Mode::Strict,
    );
    assert!(refusal.value.is_none());
    hql::io::set_host(hql::io::Policy::Open, std::path::PathBuf::from("."));
}

#[test]
fn every_format_writes_a_time_value_as_its_iso_text() {
    let program = "{on: Date(\"2026-10-24\"), at: Instant(\"2026-10-24T07:30:00Z\"), took: Duration(\"PT1H30M\")}";
    assert_eq!(
        hql(&["--to", "json", "--compact", "eval", program]),
        "{\"at\":\"2026-10-24T07:30:00Z\",\"on\":\"2026-10-24\",\"took\":\"PT1H30M\"}\n"
    );
    assert_eq!(
        hql(&["--to", "yaml", "eval", program]),
        "at: 2026-10-24T07:30:00Z\non: 2026-10-24\ntook: PT1H30M\n\n"
    );
    assert_eq!(
        hql(&["--to", "csv", "eval", &format!("[{program}]")]),
        "at,on,took\r\n2026-10-24T07:30:00Z,2026-10-24,PT1H30M\n"
    );
}

/// A time field written to an SQLite table is its ISO 8601 text, and read
/// back from the table as the value it was.
#[test]
fn an_sqlite_table_keeps_a_time_value_as_its_text() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("hql.toml"),
        "[endpoint.log]\nkind = \"sqlite\"\npath = \"log.sqlite\"\ntable = \"log\"\n",
    )
    .unwrap();
    let vault = vault::load(directory.path()).unwrap();
    let declared =
        "type Entry {\n    id : Int\n    on : Date\n    at : Instant\n    took : Duration\n}\n";
    let written = hql::run(
        &format!(
            "{declared}[Entry(id = 1, on = Date(\"2026-10-24\"), at = Instant(\"2026-10-24T07:30:00Z\"), took = Duration(\"PT5M\")), \
             Entry(id = 2, on = Date(\"2025-01-02\"), at = Instant(\"2025-01-02T00:00:00Z\"), took = Duration(\"PT1H\"))] \
             | target(\"log\")\n\"written\""
        ),
        &vault,
        Mode::Strict,
    );
    assert_eq!(
        written.value.map(|value| value.to_string()).as_deref(),
        Some("written"),
        "{:?}",
        written.diagnostics
    );
    let database = rusqlite::Connection::open(directory.path().join("log.sqlite")).unwrap();
    let on: String = database
        .query_row(
            "SELECT on_ FROM (SELECT \"on\" AS on_ FROM log WHERE id = 1)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(on, "2026-10-24");
    let read = hql::run(
        &format!(
            "{declared}entries : Seq<Entry> = source(\"log\")\n\
             entries | sort(by = e => e.on) | map(e => [str(e.on), str(e.at), str(e.took)])"
        ),
        &vault,
        Mode::Strict,
    );
    assert_eq!(
        read.value.map(|value| value.to_string()).as_deref(),
        Some(
            "[[\"2025-01-02\", \"2025-01-02T00:00:00Z\", \"PT1H\"], [\"2026-10-24\", \"2026-10-24T07:30:00Z\", \"PT5M\"]]"
        ),
        "{:?}",
        read.diagnostics
    );
}
