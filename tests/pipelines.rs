//! Pipeline steps, their shapes, and the combiners a fold names: what a
//! planner may assume about splitting a step, and the laws that make the
//! split give the sequential answer.

use hql::extensions::{COMBINERS, Combiner, Shape, catalogue};
use hql::{check, eval, pipeline};

fn shapes(source: &str) -> Vec<(String, Shape)> {
    pipeline(source).unwrap_or_else(|error| panic!("{source}: {error}"))
}

fn holds(source: &str) {
    assert_eq!(
        eval(source).unwrap_or_else(|error| panic!("{source}: {error}")),
        hql::types::Value::Bool(true),
        "{source}"
    );
}

#[test]
fn every_core_step_declares_the_shape_its_table_states() {
    let core = catalogue(&hql::extensions::Config::default())
        .into_iter()
        .find(|provider| provider.name == "core")
        .unwrap();
    let shape_of = |name: &str| {
        core.steps
            .iter()
            .find(|step| step.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .shape
    };
    for (name, shape) in [
        ("filter", Shape::Elementwise),
        ("map", Shape::Elementwise),
        ("of_type", Shape::Elementwise),
        ("group", Shape::Partition),
        ("size", Shape::Reduction),
        ("count", Shape::Reduction),
        ("contains", Shape::Reduction),
        ("reduce", Shape::Barrier),
        ("sort", Shape::Barrier),
        ("take", Shape::Barrier),
        ("get", Shape::Barrier),
    ] {
        assert_eq!(shape_of(name), shape, "{name}");
    }
}

#[test]
fn a_pipeline_lists_its_steps_with_their_shapes() {
    assert_eq!(
        shapes("[3, 1, 2] | filter(n => n > 1) | map(n => n * 2) | sort | take(1)"),
        vec![
            ("filter".to_owned(), Shape::Elementwise),
            ("map".to_owned(), Shape::Elementwise),
            ("sort".to_owned(), Shape::Barrier),
            ("take".to_owned(), Shape::Barrier),
        ]
    );
    assert_eq!(
        shapes(r#"["a", "b", "a"] | group(by = w => w)"#),
        vec![("group".to_owned(), Shape::Partition)]
    );
}

#[test]
fn a_receiver_call_in_a_lambda_is_evaluated_within_its_step() {
    assert_eq!(
        shapes(r#"[" a ", "b "] | map(x => x.trim())"#),
        vec![("map".to_owned(), Shape::Elementwise)]
    );
}

#[test]
fn a_fold_is_a_reduction_by_a_combiner_and_a_barrier_by_a_lambda() {
    for (source, shape) in [
        ("[1, 2, 3] | reduce(0, +)", Shape::Reduction),
        ("[1, 2, 3] | reduce(10, +)", Shape::Reduction),
        ("[1, 2, 3] | reduce(1, *)", Shape::Reduction),
        ("[1, 2, 3] | reduce(max)", Shape::Reduction),
        ("[[1], [2]] | reduce([], concat)", Shape::Reduction),
        // The lambda only calls a combiner; recognising it is not required.
        ("[1, 2, 3] | reduce(0, (a, b) => a + b)", Shape::Barrier),
        ("[1, 2, 3] | reduce((a, b) => max(a, b))", Shape::Barrier),
    ] {
        assert_eq!(
            shapes(source),
            vec![("reduce".to_owned(), shape)],
            "{source}"
        );
    }
}

/// Values a combiner combines, each written in HQL, generated from a seed so
/// the inputs are many and the test stays deterministic.
fn samples(combiner: &Combiner) -> Vec<String> {
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % 7
    };
    (0..6)
        .map(|_| {
            let (a, b) = (next(), next());
            match combiner.name {
                "+" | "*" | "min" | "max" => format!("{a}"),
                "union" => format!("Set([{a}, {b}])"),
                "concat" => format!("[{a}, {b}]"),
                "merge" => format!(r#"Map(["k{a}", "j{b}"], [{a}, {b}])"#),
                other => panic!("no samples for {other}"),
            }
        })
        .collect()
}

fn applied(combiner: &Combiner, left: &str, right: &str) -> String {
    match combiner.name {
        "+" | "*" => format!("({left} {} {right})", combiner.name),
        name => format!("{name}({left}, {right})"),
    }
}

/// The identity as a value of the samples' type.
fn identity(combiner: &Combiner) -> Option<&'static str> {
    combiner.identity.map(|written| match combiner.name {
        "union" => "Set<Int>([])",
        "merge" => "Map<String, Int>([], [])",
        "concat" => "List<Int>([])",
        _ => written,
    })
}

#[test]
fn every_combiner_holds_the_laws_it_declares() {
    assert_eq!(COMBINERS.len(), 7);
    for combiner in COMBINERS {
        let values = samples(combiner);
        for a in &values {
            if let Some(e) = identity(combiner) {
                holds(&format!(
                    "{} == {a} and {} == {a}",
                    applied(combiner, e, a),
                    applied(combiner, a, e)
                ));
            }
            for b in &values {
                if combiner.commutative {
                    holds(&format!(
                        "{} == {}",
                        applied(combiner, a, b),
                        applied(combiner, b, a)
                    ));
                }
                if combiner.associative {
                    for c in values.iter().take(3) {
                        let left = applied(combiner, &applied(combiner, a, b), c);
                        let right = applied(combiner, a, &applied(combiner, b, c));
                        holds(&format!("{left} == {right}"));
                    }
                }
            }
        }
    }
    // An order-sensitive combiner is one whose order shows.
    holds(r#"merge(Map(["k"], [1]), Map(["k"], [2])) != merge(Map(["k"], [2]), Map(["k"], [1]))"#);
    holds("concat([1], [2]) != concat([2], [1])");
}

#[test]
fn a_known_fold_split_into_partitions_gives_the_sequential_result() {
    for combiner in COMBINERS.iter().filter(|combiner| combiner.associative) {
        let values = samples(combiner);
        let list = format!("[{}]", values.join(", "));
        for split in 1..values.len() {
            // Each split folds from its first element, and the partial
            // results combine in input order.
            let whole = format!("xs | reduce({})", combiner.name);
            let combined = format!(
                "(xs[0:{split}] | reduce({c})).and_then(l => (xs[{split}:] | reduce({c})).and_then(r => some({})))",
                applied(combiner, "l", "r"),
                c = combiner.name
            );
            holds(&format!("xs = {list}\n({whole}) == {combined}"));
        }
    }
    // A non-identity initial value is applied once, at the front.
    for split in 0..=4 {
        holds(&format!(
            "xs = [4, 1, 7, 3]\n(xs | reduce(10, +)) == \
             10 + (xs[0:{split}] | reduce(0, +)) + (xs[{split}:] | reduce(0, +))"
        ));
    }
}

#[test]
fn each_aggregation_equals_its_longhand() {
    for (shorthand, longhand) in [
        ("sum", "reduce(0, +)"),
        ("product", "reduce(1, *)"),
        ("min", "reduce(min)"),
        ("max", "reduce(max)"),
    ] {
        for input in ["[4, 1, 7, 3]", "List<Int>([])"] {
            holds(&format!(
                "({input} | {shorthand}) == ({input} | {longhand})"
            ));
        }
    }
    for (shorthand, longhand, input) in [
        (
            "merge_all",
            "reduce(Map<String, Int>([], []), merge)",
            r#"[Map(["a"], [1]), Map(["a", "b"], [2, 3])]"#,
        ),
        (
            "union_all",
            "reduce(Set<Int>([]), union)",
            "[Set([1, 2]), Set([2, 3])]",
        ),
        (
            "concat_all",
            "reduce(List<Int>([]), concat)",
            "[[1, 2], [3]]",
        ),
        (
            "concat_all",
            "reduce(List<Int>([]), concat)",
            "List<List<Int>>([])",
        ),
    ] {
        holds(&format!(
            "({input} | {shorthand}) == ({input} | {longhand})"
        ));
    }
    // `average` carries a sum and a count and projects their quotient.
    holds(
        "type Mean {\n  sum : Float\n  count : Int\n}\n\
         xs = [1.0, 2.0, 4.5]\n\
         s = xs | reduce(initial = Mean(sum = 0.0, count = 0), \
         by = (s, x) => Mean(sum = s.sum + x, count = s.count + 1))\n\
         (xs | average) == some(s.sum / float(s.count))",
    );
    holds("empty : List<Float> = []\n(empty | average) == none");
}

#[test]
fn an_order_sensitive_fold_refuses_a_set_and_says_to_sort_first() {
    for source in [
        "Set([[1], [2]]) | reduce(List<Int>([]), concat)",
        r#"Set([Map(["a"], [1])]) | reduce(Map<String, Int>([], []), merge)"#,
    ] {
        let refused = check(source).unwrap_err();
        assert!(
            refused.message().contains("sort it first"),
            "{source}: {refused}"
        );
    }
    // A commutative combiner's fold accepts any collection.
    assert_eq!(
        eval("Set([3, 1, 2]) | reduce(0, +)").unwrap(),
        hql::types::Value::Int(6)
    );
}

#[test]
fn an_unknown_fold_names_the_nearest_combiner_and_nothing_else() {
    let refused = check("[3, 1, 2] | reduce(mn)").unwrap_err();
    assert!(
        refused.message().contains("did you mean `min`"),
        "{refused}"
    );
    let refused = check("[3, 1, 2] | reduce(foo)").unwrap_err();
    assert!(!refused.message().contains("cos"), "{refused}");
    assert!(refused.message().contains("`concat`"), "{refused}");
}

const OPERATIONS: &str = "fn double(n: Int) : Int = n * 2
twice : Elementwise<Int, Int> = double
doubled : Elementwise<Int, Int> = n => n * 2
total : Reduction<List<Int>, Int> = xs => xs | sum
";

#[test]
fn a_user_operation_of_an_elementwise_type_runs_as_a_step_as_map_would() {
    let by_map = eval("[1, 2, 3] | map(n => n * 2)").unwrap();
    for step in ["twice", "doubled"] {
        let source = format!("{OPERATIONS}[1, 2, 3] | {step}");
        assert_eq!(eval(&source).unwrap(), by_map, "{step}");
    }
    // A set maps to a set, as `map` gives it.
    assert_eq!(
        eval(&format!("{OPERATIONS}Set<Int>([1, 2]) | doubled")).unwrap(),
        eval("Set<Int>([1, 2]) | map(n => n * 2)").unwrap()
    );
    // Over one element it is the function it is.
    assert_eq!(
        eval(&format!("{OPERATIONS}[3 | doubled, doubled(4)]")).unwrap(),
        eval("[6, 8]").unwrap()
    );
    assert_eq!(
        eval(&format!("{OPERATIONS}[1, 2, 3] | doubled | total")).unwrap(),
        eval("12").unwrap()
    );
}

#[test]
fn a_user_operation_enters_the_pipeline_at_the_shape_its_type_states() {
    assert_eq!(
        shapes(&format!(
            "{OPERATIONS}fn plain(xs: List<Int>) : List<Int> = xs\n\
             [1, 2, 3] | filter(n => n > 1) | doubled | plain | total"
        )),
        vec![
            ("filter".to_owned(), Shape::Elementwise),
            ("doubled".to_owned(), Shape::Elementwise),
            ("plain".to_owned(), Shape::Barrier),
            ("total".to_owned(), Shape::Reduction),
        ]
    );
}

#[test]
fn an_elementwise_body_reads_its_one_element_and_nothing_else() {
    for source in [
        "limit = 10\nscaled : Elementwise<Int, Int> = n => n * limit\nscaled(1)",
        "words = [\"a\"]\nknown : Elementwise<String, Bool> = w => words | contains(w)\nknown(\"a\")",
        "limit = 10\nfn scale(n: Int) : Int = n * limit\n\
         scaled : Elementwise<Int, Int> = scale\nscaled(1)",
    ] {
        let refused = check(source).unwrap_err().to_string();
        assert!(
            refused.contains("reads its one element"),
            "{source}: {refused}"
        );
    }
    // Its own parameter, its lambdas' parameters, literals and functions
    // are within the element.
    check(
        "fn inc(n: Int) : Int = n + 1\n\
         ok : Elementwise<List<Int>, Int> = xs => xs | map(x => inc(x)) | sum\n\
         ok([1])",
    )
    .unwrap();
}

#[test]
fn a_value_at_an_operation_type_is_the_function_its_type_states() {
    let refused = check("wrong : Elementwise<Int, Int> = (n: Int) => \"n\"\n1")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("Elementwise<Int, Int>"), "{refused}");
    let refused = check("doubled : Elementwise<Int, Int> = n => n * 2\n[\"a\"] | doubled")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("each element"), "{refused}");
}

#[test]
fn the_localhost_plan_puts_every_step_in_one_stage() {
    let planned = hql::plan(
        &format!("{OPERATIONS}[3, 1, 2] | filter(n => n > 1) | doubled | sort | take(1) | total"),
        &hql::vault::Vault::empty(),
        &hql::backends::LOCALHOST,
    )
    .unwrap();
    assert_eq!(planned.len(), 1);
    let names: Vec<&str> = planned[0]
        .steps
        .iter()
        .map(|step| step.name.as_str())
        .collect();
    assert_eq!(names, ["filter", "doubled", "sort", "take", "total"]);
    assert_eq!(planned[0].input, "[3, 1, 2]");
    for step in &planned[0].steps {
        assert_eq!(
            step.stage,
            hql::backends::Stage {
                index: 0,
                strategy: hql::backends::Strategy::InProcess
            },
            "{}",
            step.name
        );
    }
}
