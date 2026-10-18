//! `Stream<T>`: values in a stable order whose production need not end.

use hql::types::{TypeRef, Value};

fn check(source: &str) -> Result<TypeRef, String> {
    hql::check(source).map_err(|failure| failure.message())
}

fn eval(source: &str) -> Value {
    let vault = hql::vault::Vault::empty();
    let outcome = hql::run(source, &vault, hql::reporting::Mode::Strict);
    assert!(!outcome.failed(), "{source}: {:?}", outcome.reports);
    outcome.value.expect("a value")
}

fn stream(element: TypeRef) -> TypeRef {
    hql::types::collections::STREAM.apply([element])
}

#[test]
fn a_sequence_is_a_stream_and_a_set_is_not() {
    let list = TypeRef::list(TypeRef::INT);
    assert!(list.is(&stream(TypeRef::INT)));
    assert!(TypeRef::seq(TypeRef::INT).is(&stream(TypeRef::INT)));
    assert!(!TypeRef::set(TypeRef::INT).is(&stream(TypeRef::INT)));
    assert!(!stream(TypeRef::INT).is(&TypeRef::collection(TypeRef::INT)));
}

#[test]
fn map_filter_and_drop_keep_a_stream_a_stream_and_take_bounds_it() {
    for (body, result) in [
        ("xs | map(n => n + 1)", "Stream<Int>"),
        ("xs | map(n => \"x\")", "Stream<String>"),
        ("xs | filter(n => n > 1)", "Stream<Int>"),
        ("xs | drop(2)", "Stream<Int>"),
        ("xs | take(2)", "List<Int>"),
    ] {
        let source = format!("fn f(xs: Stream<Int>) : {result} = {body}\n1");
        assert_eq!(check(&source), Ok(TypeRef::INT), "{body}");
    }
}

#[test]
fn a_whole_input_operation_over_a_stream_names_how_to_bound_it() {
    for body in ["xs | size", "xs | sort", "xs | count(1)", "xs | sum"] {
        let source = format!("fn f(xs: Stream<Int>) : Int = {body}\n1");
        let message = check(&source).expect_err(body);
        assert!(
            message.contains("may not end") && message.contains("take(n)"),
            "{body}: {message}"
        );
    }
    let bounded = "fn f(xs: Stream<Int>) : List<Int> = xs | take(3) | sort\n1";
    assert_eq!(check(bounded), Ok(TypeRef::INT));
}

#[test]
fn drop_skips_a_prefix_of_anything_with_an_order() {
    assert_eq!(eval("[1, 2, 3] | drop(1)"), eval("[2, 3]"));
    assert_eq!(eval("[1, 2, 3] | drop(5)"), eval("[1] | drop(1)"));
    let message = check("{1, 2} | drop(1)").expect_err("a set has no order");
    assert!(message.contains("sort it first"), "{message}");
}

#[test]
fn a_list_passes_where_a_stream_is_asked_for() {
    let source = "fn first_two(xs: Stream<Int>) : List<Int> = xs | take(2)\nfirst_two([3, 1, 2])";
    assert_eq!(eval(source), eval("[3, 1]"));
}
