//! Traits are strict: an implementation is declared in HQL, natively or with
//! HQL bodies, belongs with its trait or its type, and is the only one for
//! its trait and type.

use hql::{check, eval};

fn refused(source: &str) -> String {
    match check(source) {
        Err(diagnostic) => diagnostic.message(),
        Ok(inferred) => panic!("{source}: expected a refusal, got {inferred}"),
    }
}

fn shown(source: &str) -> String {
    eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message()))
        .to_string()
}

#[test]
fn a_builtin_type_satisfies_a_core_trait_through_its_declared_native_impl() {
    for (source, expected) in [
        ("fn f<T : Eq>(a: T, b: T) : Bool = a.eq(b)\nf(1, 1)", "true"),
        (
            "fn f<T : Eq>(a: T, b: T) : Bool = a.eq(b)\nf(true, false)",
            "false",
        ),
        (
            "fn f<T : Ord>(a: T, b: T) : Ordering = a.compare(b)\nf(\"a\", \"b\")",
            "Less",
        ),
        ("fn f<T : Add>(a: T, b: T) : T = a + b\nf(1, 2)", "3"),
        (
            "fn f<T : Add>(a: T, b: T) : T = a.add(b)\nf(1.5, 2.0)",
            "3.5",
        ),
        ("fn f<T : Rem>(a: T, b: T) : T = a.rem(b)\nf(7, 3)", "1"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn what_std_does_not_declare_a_builtin_type_does_not_implement() {
    let message = refused("fn f<T : Add>(a: T, b: T) : T = a + b\nf(\"a\", \"b\")");
    assert!(
        message.contains("String does not implement `Add`"),
        "{message}"
    );
    let message = refused("fn f<T : Ord>(a: T, b: T) : Ordering = a.compare(b)\nf(true, false)");
    assert!(
        message.contains("Bool does not implement `Ord`"),
        "{message}"
    );
}

#[test]
fn a_program_does_not_reimplement_a_core_trait_for_a_builtin_type() {
    let message = refused("impl Eq for Int {\n    fn eq(self, other: Int) : Bool = true\n}\n1");
    assert!(
        message.contains("already implemented for Int, in `std/core`, natively"),
        "{message}"
    );
}

#[test]
fn an_implementation_belongs_with_its_trait_or_its_type() {
    let message =
        refused("impl Add for String {\n    fn add(self, other: String) : String = self\n}\n1");
    assert!(
        message.contains("belongs with `Add` or with String")
            && message.contains("this program declares neither"),
        "{message}"
    );
    // Its own trait for a built-in type, or a core trait for its own type.
    assert_eq!(
        shown(
            "trait Twice {\n    fn twice(self) : Self\n}\nimpl Twice for Int {\n    fn twice(self) : Int = self * 2\n}\n4.twice()"
        ),
        "8"
    );
    assert_eq!(
        shown(
            "type V { n : Int }\nimpl Eq for V {\n    fn eq(self, other: V) : Bool = self.n == other.n\n}\nV(n = 1).eq(V(n = 1))"
        ),
        "true"
    );
}

#[test]
fn a_bodyless_impl_is_a_native_one_and_a_program_declares_none() {
    let message = refused("type V { n : Int }\nimpl Eq for V\n1");
    assert!(
        message.contains("has no body, and no extension implements it"),
        "{message}"
    );
    // The standard library's own header, read again, is the declaration
    // already in force.
    assert_eq!(
        check("impl Ord for Int\n1").map(|ty| ty.to_string()),
        Ok("Int".to_owned())
    );
}

#[test]
fn every_native_impl_is_declared_and_every_declared_one_is_implemented() {
    assert_eq!(hql::extensions::bind(), Ok(()));
    let headers = hql::library::native_impls().expect("std parses");
    assert!(headers.iter().any(|header| header.module == "core"
        && header.trait_name == "Ord"
        && header.target == "Int"));
    assert!(
        !headers
            .iter()
            .any(|header| header.target == "Bool" && header.trait_name == "Ord")
    );
}
