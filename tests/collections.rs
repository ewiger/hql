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
        assert_eq!(
            eval(&source).unwrap().to_string(),
            "[[\"robin\"], [\"owl\", \"owl\"]]"
        );
    }
}

#[test]
fn sorting_preserves_occurrences_and_equal_key_positions_in_a_list() {
    let source = "groups = [ [\"owl\", \"owl\"], [\"robin\"], [\"raven\"], [\"robin\"] ]\n\
                  groups | sort(by = group => size(group))";
    assert_eq!(
        eval(source).unwrap().to_string(),
        "[[\"robin\"], [\"raven\"], [\"robin\"], [\"owl\", \"owl\"]]"
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
            collections::sort_type(&system, &input, SortOrder::Intrinsic),
            Ok(TypeRef::list(TypeRef::INT))
        );
    }
    let input = TypeRef::set(TypeRef::list(TypeRef::STR));
    assert!(collections::sort_type(&system, &input, SortOrder::Intrinsic).is_err());
    assert_eq!(
        collections::sort_type(&system, &input, SortOrder::ExplicitComparison),
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
    assert_eq!(eval("{1.5f, 2.5f} | sum"), Ok(Value::Float(4.0)));
    assert_eq!(
        eval("{1.5, 2.5} | sum").map(|sum| sum.to_string()),
        Ok("4.0".to_owned())
    );
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

#[test]
fn of_type_selects_and_narrows_mixed_values() {
    let source = "[1, \"owl\", 2] | of_type(Int)";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(eval(source).unwrap().to_string(), "[1, 2]");
    assert_eq!(eval(source).unwrap().type_of(), TypeRef::list(TypeRef::INT));
}

#[test]
fn match_consumes_an_option_and_a_literal_and_is_exhaustive() {
    let source = "match some(3) { some(n) => n + 1, none => 0 }";
    assert_eq!(check(source), Ok(TypeRef::INT));
    assert_eq!(eval(source), Ok(Value::Int(4)));
    assert_eq!(
        eval("match none { some(n) => n, none => 0 }"),
        Ok(Value::Int(0))
    );
    assert_eq!(
        eval("match 3 { 1 => \"one\", _ => \"many\" }")
            .unwrap()
            .to_string(),
        "many"
    );
    assert_eq!(
        eval("match false { true => 1, false => 0 }"),
        Ok(Value::Int(0))
    );
    for (source, needs) in [
        ("match 3 { 1 => \"one\" }", "`_`"),
        ("match some(1) { some(n) => n }", "`none`"),
        ("match true { true => 1 }", "`false`"),
    ] {
        let error = check(source).unwrap_err().to_string();
        assert!(error.contains(needs), "{source}: {error}");
    }
}

#[test]
fn a_guarded_arm_does_not_count_toward_exhaustiveness() {
    let arms = "some(p) given p > 400 => \"long\", some(_) => \"short\", none => \"unknown\"";
    assert_eq!(
        eval(&format!("match some(500) {{ {arms} }}"))
            .unwrap()
            .to_string(),
        "long"
    );
    assert_eq!(
        eval(&format!("match some(5) {{ {arms} }}"))
            .unwrap()
            .to_string(),
        "short"
    );
    assert!(check("match some(5) { some(p) given p > 4 => 1, none => 0 }").is_err());
}

#[test]
fn given_without_otherwise_is_an_option() {
    assert_eq!(
        check("\"adult\" given 20 >= 18 otherwise \"minor\""),
        Ok(TypeRef::STR)
    );
    assert_eq!(
        check("\"long\" given 1 > 2"),
        Ok(TypeRef::optional(TypeRef::STR))
    );
    assert_eq!(eval("\"long\" given 1 > 2").unwrap().to_string(), "none");
    assert_eq!(
        eval("1 given false otherwise 2 given true otherwise 3"),
        Ok(Value::Int(2))
    );
    let error = check("1 given some(true) otherwise 2")
        .unwrap_err()
        .to_string();
    assert!(error.contains("unwrap_or"), "{error}");
}

#[test]
fn somes_keeps_the_present_values_in_the_input_collection_type() {
    let source = "[some(1), none, some(3)] | somes";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(eval(source).unwrap().to_string(), "[1, 3]");
    assert_eq!(
        check("{some(1), none} | somes"),
        Ok(TypeRef::set(TypeRef::INT))
    );
    assert_eq!(
        eval("[1, 2, 3] | map(n => n given n % 2 == 1) | somes")
            .unwrap()
            .to_string(),
        "[1, 3]"
    );
}

#[test]
fn tee_binds_a_branch_and_passes_its_input_on_unchanged() {
    let source = "xs = [5, 3, 8, 1]\nmain = xs\n| tee total = sum\n| filter(n => n > 2)\n| size\n[main, total]";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(eval(source).unwrap().to_string(), "[3, 17]");
    // A later step of the same pipeline reads the bound name.
    assert_eq!(
        eval("[1, 2] | tee n = size | map(x => x * n)")
            .unwrap()
            .to_string(),
        "[2, 4]"
    );
    // On one line the branch is one step; the next `|` continues the main line.
    assert_eq!(
        eval("[1, 2, 3] | tee d = map(x => x * 2) | size"),
        Ok(Value::Int(3))
    );
}

#[test]
fn an_indented_tee_branch_runs_until_the_main_line_resumes() {
    let source = "[5, 3, 8, 1]\n| tee top =\n    sort\n    | take(2)\n| size\ntop";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(eval(source).unwrap().to_string(), "[1, 3]");
}

#[test]
fn a_constructor_is_a_pipeline_step_with_the_piped_value_first() {
    for (piped, called) in [
        ("[1, 2, 2] | Set", "Set([1, 2, 2])"),
        (
            "[\"a\", \"b\"] | Map([1, 2])",
            "Map([\"a\", \"b\"], [1, 2])",
        ),
        ("[] | List<Int>", "List<Int>([])"),
        ("[3, 1] | sort | List", "List([3, 1] | sort)"),
    ] {
        assert_eq!(check(piped), check(called), "{piped}");
        assert_eq!(eval(piped), eval(called), "{piped}");
    }
    let abstract_type = check("[1] | Seq").unwrap_err().to_string();
    assert!(abstract_type.contains("`List`"), "{abstract_type}");
    let lowercase = check("[1] | list").unwrap_err().to_string();
    assert!(lowercase.contains("`List`"), "{lowercase}");
}

#[test]
fn a_combiner_names_how_two_values_become_one() {
    for (source, expected) in [
        ("min(3, 5)", "3"),
        ("max(\"a\", \"b\")", "b"),
        ("concat([1], [2, 3])", "[1, 2, 3]"),
        ("merge({a: 1, b: 2}, {b: 3, c: 4})", "{a: 1, b: 3, c: 4}"),
        ("[1, 2, 3] | reduce(0, +)", "6"),
        ("[2, 3, 4] | reduce(*)", "24"),
        ("[4, 9, 2] | reduce(max)", "9"),
        ("[ [1], [2] ] | reduce([], concat)", "[1, 2]"),
        ("[{a: 1}, {b: 2}] | reduce({:}, merge)", "{a: 1, b: 2}"),
        // A combiner that ignores order folds a set; one that does not, a sequence.
        ("{3, 1, 2} | reduce(0, +)", "6"),
    ] {
        let value = hql::eval(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(value.to_string(), expected, "{source}");
    }
    let ordered =
        hql::check("{ {a: 1} } | reduce({:}, merge)").expect_err("merge cares about order");
    assert!(
        ordered.message().contains("sort it first"),
        "{}",
        ordered.message()
    );
    let mixed = hql::check("min(1, \"a\")").expect_err("two types");
    assert!(mixed.message().contains("one type"), "{}", mixed.message());
}

#[test]
fn the_named_aggregations_equal_their_folds_even_over_nothing() {
    for (source, expected) in [
        ("[] | sum", "0"),
        ("[2, 3, 4] | product", "24"),
        ("[] | product", "1"),
        ("[1, 2] | average", "1.5"),
        ("[] | List<Int> | average", "none"),
        ("[{a: 1}, {a: 2, b: 3}] | merge_all", "{a: 2, b: 3}"),
        ("[ [1], [2, 3] ] | concat_all", "[1, 2, 3]"),
    ] {
        let value = hql::eval(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(value.to_string(), expected, "{source}");
    }
    assert_eq!(hql::check("[] | sum"), Ok(hql::types::TypeRef::INT));
}

#[test]
fn a_constructor_is_a_function_value_where_one_is_expected() {
    assert_eq!(
        hql::eval("[ [1, 1], [2] ] | map(Set)").unwrap().to_string(),
        "[{1}, {2}]"
    );
    assert_eq!(
        hql::check("[ [1, 1], [2] ] | map(Set)")
            .unwrap()
            .to_string(),
        "List<Set<Int>>"
    );
    let bare = hql::check("f = Set\n1").expect_err("not a value on its own");
    assert!(
        bare.message().contains("is a constructor: call it"),
        "{}",
        bare.message()
    );
    let named = hql::check("fn Total(x: Int) : Int = x\n1").expect_err("constructor casing");
    assert!(
        named.message().contains("snake_case"),
        "{}",
        named.message()
    );
}

#[test]
fn collection_steps_compare_as_equality_does() {
    let shown = |source: &str| {
        eval(source)
            .unwrap_or_else(|error| panic!("{source}: {}", error.message()))
            .to_string()
    };
    let filtered_none = "[none, some(1) | filter(x => x > 5)]";
    for (source, expected) in [
        ("[1, 1.0] | approx_distinct".to_owned(), "1"),
        ("[[1, 2.5], [1.0, 2.5]] | approx_distinct".to_owned(), "1"),
        (format!("{filtered_none} | approx_distinct"), "1"),
        (
            format!("{filtered_none} | count_by(x => x)"),
            "Map {none: 2}",
        ),
        (format!("{filtered_none} | group(x => x) | size"), "1"),
    ] {
        assert_eq!(shown(&source), expected, "{source}");
    }
    let declared = "type P { a : Int, b : Int }\n\
                    impl Eq for P { fn eq(self, other: P) : Bool = self.a == other.a }\n\
                    xs = [P(a = 1, b = 1), P(a = 2, b = 2)]\n\
                    ys = [P(a = 1, b = 9)]\n";
    for (query, expected) in [
        ("xs | count(P(a = 1, b = 9))", "1"),
        ("xs | contains(P(a = 1, b = 9))", "true"),
        (
            "xs | join(ys, left = x => x, right = y => y, into = (x, y) => 1) | size",
            "1",
        ),
        (
            "xs | semi_join(ys, left = x => x, right = y => y) | size",
            "1",
        ),
    ] {
        assert_eq!(shown(&format!("{declared}{query}")), expected, "{query}");
    }
}

#[test]
fn take_orders_a_set_by_an_implementation_of_ord() {
    let source = "type Version { major : Int, minor : Int }\n\
                  impl Ord for Version {\n\
                    fn compare(self, other: Version) : Ordering =\n\
                      compare(self.major * 100 + self.minor, other.major * 100 + other.minor)\n\
                  }\n\
                  vs = Set([Version(major = 3, minor = 0), Version(major = 1, minor = 10),\n\
                            Version(major = 0, minor = 9), Version(major = 2, minor = 0)])\n\
                  vs | take(2) | map(v => v.major)";
    assert_eq!(
        eval(source)
            .unwrap_or_else(|error| panic!("{}", error.message()))
            .to_string(),
        "[0, 1]"
    );
}
