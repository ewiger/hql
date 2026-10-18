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

#[test]
fn map_keeps_the_input_collection_type_and_a_set_maps_to_its_image() {
    let source = "{1, 2, 3} | map(n => n % 2)";
    assert_eq!(check(source), Ok(TypeRef::set(TypeRef::INT)));
    let image = eval(source).unwrap();
    assert_eq!(image.type_of(), TypeRef::set(TypeRef::INT));
    assert_eq!(eval(&format!("{source} | size")), Ok(Value::Int(2)));

    assert_eq!(
        check("[1, 2] | map(n => n + 1)"),
        Ok(TypeRef::list(TypeRef::INT))
    );
    assert_eq!(
        check("open : Seq<Int> = [1, 2]\nopen | map(n => n + 1)"),
        Ok(TypeRef::seq(TypeRef::INT))
    );
}

#[test]
fn take_and_sort_keep_a_sequence_type_and_order_anything_else_into_a_list() {
    for source in [
        "open : Seq<Int> = [3, 1, 2]\nopen | take(2)",
        "open : Seq<Int> = [3, 1, 2]\nopen | sort",
    ] {
        assert_eq!(check(source), Ok(TypeRef::seq(TypeRef::INT)), "{source}");
    }
    for source in [
        "{3, 1, 2} | take(2)",
        "{3, 1, 2} | sort",
        "[3, 1, 2] | sort",
    ] {
        assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)), "{source}");
    }
}

#[test]
fn reduce_folds_from_an_initial_value_or_from_the_first_element() {
    assert_eq!(
        check("[1, 2, 3] | reduce(0, (t, n) => t + n)"),
        Ok(TypeRef::INT)
    );
    assert_eq!(
        eval("[1, 2, 3] | reduce(0, (t, n) => t + n)"),
        Ok(Value::Int(6))
    );
    assert_eq!(
        eval("[1, 2, 3] | reduce(initial = 10, by = (t, n) => t - n)"),
        Ok(Value::Int(4))
    );
    let from_first = "[3, 9, 4] | reduce((a, b) => a + b)";
    assert_eq!(check(from_first), Ok(TypeRef::optional(TypeRef::INT)));
    assert_eq!(eval(from_first).unwrap().to_string(), "16");
    let empty = "empty : List<Int> = []\nempty | reduce((a, b) => a + b)";
    assert_eq!(eval(empty).unwrap().to_string(), "none");
}

#[test]
fn reduce_refuses_a_set_because_it_has_no_positions() {
    let error = check("{1, 2} | reduce(0, (t, n) => t + n)").unwrap_err();
    assert!(error.to_string().contains("sort it first"), "{error}");
}

#[test]
fn group_partitions_by_key_and_keeps_input_order_within_a_group() {
    let source = "[\"owl\", \"raven\", \"owl\"] | group(by = w => w)";
    assert_eq!(
        check(source),
        Ok(TypeRef::set(
            builtin::GROUP.apply([TypeRef::STR, TypeRef::STR])
        ))
    );
    assert_eq!(eval(&format!("{source} | size")), Ok(Value::Int(2)));
    assert_eq!(
        eval("[1, 2, 3, 4] | group(by = n => n % 2) | sort(by = g => g.key) | map(g => g.items)")
            .unwrap()
            .to_string(),
        "[[2, 4], [1, 3]]"
    );
}

#[test]
fn sum_min_and_max_equal_their_folds_and_have_empty_results() {
    assert_eq!(eval("[1, 2, 3] | sum"), Ok(Value::Int(6)));
    assert_eq!(eval("{1.5, 2.5} | sum"), Ok(Value::Float(4.0)));
    assert_eq!(
        eval("empty : List<Int> = []\nempty | sum"),
        Ok(Value::Int(0))
    );
    assert_eq!(check("[3, 1] | min"), Ok(TypeRef::optional(TypeRef::INT)));
    assert_eq!(eval("{3, 1, 2} | min").unwrap().to_string(), "1");
    assert_eq!(eval("[3, 1, 2] | max").unwrap().to_string(), "3");
    assert_eq!(
        eval("empty : List<Int> = []\nempty | max")
            .unwrap()
            .to_string(),
        "none"
    );
    assert!(check("[true] | sum").is_err());
    assert!(check("[true] | min").is_err());
}
