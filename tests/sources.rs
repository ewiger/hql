//! Configured data sources: `[sources.<id>]` in a vault's `hql.toml`, read by
//! `source("id")`, typed offline from the configuration and bounded by the
//! run context. See doc/proposals/HQL-0012/README.md.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::fs;
use tempfile::{TempDir, tempdir};

/// A vault with two vault sources, a CSV file, a JSON array and an SQLite
/// table, configured by `config` appended to the sources every case shares.
fn configured(extra: &str) -> (TempDir, Vault) {
    let dir = tempdir().unwrap();
    let root = dir.path();
    for (path, text) in [
        ("field_notes/wren.md", "# Wren\n"),
        ("field_notes/a/kite.md", "# Kite\n"),
        ("field_notes/owl.md", "# Owl\n"),
        ("archive/heron.md", "# Heron\n"),
        ("archive/finch.md", "# Finch\n"),
        ("data/counts.csv", "species,count\nwren,3\nrobin,5\n"),
        (
            "data/sightings.json",
            r#"[{"species": "owl"}, {"species": "kite"}]"#,
        ),
        ("data/scalar.json", "3"),
    ] {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let database = rusqlite::Connection::open(root.join("data/birds.sqlite")).unwrap();
    database
        .execute_batch(
            "create table birds(id integer primary key, name text, wing real);
             insert into birds(id, name, wing) values (2, 'owl', 1.5), (1, 'kite', null);",
        )
        .unwrap();
    fs::write(
        root.join("hql.toml"),
        format!(
            r#"
[sources.field_notes]
kind = "vault"
root = "field_notes"

[sources.archive]
kind = "vault"
root = "archive"

[sources.counts]
kind = "csv"
path = "data/counts.csv"

[sources.sightings]
kind = "json"
path = "data/sightings.json"

[sources.scalar]
kind = "json"
path = "data/scalar.json"

[sources.birds]
kind = "sqlite"
path = "data/birds.sqlite"
table = "birds"
{extra}"#
        ),
    )
    .unwrap();
    let vault = vault::load(root).unwrap();
    (dir, vault)
}

fn value(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    let messages: Vec<_> = outcome
        .diagnostics
        .iter()
        .map(|r| r.message.clone())
        .collect();
    outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {messages:?}"))
        .to_string()
}

fn refusal(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(outcome.failed(), "{source} ran");
    outcome
        .diagnostics
        .iter()
        .map(|report| report.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn two_vault_sources_are_read_side_by_side_in_name_order() {
    let (_dir, vault) = configured("");
    let program = r#"
notes = source("field_notes")
archive = source("archive")
[notes.cards | take(2) | map(c => c.name), archive.cards | take(2) | map(c => c.name)]
"#;
    let first = value(program, &vault);
    assert_eq!(first, r#"[["kite", "owl"], ["finch", "heron"]]"#);
    assert_eq!(value(program, &vault), first);
    assert_eq!(
        hql::check_in(r#"source("field_notes")"#, &vault)
            .unwrap()
            .to_string(),
        "Vault"
    );
}

#[test]
fn a_file_source_is_a_seq_of_its_documents_in_file_order() {
    let (_dir, vault) = configured("");
    assert_eq!(
        hql::check_in(r#"source("counts")"#, &vault)
            .unwrap()
            .to_string(),
        "Seq<Data>"
    );
    assert_eq!(
        value(r#"source("sightings") | map(s => s.species)"#, &vault),
        r#"["owl", "kite"]"#
    );
    // A declared element type decodes each row, a CSV cell as its field
    // wants it.
    assert_eq!(
        value(
            "type Count { species: String, count: Int }\n\
             counts : Seq<Count> = source(\"counts\")\n\
             counts | map(c => c.count * 2)",
            &vault
        ),
        "[6, 10]"
    );
    assert!(refusal(r#"source("scalar")"#, &vault).contains("holds one array"));
}

#[test]
fn a_table_is_read_in_its_configured_order_else_by_rowid() {
    let (_dir, vault) = configured("");
    assert_eq!(
        value(r#"source("birds") | map(b => b.name)"#, &vault),
        r#"["kite", "owl"]"#
    );
    let (_dir, ordered) = configured(
        r#"
[sources.by_wing]
kind = "sqlite"
path = "data/birds.sqlite"
table = "birds"
order = ["wing"]
"#,
    );
    // NULL sorts first in SQLite's order.
    assert_eq!(
        value(r#"source("by_wing") | map(b => b.name)"#, &ordered),
        r#"["kite", "owl"]"#
    );
}

#[test]
fn an_unknown_id_is_a_name_error_naming_the_nearest() {
    let (_dir, vault) = configured("");
    let error = hql::check_in(r#"source("count")"#, &vault).unwrap_err();
    assert_eq!(error.kind, hql::diagnostics::Kind::UnknownName);
    assert!(error.message().contains("did you mean `counts`"), "{error}");
    let far = hql::check_in(r#"source("weather")"#, &vault).unwrap_err();
    assert!(far.message().contains("[sources.<id>]"), "{far}");
}

#[test]
fn an_id_is_a_literal_and_never_a_connection_string() {
    let (_dir, vault) = configured("");
    let computed = hql::check_in("id = \"counts\"\nsource(id)", &vault).unwrap_err();
    assert!(computed.message().contains("literal id"), "{computed}");
    let url = hql::check_in(r#"source("postgres://db/notes")"#, &vault).unwrap_err();
    assert!(url.message().contains("belong in configuration"), "{url}");
    let cased = hql::check_in(r#"source("Field-Notes")"#, &vault).unwrap_err();
    assert!(cased.message().contains("snake_case"), "{cased}");
    let format = hql::check_in(r#"source("counts", Json)"#, &vault).unwrap_err();
    assert!(format.message().contains("name none"), "{format}");
}

#[test]
fn checking_reads_configuration_and_opens_nothing() {
    let (dir, vault) = configured("");
    fs::remove_file(dir.path().join("data/counts.csv")).unwrap();
    fs::remove_dir_all(dir.path().join("field_notes")).unwrap();
    assert!(hql::check_in(r#"source("counts") | take(1)"#, &vault).is_ok());
    assert!(hql::check_in(r#"source("field_notes").cards"#, &vault).is_ok());
    assert!(refusal(r#"source("counts")"#, &vault).contains("source `counts`"));
}

#[test]
fn a_source_over_the_run_contexts_bound_fails_naming_both() {
    let (_dir, vault) = configured("\n[execution]\nmax_elements = 1\n");
    let error = refusal(r#"source("counts")"#, &vault);
    assert!(
        error.contains("source `counts`") && error.contains("max_elements = 1"),
        "{error}"
    );
    let (_dir, vault) = configured("\n[execution]\nmax_bytes = 8\n");
    let error = refusal(r#"source("counts")"#, &vault);
    assert!(error.contains("max_bytes = 8"), "{error}");
}

#[test]
fn a_malformed_configuration_refuses_the_vault() {
    for (entry, says) in [
        (
            "[sources.db]\nkind = \"sqlite\"\npath = \"x\"\ntable = \"t\"\npassword = \"hunter2\"\n",
            "password_env",
        ),
        (
            "[sources.vault]\nkind = \"csv\"\npath = \"x\"\n",
            "`vault` is the vault",
        ),
        (
            "[sources.Notes]\nkind = \"csv\"\npath = \"x\"\n",
            "snake_case",
        ),
        (
            "[sources.db]\nkind = \"postgres\"\n",
            "unknown kind `postgres`",
        ),
        ("[sources.db]\nkind = \"csv\"\n", "names its `path`"),
        (
            "[sources.db]\nkind = \"csv\"\npath = \"x\"\nstart = \"earliest\"\n",
            "no `start`",
        ),
    ] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("hql.toml"), entry).unwrap();
        let error = vault::load(dir.path()).unwrap_err().to_string();
        assert!(error.contains(says), "{entry}: {error}");
    }
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        "[sources.db]\nkind = \"sqlite\"\npath = \"x\"\ntable = \"t\"\npassword_env = \"DB_PASSWORD\"\n",
    )
    .unwrap();
    assert!(vault::load(dir.path()).is_ok());
}

#[test]
fn a_document_reads_the_sources_its_vault_configures() {
    let (dir, _) = configured("");
    let page = dir.path().join("archive/report.hmd");
    fs::write(
        &page,
        "# Report\n\n```hql#eval\nsource(\"counts\") | size\n```\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir.path())
        .args(["render", &page.display().to_string()])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains('2'), "{text}");
}

#[test]
fn a_vaults_cards_are_keyed_by_name() {
    let (_dir, vault) = configured("");
    let notes = r#"notes = source("field_notes")
"#;
    assert_eq!(
        value(
            &format!("{notes}notes.cards.get(\"owl\") | map(c => c.title)"),
            &vault
        ),
        "Owl"
    );
    assert_eq!(
        value(&format!("{notes}get(notes.cards, \"heron\")"), &vault),
        "none"
    );
    assert_eq!(
        hql::check_in(r#"source("archive").cards.get("finch")"#, &vault)
            .unwrap()
            .to_string(),
        "Option<Card>"
    );
    let refused = hql::check_in(r#"[1, 2].get("x")"#, &vault).unwrap_err();
    assert!(refused.message().contains("T : Doc"), "{refused}");
}

#[test]
fn tap_reads_a_finite_source_one_element_at_a_time() {
    let (_dir, vault) = configured("");
    assert_eq!(
        hql::check_in(r#"tap("counts")"#, &vault)
            .unwrap()
            .to_string(),
        "Stream<Data>"
    );
    assert_eq!(
        value(
            r#"tap("sightings") | map(s => s.species) | take(1)"#,
            &vault
        ),
        r#"["owl"]"#
    );
    assert_eq!(
        value(r#"tap("birds") | map(b => b.name) | take(2)"#, &vault),
        r#"["kite", "owl"]"#
    );
    assert_eq!(
        value(
            "type Count { species: String, count: Int }\n\
             counts : Stream<Count> = tap(\"counts\")\n\
             counts | map(c => c.count) | take(2)",
            &vault
        ),
        "[3, 5]"
    );
    let vault_tap = hql::check_in(r#"tap("field_notes")"#, &vault).unwrap_err();
    assert!(vault_tap.message().contains(".cards"), "{vault_tap}");
}

/// HQL-0037: a source bound once and read by several pipelines and the
/// branches of a `fork` is read once, so `max_elements` counts its elements
/// once.
#[test]
fn a_bound_source_counts_its_elements_once() {
    let (_dir, vault) = configured("\n[execution]\nmax_elements = 2\n");
    let program = r#"
counts = source("counts")
parts = counts | fork { first = take(1), all = size() }
[counts | size, parts.all, parts.first | size]
"#;
    assert_eq!(value(program, &vault), "[2, 2, 1]");
}

/// HQL-0035: a source's boundary is written field by field under
/// `[sources.<id>.fields.<field>]`, and a malformed one refuses the vault.
#[test]
fn a_boundary_is_configured_field_by_field() {
    use hql::sources::{Absence, External};
    let (_dir, vault) = configured(
        "\n[sources.people]\nkind = \"json\"\npath = \"data/sightings.json\"\n\
         [sources.people.fields.id]\nname = \"user_id\"\ntype = \"string\"\nposition = 1\n\
         [sources.people.fields.name]\npath = \"profile.display_name\"\nwrite = false\n\
         absent = [\"missing\", \"null\"]\n",
    );
    let people = vault.sources.get("people").expect("a configured source");
    let id = &people.fields["id"];
    assert_eq!(id.external, Some(External::Name("user_id".to_owned())));
    assert_eq!(id.external_type.as_deref(), Some("string"));
    assert_eq!(id.position, Some(1));
    let name = &people.fields["name"];
    assert_eq!(
        name.external,
        Some(External::Path(vec![
            "profile".to_owned(),
            "display_name".to_owned()
        ]))
    );
    assert_eq!(name.writable, Some(false));
    assert_eq!(name.readable, None);
    assert_eq!(name.absent, Some(vec![Absence::Missing, Absence::Null]));
    let source = "[sources.db]\nkind = \"csv\"\npath = \"x\"\n";
    for (fields, says) in [
        ("[sources.db.fields]\nid = 3\n", "is a table"),
        (
            "[sources.db.fields.id]\ncolumn = \"x\"\n",
            "takes no `column`",
        ),
        ("[sources.db.fields.id]\nname = 3\n", "is a string"),
        (
            "[sources.db.fields.id]\nname = \"a\"\npath = \"b\"\n",
            "once",
        ),
        ("[sources.db.fields.id]\npath = \"a.b\"\n", "does not nest"),
        ("[sources.db.fields.id]\nposition = 0\n", "counted from 1"),
        (
            "[sources.db.fields.id]\nread = \"yes\"\n",
            "`true` or `false`",
        ),
        (
            "[sources.db.fields.id]\nabsent = [\"blank\"]\n",
            "what reads as absent",
        ),
        (
            "[sources.db.fields.id.sqlite]\ntype = \"TEXT\"\n",
            "this source is `csv`",
        ),
        (
            "[sources.db.fields.id.csv]\nquote = \"'\"\n",
            "owns no field key `quote`",
        ),
    ] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("hql.toml"), format!("{source}{fields}")).unwrap();
        let error = vault::load(dir.path()).unwrap_err().to_string();
        assert!(error.contains(says), "{fields}: {error}");
    }
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        "[sources.notes]\nkind = \"vault\"\nroot = \"notes\"\n[sources.notes.fields.id]\nname = \"x\"\n",
    )
    .unwrap();
    let error = vault::load(dir.path()).unwrap_err().to_string();
    assert!(error.contains("no `fields`"), "{error}");
}

/// HQL-0035: a source read as a declared record goes through its boundary.
/// A table reads only the columns the boundary reads, so a closed record
/// leaving `wing` out reads it; a CSV header without a required field, and
/// a boundary naming a field the element type lacks, are refused.
#[test]
fn a_source_is_read_through_its_boundary() {
    let (_dir, vault) = configured(
        "\n[sources.birds.fields.label]\nname = \"name\"\n\
         [sources.tallies]\nkind = \"csv\"\npath = \"data/counts.csv\"\n\
         [sources.tallies.fields.total]\nname = \"amount\"\n",
    );
    let birds = "type Bird {\n    id : Int\n    label : String\n}\n\
                 seen : Seq<Bird> = source(\"birds\")\nseen | map(b => b.label) | List";
    assert_eq!(value(birds, &vault), r#"["kite", "owl"]"#);
    let tallies = "type Tally {\n    species : String\n    total : Int\n}\n\
                   seen : Seq<Tally> = source(\"tallies\")\nseen | size";
    let error = refusal(tallies, &vault);
    assert!(
        error.contains("field `total`") && error.contains("no field `amount`"),
        "{error}"
    );
    let (_dir, vault) = configured(
        "\n[sources.typed]\nkind = \"csv\"\npath = \"data/counts.csv\"\nelement = \"Tally\"\n\
         [sources.typed.fields.colour]\nname = \"c\"\n",
    );
    let typed = "type Tally {\n    species : String\n    count : Int\n}\nsource(\"typed\") | size";
    let refused = hql::check_in(typed, &vault).unwrap_err().message();
    assert!(refused.contains("field `colour`"), "{refused}");
}

/// HQL-0035: a JSON `null` in an optional field is absent by default, and
/// refused where the field's boundary reads only a missing key as absent.
#[test]
fn a_null_reads_as_absent_unless_the_boundary_says_otherwise() {
    let (dir, _) = configured("");
    fs::write(
        dir.path().join("data/readings.json"),
        r#"[{"station": "north", "reading": null}]"#,
    )
    .unwrap();
    let reading = "type Reading {\n    station : String\n    reading? : Int\n}\n\
                   seen : Seq<Reading> = source(\"readings\")\nseen | map(r => r.reading) | List";
    let config = |extra: &str| {
        fs::write(
            dir.path().join("hql.toml"),
            format!("[sources.readings]\nkind = \"json\"\npath = \"data/readings.json\"\n{extra}"),
        )
        .unwrap();
        vault::load(dir.path()).unwrap()
    };
    assert_eq!(value(reading, &config("")), "[none]");
    let error = refusal(
        reading,
        &config("[sources.readings.fields.reading]\nabsent = [\"missing\"]\n"),
    );
    assert!(
        error.contains("field `reading`") && error.contains("holds null"),
        "{error}"
    );
}

/// HQL-0035: `target("id")` writes a `Table<R>` or `Seq<R>` through the
/// boundary, the columns in position order, a field with `write = false`
/// left out, in the target's write mode.
#[test]
fn a_configured_target_is_written_through_its_boundary() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("out")).unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        r#"
[sources.ledger]
kind = "csv"
path = "out/ledger.csv"
mode = "append"
[sources.ledger.fields.count]
name = "Total"
position = 1

[sources.people]
kind = "json"
path = "out/people.json"
[sources.people.fields.name]
path = "profile.display_name"

[sources.birds]
kind = "sqlite"
path = "out/birds.sqlite"
table = "birds"
[sources.birds.fields.note]
write = false

[sources.fresh]
kind = "json_lines"
path = "out/fresh.jsonl"
mode = "create"
"#,
    )
    .unwrap();
    let vault = vault::load(dir.path()).unwrap();
    let types = "type Tally {\n    species : String\n    count : Int\n}\n\
                 type Person {\n    id : String\n    name : String\n    balance : Decimal\n}\n\
                 type Bird {\n    id : Int\n    name : String\n    note? : String\n}\n";
    let program = format!(
        "{types}a = [Tally(species = \"wren\", count = 2)] | target(\"ledger\")\n\
         b = [Tally(species = \"kite\", count = 5)] | Table | target(\"ledger\")\n\
         c = [Person(id = \"u-1\", name = \"Ada\", balance = 12.50)] | target(\"people\")\n\
         d = [Bird(id = 1, name = \"kite\", note = \"x\"), Bird(id = 2, name = \"owl\")] \
         | target(\"birds\")\n\
         e = [Tally(species = \"owl\", count = 1)] | target(\"fresh\")\n\"written\""
    );
    assert_eq!(value(&program, &vault), "written");
    let read = |file: &str| fs::read_to_string(dir.path().join("out").join(file)).unwrap();
    assert_eq!(read("ledger.csv"), "Total,species\n2,wren\n5,kite\n");
    let people: serde_json::Value = serde_json::from_str(&read("people.json")).unwrap();
    assert_eq!(
        people,
        serde_json::json!([{"id": "u-1", "balance": "12.50", "profile": {"display_name": "Ada"}}])
    );
    let database = rusqlite::Connection::open(dir.path().join("out/birds.sqlite")).unwrap();
    let columns: Vec<String> = database
        .prepare("SELECT * FROM birds")
        .unwrap()
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(columns, ["id", "name"]);
    // `create` refuses what exists; `append` adds rows after the header;
    // `replace`, the default, writes the table again.
    let again = format!(
        "{types}a = [Tally(species = \"owl\", count = 1)] | target(\"ledger\")\n\
         d = [Bird(id = 3, name = \"heron\")] | target(\"birds\")\n\"written\""
    );
    assert_eq!(value(&again, &vault), "written");
    assert_eq!(read("ledger.csv"), "Total,species\n2,wren\n5,kite\n1,owl\n");
    let rows: i64 = database
        .query_row("SELECT count(*) FROM birds", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);
    let error = refusal(
        &format!("{types}[Tally(species = \"owl\", count = 1)] | target(\"fresh\")"),
        &vault,
    );
    assert!(error.contains("`mode = \"create\"`"), "{error}");
    for (program, says) in [
        ("[1, 2] | target(\"ledger\")", "declared record"),
        (
            "[Tally(species = \"a\", count = 1)] | target(\"ledger\", Json)",
            "format is its kind's",
        ),
    ] {
        let refused = hql::check_in(&format!("{types}{program}"), &vault)
            .unwrap_err()
            .message();
        assert!(refused.contains(says), "{program}: {refused}");
    }
}

/// HQL-0035: a source's strictness decides what a read does with an
/// external field no HQL field reads, and leaves the type's openness alone.
#[test]
fn strictness_decides_what_an_unmapped_field_does() {
    let closed = "type Species {\n    species : String\n}\n\
                  seen : Seq<Species> = source(\"tallies\")\nseen | map(s => s.species) | List";
    let with = |strictness: &str| {
        configured(&format!(
            "\n[sources.tallies]\nkind = \"csv\"\npath = \"data/counts.csv\"\n{strictness}\n"
        ))
    };
    // `compatible`, the default: the closed record refuses `count`.
    let (_dir, vault) = with("");
    assert!(refusal(closed, &vault).contains("count"));
    let (_dir, vault) = with("strictness = \"open\"");
    assert_eq!(value(closed, &vault), r#"["wren", "robin"]"#);
    let (_dir, vault) = with("strictness = \"warn\"");
    let outcome = hql::run(closed, &vault, Mode::Strict);
    assert_eq!(
        outcome.value.map(|value| value.to_string()).as_deref(),
        Some(r#"["wren", "robin"]"#)
    );
    let warnings: Vec<&str> = outcome
        .diagnostics
        .iter()
        .map(|report| report.message.as_str())
        .collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("no field `count`"), "{warnings:?}");
    let (_dir, vault) = with("strictness = \"strict\"");
    let error = refusal(closed, &vault);
    assert!(
        error.contains("strict") && error.contains("`count`"),
        "{error}"
    );
    // A table's unread columns meet the source's strictness before any row.
    let (_dir, vault) = configured("strictness = \"strict\"\n");
    let birds = "type Bird {\n    id : Int\n    name : String\n}\n\
                 seen : Seq<Bird> = source(\"birds\")\nseen | size";
    let error = refusal(birds, &vault);
    assert!(error.contains("`wing`"), "{error}");
}

#[test]
fn a_table_reads_as_its_column_types_say() {
    let extra = r#"
[sources.flags]
kind = "sqlite"
path = "data/birds.sqlite"
table = "flags"

[sources.prices]
kind = "sqlite"
path = "data/birds.sqlite"
table = "prices"

[sources.texts]
kind = "sqlite"
path = "data/birds.sqlite"
table = "texts"

[sources.named]
kind = "sqlite"
path = "data/birds.sqlite"
table = "named"

[sources.keyed]
kind = "sqlite"
path = "data/birds.sqlite"
table = "keyed"

[sources.blobs]
kind = "sqlite"
path = "data/birds.sqlite"
table = "blobs"
"#;
    let (dir, vault) = configured(extra);
    rusqlite::Connection::open(dir.path().join("data/birds.sqlite"))
        .unwrap()
        .execute_batch(
            "create table flags(id integer, ok integer, wing integer);
             insert into flags values (1, 1, 3), (2, 0, 4);
             create table prices(id integer, price numeric);
             insert into prices values (1, '12345678901234567.89');
             create table texts(id integer, price text);
             insert into texts values (1, '12345678901234567.89');
             create view named as select id, name from birds;
             create table keyed(id integer primary key, name text) without rowid;
             insert into keyed values (2, 'b'), (1, 'a');
             create table blobs(id integer, data blob);
             insert into blobs values (1, x'ff00c3');",
        )
        .unwrap();
    let flag = "type F { id : Int, ok : Bool, wing : Float }\n";
    assert_eq!(
        value(
            &format!(
                "{flag}fs : Seq<F> = source(\"flags\")\nfs | map(f => [str(f.ok), str(f.wing)]) | List"
            ),
            &vault
        ),
        r#"[["true", "3.0"], ["false", "4.0"]]"#
    );
    let price = "type P { id : Int, price : Decimal }\n";
    assert!(
        refusal(
            &format!("{price}ps : Seq<P> = source(\"prices\")\nps | List"),
            &vault
        )
        .contains("lossy representation")
    );
    assert_eq!(
        value(
            &format!("{price}ps : Seq<P> = source(\"texts\")\nps | map(p => p.price) | List"),
            &vault
        ),
        "[12345678901234567.89]"
    );
    assert_eq!(
        value("xs : Seq<Data> = source(\"named\")\nxs | size", &vault),
        "2"
    );
    assert_eq!(
        value("xs : Seq<Data> = source(\"keyed\")\nxs | size", &vault),
        "2"
    );
    let blob = "type B { id : Int, data : String }\n";
    assert!(
        refusal(
            &format!("{blob}bs : Seq<B> = source(\"blobs\")\nbs | List"),
            &vault
        )
        .contains("not UTF-8")
    );
}

#[test]
fn an_unreadable_configuration_is_an_error_not_an_empty_one() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        b"# caf\xe9\n[sources.prices]\nkind = \"json\"\npath = \"p.json\"\n",
    )
    .unwrap();
    let error = match vault::load(dir.path()) {
        Err(error) => error.to_string(),
        Ok(vault) => refusal("xs : Seq<Data> = source(\"prices\")\nxs | List", &vault),
    };
    assert!(error.contains("hql.toml"), "{error}");
    assert!(!error.contains("no source"), "{error}");
}

#[test]
fn a_column_name_matches_without_regard_to_case_and_a_write_needs_a_column() {
    let extra = r#"
[sources.cased]
kind = "sqlite"
path = "data/birds.sqlite"
table = "cased"

[sources.out]
kind = "sqlite"
path = "data/out.sqlite"
table = "o"
[sources.out.fields.id]
write = false
"#;
    let (dir, vault) = configured(extra);
    rusqlite::Connection::open(dir.path().join("data/birds.sqlite"))
        .unwrap()
        .execute_batch(
            "create table cased(ID integer, Name text); insert into cased values (1, 'a');",
        )
        .unwrap();
    assert_eq!(
        value(
            "type T { id : Int, name : String }\nts : Seq<T> = source(\"cased\")\nts | map(t => t.name) | List",
            &vault
        ),
        r#"["a"]"#
    );
    let refused = refusal(
        "type O { id : Int }\nw = [O(id = 1)] | target(\"out\")\n1",
        &vault,
    );
    assert!(
        refused.contains("no field of the record is written"),
        "{refused}"
    );
}

#[test]
fn fields_that_swap_names_or_read_a_path_decode_a_closed_record() {
    let extra = r#"
[sources.swap]
kind = "json"
path = "data/swap.json"
[sources.swap.fields.x]
name = "y"
[sources.swap.fields.y]
name = "x"

[sources.profiles]
kind = "json"
path = "data/profiles.json"
[sources.profiles.fields.id]
name = "user_id"
[sources.profiles.fields.name]
path = "profile.display_name"
"#;
    let (dir, vault) = configured(extra);
    fs::write(
        dir.path().join("data/swap.json"),
        r#"[{"x": "X", "y": "Y"}]"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("data/profiles.json"),
        r#"[{"user_id": "u-1", "profile": {"display_name": "Ada"}}]"#,
    )
    .unwrap();
    assert_eq!(
        value(
            "type S { x : String, y : String }\nss : Seq<S> = source(\"swap\")\nss | map(s => [s.x, s.y]) | List",
            &vault
        ),
        r#"[["Y", "X"]]"#
    );
    assert_eq!(
        value(
            "type P { id : String, name : String }\nps : Seq<P> = source(\"profiles\")\nps | map(p => p.name) | List",
            &vault
        ),
        r#"["Ada"]"#
    );
}

#[test]
fn a_decimal_written_to_a_table_keeps_its_digits() {
    let extra = r#"
[sources.ledger]
kind = "sqlite"
path = "data/ledger.sqlite"
table = "ledger"
"#;
    let (_dir, vault) = configured(extra);
    let declared = "type L { id : Int, amount : Decimal }\n";
    value(
        &format!(
            "{declared}w = [L(id = 1, amount = 12345678901234567.89)] | target(\"ledger\")\n1"
        ),
        &vault,
    );
    assert_eq!(
        value(
            &format!("{declared}ls : Seq<L> = source(\"ledger\")\nls | map(l => l.amount) | List"),
            &vault
        ),
        "[12345678901234567.89]"
    );
}
