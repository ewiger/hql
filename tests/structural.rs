//! Structural types (HQL-0030): a plain record is closed, one narrowing
//! `Data` or `DataContract` is an open view, and `data | T` reads a `Data`
//! as a declared type without changing it. A contract (HQL-0032) adds rules
//! and defaults, and every value read as one holds its rules.

use hql::{check, eval};

const USER: &str = "type User : Data = {\n    id: String\n    name: String\n}\n";
const CONTACT: &str = "type Contact : Data = {\n    phone: String\n}\n";
const POINT: &str = "type Point = {\n    x: Float\n    y: Float\n}\n";
const ALICE: &str = "d : Data = {id: \"42\", name: \"Alice\", phone: \"+41\"}\n";

fn value(source: &str) -> String {
    eval(source)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn failure(source: &str) -> String {
    eval(source)
        .err()
        .unwrap_or_else(|| panic!("{source} evaluated"))
        .message()
}

#[test]
fn an_open_view_reads_its_fields_and_tolerates_the_rest() {
    assert_eq!(
        value(&format!("{USER}{ALICE}u = d | User\n[u.id, u.name]")),
        r#"["42", "Alice"]"#
    );
    assert_eq!(
        check(&format!("{USER}{ALICE}d | User"))
            .unwrap()
            .to_string(),
        "User"
    );
    // Only the declared surface is visible to the checker.
    let hidden = check(&format!("{USER}{ALICE}u = d | User\nu.phone"))
        .unwrap_err()
        .message();
    assert!(hidden.contains("no field `phone`"), "{hidden}");
}

#[test]
fn typing_keeps_the_data_and_a_second_view_reads_it() {
    assert_eq!(
        value(&format!("{USER}{CONTACT}{ALICE}(d | User | Contact).phone")),
        "+41"
    );
    assert_eq!(
        value(&format!("{USER}{ALICE}(d | User | Data).phone")),
        "\"+41\""
    );
}

#[test]
fn a_closed_type_is_exactly_its_fields() {
    assert_eq!(
        value(&format!("{POINT}d : Data = {{x: 1.0, y: 2.0}}\nd | Point")),
        "Point(x = 1.0, y = 2.0)"
    );
    let extra = failure(&format!(
        "{POINT}d : Data = {{x: 1.0, y: 2.0, z: 3.0}}\nd | Point"
    ));
    assert!(
        extra.contains("field `z`: Point declares no such field"),
        "{extra}"
    );
    // A plain body, `type A {…}`, is the same closed type as `type A = {…}`.
    let braces = failure("type Q {\n    x: Int\n}\nd : Data = {x: 1, y: 2}\nd | Q");
    assert!(braces.contains("field `y`"), "{braces}");
}

#[test]
fn optional_fields_say_nothing_of_undeclared_ones() {
    let closed = "type Mail = {\n    id: String\n    email?: String\n}\n";
    assert_eq!(
        value(&format!(
            "{closed}d : Data = {{id: \"1\"}}\n(d | Mail).email"
        )),
        "none"
    );
    assert!(
        failure(&format!(
            "{closed}d : Data = {{id: \"1\", country: \"CH\"}}\nd | Mail"
        ))
        .contains("field `country`")
    );
    let open = "type Mail : Data = {\n    id: String\n    email?: String\n}\n";
    assert_eq!(
        value(&format!(
            "{open}d : Data = {{id: \"1\", country: \"CH\"}}\n(d | Mail).id"
        )),
        "1"
    );
}

#[test]
fn every_field_that_does_not_read_is_reported_at_once() {
    let user =
        "type User : DataContract = {\n    id: Int\n    name: String\n    email?: String\n}\n";
    let report = failure(&format!(
        "{user}d : Data = {{id: \"abc\", email: 123}}\nd | User"
    ));
    for says in [
        "field `id`: \"abc\" is not Int",
        "field `name`: it is missing",
        "field `email`: 123 is not String",
    ] {
        assert!(report.contains(says), "{says}: {report}");
    }
}

#[test]
fn a_collection_or_an_optional_is_read_element_by_element() {
    assert_eq!(
        check(&format!("{USER}ds : List<Data> = []\nds | User"))
            .unwrap()
            .to_string(),
        "List<User>"
    );
    assert_eq!(
        value(&format!(
            "{USER}ds : List<Data> = [{{id: \"1\", name: \"a\"}}, {{id: \"2\", name: \"b\", x: 1}}]\n\
             ds | User | map(u => u.id)"
        )),
        r#"["1", "2"]"#
    );
    assert_eq!(
        check(&format!(
            "{USER}t : Data = {{u: {{id: \"1\", name: \"a\"}}}}\nt.u | User"
        ))
        .unwrap()
        .to_string(),
        "Option<User>"
    );
}

#[test]
fn contracts_are_closed_or_open_by_their_category() {
    let payment = "type Payment : TypeContract = {\n    id: String\n}\n";
    assert!(
        failure(&format!(
            "{payment}d : Data = {{id: \"p\", extra: 1}}\nd | Payment"
        ))
        .contains("field `extra`")
    );
    let open = "type Payment : DataContract = {\n    id: String\n}\n";
    assert_eq!(
        value(&format!(
            "{open}d : Data = {{id: \"p\", extra: 1}}\n(d | Payment).id"
        )),
        "p"
    );
    // A DataContract is open by being Data: it needs no `Data` beside it.
    assert_eq!(
        check(&format!("{open}x : Data = Payment(id = \"p\")\nx"))
            .unwrap()
            .to_string(),
        "Data"
    );
}

#[test]
fn the_empty_keyed_literal_takes_a_declared_map_type() {
    assert_eq!(
        check("m : Map<String, Int> = {:}\nm").unwrap().to_string(),
        "Map<String, Int>"
    );
    assert_eq!(
        value("m : Map<String, Int> = {:}\nm == Map<String, Int>([], [])"),
        "true"
    );
    assert_eq!(check("{:}").unwrap().to_string(), "Data");
    assert_eq!(check("d : Data = {:}\nd").unwrap().to_string(), "Data");
}

const ACCOUNT: &str = "type Account : DataContract = {\n    status: String\n    closed_at?: String\n    age?: Int\n    require status == \"closed\" => closed_at?\n    require age? >= 18\n}\n";

#[test]
fn a_contract_rule_reads_an_optional_field_only_where_it_is_present() {
    // `age?` is absent, so `require age? >= 18` does not apply.
    assert_eq!(
        value(&format!(
            "{ACCOUNT}d : Data = {{status: \"open\"}}\n(d | Account).status"
        )),
        "open"
    );
    let broken = failure(&format!(
        "{ACCOUNT}d : Data = {{status: \"closed\", age: 3}}\nd | Account"
    ));
    assert!(
        broken.contains("`require status == \"closed\" => closed_at?` does not hold")
            && broken.contains("`require age? >= 18` does not hold"),
        "{broken}"
    );
}

#[test]
fn a_nested_contract_is_checked_at_its_path() {
    let source = "type Inner : TypeContract = {\n    n: Int\n    require n > 0\n}\n\
                  type Outer : TypeContract = {\n    inner: Inner\n}\n\
                  d : Data = {inner: {n: 0}}\nd | Outer";
    assert!(
        failure(source).contains("field `inner`: `require n > 0` does not hold"),
        "{}",
        failure(source)
    );
}

#[test]
fn a_field_may_be_named_require_or_validate() {
    let source = "type P : TypeContract = {\n    require: Int\n    validate?: Bool\n}\n\
                  d : Data = {require: 1}\n(d | P).require";
    assert_eq!(value(source), "1");
}

#[test]
fn a_decoded_stream_is_checked_one_document_at_a_time() {
    let sightings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/structural/sightings.jsonl"
    );
    let source = format!(
        "type Sighting : DataContract = {{\n    species: String\n    require species != \"wren\"\n}}\n\
         source(file(\"{sightings}\"), Json) | Sighting | map(s => s.species) | List"
    );
    assert!(
        failure(&source).contains("`require species != \"wren\"` does not hold"),
        "{}",
        failure(&source)
    );
}
