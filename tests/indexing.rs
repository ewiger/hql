//! `xs[i]`: the element at a position of a sequence or a tree, as an option.
//! See doc/issues/0027-sequence-indexing.md.

use hql::{check, eval};

fn shown(source: &str) -> String {
    eval(source).unwrap().to_string()
}

fn refused(source: &str, says: &str) {
    let message = check(source).unwrap_err().to_string();
    assert!(message.contains(says), "{source}: {message}");
}

const NATURALS: &str = "fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
";

#[test]
fn a_position_counts_from_zero_and_past_either_end_is_absence() {
    assert_eq!(shown("xs = [10, 20]\nxs[0]"), "10");
    assert_eq!(shown("[10, 20][1]"), "20");
    assert_eq!(shown("[10, 20][2]"), "none");
    assert_eq!(shown("[10, 20][-1]"), "none");
    assert_eq!(check("[10, 20][0]").unwrap().to_string(), "Option<Int>");
}

#[test]
fn a_tree_is_indexed_when_it_is_a_sequence() {
    assert_eq!(shown("{a: [1, 2]}.a[1]"), "2");
    assert_eq!(
        check("{a: [1, 2]}.a[1]").unwrap().to_string(),
        "Option<Data>"
    );
    let wrong = eval("{a: 1}.a[0]").unwrap_err().to_string();
    assert!(wrong.contains("needs a sequence"), "{wrong}");
}

#[test]
fn an_optional_receiver_nests_no_deeper() {
    // `[ [` with a space: `[[` opens a document reference.
    let nested = "xs = [ [1, 2] ]\n";
    assert_eq!(
        check(&format!("{nested}xs[0][1]")).unwrap().to_string(),
        "Option<Int>"
    );
    assert_eq!(shown(&format!("{nested}xs[0][1]")), "2");
    assert_eq!(shown(&format!("{nested}xs[3][1]")), "none");
}

#[test]
fn only_a_sequence_has_positions() {
    refused("Set([1, 2])[0]", "no positions");
    refused("Map([\"a\"], [1])[0]", "read a value with `get`");
    refused("[1][\"x\"]", "a position is an Int");
    refused("1[0]", "Int is not indexed");
    refused(&format!("{NATURALS}naturals(1)[0]"), "take(n)");
}

#[test]
fn a_bracket_after_a_space_or_a_line_break_is_a_list() {
    assert_eq!(shown("xs = [1]\n[2]"), "[2]");
}
