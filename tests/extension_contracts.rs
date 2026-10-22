//! Registered step contracts govern checking, runtime result types, and help.

use hql::extensions::{Config, catalogue};
use hql::reporting::Mode;

#[test]
fn every_catalogue_signature_is_generated_from_its_structural_contract() {
    let providers = catalogue(&Config::default());
    let count: usize = providers.iter().map(|provider| provider.steps.len()).sum();
    // The core, string, seq, math, option, result, graph, format and io
    // modules of the prelude, and the four imported where they are used.
    assert_eq!(count, 164);
    for provider in providers {
        for step in provider.steps {
            let mut lines = Vec::new();
            for spec in step.specs {
                spec.validate().unwrap();
                lines.push(spec.signature(step.name));
            }
            assert_eq!(step.signature, lines.join("\n"));
            assert!(step.signature.contains(" -> "));
        }
    }
}

#[test]
fn invalid_structural_signatures_are_rejected_before_use() {
    use hql::extensions::spec::{OperationSpec, Output, TypePattern};
    let spec = OperationSpec {
        parameters: &[],
        input: TypePattern::Any,
        arguments: &[],
        output: Output::Type(TypePattern::Var("undeclared")),
    };
    assert!(
        spec.validate()
            .unwrap_err()
            .to_string()
            .contains("undeclared")
    );
    let spec = OperationSpec {
        output: Output::Type(TypePattern::Apply(hql::types::collections::LIST, &[])),
        ..spec
    };
    assert!(spec.validate().unwrap_err().to_string().contains("arity"));
}

#[test]
fn contracts_reject_unknown_duplicate_missing_and_mistyped_arguments() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    for source in [
        "[1] | take(1, 2)",
        "[1] | take(n = 1, n = 2)",
        "[1] | take(1, n = 2)",
        "[1] | take(wrong = 1)",
        "[1] | take",
        "[1] | take(true)",
        "[1] | size(unknown)",
        "[1] | count(wrong = 1)",
        "[1] | contains(\"one\")",
        "[1] | map(by = n => n, by = n => n)",
        "[1] | map(n => n, unknown)",
        "[1] | filter(n => n)",
        "[1] | filter((a, b) => true)",
        "[1] | sort(wrong = n => n)",
        "[1] | sort((a, b) => true)",
        "1 | markdown(unknown)",
        "@[[oauth]] | Graph(unknown)",
        "@[[oauth]] | uplinks(unknown)",
        "@[[oauth]] | downlinks(unknown)",
        "cards | expand(1, \"both\")",
        "cards | expand(depth = 1, 2)",
        "import lexical\ncards | lexical(query = 1)",
        "import lexical\ncards | lexical(\"owl\", \"robin\")",
        "import semantic\ncards | semantic(wrong = \"owl\")",
    ] {
        for mode in [Mode::Strict, Mode::Collect] {
            let outcome = hql::run(source, &vault, mode);
            assert!(outcome.failed(), "{source}");
            assert!(outcome.value.is_none(), "{source}");
        }
    }
}

#[test]
fn every_registered_step_runs_at_its_declared_result_type() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/birds")).unwrap();
    for source in [
        "[1, 1] | size",
        "[1, 1] | count(1)",
        "[1] | contains(1)",
        "Map([\"owl\"], [1]) | get(\"owl\")",
        "{2, 1} | sort",
        "[1, 2] | take(n = 1)",
        "[1, 2] | drop(1)",
        "[1, 2] | filter(n => n == 1)",
        "[1, 2] | map(n => n + 1)",
        "[1, 2] | of_type(Int)",
        "@[[barn-owl]] | uplinks",
        "@[[barn-owl]] | downlinks",
        "cards | expand(direction = \"both\", depth = 1)",
        "cards | Graph",
        "cards | Graph | format(Markdown, limit = 1)",
        "[1] | json",
        "[1] | text",
        "[1] | yaml",
        "[1] | csv",
        "[1] | hmd",
        "cards | Graph | d2",
        "cards | Graph | mermaid",
        "import lexical\ncards | lexical(query = \"owl\")",
        "import semantic\ncards | semantic(\"night hunting birds\")",
        "import lexical\ncards | lexical(\"owl\") | filter(h => true)",
        "import lexical\nranked = cards | lexical(\"owl\") | filter(h => false)\nranked.query",
    ] {
        let outcome = hql::run(source, &vault, Mode::Strict);
        assert!(!outcome.failed(), "{source}: {:?}", outcome.reports);
        assert_eq!(
            outcome.value.unwrap().type_of(),
            outcome.inferred.unwrap(),
            "{source}"
        );
    }
}

/// A name declared more than once is checked against the first declaration
/// its input and arguments fit, and its result is the one that declaration
/// states.
#[test]
fn an_overloaded_operation_takes_the_first_declaration_a_call_fits() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/birds")).unwrap();
    for (source, expected) in [
        ("[3, 1] | sort", "List<Int>"),
        ("{3, 1} | sort", "List<Int>"),
        ("[3, 1] | sort(by = n => n)", "List<Int>"),
        ("{3, 1} | sort((a, b) => compare(b, a))", "List<Int>"),
        ("[1, 2] | take(1)", "List<Int>"),
        ("{1, 2} | take(1)", "List<Int>"),
        ("{1, 2} | map(n => n > 1)", "Set<Bool>"),
        ("[1, 2] | reduce((a, b) => a + b)", "Option<Int>"),
        ("[1, 2] | reduce(0, (a, b) => a + b)", "Int"),
        ("[1, 2] | count", "Int"),
        ("[1, 2] | count(1)", "Int"),
        ("[] | sum", "Int"),
        ("[1.5] | sum", "Float"),
        ("[[1], [2]] | emit", "List<Int>"),
        ("-1 | abs", "Int"),
        ("2.0 | pow(2.0)", "Float"),
        ("Map([\"a\"], [1]) | with(\"b\", 2)", "Map<String, Int>"),
        ("some(2) | map(n => n > 1)", "Option<Bool>"),
        ("some([1, 2]) | map(n => n > 1)", "List<Bool>"),
        ("some(2) | filter(n => n > 1)", "Option<Int>"),
        ("ok(2) | map(n => n + 1)", "Result<Int, Never>"),
        ("cards | first | filter(c => c | is(Card))", "Option<Card>"),
        (
            "import lexical\ncards | lexical(\"owl\") | sort(by = h => h.score)",
            "Seq<Hit<Card>>",
        ),
        (
            "import lexical\ncards | lexical(\"owl\") | take(1)",
            "Ranking<Card>",
        ),
        (
            "import lexical\ncards | lexical(\"owl\") | reverse",
            "Seq<Hit<Card>>",
        ),
    ] {
        let outcome = hql::run(source, &vault, Mode::Strict);
        assert!(!outcome.failed(), "{source}: {:?}", outcome.reports);
        assert_eq!(outcome.inferred.unwrap().to_string(), expected, "{source}");
    }
    for (source, message) in [
        (
            "[\"a\"] | sum",
            "`sum` needs Collection<Int> or Collection<Float>, not List<String>",
        ),
        ("{true} | take(1)", "an order of their own"),
        ("[1] | collect", "`collect` needs `n`"),
        // An open tree is a list or not only at run time.
        ("[1, \"a\"] | first | map(x => x)", "not Data"),
    ] {
        let outcome = hql::run(source, &vault, Mode::Strict);
        assert!(
            outcome
                .reports
                .iter()
                .any(|report| report.message.contains(message)),
            "{source}: {:?}",
            outcome.reports
        );
    }
}
