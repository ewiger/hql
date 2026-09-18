//! `type` declarations: the statement, and what makes one hold.
//!
//! A declaration is checked and yields `Unit`. It introduces no value, so
//! everything here is about whether what a declaration says is coherent.

use hql::{
    check,
    diagnostics::Diagnostic,
    eval,
    types::{TypeRef, Value},
};
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
            Ok(TypeRef::UNIT),
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
    assert_eq!(check("type Colour"), Ok(TypeRef::UNIT));
    assert_eq!(eval("type Colour"), Ok(Value::Unit));
    assert_eq!(check("type Colour\n1 + 1"), Ok(TypeRef::INT));
    assert_eq!(eval("type Colour\n1 + 1"), Ok(Value::Int(2)));
}

#[test]
fn every_written_form_parses() {
    for source in [
        "type Colour",
        "type Colour : Data",
        "type Pair<A, B>",
        "type Fruit : {Data, Orderable}",
        "type Person { name : String }",
        "type Person {\n    name : String\n    born? : Data\n}",
        "type Person { name : String, born? : Data }",
        "type Holder<T> { held : T }",
        "type Narrow : Card { metadata : Data }",
        "type Pairing<S, T> : Edge<S, T>",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
}

#[test]
fn colon_separates_parents_bounds_and_record_fields() {
    for source in [
        "type X : Card { title : String }",
        "type X : {Card, Doc} { title : String }",
        "type Box<T> { value : T } where T : Orderable",
        "type SortedMap<K, V> : OrderedMap<K, V> where K : Orderable",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
}

#[test]
fn the_former_subtype_operator_is_a_syntax_error() {
    for source in [
        "type X <: Card",
        "type X <: {Card, Doc}",
        "type Box<T> where T <: Orderable",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}"
        );
    }
}

#[test]
fn a_parent_must_exist() {
    assert!(refused("type Colour : Nonesuch").contains("unknown type `Nonesuch`"));
    assert!(refused("type Colour { shade : Nonesuch }").contains("unknown type `Nonesuch`"));
    // A parent declared earlier in the same program does exist.
    assert_eq!(check("type Base\ntype Narrow : Base"), Ok(TypeRef::UNIT));
}

#[test]
fn a_declaration_may_restate_a_built_in_type_but_not_contradict_it() {
    // This is what `std/` is: the declarations the binary already holds,
    // written down in one place as source rather than as prose.
    assert_eq!(check("type Card : Doc"), Ok(TypeRef::UNIT));
    assert_eq!(check("type ConceptCard : {Card, Doc}"), Ok(TypeRef::UNIT));
    assert_eq!(check("type Ranking<T> : Seq<Hit<T>>"), Ok(TypeRef::UNIT));

    let contradiction = refused("type Card : Edge");
    assert!(
        contradiction.contains("`Card` is built in"),
        "{contradiction}"
    );
    assert!(
        contradiction.contains("does not narrow Edge"),
        "{contradiction}"
    );
    assert!(refused("type Ranking<T> : Seq<T>").contains("contradicts it"));
}

#[test]
fn a_supertype_set_a_value_could_never_satisfy_is_refused() {
    let refusal = refused("type Impossible : {Int, String}");
    assert!(
        refusal.contains("nothing is both Int and String"),
        "{refusal}"
    );
    // Related parents are redundant rather than contradictory, and a parent
    // the binary does not know is a declaration of its own.
    assert_eq!(check("type Narrow : {Card, Doc}"), Ok(TypeRef::UNIT));
    assert_eq!(
        check("type Idea\ntype Narrow : {Card, Idea}"),
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn contracts_and_unions_never_exclude_each_other_in_a_supertype_set() {
    // Only two concrete types can share nothing. A contract is satisfied by
    // other types, so two of them are two claims one value can meet — which
    // is what `type Int : {Scalar, Orderable}` relies on.
    for source in [
        "type Stamp : {Scalar, Orderable}",
        "type Flag : {Bool, Orderable}",
        "type Fruit : {Data, Orderable}",
        "type Int : {Scalar, Orderable}",
        "type Bool : Scalar",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
    // The built-in `Bool` is a leaf with no order, and restating it otherwise
    // contradicts the binary.
    assert!(refused("type Bool : {Scalar, Orderable}").contains("does not narrow Orderable"));
    assert!(refused("type Scalar").contains("contradicts its built-in kind"));
}

#[test]
fn a_union_lists_its_alternatives() {
    for source in [
        "type Id = union {Int, String}",
        "type Id = union {\n    Int,\n    String\n}",
        "type Id = union {Int, String,}",
        // A member may apply the union itself: that is what makes a tree.
        "type Json = union {Scalar, Seq<Json>, Map<String, Json>}",
        "type Tree<T> = union {T, Seq<Tree<T>>}",
        "type Id = union {Int, String}\ntype Holder { id : Id }",
        // What `std/core.hql` says, and what the binary holds.
        "type Data = union {Scalar, Seq<Data>, Map<String, Data>}",
        "type Data = union {Map<String, Data>, Scalar, Seq<Data>}",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
    assert_eq!(eval("type Id = union {Int, String}"), Ok(Value::Unit));
}

#[test]
fn a_union_that_does_not_hold_is_refused() {
    assert!(refused("type Id = union {Int, Nonesuch}").contains("unknown type `Nonesuch`"));
    assert!(refused("type Id = union {Int, Int}").contains("lists Int twice"));
    assert!(refused("type Id = union {Seq<Int>, Seq<Int>}").contains("lists Seq<Int> twice"));
    assert!(refused("type Id = union {Int, Id}").contains("lists itself as a member"));
    assert!(refused("type Id = union {Map<Int>}").contains("expects 2 type arguments"));
    // A refused union is withdrawn, so its name is free for a second attempt.
    assert!(refused("type Id = union {Id}\ntype Use { id : Id }").contains("lists itself"));

    // A built-in union is restated member for member.
    let narrower = refused("type Data = union {Scalar}");
    assert!(
        narrower.contains("built in as union {Scalar, Seq<Data>, Map<String, Data>}"),
        "{narrower}"
    );
    assert!(
        refused("type Data = union {Scalar, Seq<Data>, Map<String, Data>, Card}")
            .contains("contradicts it")
    );
    assert!(refused("type Card = union {Int, String}").contains("not a union"));
}

#[test]
fn a_union_is_written_with_equals_and_nothing_else() {
    for source in [
        "type Id = {Int, String}",
        "type Id = Int",
        "type Id = union",
        "type Id = union {}",
        "type Id = union {Int} : Data",
        "type Id : Data = union {Int}",
        "abstract type Id = union {Int}",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
    // `union` is a word only after `=` in a declaration; elsewhere it is a name.
    assert_eq!(check("union = 1\nunion"), Ok(TypeRef::INT));
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
    assert_eq!(check("type Holder<T> { held : T }"), Ok(TypeRef::UNIT));
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
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn narrowing_may_not_turn_a_required_field_into_an_optional_one() {
    // A subtype narrows its parent. Making a required key optional widens the
    // shape, so a value of the subtype could fail to be one of the parent.
    let refusal = refused("type Base { header : Data }\ntype Narrow : Base { header? : Data }");
    assert!(refusal.contains("`Base` requires `header`"), "{refusal}");
    assert_eq!(
        check("type Base { header? : Data }\ntype Narrow : Base { header? : Data }"),
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn a_declaration_is_a_statement_and_not_an_expression() {
    // `type` only opens a statement; a name may still be bound to a value.
    assert_eq!(check("type = 1\ntype"), Ok(TypeRef::INT));
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
    assert_eq!(
        check(&together),
        Ok(TypeRef::UNIT),
        "{:?}",
        check(&together)
    );
}
