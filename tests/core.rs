//! Public API coverage of syntax, typing, evaluation, and diagnostics.

use hql::{check, diagnostics::Diagnostic, eval, types::Type, values::Value};

#[test]
fn literals_and_addition() {
    for (source, value, ty) in [
        ("0", Value::Int(0), Type::Int),
        (" 40 + 2 ", Value::Int(42), Type::Int),
        ("-2 + 3 + -4", Value::Int(-3), Type::Int),
        ("true", Value::Bool(true), Type::Bool),
        ("false", Value::Bool(false), Type::Bool),
        ("-9223372036854775808", Value::Int(i64::MIN), Type::Int),
        ("9223372036854775807", Value::Int(i64::MAX), Type::Int),
        ("\u{2003}1 +\n2", Value::Int(3), Type::Int),
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
        "trueish",
        "1;",
        "1 - 2",
        "(1)",
        "1.5",
        "-",
        "- 1",
        "1 ++ 2",
        "resolve([[alice]])",
        "9223372036854775808",
        "-9223372036854775809",
        "😀",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}"
        );
    }
}

#[test]
fn type_errors_point_to_the_operand() {
    assert_eq!(check("1 + true"), Err(Diagnostic::Type { span: 4..8 }));
    assert_eq!(eval("false + 1"), Err(Diagnostic::Type { span: 0..5 }));
    assert_eq!(
        check("\u{2003}true + 1"),
        Err(Diagnostic::Type { span: 3..7 })
    );
}

#[test]
fn checking_does_not_evaluate_and_evaluation_checks_first() {
    let source = "9223372036854775807 + 1";
    assert_eq!(check(source), Ok(Type::Int));
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
    assert!(matches!(check("1 + 😀"), Err(Diagnostic::Syntax { span, .. }) if span == (4..8)));
    assert!(matches!(check("1 +"), Err(Diagnostic::Syntax { span, .. }) if span == (3..3)));
}

#[test]
fn bounds_ast_depth_and_evaluates_left_to_right() {
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
