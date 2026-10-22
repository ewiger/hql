//! Joins over one other sequence, the binary form of HQL-0013: each mode
//! over duplicates on both sides, in the order of positions, by keys, a
//! predicate or both, and the refusals the record names.

fn value(source: &str) -> String {
    hql::eval(source)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refusal(source: &str) -> String {
    hql::check(source)
        .err()
        .unwrap_or_else(|| panic!("{source} checked"))
        .message()
}

const SIDES: &str = "xs = [1, 2, 2, 3]\nys = [2, 3, 3, 4]\n";

#[test]
fn each_mode_keeps_what_it_names_in_the_order_of_positions() {
    let pair = "left = x => x, right = y => y, into = (x, y) => [x, y]";
    assert_eq!(
        value(&format!("{SIDES}xs | join(ys, {pair})")),
        "[[2, 2], [2, 2], [3, 3], [3, 3]]"
    );
    let shown = "left = x => x, right = y => y, into = (x, y) => {x: x, y: y}";
    assert_eq!(
        value(&format!("{SIDES}xs | left_join(ys, {shown})")),
        "[{x: 1, y: empty}, {x: 2, y: 2}, {x: 2, y: 2}, {x: 3, y: 3}, {x: 3, y: 3}]"
    );
    assert_eq!(
        value(&format!("{SIDES}xs | right_join(ys, {shown})")),
        "[{x: 2, y: 2}, {x: 2, y: 2}, {x: 3, y: 3}, {x: 3, y: 3}, {x: empty, y: 4}]"
    );
    assert_eq!(
        value(&format!("{SIDES}xs | full_join(ys, {shown})")),
        "[{x: 1, y: empty}, {x: 2, y: 2}, {x: 2, y: 2}, {x: 3, y: 3}, {x: 3, y: 3}, \
         {x: empty, y: 4}]"
    );
    assert_eq!(
        value("[1, 2] | cross_join([10, 20], into = (x, y) => x + y)"),
        "[11, 21, 12, 22]"
    );
}

#[test]
fn keys_a_predicate_and_both_match_as_defined() {
    let into = "into = (x, y) => [x, y]";
    assert_eq!(
        value(&format!(
            "{SIDES}xs | join(ys, left = x => x, right = y => y - 1, {into})"
        )),
        "[[1, 2], [2, 3], [2, 3], [2, 3], [2, 3], [3, 4]]"
    );
    assert_eq!(
        value(&format!(
            "{SIDES}xs | join(ys, on = (x, y) => x > y, {into})"
        )),
        "[[3, 2]]"
    );
    assert_eq!(
        value(&format!(
            "{SIDES}xs | join(ys, left = x => x % 2, right = y => y % 2, \
             on = (x, y) => x < y, {into})"
        )),
        "[[1, 3], [1, 3], [2, 4], [2, 4]]"
    );
}

#[test]
fn a_semi_join_keeps_an_element_once_and_an_anti_join_the_rest() {
    let keys = "left = x => x, right = y => y";
    assert_eq!(
        value(&format!("{SIDES}xs | semi_join(ys, {keys})")),
        "[2, 2, 3]"
    );
    assert_eq!(value(&format!("{SIDES}xs | anti_join(ys, {keys})")), "[1]");
    assert_eq!(
        value(&format!(
            "{SIDES}xs | semi_join(ys, on = (x, y) => x == y + 1)"
        )),
        "[3]"
    );
}

#[test]
fn a_join_names_what_is_missing() {
    assert!(
        refusal("[1] | join([2], into = (x, y) => x)").contains("`cross_join`"),
        "no condition"
    );
    assert!(refusal("[1] | join([2], left = x => x, into = (x, y) => x)").contains("together"));
    assert!(
        refusal("Set([1]) | join([2], on = (x, y) => true, into = (x, y) => x)").contains("`sort`")
    );
    // `seq.join` and `string.join` keep their own forms.
    assert_eq!(value("[\"a\", \"b\"] | join(\", \")"), "a, b");
    assert_eq!(
        value("[1, 2] | join([2], by = (x, y) => x == y) | size"),
        "1"
    );
}
