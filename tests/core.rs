//! Public API coverage of syntax, typing, evaluation, and diagnostics.

use hql::{
    check,
    diagnostics::Diagnostic,
    eval,
    types::{TypeRef, Value},
};
use std::ops::Range;

fn decimal(text: &str) -> Value {
    Value::Decimal(hql::decimal::Decimal::parse(text).expect("a decimal literal"))
}

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
        ("1.5f", Value::Float(1.5), TypeRef::FLOAT),
        ("-0.25f + 0.5f", Value::Float(0.25), TypeRef::FLOAT),
        ("1.0f + 2.0f", Value::Float(3.0), TypeRef::FLOAT),
        ("007.50f", Value::Float(7.5), TypeRef::FLOAT),
        ("2f", Value::Float(2.0), TypeRef::FLOAT),
        ("6.02e23f", Value::Float(6.02e23), TypeRef::FLOAT),
        ("1.5", decimal("1.5"), TypeRef::DECIMAL),
        ("-0.25 + 0.5", decimal("0.25"), TypeRef::DECIMAL),
        ("0.1 + 0.2", decimal("0.3"), TypeRef::DECIMAL),
        ("1.5e-3", decimal("0.0015"), TypeRef::DECIMAL),
        ("1.5 + 2", decimal("3.5"), TypeRef::DECIMAL),
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
        "1 -",
        "1 * * 2",
        "1 < 2 < 3",
        "1 == 1 == true",
        "not",
        "1.",
        ".5",
        "1.2.3",
        "1e3",
        "1.5e",
        "1.5fx",
        "1.5f2",
        "-",
        "- 1",
        "1 ++ 2",
        "9223372036854775808",
        "-9223372036854775809",
        "\u{1f600}",
        "@[[unterminated",
        "\"unterminated",
        "max(",
        "max(1,",
        "[1] | reduce(0, +",
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
    // failure moved from the scanner to the checker. `resolve(@[[alice]])` is
    // the withdrawn spelling, and it still fails — for the right reason.
    for source in ["trueish", "resolve(@[[alice]])", "nosuchthing"] {
        assert!(
            matches!(check(source), Err(Diagnostic::Name { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
}

#[test]
fn addition_does_not_mix_int_and_float() {
    assert_eq!(type_error("1 + 2.0f"), 4..8);
    assert_eq!(type_error("1.0f + 2"), 7..8);
    assert_eq!(eval_type_error("1.0f + true"), 7..11);
    assert_eq!(eval_type_error("true + 1.0f"), 0..4);
    assert_eq!(type_error("1.0 + 2.0f"), 6..10);
}

#[test]
fn float_addition_reports_overflow_instead_of_infinity() {
    let large = format!("{}.0f", "1".repeat(309));
    assert_eq!(check(&large), Ok(TypeRef::FLOAT));
    let source = format!("{large} + {large}");
    assert!(matches!(eval(&source), Err(Diagnostic::Overflow { .. })));
    assert!(matches!(
        check(&format!("{}.0f", "1".repeat(400))),
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
    assert_eq!(check("@[[alice]] -> @[[bob]]"), Ok(TypeRef::EDGE));
    assert_eq!(
        check("@[[alice]] -> @[[bob]] { relation: \"parent\" }"),
        Ok(TypeRef::EDGE)
    );
    match eval("@[[alice]] -> @[[bob]]") {
        Ok(Value::Edge(edge)) => {
            assert_eq!(edge.source, "alice");
            assert_eq!(edge.target, "bob");
        }
        other => panic!("expected an edge, got {other:?}"),
    }
}

#[test]
fn a_document_reference_is_written_with_an_at_sign() {
    for (source, target) in [
        ("@[[alice]]", "alice"),
        ("@[[people/alice]]", "people/alice"),
        ("@[[alice#Bio]]", "alice#Bio"),
        ("@[[ A Room of One's Own ]]", "A Room of One's Own"),
    ] {
        assert_eq!(check(source).unwrap().to_string(), "Option<Card>");
        match eval(&format!("{source} -> @[[x]]")) {
            Ok(Value::Edge(edge)) => assert_eq!(edge.source, target),
            other => panic!("{source}: expected an edge, got {other:?}"),
        }
    }
}

#[test]
fn two_brackets_are_a_list_of_lists() {
    assert_eq!(
        check("[[1, 2], [3]]").unwrap().to_string(),
        "List<List<Int>>"
    );
    assert!(check("[[]]").unwrap().to_string().starts_with("List<List<"));
    assert_eq!(eval("[[1, 2], [3]] | emit | sum"), Ok(Value::Int(6)));
}

#[test]
fn a_reference_is_refused_with_what_it_lacks() {
    let message = |source: &str| check(source).unwrap_err().message().to_owned();
    assert_eq!(message("@[[]]"), "a document reference needs a name");
    assert_eq!(
        message("@[[alice"),
        "unterminated document reference: expected `]]`"
    );
    assert_eq!(
        message("@alice"),
        "`@` opens a document reference: `@[[name]]`"
    );
    assert!(message("[[alice]]").contains("`@[[alice]]`"));
    assert!(message("1 -> @[[bob]]").contains("a link endpoint is a document reference"));
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
    assert_eq!(check("@[[alice]]").unwrap().to_string(), "Option<Card>");
    assert_eq!(
        check("@[[alice]].header").unwrap().to_string(),
        "Option<Data>"
    );
    assert_eq!(
        check("x : Option<Card> = @[[alice]]\nx")
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
    let floats = vec!["0.5f"; 256].join(" + ");
    assert_eq!(eval(&floats), Ok(Value::Float(128.0)));
    assert_eq!(
        eval("1.0f + (10000000000000000.0f + -10000000000000000.0f)"),
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
    match check("1 | mapp") {
        Err(Diagnostic::Name { message, .. }) => assert!(message.contains("`map`"), "{message}"),
        other => panic!("expected a name error, got {other:?}"),
    }
}

#[test]
fn a_retired_step_names_what_replaces_it() {
    for (source, replacement) in [
        ("@[[oauth]] | graph", "`Graph`"),
        ("1 | table", "`Table`"),
        ("[1] | typed(Int)", "`of_type`"),
    ] {
        let failed = hql::check_in(
            source,
            &hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap(),
        )
        .expect_err(source);
        assert!(
            failed.message().contains(replacement),
            "{source}: {}",
            failed.message()
        );
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
    let failed = check("import nonesuch\n1").expect_err("no such module");
    assert!(
        failed.message().contains("no module named `nonesuch`"),
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
    let shadowed = check("format = 1\n1 | format.text").expect_err("format is bound");
    assert!(
        shadowed
            .message()
            .contains("reads a field rather than naming a step"),
        "{}",
        shadowed.message()
    );
    // Encoded text is a String, whatever the format.
    assert_eq!(check("format = 1\n1 | text"), Ok(TypeRef::STR));
    assert_eq!(check("1 | format.text"), Ok(TypeRef::STR));
}

#[test]
fn arithmetic_has_precedence_and_truncates_integer_division() {
    for (source, value) in [
        ("10 - 3 - 2", Value::Int(5)),
        ("1 -2", Value::Int(-1)),
        ("2 + 3 * 4", Value::Int(14)),
        ("(2 + 3) * 4", Value::Int(20)),
        ("7 / 2", Value::Int(3)),
        ("-7 / 2", Value::Int(-3)),
        ("7 % 3", Value::Int(1)),
        ("-7 % 3", Value::Int(-1)),
        ("7.0f / 2.0f", Value::Float(3.5)),
        ("1.5f * 2.0f - 0.5f", Value::Float(2.5)),
        ("7.0 / 2.0", decimal("3.5")),
        ("1.5 * 2.0 - 0.5", decimal("2.5")),
        ("7 / 2.0", decimal("3.5")),
    ] {
        assert_eq!(eval(source), Ok(value), "{source}");
    }
    assert_eq!(type_error("7 / 2.0f"), 4..8);
    assert!(matches!(eval("1 / 0"), Err(Diagnostic::Runtime { .. })));
    assert!(matches!(eval("1 % 0"), Err(Diagnostic::Runtime { .. })));
    assert!(matches!(
        eval("1.0f / 0.0f"),
        Err(Diagnostic::Runtime { .. })
    ));
    assert!(matches!(eval("1.0 / 0"), Err(Diagnostic::Runtime { .. })));
    assert!(matches!(
        eval("-9223372036854775808 / -1"),
        Err(Diagnostic::Overflow { .. })
    ));
    assert!(matches!(
        eval("9223372036854775807 * 2"),
        Err(Diagnostic::Overflow { .. })
    ));
}

#[test]
fn ordering_relations_compare_one_orderable_type() {
    for (source, value) in [
        ("1 < 2", true),
        ("2 <= 2", true),
        ("3 > 4", false),
        ("1.5 >= 1.5", true),
        ("\"apple\" < \"banana\"", true),
        ("1 + 1 < 3", true),
    ] {
        assert_eq!(check(source), Ok(TypeRef::BOOL), "{source}");
        assert_eq!(eval(source), Ok(Value::Bool(value)), "{source}");
    }
    // No widening but an Int's to a Decimal, and no comparing what has no
    // order.
    assert_eq!(eval("1 < 2.0"), Ok(Value::Bool(true)));
    assert!(matches!(check("1 < 2.0f"), Err(Diagnostic::Type { .. })));
    assert!(matches!(check("1.0 < 2.0f"), Err(Diagnostic::Type { .. })));
    assert!(matches!(
        check("true < false"),
        Err(Diagnostic::Type { .. })
    ));
    assert!(matches!(
        check("{ a: 1 } < { a: 2 }"),
        Err(Diagnostic::Type { .. })
    ));
}

#[test]
fn a_comparison_is_not_mistaken_for_a_constructor() {
    assert_eq!(eval("n = 1\nn < 3"), Ok(Value::Bool(true)));
    assert_eq!(eval("n = 5\nm = 3\nn > m"), Ok(Value::Bool(true)));
    assert_eq!(check("List<Int>([1, 2])").unwrap().to_string(), "List<Int>");
}

#[test]
fn connectives_take_bools_and_short_circuit() {
    for (source, value) in [
        ("true and false", false),
        ("true or false", true),
        ("not true", false),
        ("not not true", true),
        ("1 < 2 and 2 < 3", true),
        ("false and true or true", true),
        ("true or true and false", true),
        ("not 1 == 2", true),
        // The right operand is never evaluated, so it cannot fail.
        ("false and 1 / 0 == 0", false),
        ("true or 1 / 0 == 0", true),
    ] {
        assert_eq!(eval(source), Ok(Value::Bool(value)), "{source}");
    }
    assert_eq!(type_error("true and 1"), 9..10);
    assert_eq!(type_error("not 1"), 4..5);
    assert!(matches!(
        check("@[[alice]].metadata.done and true"),
        Err(Diagnostic::Type { .. })
    ));
}

#[test]
fn connective_chains_are_bounded_like_arithmetic() {
    let at_limit = vec!["true"; 256].join(" and ");
    assert_eq!(eval(&at_limit), Ok(Value::Bool(true)));
    assert!(matches!(
        eval(&format!("{at_limit} and true")),
        Err(Diagnostic::Syntax { .. })
    ));
}

#[test]
fn a_call_with_a_hole_is_a_function_of_the_arguments_left_open() {
    let add = "fn add(a: Int, b: Int) : Int = a + b\n";
    let unary = TypeRef::function(vec![TypeRef::INT], TypeRef::INT).unwrap();
    let binary = TypeRef::function(vec![TypeRef::INT, TypeRef::INT], TypeRef::INT).unwrap();
    assert_eq!(check(&format!("{add}add(10, _)")), Ok(unary));
    assert_eq!(check(&format!("{add}add(_, _)")), Ok(binary));
    assert_eq!(eval(&format!("{add}add(_, 2)(5)")), Ok(Value::Int(7)));
    assert_eq!(
        eval(&format!("{add}[1, 2] | map(add(10, _))")),
        eval("[11, 12]")
    );
    let refused = check(&format!("{add}add(\"x\", _)")).expect_err("a String is no Int");
    assert!(
        refused.message().contains("parameter `a`"),
        "{}",
        refused.message()
    );
    let bare = check("_").expect_err("a bare hole");
    assert!(
        bare.message().contains("whole argument"),
        "{}",
        bare.message()
    );
}

#[test]
fn a_type_test_that_always_holds_is_a_warning() {
    let vault = hql::vault::Vault::empty();
    let outcome = hql::run(
        "[1, 2] | of_type(Int)",
        &vault,
        hql::reporting::Mode::Strict,
    );
    assert!(!outcome.failed());
    let warning = outcome.reports.iter().next().expect("a warning");
    assert_eq!(warning.severity, hql::reporting::Severity::Warning);
    assert_eq!(warning.stage, "type");
    assert!(
        warning.message.contains("always true"),
        "{}",
        warning.message
    );
    // An open tree is asked at run time, so it warns of nothing.
    let open = hql::run("{a: 1}.a | is(Int)", &vault, hql::reporting::Mode::Strict);
    assert!(open.reports.is_empty(), "{:?}", open.reports);
}

#[test]
fn a_generic_function_takes_its_types_from_each_call() {
    let first = "fn first<T>(xs: List<T>) : Option<T> = xs | reduce((a, b) => a)\n";
    assert_eq!(
        check(&format!("{first}first([1, 2])")),
        Ok(TypeRef::optional(TypeRef::INT))
    );
    assert_eq!(
        check(&format!("{first}first([\"a\"])")),
        Ok(TypeRef::optional(TypeRef::STR))
    );
    assert_eq!(eval(&format!("{first}first([4, 5])")), eval("some(4)"));
    // The signature is written with the parameter, not its variable.
    let signature = check(&format!("{first}first")).unwrap().to_string();
    assert_eq!(signature, "List<T> -> Option<T>");
}

#[test]
fn a_generic_body_relies_only_on_what_its_parameter_promises() {
    let concrete = check("fn bad<T>(x: T) : Int = x + 1\n1").expect_err("T is no Int");
    assert!(
        concrete.message().contains("addition"),
        "{}",
        concrete.message()
    );
    let ordered = "fn smallest<T : Orderable>(xs: List<T>) : Option<T> = xs | min\n";
    assert_eq!(eval(&format!("{ordered}smallest([3, 1])")), eval("some(1)"));
    let refused =
        check(&format!("{ordered}smallest([{{1}}, {{2}}])")).expect_err("a set has no order");
    assert!(
        refused.message().contains("needs T : Orderable"),
        "{}",
        refused.message()
    );
    let named = check("fn f<Elem>(x: Elem) : Elem = x\n1").expect_err("one letter");
    assert!(
        named.message().contains("one capital letter"),
        "{}",
        named.message()
    );
}

#[test]
fn an_alias_is_a_function_whose_types_the_checker_writes() {
    let vault = hql::vault::Vault::empty();
    let signature = |source: &str| {
        let outcome = hql::check_reported(source, &vault, hql::reporting::Mode::Strict);
        assert!(!outcome.failed(), "{source}: {:?}", outcome.reports);
        outcome
            .reports
            .iter()
            .find(|report| report.code.as_deref() == Some("signature"))
            .map(|report| report.message.clone())
            .unwrap()
    };
    assert_eq!(
        signature("first := take(1)\n1"),
        "first : Stream<T> -> List<T>"
    );
    assert_eq!(
        signature("top(n) := sort | take(n)\n1"),
        "top : (Collection<T>, Int) -> List<T> where T : Orderable"
    );
    assert_eq!(
        signature("pick(xs: List<Int>, n) := xs | take(n)\n1"),
        "pick : (List<Int>, Int) -> List<Int>"
    );
    assert_eq!(
        eval("top(n) := sort | take(n)\n[3, 1, 2] | top(2)"),
        eval("[1, 2]")
    );
    assert_eq!(
        eval("top(n) := sort | take(n)\ntop([3, 1, 2], 1)"),
        eval("[1]")
    );
}

#[test]
fn an_alias_whose_body_fixes_no_type_is_refused() {
    let unknown = check("by_name(n) := sort(by = c => c.name) | take(n)\n1").expect_err("c.name");
    assert!(
        unknown.message().contains("no field `name`") && unknown.message().contains("write `fn`"),
        "{}",
        unknown.message()
    );
    let input = check("double(x) := x * 2\n1").expect_err("an expression needs its input typed");
    assert!(input.message().contains("annotated"), "{}", input.message());
    let vault = hql::vault::Vault::empty();
    let shadow = hql::run(
        "sum := take(1)\n[1] | sum",
        &vault,
        hql::reporting::Mode::Strict,
    );
    assert!(
        shadow
            .reports
            .iter()
            .any(|report| report.message.contains("shadows the step"))
    );
}

#[test]
fn a_destructuring_pattern_breaks_lines_and_ends_in_a_comma() {
    for pattern in ["{a, b}", "{a,\n b}", "{ a, b, }", "{\n  a,\n  b,\n}"] {
        let source = format!("type P = {{a: Int, b: Int}}\n{pattern} = P(a = 1, b = 2)\na + b");
        assert_eq!(eval(&source), Ok(Value::Int(3)), "{source}");
    }
}

#[test]
fn import_and_impl_are_keywords_only_by_position() {
    assert_eq!(eval("import = 5\nimport"), Ok(Value::Int(5)));
    assert_eq!(eval("impl = 2\nimpl < 3"), Ok(Value::Bool(true)));
    assert_eq!(eval("module = 1\nmodule"), Ok(Value::Int(1)));
}
