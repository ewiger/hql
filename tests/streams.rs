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

const NATURALS: &str = "fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
";

#[test]
fn take_ends_a_traversal_that_would_not_end_by_itself() {
    let source = format!("{NATURALS}naturals(1) | take(4)");
    assert_eq!(check(&source), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(eval(&source), eval("[1, 2, 3, 4]"));
}

#[test]
fn map_filter_and_drop_run_one_value_at_a_time() {
    let source = format!(
        "{NATURALS}naturals(1) | filter(n => n % 2 == 0) | map(n => n * n) | drop(1) | take(3)"
    );
    assert_eq!(eval(&source), eval("[16, 36, 64]"));
}

#[test]
fn an_iterating_function_yields_in_the_order_it_reaches_each_yield() {
    let source = "fn twice(xs: List<Int>) : Seq<Int> {
    for x in xs {
        doubled = x * 2
        yield x
        yield doubled
    }
}
twice([1, 2, 3])";
    assert_eq!(check(source), Ok(TypeRef::seq(TypeRef::INT)));
    assert_eq!(eval(source), eval("[1, 2, 2, 4, 3, 6]"));
    assert_eq!(eval(&format!("{source} | sum")), Value::Int(18));
    let nested = source.replace("twice([1, 2, 3])", "[ twice([1, 2, 3]) ]");
    assert_eq!(eval(&nested), eval("[ [1, 2, 2, 4, 3, 6] ]"));
}

#[test]
fn two_uses_of_one_generated_sequence_see_the_same_values() {
    let source = "fn three() : Seq<Int> {
    yield 1
    yield 2
    yield 3
}
xs = three()
[xs | sum, xs | take(2) | sum]";
    assert_eq!(eval(source), eval("[6, 3]"));
}

#[test]
fn yield_belongs_to_the_block_body_of_a_user_fn() {
    for source in [
        "yield 1",
        "[1] | map(x => yield x)",
        "for x in [1] { yield x }",
    ] {
        let message = check(source).expect_err(source);
        assert!(
            message.contains("block body of a user `fn`"),
            "{source}: {message}"
        );
    }
}

#[test]
fn an_iterating_function_declares_what_it_yields() {
    for (source, message) in [
        (
            "fn f(xs: List<Int>) : List<Int> {\n    for x in xs {\n        yield x\n    }\n}\n1",
            "returns Seq<T> or Stream<T>",
        ),
        (
            "fn f(xs: List<Int>) : Seq<String> {\n    for x in xs {\n        yield x\n    }\n}\n1",
            "yields String, and this is Int",
        ),
        ("fn f() : Seq<Int> {\n    x = 1\n}\n1", "never yields"),
        (
            "fn f() : Seq<Int> {\n    for x in 1 {\n        yield x\n    }\n}\n1",
            "`for` traverses",
        ),
        (&format!("{NATURALS}List(naturals(1))"), "may not end"),
    ] {
        let failure = check(source).expect_err(source);
        assert!(failure.contains(message), "{source}: {failure}");
    }
}

#[test]
fn collect_batches_by_position_and_emit_undoes_it() {
    assert_eq!(
        check("[1, 2, 3, 4, 5] | collect(2)"),
        Ok(TypeRef::list(TypeRef::list(TypeRef::INT)))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5] | collect(2)"),
        eval("[ [1, 2], [3, 4], [5] ]")
    );
    assert_eq!(
        eval("[] | List<Int> | collect(2)"),
        eval("[] | List<List<Int>>")
    );
    assert_eq!(eval("[1, 2, 3] | collect(2) | emit"), eval("[1, 2, 3]"));
    let open = format!("{NATURALS}naturals(1) | collect(3)");
    assert_eq!(
        check(&open),
        Ok(hql::types::collections::STREAM.apply([TypeRef::list(TypeRef::INT)]))
    );
    assert_eq!(
        eval(&format!("{open} | map(b => b | sum) | take(2)")),
        eval("[6, 15]")
    );
    assert_eq!(
        eval(&format!("{open} | emit | drop(1) | take(3)")),
        eval("[2, 3, 4]")
    );
}

#[test]
fn collect_and_emit_refuse_what_has_no_positions_or_no_batch() {
    for (source, message) in [
        ("[1, 2] | collect(0)", "at least one value"),
        ("{1, 2} | collect(2)", "sort it first"),
        ("[{1}, {2}] | emit", "sort each one first"),
        ("{1, 2} | List", "`sort` it into one"),
    ] {
        let failure = check(source).expect_err(source);
        assert!(failure.contains(message), "{source}: {failure}");
    }
    let flatten = check("[ [1] ] | flatten").expect_err("flatten is emit");
    assert!(flatten.contains("`emit`"), "{flatten}");
}

#[test]
fn flat_map_is_map_then_emit() {
    let source = "fn upto(n: Int) : Seq<Int> {
    for i in [1, 2, 3] | filter(i => i <= n) {
        yield i
    }
}
";
    assert_eq!(
        eval(&format!("{source}[1, 3] | flat_map(upto)")),
        eval(&format!("{source}[1, 3] | map(upto) | emit"))
    );
    assert_eq!(
        eval(&format!("{source}[1, 3] | flat_map(upto)")),
        eval("[1, 1, 2, 3]")
    );
    assert_eq!(
        eval(&format!(
            "{NATURALS}naturals(1) | flat_map(n => [n, n]) | take(4)"
        )),
        eval("[1, 1, 2, 2]")
    );
}
