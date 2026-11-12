//! The `Sqlite` transport of `io.db.sqlite` (HQL-0047, HQL-0048): one table
//! of a database file, constructed in a program with no configured
//! endpoint. It takes no format, reads as rows, and writes a declared
//! record a column per field. Every case runs against a file in a
//! temporary directory.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::path::Path;
use tempfile::{TempDir, tempdir};

const ORDER: &str =
    "type Order { id : Int, customer : String, total : Decimal, paid : Bool, note? : String }\n";

fn empty() -> (TempDir, Vault) {
    let dir = tempdir().unwrap();
    let vault = vault::load(dir.path()).unwrap();
    (dir, vault)
}

/// A program's value as compact JSON.
fn value(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(!outcome.failed(), "{source}: {:?}", outcome.diagnostics);
    outcome
        .value
        .map(|value| value.to_string())
        .unwrap_or_default()
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

fn database(dir: &Path) -> String {
    dir.join("shop.db").display().to_string()
}

fn seeded(dir: &Path, vault: &Vault) -> String {
    let path = database(dir);
    value(
        &format!(
            "import io.db.sqlite\n{ORDER}\
             [Order(id = 2, customer = \"grace\", total = 42.00, paid = false), \
              Order(id = 1, customer = \"ada\", total = 120.50, paid = true, note = \"it's\")]\n\
             | target(sqlite({path:?}, table = \"orders\"))"
        ),
        vault,
    );
    path
}

#[test]
fn a_record_written_to_a_table_reads_back_as_the_record() {
    let (dir, vault) = empty();
    let path = seeded(dir.path(), &vault);
    let read = value(
        &format!(
            "import io.db.sqlite\n{ORDER}\
             orders : Seq<Order> = source(sqlite({path:?}, table = \"orders\", order = [\"id\"]))\n\
             orders | map(o => [o.id, o.total, o.paid])"
        ),
        &vault,
    );
    assert_eq!(read, "[[1, 120.50, true], [2, 42.00, false]]");
    // A record that reads some of the columns reads only those.
    let some = value(
        &format!(
            "import io.db.sqlite\ntype Who {{ id : Int, customer : String }}\n\
             who : Seq<Who> = source(sqlite({path:?}, table = \"orders\", order = [\"id\"]))\n\
             who | map(w => w.customer)"
        ),
        &vault,
    );
    assert_eq!(some, r#"["ada", "grace"]"#);
    // Without an annotation a row is the table's own values.
    let rows = value(
        &format!(
            "import io.db.sqlite\n\
             source(sqlite({path:?}, \"orders\", [\"id\"])) | map(r => r.paid)"
        ),
        &vault,
    );
    assert_eq!(rows, "[1, 0]");
}

#[test]
fn a_table_is_a_seq_its_rows_may_be_tapped_and_a_write_replaces_it() {
    let (dir, vault) = empty();
    let path = seeded(dir.path(), &vault);
    let types = hql::check_in(
        &format!("import io.db.sqlite\nsource(sqlite({path:?}, table = \"orders\"))"),
        &vault,
    )
    .unwrap();
    assert_eq!(types.to_string(), "Seq<Data>");
    let tapped = value(
        &format!(
            "import io.db.sqlite\n{ORDER}\
             orders : Stream<Order> = tap(sqlite({path:?}, table = \"orders\"))\n\
             orders | map(o => o.id) | take(1)"
        ),
        &vault,
    );
    assert_eq!(tapped, "[2]");
    // A second write replaces the rows the first one wrote.
    value(
        &format!(
            "import io.db.sqlite\n{ORDER}\
             [Order(id = 9, customer = \"linus\", total = 7.99, paid = true)]\n\
             | target(sqlite({path:?}, table = \"orders\"))"
        ),
        &vault,
    );
    let ids = value(
        &format!(
            "import io.db.sqlite\nsource(sqlite({path:?}, table = \"orders\")) | map(r => r.id)"
        ),
        &vault,
    );
    assert_eq!(ids, "[9]");
}

#[test]
fn a_table_takes_no_format_rows_of_a_declared_record_and_no_sink() {
    let (_dir, vault) = empty();
    let error = refusal(
        "import io.db.sqlite\nsource(sqlite(\"x.db\", table = \"t\"), Json)",
        &vault,
    );
    assert!(
        error.contains("rows of named columns, not bytes"),
        "{error}"
    );
    let error = refusal(
        "import io.db.sqlite\n[{a: 1}] | target(sqlite(\"x.db\", table = \"t\"))",
        &vault,
    );
    assert!(error.contains("rows of a declared record"), "{error}");
    let error = refusal(
        &format!(
            "import io.db.sqlite\n{ORDER}\
             [Order(id = 1, customer = \"a\", total = 1.0, paid = true)] \
             | sink(sqlite(\"x.db\", table = \"t\"))"
        ),
        &vault,
    );
    assert!(error.contains("written whole by `target`"), "{error}");
}

#[test]
fn the_constructor_is_imported_by_its_module_and_is_a_direct_row_transport() {
    let (_dir, vault) = empty();
    let error = refusal(r#"source(sqlite("x.db", table = "t"))"#, &vault);
    assert!(error.contains("import io.db.sqlite"), "{error}");
    for supertype in ["Direct", "RowTransport", "Replayable", "Writable"] {
        let checked = hql::check_in(
            &format!("import io.db.sqlite\nsqlite(\"x.db\", table = \"t\") | is({supertype})"),
            &vault,
        )
        .unwrap();
        assert_eq!(checked.to_string(), "Bool", "{supertype}");
    }
    let error = refusal(
        "import io.db.sqlite\nsource(sqlite(\"missing.db\", table = \"nope\"))",
        &vault,
    );
    assert!(error.contains("table `nope`"), "{error}");
}
