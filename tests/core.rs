//! Public API coverage of syntax, typing, evaluation, and diagnostics.

use hql::{
    check,
    diagnostics::Diagnostic,
    eval,
    types::{TypeRef, Value},
};
use std::ops::Range;

fn type_error(source: &str) -> Range<usize> {
    match check(source).or_else(|_| eval(source).map(|_| TypeRef::UNIT)) {
        Err(Diagnostic::Type { span, .. }) => span,
        other => panic!("{source}: expected a type error, got {other:?}"),
    }
}

fn eval_type_error(source: &str) -> Range<usize> {
    match eval(source) {
        Err(Diagnostic::Type { span, .. }) => span,
        other => panic!("{source}: expected a type error, got {other:?}"),
    }
}

#[test]
fn literals_and_addition() {
    for (source, value, ty) in [
        ("0", Value::Int(0), TypeRef::INT),
        (" 40 + 2 ", Value::Int(42), TypeRef::INT),
        ("-2 + 3 + -4", Value::Int(-3), TypeRef::INT),
        ("true", Value::Bool(true), TypeRef::BOOL),
        ("false", Value::Bool(false), TypeRef::BOOL),
        ("-9223372036854775808", Value::Int(i64::MIN), TypeRef::INT),
        ("9223372036854775807", Value::Int(i64::MAX), TypeRef::INT),
        ("\u{2003}1 +\n2", Value::Int(3), TypeRef::INT),
        ("1.5", Value::Float(1.5), TypeRef::FLOAT),
        ("-0.25 + 0.5", Value::Float(0.25), TypeRef::FLOAT),
        ("1.0 + 2.0", Value::Float(3.0), TypeRef::FLOAT),
        ("007.50", Value::Float(7.5), TypeRef::FLOAT),
    ] {
        assert_eq!(check(source), Ok(ty), "{source}");
        assert_eq!(eval(source), Ok(value), "{source}");
    }
}

#[test]
fn rejects_incomplete_or_unsupported_syntax() {
    for source in [
        "",
        " ",
        "1 +",
        "1 2",
        "1;",
        "1 - 2",
        "1.",
        ".5",
        "1.2.3",
        "1e3",
        "1.5e3",
        "-",
        "- 1",
        "1 ++ 2",
        "9223372036854775808",
        "-9223372036854775809",
        "\u{1f600}",
        "[[unterminated",
        "\"unterminated",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
}

#[test]
fn an_unknown_name_is_a_name_error_rather_than_a_syntax_one() {
    // These parse now that the grammar has identifiers and calls, so the
    // failure moved from the scanner to the checker. `resolve([[alice]])` is
    // the withdrawn spelling, and it still fails — for the right reason.
    for source in ["trueish", "resolve([[alice]])", "nosuchthing"] {
        assert!(
            matches!(check(source), Err(Diagnostic::Name { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
}

#[test]
fn addition_does_not_mix_int_and_float() {
    assert_eq!(type_error("1 + 2.0"), 4..7);
    assert_eq!(type_error("1.0 + 2"), 6..7);
    assert_eq!(eval_type_error("1.0 + true"), 6..10);
    assert_eq!(eval_type_error("true + 1.0"), 0..4);
}

#[test]
fn float_addition_reports_overflow_instead_of_infinity() {
    let large = format!("{}.0", "1".repeat(309));
    assert_eq!(check(&large), Ok(TypeRef::FLOAT));
    let source = format!("{large} + {large}");
    assert!(matches!(eval(&source), Err(Diagnostic::Overflow { .. })));
    assert!(matches!(
        check(&format!("{}.0", "1".repeat(400))),
        Err(Diagnostic::Syntax { .. })
    ));
}

#[test]
fn type_errors_point_to_the_operand() {
    assert_eq!(type_error("1 + true"), 4..8);
    assert_eq!(eval_type_error("false + 1"), 0..5);
    assert_eq!(type_error("\u{2003}true + 1"), 3..7);
}

#[test]
fn checking_does_not_evaluate_and_evaluation_checks_first() {
    let source = "9223372036854775807 + 1";
    assert_eq!(check(source), Ok(TypeRef::INT));
    assert_eq!(eval(source), Err(Diagnostic::Overflow { span: 0..23 }));
    assert!(matches!(
        eval("-9223372036854775808 + -1"),
        Err(Diagnostic::Overflow { .. })
    ));
    assert!(matches!(
        eval("9223372036854775807 + 1 + true"),
        Err(Diagnostic::Type { .. })
    ));
}

#[test]
fn diagnostics_use_utf8_byte_ranges_and_eof_positions() {
    assert!(
        matches!(check("1 + \u{1f600}"), Err(Diagnostic::Syntax { span, .. }) if span == (4..8))
    );
    assert!(matches!(check("1 +"), Err(Diagnostic::Syntax { span, .. }) if span == (3..3)));
}

#[test]
fn bounds_addition_depth_and_evaluates_left_to_right() {
    let at_limit = vec!["1"; 256].join(" + ");
    assert_eq!(eval(&at_limit), Ok(Value::Int(256)));
    assert!(matches!(
        eval(&format!("{at_limit} + 1")),
        Err(Diagnostic::Syntax { .. })
    ));
    assert!(matches!(
        eval("9223372036854775807 + 1 + -1"),
        Err(Diagnostic::Overflow { .. })
    ));
}

#[test]
fn a_program_returns_its_last_expression_and_bindings_retain_values() {
    assert_eq!(eval("answer = 40 + 2\nanswer"), Ok(Value::Int(42)));
    assert_eq!(eval("a = 1\nb = a + 1\nb + 1"), Ok(Value::Int(3)));
    // An unbound earlier expression is evaluated and discarded.
    assert_eq!(eval("1 + 1\n2 + 2"), Ok(Value::Int(4)));
    // A program that ends in a binding has nothing to show.
    assert_eq!(eval("a = 1"), Ok(Value::Unit));
    assert_eq!(check("a = 1"), Ok(TypeRef::UNIT));
}

#[test]
fn a_binding_may_be_annotated_and_the_annotation_is_checked() {
    assert_eq!(eval("n : Int = 1\nn"), Ok(Value::Int(1)));
    assert!(matches!(
        check("n : Bool = 1\nn"),
        Err(Diagnostic::Type { .. })
    ));
    assert!(matches!(
        check("n : Nonesuch = 1"),
        Err(Diagnostic::Name { .. })
    ));
}

#[test]
fn parentheses_group_and_comments_are_skipped() {
    assert_eq!(eval("(1)"), Ok(Value::Int(1)));
    assert_eq!(eval("(1 + 2) + 3"), Ok(Value::Int(6)));
    assert_eq!(eval("// a note\n1 + 1 // another\n"), Ok(Value::Int(2)));
}

#[test]
fn strings_and_data_literals() {
    assert_eq!(check("\"text\""), Ok(TypeRef::STR));
    assert_eq!(eval("\"a\\nb\""), Ok(Value::Str("a\nb".into())));
    assert_eq!(check("{ relation: \"parent\" }"), Ok(TypeRef::DATA));
}

#[test]
fn the_link_operator_builds_an_edge() {
    assert_eq!(check("[[alice]] -> [[bob]]"), Ok(TypeRef::EDGE));
    assert_eq!(
        check("[[alice]] -> [[bob]] { relation: \"parent\" }"),
        Ok(TypeRef::EDGE)
    );
    match eval("[[alice]] -> [[bob]]") {
        Ok(Value::Edge(edge)) => {
            assert_eq!(edge.source, "alice");
            assert_eq!(edge.target, "bob");
        }
        other => panic!("expected an edge, got {other:?}"),
    }
}

#[test]
fn cards_and_traversal_say_they_need_a_vault_rather_than_finding_nothing() {
    for source in ["cards", "cards | count", "cards | expand(depth = 1)"] {
        assert!(
            matches!(check(source), Err(Diagnostic::Name { .. })),
            "{source}"
        );
    }
}

#[test]
fn generics_are_written_with_angle_brackets() {
    // An angle bracket always means the type world; a square bracket stays
    // value-level. See doc/models/behavior/bind-vs-apply-in-generic-types.md.
    assert_eq!(check("[[alice]]").unwrap().to_string(), "Option<Card>");
    assert_eq!(
        check("[[alice]].header").unwrap().to_string(),
        "Option<Data>"
    );
    assert_eq!(
        check("x : Option<Card> = [[alice]]\nx")
            .unwrap()
            .to_string(),
        "Option<Card>"
    );
    // Nesting closes with two separate `>`, so `>>` needs no special token.
    assert!(matches!(
        check("x : Seq<Hit<Card>> = 1"),
        Err(Diagnostic::Type { .. })
    ));
}

#[test]
fn square_brackets_are_not_the_type_world() {
    assert_eq!(check("[1, 2]"), Ok(TypeRef::list(TypeRef::INT)));
    for source in ["x : Set[Card] = 1", "x : Option[Int] = 1"] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
}

#[test]
fn a_lambda_is_an_argument_and_not_a_value() {
    assert!(matches!(check("c => c"), Err(Diagnostic::Type { .. })));
    assert!(matches!(check("f = c => c"), Err(Diagnostic::Type { .. })));
}

#[test]
fn a_step_without_an_input_says_so() {
    match check("take()") {
        Err(Diagnostic::Type { message, .. }) => assert!(message.contains("needs an input")),
        other => panic!("expected a type error, got {other:?}"),
    }
}

#[test]
fn a_direct_step_call_takes_its_input_as_the_first_argument() {
    assert_eq!(eval("take([3, 2, 1], 2)").unwrap().to_string(), "[3, 2]");
    assert!(matches!(check("take(5)"), Err(Diagnostic::Type { .. })));
}

#[test]
fn long_addition_chains_preserve_grouping_and_diagnostics() {
    let floats = vec!["0.5"; 256].join(" + ");
    assert_eq!(eval(&floats), Ok(Value::Float(128.0)));
    assert_eq!(
        eval("1.0 + (10000000000000000.0 + -10000000000000000.0)"),
        Ok(Value::Float(1.0))
    );
    let prefix = vec!["1"; 255].join(" + ");
    let invalid = format!("{prefix} + false");
    assert_eq!(type_error(&invalid), prefix.len() + 3..invalid.len());
    assert_eq!(
        eval("1 + 9223372036854775807 + -1"),
        Err(Diagnostic::Overflow { span: 0..23 })
    );
}

#[test]
fn a_near_miss_on_a_step_name_is_suggested() {
    match check("1 | tabl") {
        Err(Diagnostic::Name { message, .. }) => assert!(message.contains("`table`"), "{message}"),
        other => panic!("expected a name error, got {other:?}"),
    }
}

#[test]
fn an_unimported_step_reports_the_import_rather_than_a_misspelling() {
    let failed = check("1 | lexical(\"anything\")").expect_err("lexical is not imported");
    assert!(
        failed.message().contains("write `import lexical`"),
        "{}",
        failed.message()
    );

    let unknown = check("1 | nonesuch").expect_err("no such step");
    assert!(
        unknown.message().contains("is not a pipeline step"),
        "{}",
        unknown.message()
    );
}

#[test]
fn an_import_names_a_registered_extension() {
    let failed = check("import nonesuch\n1").expect_err("no such extension");
    assert!(
        failed.message().contains("no extension named `nonesuch`"),
        "{}",
        failed.message()
    );
    assert_eq!(check("import lexical\n1"), Ok(TypeRef::INT));
}

#[test]
fn a_binding_shadows_the_namespace_and_not_the_step() {
    // The core's own steps show the rule without needing a vault: a value
    // bound to an extension's name hides the qualified form, and the bare
    // form goes on resolving, because a step is only ever read after `|`.
    let shadowed = check("present = 1\n1 | present.text").expect_err("present is bound");
    assert!(
        shadowed
            .message()
            .contains("reads a field rather than naming a step"),
        "{}",
        shadowed.message()
    );
    assert_eq!(check("present = 1\n1 | text"), Ok(TypeRef::PRESENTATION));
    assert_eq!(check("1 | present.text"), Ok(TypeRef::PRESENTATION));
}
