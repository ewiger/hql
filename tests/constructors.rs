//! A type's constructor is an operation: declared in HQL beside its type,
//! called or piped, selected by its first argument, and passed as a value.
//! See doc/proposals/HQL-0018/README.md.

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

const TEMP: &str = "type Temp { kelvin : Float }
fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0)
fn Temp(label: String) : Temp = Temp(kelvin = 0.0)
";

#[test]
fn both_forms_of_a_standard_constructor_are_one_call() {
    assert_eq!(shown("[1, 2, 2] | Set"), shown("Set([1, 2, 2])"));
    assert_eq!(shown("[\"a\"] | Map([1])"), shown("Map([\"a\"], [1])"));
    assert_eq!(typed("[] | List<Int>"), typed("List<Int>([])"));
    refused("[1] | Seq", "List");
}

#[test]
fn a_standard_constructor_refuses_what_no_declared_form_takes() {
    refused("Graph(1)", "Graph");
    refused("Channel<Int>(1)", "String");
}

#[test]
fn a_function_is_named_after_a_type_only_to_construct_it() {
    refused(
        "fn Total(xs: List<Int>) : Int = xs | sum\n1",
        "names no type",
    );
    refused(
        "type Temp { kelvin : Float }\nfn Temp(c: Float) : Int = 1\n1",
        "a constructor returns its type",
    );
    refused(
        "abstract type Shape\nfn Shape(x: Int) : Shape = Shape(x)\n1",
        "abstract",
    );
}

#[test]
fn a_program_constructor_is_selected_by_its_first_argument() {
    assert_eq!(shown(&format!("{TEMP}Temp(1.0).kelvin")), "274.0");
    assert_eq!(shown(&format!("{TEMP}(\"zero\" | Temp).kelvin")), "0.0");
    assert_eq!(shown(&format!("{TEMP}Temp(kelvin = 3.0).kelvin")), "3.0");
    assert_eq!(typed(&format!("{TEMP}[1.0] | map(Temp)")), "List<Temp>");
    refused(&format!("{TEMP}Temp(true)"), "no constructor taking Bool");
    refused(
        "type Temp { kelvin : Float }\nfn Temp(a: Float) : Temp = Temp(kelvin = a)\n\
         fn Temp(b: Float) : Temp = Temp(kelvin = b)\n1",
        "differ in their first parameter",
    );
}

#[test]
fn a_constructor_value_the_expected_parameter_leaves_ambiguous_is_refused() {
    refused(&format!("{TEMP}[] | map(Temp)"), "`Float`, `String`");
    refused(&format!("{TEMP}[true] | map(Temp)"), "no form of it does");
    refused("[[]] | map(Graph)", "ambiguous");
    assert_eq!(typed("[[1], [2]] | map(Set)"), "List<Set<Int>>");
}

#[test]
fn text_is_parsed_by_constructing_data_in_a_format_that_decodes_data() {
    assert_eq!(shown("Data(\"[1, 2]\", Json)"), "[1, 2]");
    assert_eq!(shown("\"a: 1\" | Data(Yaml)"), "{a: 1}");
    assert_eq!(shown("\"a\\n1\" | Data(Csv)"), "[{a: \"1\"}]");
    assert_eq!(typed("\"1\" | Data(Json)"), "Data");
    refused("\"x\" | Data(Lines)", "Lines decodes String");
    refused("Data(1, Json)", "parses a String");
    refused("\"x\" | Data", "parses text in a format");
    let broken = eval("\"{\" | Data(Json)").unwrap_err().to_string();
    assert!(broken.contains("not JSON"), "{broken}");
    let several = eval("\"1 2\" | Data(Json)").unwrap_err().to_string();
    assert!(several.contains("parses one"), "{several}");
}

#[test]
fn a_generic_variadic_parameter_binds_one_type() {
    let pack = "fn pack<T>(xs: ...T) : List<T> = xs\n";
    assert_eq!(typed(&format!("{pack}pack(1, 2)")), "List<Int>");
    assert_eq!(shown(&format!("{pack}pack(1, 2)")), "[1, 2]");
    assert_eq!(shown(&format!("{pack}pack()")), "[]");
    refused(&format!("{pack}pack(1, \"a\")"), "fill one type");
}
