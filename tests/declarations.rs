//! `type` declarations: the statement, and what makes one hold.
//!
//! A declaration is checked and yields `Unit`. It introduces no value, so
//! everything here is about whether what a declaration says is coherent.

use hql::{check, diagnostics::Diagnostic, eval, types::Type, values::Value};
use std::fs;
use std::path::Path;

fn refused(source: &str) -> String {
    match check(source) {
        Err(diagnostic) => diagnostic.message(),
        Ok(inferred) => panic!("{source}: expected a refusal, got {inferred}"),
    }
}

#[test]
fn every_file_in_the_standard_library_parses_and_checks() {
    // The folder is walked rather than listed, so a file added to `std/`
    // cannot be silently left unchecked.
    let mut checked = 0;
    for entry in fs::read_dir("std").expect("the standard library") {
        let path = entry.expect("an entry").path();
        if path.extension().and_then(|name| name.to_str()) != Some("hql") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("a readable file");
        assert_eq!(
            check(&source),
            Ok(Type::Unit),
            "{}: {:?}",
            path.display(),
            check(&source)
        );
        checked += 1;
    }
    assert!(checked >= 6, "only {checked} files were checked");
}

#[test]
fn a_declaration_yields_unit_and_the_program_carries_on() {
    assert_eq!(check("type Colour"), Ok(Type::Unit));
    assert_eq!(eval("type Colour"), Ok(Value::Unit));
    assert_eq!(check("type Colour\n1 + 1"), Ok(Type::Int));
    assert_eq!(eval("type Colour\n1 + 1"), Ok(Value::Int(2)));
}

#[test]
fn every_written_form_parses() {
    for source in [
        "type Colour",
        "type Colour <: Data",
        "type Pair<A, B>",
        "type Fruit <: {Data, Orderable}",
        "type Person { name : String }",
        "type Person {\n    name : String\n    born? : Data\n}",
        "type Person { name : String, born? : Data }",
        "type Holder<T> { held : T }",
        "type Narrow <: Card { metadata : Data }",
        "type Pairing<S, T> <: Edge<S, T>",
    ] {
        assert_eq!(check(source), Ok(Type::Unit), "{source}");
    }
}

#[test]
fn a_parent_must_exist() {
    assert!(refused("type Colour <: Nonesuch").contains("unknown type `Nonesuch`"));
    assert!(refused("type Colour { shade : Nonesuch }").contains("unknown type `Nonesuch`"));
    // A parent declared earlier in the same program does exist.
    assert_eq!(check("type Base\ntype Narrow <: Base"), Ok(Type::Unit));
}

#[test]
fn a_declaration_may_restate_a_built_in_type_but_not_contradict_it() {
    // This is what `std/` is: the declarations the binary already holds,
    // written down in one place as source rather than as prose.
    assert_eq!(check("type Card <: Doc"), Ok(Type::Unit));
    assert_eq!(check("type ConceptCard <: {Card, Doc}"), Ok(Type::Unit));
    assert_eq!(check("type Ranking<T> <: Seq<Hit<T>>"), Ok(Type::Unit));

    let contradiction = refused("type Card <: Edge");
    assert!(
        contradiction.contains("`Card` is built in"),
        "{contradiction}"
    );
    assert!(
        contradiction.contains("does not narrow Edge"),
        "{contradiction}"
    );
    assert!(refused("type Ranking<T> <: Seq<T>").contains("contradicts it"));
}

#[test]
fn a_supertype_set_a_value_could_never_satisfy_is_refused() {
    let refusal = refused("type Impossible <: {Int, String}");
    assert!(
        refusal.contains("nothing is both Int and String"),
        "{refusal}"
    );
    // Related parents are redundant rather than contradictory, and a parent
    // the binary does not know is a declaration of its own.
    assert_eq!(check("type Narrow <: {Card, Doc}"), Ok(Type::Unit));
    assert_eq!(
        check("type Idea\ntype Narrow <: {Card, Idea}"),
        Ok(Type::Unit)
    );
}

#[test]
fn a_name_is_declared_once_and_a_field_appears_once() {
    assert!(refused("type Colour\ntype Colour").contains("is declared twice"));
    assert!(
        refused("type Person { name : String, name : Data }").contains("declares `name` twice")
    );
}

#[test]
fn a_type_parameter_is_a_name_no_type_already_has() {
    assert!(refused("type Holder<Card>").contains("`Card` is a type"));
    assert!(refused("type Pair<T, T>").contains("takes `T` twice"));
    // Inside the body a parameter is an ordinary type name, and it takes no
    // arguments of its own.
    assert_eq!(check("type Holder<T> { held : T }"), Ok(Type::Unit));
    assert!(refused("type Holder<T> { held : T<Card> }").contains("takes no arguments"));
}

#[test]
fn arity_is_checked_where_arguments_are_written() {
    assert!(
        refused("type Pair<A, B>\ntype Use { held : Pair<Card> }")
            .contains("takes 2 type arguments")
    );
    // A bare name is the constructor itself — `Graph<Card, Link>` passes
    // `Link`, not a `Link` of something — so it is not an arity failure.
    assert_eq!(
        check("type Pair<A, B>\ntype Use { held : Pair }"),
        Ok(Type::Unit)
    );
}

#[test]
fn narrowing_may_not_turn_a_required_field_into_an_optional_one() {
    // A subtype narrows its parent. Making a required key optional widens the
    // shape, so a value of the subtype could fail to be one of the parent.
    let refusal = refused("type Base { header : Data }\ntype Narrow <: Base { header? : Data }");
    assert!(refusal.contains("`Base` requires `header`"), "{refusal}");
    assert_eq!(
        check("type Base { header? : Data }\ntype Narrow <: Base { header? : Data }"),
        Ok(Type::Unit)
    );
}

#[test]
fn a_declaration_is_a_statement_and_not_an_expression() {
    // `type` only opens a statement; a name may still be bound to a value.
    assert_eq!(check("type = 1\ntype"), Ok(Type::Int));
    assert!(matches!(
        check("1 | type Colour"),
        Err(Diagnostic::Syntax { .. }) | Err(Diagnostic::Name { .. })
    ));
}

#[test]
fn the_standard_library_is_where_the_declarations_live() {
    // Loading every module together must also hold: the files are one library
    // and a name declared twice across them would be a real collision.
    let mut together = String::new();
    for name in [
        "core",
        "collections",
        "doc",
        "graph",
        "knowledge",
        "retrieval",
    ] {
        let path = Path::new("std").join(format!("{name}.hql"));
        together.push_str(&fs::read_to_string(path).expect("a module"));
        together.push('\n');
    }
    assert_eq!(check(&together), Ok(Type::Unit), "{:?}", check(&together));
}

#[test]
fn the_vocabulary_holds_no_name_the_library_no_longer_declares() {
    // `List` is withdrawn: a claim about representation earns a name only
    // where the language tells two representations apart, and HQL does not.
    // The checker must not go on resolving a name `std/` has dropped.
    assert!(refused("cards : List<Card> = 1").contains("unknown type `List`"));
    assert!(refused("type Held <: List<Card>").contains("unknown type `List`"));
    assert_eq!(check("type Held <: Seq<Card>"), Ok(Type::Unit));
}
