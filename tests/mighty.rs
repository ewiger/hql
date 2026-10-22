//! Records, receiver calls, traits and results: the language HQL-0024 adds,
//! at the refusals the Mighty Types corpus does not show.

use hql::{check, diagnostics::Diagnostic, eval, types::TypeRef};

fn refused(source: &str) -> String {
    match check(source) {
        Err(diagnostic) => diagnostic.message(),
        Ok(inferred) => panic!("{source}: expected a refusal, got {inferred}"),
    }
}

fn shown(source: &str) -> String {
    eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message()))
        .to_string()
}

#[test]
fn a_record_is_constructed_by_field_name_or_position() {
    assert_eq!(
        shown("type P { x : Int, y : Int }\nP(x = 1, y = 2)"),
        "P(x = 1, y = 2)"
    );
    assert_eq!(shown("type P { x : Int, y : Int }\nP(1, 2).y"), "2");
    assert_eq!(
        check("type P { x : Int, y : Int }\nP(x = 1, y = 2)")
            .unwrap()
            .to_string(),
        "P"
    );
    assert!(refused("type P { x : Int, y : Int }\nP(x = 1)").contains("needs `y`"));
    assert!(refused("type P { x : Int }\nP(z = 1)").contains("no field `z`"));
    assert!(refused("type P { x : Int }\nP(x = 1, x = 2)").contains("supplied twice"));
    assert!(refused("type P { x : Int }\nP(x = \"one\")").contains("`x` of `P` is Int"));
    assert!(refused("type P { x : Int }\nP(x = 1).y").contains("no field `y`"));
}

#[test]
fn a_generic_record_takes_its_parameters_from_its_fields() {
    assert_eq!(
        check("type Box<T> { held : T }\nBox(held = [1, 2])")
            .unwrap()
            .to_string(),
        "Box<List<Int>>"
    );
    assert_eq!(
        check("type Box<T> { held : T }\nBox(held = 1).held").unwrap(),
        TypeRef::INT
    );
    // An optional field reads as an Option, and may be left out.
    assert_eq!(
        check("type U { name : String, email? : String }\nU(name = \"a\").email")
            .unwrap()
            .to_string(),
        "Option<String>"
    );
    assert_eq!(
        shown("type U { name : String, email? : String }\nU(name = \"a\").email"),
        "none"
    );
}

#[test]
fn a_union_of_records_is_matched_member_by_member() {
    let source = "type A { a : Int }\ntype B { b : Int }\ntype E = union { A, B }\n\
                  xs : List<E> = [A(a = 1), B(b = 2)]\n";
    assert_eq!(
        shown(&format!(
            "{source}xs | map(e => match e {{ A x => x.a, B y => y.b }}) | sum"
        )),
        "3"
    );
    assert!(
        refused(&format!("{source}xs | map(e => match e {{ A x => x.a }})"))
            .contains("cover every member of the union")
    );
    // Without the annotation the elements meet nowhere.
    assert!(
        refused("type A { a : Int }\ntype B { b : Int }\n[A(a = 1), B(b = 2)]")
            .contains("no common type")
    );
}

#[test]
fn a_receiver_call_is_a_call_with_the_receiver_first() {
    assert_eq!(shown("\"  hi \".trim().upper()"), "HI");
    assert_eq!(shown("[3, 1, 2].sort().first().unwrap_or(0)"), "1");
    assert_eq!(shown("7.max(3)"), "7");
    assert_eq!(shown("1.compare(2)"), "Less");
    assert_eq!(shown("string.trim(\"  x  \")"), "x");
    // One name, two modules: the receiver's type tells them apart.
    assert_eq!(shown("\"abc\".contains(\"b\")"), "true");
    assert_eq!(shown("[1, 2].contains(2)"), "true");
    // A function of another type is refused by its input, not as unknown.
    assert!(refused("1.trim()").contains("`trim` needs String, not Int"));
    // A field read is not a call.
    assert!(refused("type P { x : Int }\nP(x = 1).x()").contains("no function `x`"));
}

#[test]
fn a_receiver_call_reaches_only_a_declaration_with_self() {
    // `fn abs(self: Int)`, `fn size<T>(self: Collection<T>)`: written
    // receivers, outside a receiver module.
    assert_eq!(shown("(0 - 42).abs()"), "42");
    assert_eq!(shown("[1, 2].size()"), "2");
    assert_eq!(shown("Map([\"a\"], [1]).values()"), "[1]");
    // `fn range(start: Int, …)` has no receiver: a first parameter is never
    // one, so the call and the pipeline reach it and `.range` does not.
    assert_eq!(shown("range(1, 3)"), "[1, 2]");
    assert_eq!(shown("1 | range(3)"), "[1, 2]");
    assert!(refused("1.range(3)").contains("`range` takes no receiver"));
    // `regex` declares its functions over a text and a pattern; `split` of a
    // String is the receiver function, whichever modules are visible.
    assert!(
        refused("import regex\n\"ab\".is_match(\"a\")").contains("`is_match` takes no receiver")
    );
    assert_eq!(
        shown("import regex\n\"a,b\".split(\",\")"),
        "[\"a\", \"b\"]"
    );
    assert_eq!(shown("import regex\nis_match(\"ab\", \"a\")"), "true");
}

#[test]
fn str_and_fmt_write_values_as_text() {
    assert_eq!(shown("str([1, \"a\"])"), "[1, \"a\"]");
    assert_eq!(shown("fmt(\"{}-{}\", 1, none)"), "1-none");
    assert_eq!(shown("[1, 2] | map(str) | join(\"+\")"), "1+2");
    assert!(eval("fmt(\"{}\")").is_err());
}

#[test]
fn a_trait_is_implemented_once_per_type_and_dispatched_on_the_value() {
    let shape = "type C { r : Float }\ntype S { s : Float }\n\
                 trait Shape {\n    fn area(self) : Float\n    fn name(self) : String = \"shape\"\n}\n\
                 impl Shape for C {\n    fn area(self) : Float = self.r\n    fn name(self) : String = \"c\"\n}\n\
                 impl Shape for S {\n    fn area(self) : Float = self.s * self.s\n}\n";
    assert_eq!(shown(&format!("{shape}C(r = 2.0).area()")), "2.0");
    assert_eq!(shown(&format!("{shape}S(s = 2.0).name()")), "shape");
    assert_eq!(
        shown(&format!(
            "{shape}fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum\n\
             total([S(s = 1.0), S(s = 2.0)])"
        )),
        "5.0"
    );
    assert!(
        refused(&format!(
            "{shape}fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum\n\
             total([1])"
        ))
        .contains("Int does not implement `Shape`")
    );
    assert!(
        refused(&format!(
            "{shape}impl Shape for C {{\n    fn area(self) : Float = 1.0\n}}\n1"
        ))
        .contains("already implemented for C, in this program")
    );
    assert!(
        refused("trait T { fn f(self) : Int }\ntype A { a : Int }\nimpl T for A {\n}\n1")
            .contains("lacks `f`")
    );
    assert!(
        refused("trait T { fn f(self) : Int }\ntype A { a : Int }\nimpl T for A {\n    fn g(self) : Int = 1\n}\n1")
            .contains("no function `g`")
    );
    assert!(
        refused("trait T { fn f(self) : Int }\ntype A { a : Int }\nimpl T for A {\n    fn f(self) : String = \"\"\n}\n1")
            .contains("returns Int")
    );
    assert!(refused("type A { a : Int }\nA(a = 1).area()").contains("no function `area`"));
}

#[test]
fn operators_resolve_through_the_core_traits() {
    let vec = "type V { x : Int }\nimpl Add for V {\n    fn add(self, other: V) : V = V(x = self.x + other.x)\n}\n";
    assert_eq!(
        shown(&format!("{vec}(V(x = 1) + V(x = 2) + V(x = 3)).x")),
        "6"
    );
    assert!(refused(&format!("{vec}V(x = 1) - V(x = 2)")).contains("implementing `Sub`"));
    assert!(refused(&format!("{vec}V(x = 1) + 2")).contains("takes V, not Int"));
    // The built-in types keep their built-in arithmetic and their refusals.
    assert!(refused("1 + 1.0").contains("two Int or two Float"));
    let eq = "type V { x : Int }\nimpl Eq for V {\n    fn eq(self, other: V) : Bool = self.x % 2 == other.x % 2\n}\n";
    assert_eq!(
        shown(&format!("{eq}[V(x = 1) == V(x = 3), V(x = 1) != V(x = 2)]")),
        "[true, true]"
    );
    let ord = "type V { x : Int }\nimpl Ord for V {\n    fn compare(self, other: V) : Ordering = compare(self.x, other.x)\n}\n";
    assert_eq!(shown(&format!("{ord}V(x = 1) < V(x = 2)")), "true");
    assert_eq!(
        shown(&format!(
            "{ord}[V(x = 3), V(x = 1), V(x = 2)] | sort | map(v => v.x)"
        )),
        "[1, 2, 3]"
    );
    assert_eq!(
        shown(&format!(
            "{ord}[V(x = 3), V(x = 1)] | min | map(v => v.x) | unwrap_or(0)"
        )),
        "1"
    );
    assert_eq!(shown(&format!("{ord}max(V(x = 3), V(x = 1)).x")), "3");
    // Without Ord a record is not Orderable.
    assert!(refused("type V { x : Int }\nV(x = 1) < V(x = 2)").contains("Orderable"));
}

#[test]
fn a_result_is_ok_or_err_and_a_match_covers_both() {
    let parse = "fn p(s: String) : Result<Int, String> = match s.to_int() { some(n) => ok(n), none => err(s) }\n";
    assert_eq!(shown(&format!("{parse}p(\"4\")")), "ok(4)");
    assert_eq!(shown(&format!("{parse}p(\"x\")")), "err(\"x\")");
    assert_eq!(
        shown(&format!("{parse}p(\"4\").map(n => n * 2).unwrap_or(0)")),
        "8"
    );
    assert_eq!(
        shown(&format!(
            "{parse}p(\"x\").map_err(e => e.length()).err().unwrap_or(0)"
        )),
        "1"
    );
    assert_eq!(
        shown(&format!(
            "{parse}[p(\"1\"), p(\"x\")].filter(r => r.is_ok()).size()"
        )),
        "1"
    );
    assert_eq!(
        shown(&format!(
            "{parse}match p(\"x\") {{ ok(n) => n, err(e) => e.length() }}"
        )),
        "1"
    );
    assert!(
        refused(&format!("{parse}match p(\"x\") {{ ok(n) => n }}"))
            .contains("cover both `ok(…)` and `err(…)`")
    );
    assert!(matches!(
        check("ok(1).trim()"),
        Err(Diagnostic::Type { .. })
    ));
}
