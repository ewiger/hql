//! Traits are strict: an implementation is declared in HQL, natively or with
//! HQL bodies, belongs with its trait or its type, and is the only one for
//! its trait and type.

use hql::{check, eval};

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
fn a_builtin_type_satisfies_a_core_trait_through_its_declared_native_impl() {
    for (source, expected) in [
        ("fn f<T : Eq>(a: T, b: T) : Bool = a.eq(b)\nf(1, 1)", "true"),
        (
            "fn f<T : Eq>(a: T, b: T) : Bool = a.eq(b)\nf(true, false)",
            "false",
        ),
        (
            "fn f<T : Ord>(a: T, b: T) : Ordering = a.compare(b)\nf(\"a\", \"b\")",
            "Less",
        ),
        ("fn f<T : Add>(a: T, b: T) : T = a + b\nf(1, 2)", "3"),
        (
            "fn f<T : Add>(a: T, b: T) : T = a.add(b)\nf(1.5, 2.0)",
            "3.5",
        ),
        ("fn f<T : Rem>(a: T, b: T) : T = a.rem(b)\nf(7, 3)", "1"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn what_std_does_not_declare_a_builtin_type_does_not_implement() {
    let message = refused("fn f<T : Add>(a: T, b: T) : T = a + b\nf(\"a\", \"b\")");
    assert!(
        message.contains("String does not implement `Add`"),
        "{message}"
    );
    let message = refused("fn f<T : Ord>(a: T, b: T) : Ordering = a.compare(b)\nf(true, false)");
    assert!(
        message.contains("Bool does not implement `Ord`"),
        "{message}"
    );
}

#[test]
fn a_program_does_not_reimplement_a_core_trait_for_a_builtin_type() {
    let message = refused("impl Eq for Int {\n    fn eq(self, other: Int) : Bool = true\n}\n1");
    assert!(
        message.contains("already implemented for Int, in `std/core`, natively"),
        "{message}"
    );
}

#[test]
fn an_implementation_belongs_with_its_trait_or_its_type() {
    let message =
        refused("impl Add for String {\n    fn add(self, other: String) : String = self\n}\n1");
    assert!(
        message.contains("belongs with `Add` or with String")
            && message.contains("this program declares neither"),
        "{message}"
    );
    // Its own trait for a built-in type, or a core trait for its own type.
    assert_eq!(
        shown(
            "trait Twice {\n    fn twice(self) : Self\n}\nimpl Twice for Int {\n    fn twice(self) : Int = self * 2\n}\n4.twice()"
        ),
        "8"
    );
    assert_eq!(
        shown(
            "type V { n : Int }\nimpl Eq for V {\n    fn eq(self, other: V) : Bool = self.n == other.n\n}\nV(n = 1).eq(V(n = 1))"
        ),
        "true"
    );
}

#[test]
fn a_bodyless_impl_is_a_native_one_and_a_program_declares_none() {
    let message = refused("type V { n : Int }\nimpl Eq for V\n1");
    assert!(
        message.contains("has no body, and no extension implements it"),
        "{message}"
    );
    // The standard library's own header, read again, is the declaration
    // already in force.
    assert_eq!(
        check("impl Ord for Int\n1").map(|ty| ty.to_string()),
        Ok("Int".to_owned())
    );
}

#[test]
fn every_native_impl_is_declared_and_every_declared_one_is_implemented() {
    assert_eq!(hql::extensions::bind(), Ok(()));
    let headers = hql::library::native_impls().expect("std parses");
    assert!(headers.iter().any(|header| header.module == "core"
        && header.trait_name == "Ord"
        && header.target == "Int"));
    assert!(
        !headers
            .iter()
            .any(|header| header.target == "Bool" && header.trait_name == "Ord")
    );
}

const BOX: &str = "type Box<T> { item : T }\n";

const SCALE: &str = "trait Scale<F> {
    fn scale(self, by: F) : Self
}
type V { x : Float }
impl Scale<Float> for V {
    fn scale(self, by: Float) : V = V(x = self.x * by)
}
impl Scale<Int> for V {
    fn scale(self, by: Int) : V = V(x = self.x * float(by))
}
";

#[test]
fn a_generic_trait_is_implemented_generically_and_reached_through_a_bound() {
    let program = format!(
        "{BOX}impl<T> Index<T> for Box<T> {{
    fn at(self, position: Int) : Option<T> = some(self.item)
}}
fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]
[first_of(Box(item = 7)), first_of([8, 9]), Box(item = 6).at(3), Box(item = 5)[0]]"
    );
    assert_eq!(
        check(&program).map(|ty| ty.to_string()),
        Ok("List<Option<Int>>".to_owned())
    );
    assert_eq!(shown(&program), "[7, 8, 6, 5]");
}

#[test]
fn a_bound_leaves_a_parameter_to_the_implementation_that_applies() {
    // E is filled by the argument, and T by the one implementation of
    // Index for it: Seq's for a List, Data's for a document.
    for (argument, expected) in [
        ("[\"a\"]", "Option<String>"),
        ("[[1]]", "Option<List<Int>>"),
        ("{birds: [3]}.birds.unwrap_or([])", "Option<Data>"),
    ] {
        let program = format!(
            "fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]\nfirst_of({argument})"
        );
        assert_eq!(
            check(&program).map(|ty| ty.to_string()),
            Ok(expected.to_owned()),
            "{program}"
        );
    }
    let message = refused("fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]\nfirst_of(42)");
    assert!(
        message.contains("`first_of` needs E : Index<T>, and Int does not implement `Index"),
        "{message}"
    );
}

#[test]
fn implementations_at_different_arguments_are_chosen_by_the_call() {
    assert_eq!(
        shown(&format!(
            "{SCALE}v = V(x = 1.5f)\n[v.scale(2).x, v.scale(0.5f).x]"
        )),
        "[3.0, 0.75]"
    );
    let message = refused(
        "trait Convert<T> {
    fn convert(self) : T
}
type C { v : Float }
impl Convert<Float> for C {
    fn convert(self) : Float = self.v
}
impl Convert<String> for C {
    fn convert(self) : String = str(self.v)
}
C(v = 1.0f).convert()",
    );
    assert!(
        message.contains("implemented for C at several arguments, and this call does not choose"),
        "{message}"
    );
    // A generic body reaches an implementation through its receiver alone.
    let message = refused(&format!(
        "{SCALE}fn twice<F, E : Scale<F>>(v: E, by: F) : E = v.scale(by).scale(by)\ntwice(V(x = 1.0f), 2)"
    ));
    assert!(
        message.contains("V implements `Scale` at several arguments"),
        "{message}"
    );
}

#[test]
fn a_generic_and_a_concrete_implementation_that_overlap_are_refused() {
    let message = refused(&format!(
        "{BOX}impl<T> Index<T> for Box<T> {{
    fn at(self, position: Int) : Option<T> = some(self.item)
}}
impl Index<Int> for Box<Int> {{
    fn at(self, position: Int) : Option<Int> = none
}}
1"
    ));
    assert!(
        message.contains("`impl Index<Int> for Box<Int>` overlaps `impl Index<T> for Box<T>`"),
        "{message}"
    );
}

#[test]
fn an_implementation_gives_the_trait_its_arguments_and_uses_its_parameters() {
    let message = refused(&format!(
        "{BOX}impl Index for Box<Int> {{
    fn at(self, position: Int) : Option<Int> = none
}}
1"
    ));
    assert!(
        message.contains("`Index` takes 1 type argument, not 0"),
        "{message}"
    );
    let message = refused(&format!(
        "{BOX}impl<T, U> Index<T> for Box<T> {{
    fn at(self, position: Int) : Option<T> = none
}}
1"
    ));
    assert!(
        message.contains("`U` of this impl appears in neither the type nor the trait's arguments"),
        "{message}"
    );
    let message =
        refused("trait Pick<T : Scalar> {}\ntype Q { x : Int }\nimpl Pick<List<Int>> for Q {}\n1");
    assert!(
        message.contains("`Pick` takes a Scalar here, not List<Int>"),
        "{message}"
    );
}

#[test]
fn a_union_implementation_is_the_union_own_and_not_each_member_s() {
    // `impl Index<Data> for Data` makes a document indexed, not a Bool,
    // one of Data's alternatives; a String is indexed by its own
    // implementation, at its own element type.
    let message = refused("true[0]");
    assert!(message.contains("Bool is not indexed"), "{message}");
    assert!(refused("5.at(0)").contains("Int has no function `at`"));
    assert_eq!(
        hql::check("\"hello\".at(0)").unwrap().to_string(),
        "Option<String>"
    );
}

#[test]
fn the_codec_headers_are_the_decoder_and_encoder_tables() {
    assert_eq!(hql::extensions::bind(), Ok(()));
    let headers = hql::library::native_impls().expect("std parses");
    let decoders: Vec<&str> = headers
        .iter()
        .filter(|header| header.module == "format" && header.trait_name == "Decoder")
        .map(|header| header.target.as_str())
        .collect();
    assert!(decoders.contains(&"Lines") && decoders.contains(&"Json"));
    assert!(!decoders.contains(&"D2"), "D2 is written, not read");
    let encoders = headers
        .iter()
        .filter(|header| header.trait_name == "Encoder" && header.target == "Mermaid")
        .count();
    assert_eq!(encoders, 2, "a Graph, or a document holding one");
}

#[test]
fn an_operator_resolves_through_its_own_trait_only() {
    let foo = "type V { x : Int }\n\
               trait Foo { fn add(self, o: V) : V\n fn eq(self, o: V) : Bool }\n\
               impl Foo for V {\n\
                 fn add(self, o: V) : V = V(x = self.x + o.x)\n\
                 fn eq(self, o: V) : Bool = true\n\
               }\n";
    assert!(refused(&format!("{foo}(V(x = 1) + V(x = 1)).x")).contains("implementing `Add`"));
    assert_eq!(shown(&format!("{foo}V(x = 1) == V(x = 2)")), "false");
    let both = format!(
        "{foo}impl Add for V {{ fn add(self, o: V) : V = V(x = self.x * 10 + o.x) }}\n\
         impl Eq for V {{ fn eq(self, o: V) : Bool = self.x % 2 == o.x % 2 }}\n"
    );
    assert_eq!(shown(&format!("{both}(V(x = 1) + V(x = 1)).x")), "11");
    assert_eq!(shown(&format!("{both}V(x = 1) == V(x = 3)")), "true");
}

#[test]
fn self_in_a_trait_function_is_the_implementing_type() {
    let declared = "type Base { x : Int }\n\
                    type Sub : Base { y : Int }\n\
                    trait Fresh { fn fresh(self) : Self }\n\
                    impl Fresh for Base { fn fresh(self) : Base = Base(x = 0) }\n\
                    impl Eq for Base { fn eq(self, o: Base) : Bool = self.x == o.x }\n\
                    s = Sub(x = 1, y = 2)\n";
    assert!(refused(&format!("{declared}s.fresh().y")).contains("no field"));
    assert!(refused(&format!("{declared}t : Sub = s.fresh()\nt")).contains("Base"));
    assert_eq!(shown(&format!("{declared}s.fresh().x")), "0");
    assert_eq!(
        shown(&format!("{declared}[s == Base(x = 1), Base(x = 1) == s]")),
        "[true, true]"
    );
}

#[test]
fn equality_looks_through_option_to_an_implementation_of_eq() {
    let declared = "type V { x : Int }\n\
                    impl Eq for V { fn eq(self, o: V) : Bool = true }\n\
                    a = V(x = 1)\nb = V(x = 2)\n\
                    o : Option<V> = some(a)\nn : Option<V> = none\n";
    assert_eq!(
        shown(&format!(
            "{declared}[a == b, o == b, b == o, o == some(b), n == b, n == n, o != b]"
        )),
        "[true, true, true, true, false, true, false]"
    );
}

#[test]
fn a_container_is_equal_by_its_rule_and_its_elements_by_their_eq() {
    let declared = "type V { x : Int }\n\
                    impl Eq for V { fn eq(self, o: V) : Bool = self.x % 2 == o.x % 2 }\n\
                    type W { v : V, n : Int }\n\
                    a = V(x = 1)\nb = V(x = 3)\nc = V(x = 2)\n";
    for (query, expected) in [
        ("[a] == [b]", "true"),
        ("[a] == [c]", "false"),
        ("[a, c] == [c, a]", "false"),
        ("Set([a, c]) == Set([c, b])", "true"),
        ("W(v = a, n = 1) == W(v = b, n = 1)", "true"),
        ("W(v = a, n = 1) == W(v = b, n = 2)", "false"),
        ("some([a]) == some([b])", "true"),
        ("[[a]] == [[b]]", "true"),
        ("[[a]] | contains([b])", "true"),
        ("[a] != [c]", "true"),
    ] {
        assert_eq!(shown(&format!("{declared}{query}")), expected, "{query}");
    }
}
