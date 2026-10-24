//! `xs[a:b]`: the part of a value between two positions, of the value's own
//! type, and the `Slice` capability behind it. See
//! doc/proposals/HQL-0026/README.md.

use hql::{check, eval};

fn shown(source: &str) -> String {
    eval(source).unwrap().to_string()
}

fn typed(source: &str) -> String {
    check(source).unwrap().to_string()
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
fn a_slice_clamps_its_bounds_and_never_fails() {
    let xs = "xs = [1, 2, 3, 4]\n";
    assert_eq!(shown(&format!("{xs}xs[1:3]")), "[2, 3]");
    assert_eq!(shown(&format!("{xs}xs[:]")), "[1, 2, 3, 4]");
    assert_eq!(shown(&format!("{xs}xs[2:]")), "[3, 4]");
    assert_eq!(shown(&format!("{xs}xs[:-1]")), "[1, 2, 3]");
    assert_eq!(shown(&format!("{xs}xs[-10:2]")), "[1, 2]");
    assert_eq!(shown(&format!("{xs}xs[3:100]")), "[4]");
    assert_eq!(shown(&format!("{xs}xs[3:1]")), "[]");
    assert_eq!(shown(&format!("{xs}xs[100:]")), "[]");
}

#[test]
fn a_slice_is_of_the_type_it_is_taken_from() {
    assert_eq!(typed("[1, 2][0:1]"), "List<Int>");
    assert_eq!(typed("\"abc\"[0:1]"), "String");
    assert_eq!(typed("{a: [1, 2]}.a[0:1]"), "Option<Data>");
    assert_eq!(typed("some([1, 2])[1:]"), "Option<List<Int>>");
    assert_eq!(shown("some([1, 2])[1:]"), "[2]");
    assert_eq!(
        typed("fn part<T>(xs: Seq<T>) : Seq<T> = xs[1:]\npart([1, 2])"),
        "Seq<Int>"
    );
}

#[test]
fn a_string_is_indexed_and_sliced_by_character() {
    assert_eq!(typed("\"héllo\"[1]"), "Option<String>");
    assert_eq!(shown("\"héllo\"[1]"), "é");
    assert_eq!(shown("\"héllo\"[-1]"), "o");
    assert_eq!(shown("\"héllo\"[9]"), "none");
    assert_eq!(shown("\"héllo\"[1:3]"), "él");
    assert_eq!(shown("\"hello\".slice(-3, 5)"), "llo");
    assert_eq!(shown("\"hello\".slice(1, 3)"), "el");
    assert_eq!(shown("\"hello\".slice(Range(start = 1))"), "ello");
}

#[test]
fn a_range_value_is_what_the_bracket_takes() {
    assert_eq!(
        shown("w = Range(start = 1, end = 3)\n[[1, 2, 3, 4][w], [9][w]]"),
        "[[2, 3], []]"
    );
    assert_eq!(shown("\"hello\"[Range(end = -1)]"), "hell");
}

#[test]
fn a_stream_slice_is_decided_by_its_bounds() {
    assert_eq!(typed(&format!("{NATURALS}naturals(1)[:2]")), "List<Int>");
    assert_eq!(typed(&format!("{NATURALS}naturals(1)[5:]")), "Stream<Int>");
    assert_eq!(shown(&format!("{NATURALS}naturals(1)[2:4]")), "[3, 4]");
    assert_eq!(shown(&format!("{NATURALS}naturals(1)[4:2]")), "[]");
    refused(
        &format!("{NATURALS}naturals(1)[:-1]"),
        "counted from its end",
    );
    refused(
        &format!("{NATURALS}naturals(1)[Range(end = 2)]"),
        "written `a:b`",
    );
    let computed = eval(&format!("{NATURALS}n = 0 - 1\nnaturals(1)[n:2]"))
        .unwrap_err()
        .to_string();
    assert!(computed.contains("counted from its end"), "{computed}");
}

#[test]
fn a_declared_type_is_sliced_through_its_implementation() {
    let playlist = "type Playlist { tracks : List<String> }
impl Slice for Playlist {
    fn slice(self, range: Range) : Playlist = Playlist(tracks = self.tracks[range])
}
mix = Playlist(tracks = [\"a\", \"b\", \"c\"])
";
    assert_eq!(typed(&format!("{playlist}mix[:2]")), "Playlist");
    assert_eq!(shown(&format!("{playlist}mix[-1:].tracks")), "[\"c\"]");
    refused("type Box { n : Int }\nBox(n = 1)[0:1]", "Box is not sliced");
}

#[test]
fn only_a_value_with_positions_is_sliced() {
    refused("Set([1, 2])[0:1]", "no positions");
    refused("Map([\"a\"], [1])[:1]", "read a value with `get`");
    refused("1[0:1]", "Int is not sliced");
    refused("[1][1.5]", "an Int position or a Range");
    refused("[1][\"a\":2]", "a bound of a range is an Int");
    let empty = check("xs = [1]\nxs[]").unwrap_err().to_string();
    assert!(empty.contains("a position or a range"), "{empty}");
}
