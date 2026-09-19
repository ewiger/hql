//! Registered step contracts govern checking, runtime result types, and help.

use hql::extensions::{Config, catalogue};
use hql::reporting::Mode;

#[test]
fn every_catalogue_signature_is_generated_from_its_structural_contract() {
    let providers = catalogue(&Config::default());
    let count: usize = providers.iter().map(|provider| provider.steps.len()).sum();
    assert_eq!(count, 18);
    for provider in providers {
        for step in provider.steps {
            step.spec.validate().unwrap();
            assert_eq!(step.signature, step.spec.signature(step.name));
            assert!(step.signature.contains(" -> "));
        }
    }
}

#[test]
fn invalid_structural_signatures_are_rejected_before_use() {
    use hql::extensions::spec::{Output, StepSpec, TypePattern};
    let spec = StepSpec {
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
    let spec = StepSpec {
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
        "[1] | count(value = 1)",
        "[1] | contains(\"one\")",
        "[1] | map(by = n => n, by = n => n)",
        "[1] | map(n => n, unknown)",
        "[1] | filter(n => n)",
        "[1] | filter((a, b) => true)",
        "[1] | sort(wrong = n => n)",
        "[1] | sort((a, b) => true)",
        "1 | table(unknown)",
        "[[oauth]] | graph(unknown)",
        "[[oauth]] | uplinks(unknown)",
        "[[oauth]] | downlinks(unknown)",
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
        "[1, 2] | filter(n => n == 1)",
        "[1, 2] | map(n => n + 1)",
        "[1, 2] | typed(Int)",
        "[[barn-owl]] | uplinks",
        "[[barn-owl]] | downlinks",
        "cards | expand(direction = \"both\", depth = 1)",
        "cards | graph",
        "[1] | table",
        "[1] | json",
        "[1] | text",
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
