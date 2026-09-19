//! Collection pipelines infer their types and introduce order where requested.

use hql::types::{
    TypeRef, Value, builtin,
    collections::{self, SortOrder},
};
use hql::{check, diagnostics::Diagnostic, eval};

#[test]
fn execution_preserves_checked_collection_types_and_empty_map_results() {
    for source in [
        "held : Collection<Int> = [1]\ndistinct : Collection<Int> = {2}\n[held, distinct]",
        "first : Orderable = 1\nsecond : Orderable = \"owl\"\n[first, second]",
        "first : Orderable = 1\nsecond : Orderable = \"owl\"\n{first, second}",
        "empty : List<Int> = []\nempty | map(n => n + 1)",
        "held : Orderable = 1\n[1, 2] | map(n => held)",
        "held : Collection<Int> = [1]\nList([held])",
        "held : Collection<Int> = [1]\nMap([1], [held])",
    ] {
        assert_eq!(
            eval(source).unwrap().type_of(),
            check(source).unwrap(),
            "{source}"
        );
    }
}

#[test]
fn nested_lambdas_restore_the_outer_scope() {
    assert_eq!(
        eval("n = 9\n[1, 2] | map(n => [3] | map(n => n + 1))\nn"),
        Ok(Value::Int(9))
    );
}

#[test]
fn sorting_infers_a_list_from_an_unannotated_collection() {
    for source in [
        "open = {3, 1, 2}\nopen | sort",
        "open = {3, 1, 2}\nsort(open)",
        "open = {3, 1, 2, 0} | filter(n => n != 0)\nopen | sort",
        "open : Collection<Int> = {3, 1, 2}\nopen | sort",
    ] {
        assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)), "{source}");
        assert_eq!(eval(source).unwrap().to_string(), "[1, 2, 3]", "{source}");
    }
}

#[test]
fn a_set_can_be_sorted_by_an_inferred_key_or_comparison() {
    for ordering in [
        "by = group => size(group)",
        "by = (left, right) => compare(size(left), size(right))",
    ] {
        let source =
            format!("groups = {{[\"owl\", \"owl\"], [\"robin\"]}}\ngroups | sort({ordering})");
        assert_eq!(
            check(&source),
            Ok(TypeRef::list(TypeRef::list(TypeRef::STR)))
        );
        assert_eq!(eval(&source).unwrap().to_string(), "[[robin], [owl, owl]]");
    }
}

#[test]
fn sorting_preserves_occurrences_and_equal_key_positions_in_a_list() {
    let source = "groups = [ [\"owl\", \"owl\"], [\"robin\"], [\"raven\"], [\"robin\"] ]\n\
                  groups | sort(by = group => size(group))";
    assert_eq!(
        eval(source).unwrap().to_string(),
        "[[robin], [raven], [robin], [owl, owl]]"
    );
}

#[test]
fn inference_still_checks_the_ordering_contract() {
    for source in [
        "{true, false} | sort",
        "{[1], [2]} | sort",
        "{1, 2} | sort(by = n => true)",
        "{1, 2} | sort(by = (left, right) => left == right)",
        "1 | sort",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Type { .. })),
            "{source}"
        );
    }
    assert_eq!(eval("{3, 1, 3} | sort | count"), Ok(Value::Int(2)));
}

#[test]
fn the_type_system_accepts_sorting_every_collection_view() {
    let system = builtin::system().unwrap();
    for input in [
        TypeRef::collection(TypeRef::INT),
        TypeRef::set(TypeRef::INT),
        TypeRef::seq(TypeRef::INT),
        TypeRef::list(TypeRef::INT),
    ] {
        assert_eq!(
            collections::sort_type(system, &input, SortOrder::Intrinsic),
            Ok(TypeRef::list(TypeRef::INT))
        );
    }
    let input = TypeRef::set(TypeRef::list(TypeRef::STR));
    assert!(collections::sort_type(system, &input, SortOrder::Intrinsic).is_err());
    assert_eq!(
        collections::sort_type(system, &input, SortOrder::ExplicitComparison),
        Ok(TypeRef::list(TypeRef::list(TypeRef::STR)))
    );
}
