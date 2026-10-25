//! `type` declarations: the statement, and what makes one hold.
//!
//! A declaration is checked and yields `Unit`. It introduces no value, so
//! everything here is about whether what a declaration says is coherent.

use hql::{
    check,
    diagnostics::Diagnostic,
    eval, library,
    types::{TypeRef, Value},
};
use std::fs;

fn refused(source: &str) -> String {
    match check(source) {
        Err(diagnostic) => diagnostic.message(),
        Ok(inferred) => panic!("{source}: expected a refusal, got {inferred}"),
    }
}

/// Every `.hql` file under `dir`, with the module name it is embedded as: the
/// path below its package folder, `lib/std/`, without its extension, folders joined by `.`, and a
/// folder's `mod.hql` named for the folder.
fn modules_under(
    dir: &std::path::Path,
    prefix: &str,
    found: &mut Vec<(String, std::path::PathBuf)>,
) {
    for entry in fs::read_dir(dir).expect("a builtin package") {
        let path = entry.expect("an entry").path();
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap()
            .to_owned();
        let name = if prefix.is_empty() {
            stem.clone()
        } else {
            format!("{prefix}.{stem}")
        };
        if path.is_dir() {
            modules_under(&path, &name, found);
        } else if path.extension().and_then(|name| name.to_str()) == Some("hql") {
            let name = if stem == "mod" {
                prefix.to_owned()
            } else {
                name
            };
            found.push((name, path));
        }
    }
}

#[test]
fn every_file_in_a_builtin_package_is_embedded_as_written() {
    // Each folder is walked rather than listed, subfolders included, so a
    // file added under `lib/` cannot be left out of the binary, and the copy
    // the binary reads its record fields from is the one on disk.
    let packages: [(&str, Vec<(&str, &str)>); 2] = [
        (
            "std",
            library::STD
                .iter()
                .chain(library::WRITTEN)
                .copied()
                .collect(),
        ),
        ("search", library::SEARCH.to_vec()),
    ];
    let mut folders: Vec<String> = fs::read_dir("lib")
        .expect("the builtin packages")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.is_dir())
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    folders.sort_unstable();
    assert_eq!(folders, ["search", "std"], "one folder per builtin package");
    for (package, embedded) in &packages {
        let mut found = Vec::new();
        modules_under(&std::path::Path::new("lib").join(package), "", &mut found);
        for (name, path) in &found {
            let source = embedded
                .iter()
                .find(|(module, _)| module == name)
                .unwrap_or_else(|| panic!("{} is not embedded as {name}", path.display()));
            assert_eq!(
                source.1,
                fs::read_to_string(path).expect("a readable file"),
                "{}",
                path.display()
            );
        }
        assert_eq!(found.len(), embedded.len(), "lib/{package}/");
    }
    assert_eq!(
        library::MODULES.len(),
        library::STD.len() + library::SEARCH.len()
    );
}

/// The modules a standard module imports, by name, in the order written.
fn imports_of(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("import "))
        .map(str::trim)
        .collect()
}

#[test]
fn every_module_of_the_standard_library_checks_over_its_own_imports() {
    // Each module is its own import unit, so it is checked with the modules
    // it imports, theirs in turn, and nothing else: a declaration that
    // contradicts the lattice is refused in the module that makes it, and a
    // module leaning on one it does not import fails here. A package's own
    // prelude does not apply to its own modules, so the library checks with
    // none: each file names what it imports.
    let mut vault = hql::vault::Vault::empty();
    vault.extensions.prelude = false;
    let source_of = |name: &str| {
        library::MODULES
            .iter()
            .find(|(module, _)| *module == name)
            .map(|(_, source)| *source)
            .unwrap_or_else(|| panic!("no standard module `{name}`"))
    };
    for (name, module) in library::MODULES {
        // `prelude.hql` is not a module: it is the package's global space,
        // the implicit import list of every unit outside the library.
        if *name == "prelude" {
            continue;
        }
        let mut cone: Vec<&str> = Vec::new();
        let mut pending: Vec<&str> = imports_of(module);
        while let Some(next) = pending.pop() {
            if !cone.contains(&next) {
                cone.push(next);
                pending.extend(imports_of(source_of(next)));
            }
        }
        // The cone in loading order, then the module; every import hoisted
        // to the head, since one source unit holds them all.
        let mut heads = String::new();
        let mut bodies = String::new();
        for (held, source) in library::MODULES {
            if !cone.contains(held) && held != name {
                continue;
            }
            // An empty line between modules and in place of each import
            // keeps every doc string opening after an empty line.
            bodies.push('\n');
            for line in source.lines() {
                if line.starts_with("import ") {
                    heads.push_str(line);
                    heads.push('\n');
                    bodies.push('\n');
                } else {
                    bodies.push_str(line);
                    bodies.push('\n');
                }
            }
        }
        let source = format!("{heads}{bodies}");
        assert_eq!(
            hql::check_in(&source, &vault),
            Ok(TypeRef::UNIT),
            "{name}: {:?}",
            hql::check_in(&source, &vault)
        );
    }
    assert_eq!(library::check(), Ok(()));
}

#[test]
fn a_declaration_yields_unit_and_the_program_carries_on() {
    assert_eq!(check("type Colour"), Ok(TypeRef::UNIT));
    assert_eq!(eval("type Colour"), Ok(Value::Unit));
    assert_eq!(check("type Colour\n1 + 1"), Ok(TypeRef::INT));
    assert_eq!(eval("type Colour\n1 + 1"), Ok(Value::Int(2)));
}

#[test]
fn every_written_form_parses() {
    for source in [
        "type Colour",
        "type Colour : Data",
        "type Pair<A, B>",
        "type Fruit : {Data, Orderable}",
        "type Person { name : String }",
        "type Person {\n    name : String\n    born? : Data\n}",
        "type Person { name : String, born? : Data }",
        "type Holder<T> { held : T }",
        "type Narrow : Card { metadata : Data }",
        "type Pairing<S, T> : Edge<S, T>",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
}

#[test]
fn colon_separates_parents_bounds_and_record_fields() {
    for source in [
        "type X : Card { title : String }",
        "type X : {Card, Doc} { title : String }",
        "type Box<T> { value : T } where T : Orderable",
        "type SortedMap<K, V> : OrderedMap<K, V> where K : Orderable",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
}

#[test]
fn the_former_subtype_operator_is_a_syntax_error() {
    for source in [
        "type X <: Card",
        "type X <: {Card, Doc}",
        "type Box<T> where T <: Orderable",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}"
        );
    }
}

#[test]
fn a_parent_must_exist() {
    assert!(refused("type Colour : Nonesuch").contains("unknown type `Nonesuch`"));
    assert!(refused("type Colour { shade : Nonesuch }").contains("unknown type `Nonesuch`"));
    // A parent declared earlier in the same program does exist.
    assert_eq!(check("type Base\ntype Narrow : Base"), Ok(TypeRef::UNIT));
}

#[test]
fn a_declaration_may_restate_a_built_in_type_but_not_contradict_it() {
    // This is what `lib/std/` is: the declarations the binary already holds,
    // written down in one place as source rather than as prose.
    assert_eq!(check("type Card : Doc"), Ok(TypeRef::UNIT));
    assert_eq!(check("type ConceptCard : {Card, Doc}"), Ok(TypeRef::UNIT));
    assert_eq!(check("type Ranking<T> : Seq<Hit<T>>"), Ok(TypeRef::UNIT));

    let contradiction = refused("type Card : Edge");
    assert!(
        contradiction.contains("`Card` is built in"),
        "{contradiction}"
    );
    assert!(
        contradiction.contains("does not narrow Edge"),
        "{contradiction}"
    );
    assert!(refused("type Ranking<T> : Seq<T>").contains("contradicts it"));
}

#[test]
fn a_supertype_set_a_value_could_never_satisfy_is_refused() {
    let refusal = refused("type Impossible : {Int, String}");
    assert!(
        refusal.contains("nothing is both Int and String"),
        "{refusal}"
    );
    // Related parents are redundant rather than contradictory, and a parent
    // the binary does not know is a declaration of its own.
    assert_eq!(check("type Narrow : {Card, Doc}"), Ok(TypeRef::UNIT));
    assert_eq!(
        check("type Idea\ntype Narrow : {Card, Idea}"),
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn contracts_and_unions_never_exclude_each_other_in_a_supertype_set() {
    // Only two concrete types can share nothing. A contract is satisfied by
    // other types, so two of them are two claims one value can meet — which
    // is what `type Int : {Scalar, Orderable}` relies on.
    for source in [
        "type Stamp : {Scalar, Orderable}",
        "type Flag : {Bool, Orderable}",
        "type Fruit : {Data, Orderable}",
        "type Int : {Scalar, Orderable}",
        "type Bool : Scalar",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
    // The built-in `Bool` is a leaf with no order, and restating it otherwise
    // contradicts the binary.
    assert!(refused("type Bool : {Scalar, Orderable}").contains("does not narrow Orderable"));
    assert!(refused("type Scalar").contains("contradicts its built-in kind"));
}

#[test]
fn a_union_lists_its_alternatives() {
    for source in [
        "type Id = union {Int, String}",
        "type Id = union {\n    Int,\n    String\n}",
        "type Id = union {Int, String,}",
        // A member may apply the union itself: that is what makes a tree.
        "type Value = union {Scalar, Seq<Value>, Map<String, Value>}",
        "type Tree<T> = union {T, Seq<Tree<T>>}",
        "type Id = union {Int, String}\ntype Holder { id : Id }",
        // What `lib/std/core/mod.hql` says, and what the binary holds.
        "type Data = union {Scalar, Seq<Data>, Map<String, Data>}",
        "type Data = union {Map<String, Data>, Scalar, Seq<Data>}",
    ] {
        assert_eq!(check(source), Ok(TypeRef::UNIT), "{source}");
    }
    assert_eq!(eval("type Id = union {Int, String}"), Ok(Value::Unit));
}

#[test]
fn a_union_that_does_not_hold_is_refused() {
    assert!(refused("type Id = union {Int, Nonesuch}").contains("unknown type `Nonesuch`"));
    assert!(refused("type Id = union {Int, Int}").contains("lists Int twice"));
    assert!(refused("type Id = union {Seq<Int>, Seq<Int>}").contains("lists Seq<Int> twice"));
    assert!(refused("type Id = union {Int, Id}").contains("lists itself as a member"));
    assert!(refused("type Id = union {Map<Int>}").contains("expects 2 type arguments"));
    // A refused union is withdrawn, so its name is free for a second attempt.
    assert!(refused("type Id = union {Id}\ntype Use { id : Id }").contains("lists itself"));

    // A built-in union is restated member for member.
    let narrower = refused("type Data = union {Scalar}");
    assert!(
        narrower.contains("built in as union {Scalar, Seq<Data>, Map<String, Data>}"),
        "{narrower}"
    );
    assert!(
        refused("type Data = union {Scalar, Seq<Data>, Map<String, Data>, Card}")
            .contains("contradicts it")
    );
    assert!(refused("type Card = union {Int, String}").contains("not a union"));
}

#[test]
fn a_union_is_written_with_equals_and_nothing_else() {
    for source in [
        "type Id = {Int, String}",
        "type Id = Int",
        "type Id = union",
        "type Id = union {}",
        "type Id = union {Int} : Data",
        "type Id : Data = union {Int}",
        "abstract type Id = union {Int}",
    ] {
        assert!(
            matches!(check(source), Err(Diagnostic::Syntax { .. })),
            "{source}: {:?}",
            check(source)
        );
    }
    // `union` is a word only after `=` in a declaration; elsewhere it is a name.
    assert_eq!(check("union = 1\nunion"), Ok(TypeRef::INT));
}

#[test]
fn a_name_is_declared_once_and_a_field_appears_once() {
    assert!(refused("type Colour\ntype Colour").contains("is declared twice"));
    assert!(
        refused("type Person { name : String, name : Data }").contains("declares `name` twice")
    );
}

#[test]
fn a_type_parameter_is_a_name_no_type_already_has() {
    assert!(refused("type Holder<Card>").contains("`Card` is a type"));
    assert!(refused("type Pair<T, T>").contains("takes `T` twice"));
    // Inside the body a parameter is an ordinary type name, and it takes no
    // arguments of its own.
    assert_eq!(check("type Holder<T> { held : T }"), Ok(TypeRef::UNIT));
    assert!(refused("type Holder<T> { held : T<Card> }").contains("takes no arguments"));
}

#[test]
fn arity_is_checked_where_arguments_are_written() {
    assert!(
        refused("type Pair<A, B>\ntype Use { held : Pair<Card> }")
            .contains("takes 2 type arguments")
    );
    // A bare name is the constructor itself — `Graph<Card, Link>` passes
    // `Link`, not a `Link` of something — so it is not an arity failure.
    assert_eq!(
        check("type Pair<A, B>\ntype Use { held : Pair }"),
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn narrowing_may_not_turn_a_required_field_into_an_optional_one() {
    // A subtype narrows its parent. Making a required key optional widens the
    // shape, so a value of the subtype could fail to be one of the parent.
    let refusal = refused("type Base { header : Data }\ntype Narrow : Base { header? : Data }");
    assert!(refusal.contains("`Base` requires `header`"), "{refusal}");
    assert_eq!(
        check("type Base { header? : Data }\ntype Narrow : Base { header? : Data }"),
        Ok(TypeRef::UNIT)
    );
}

#[test]
fn a_declaration_is_a_statement_and_not_an_expression() {
    // `type` only opens a statement; a name may still be bound to a value.
    assert_eq!(check("type = 1\ntype"), Ok(TypeRef::INT));
    assert!(matches!(
        check("1 | type Colour"),
        Err(Diagnostic::Syntax { .. }) | Err(Diagnostic::Name { .. })
    ));
}

#[test]
fn a_built_in_record_field_has_the_type_its_declaration_gives() {
    // Fields are read from `lib/std/`, so each one a program reads is pinned
    // here at the type its declaration states.
    for (receiver, field, declared) in [
        ("Hit<Card>", "value", "Card"),
        ("Hit<Card>", "score", "Float"),
        ("Hit<Card>", "query", "String"),
        ("Hit<Card>", "retrieval", "Data"),
        ("Ranking<Int>", "hits", "List<Hit<Int>>"),
        ("Ranking<Int>", "query", "String"),
        ("Ranking<Int>", "retrieval", "Data"),
        ("Doc", "name", "String"),
        ("Doc", "title", "String"),
        ("Doc", "path", "String"),
        ("Doc", "format", "String"),
        ("Doc", "header", "Data"),
        ("Doc", "body", "String"),
        ("Card", "title", "String"),
        ("Card", "metadata", "Data"),
        ("Card", "kind", "String"),
        ("ConceptCard", "kind", "String"),
        ("RelationCard", "metadata", "Data"),
        ("Edge", "source", "String"),
        ("Edge", "target", "String"),
        ("Edge", "data", "Data"),
        ("Graph", "nodes", "Set<Card>"),
        ("Graph", "edges", "Set<Edge>"),
        ("Group<String, Int>", "key", "String"),
        ("Group<String, Int>", "items", "List<Int>"),
        ("Option<Card>", "kind", "Option<String>"),
    ] {
        let source = format!("fn read(x: {receiver}) : {declared} = x.{field}\nread");
        assert_eq!(
            check(&source).map(|inferred| inferred.to_string()),
            Ok(format!("{receiver} -> {declared}")),
            "{source}"
        );
    }
    // A field the declaration does not have is not there, inherited or not.
    assert!(refused("fn read(d: Doc) : Data = d.metadata\nread").contains("no field `metadata`"));
    assert!(refused("fn read(h: Hit<Int>) : Data = h.provenance\nread").contains("no field"));
}

#[test]
fn a_function_narrows_with_a_wider_parameter_and_a_narrower_result() {
    use hql::types::TypeRef;
    let function = |parameter: TypeRef, result: TypeRef| {
        TypeRef::function(vec![parameter], result).expect("a declared arity")
    };
    // Accepts more and returns less: usable where Card -> Data is expected.
    let wider = function(TypeRef::DOC, TypeRef::INT);
    let expected = function(TypeRef::CARD, TypeRef::DATA);
    assert!(wider.is(&expected));
    assert!(!expected.is(&wider));
    // A narrower parameter is the wrong direction.
    assert!(!function(TypeRef::CARD, TypeRef::INT).is(&function(TypeRef::DOC, TypeRef::INT)));
    // Different arities are different constructors.
    let binary = TypeRef::function(vec![TypeRef::INT, TypeRef::INT], TypeRef::INT).unwrap();
    assert!(!binary.is(&function(TypeRef::INT, TypeRef::INT)));
}

#[test]
fn a_function_type_prints_with_an_arrow() {
    use hql::types::TypeRef;
    let unary = TypeRef::function(vec![TypeRef::CARD], TypeRef::STR).unwrap();
    assert_eq!(unary.to_string(), "Card -> String");
    let binary = TypeRef::function(vec![TypeRef::INT, TypeRef::INT], TypeRef::ORDERING).unwrap();
    assert_eq!(binary.to_string(), "(Int, Int) -> Ordering");
    assert_eq!(
        TypeRef::function(vec![], TypeRef::INT).unwrap().to_string(),
        "() -> Int"
    );
    assert_eq!(
        hql::check("fn inc(n: Int) : Int = n + 1\ninc")
            .unwrap()
            .to_string(),
        "Int -> Int"
    );
    let error = hql::check("fn inc(n: Int) : Int = n + 1\n[\"a\"] | map(inc)")
        .unwrap_err()
        .to_string();
    assert!(error.contains("Int"), "{error}");
}

#[test]
fn a_function_called_with_the_wrong_number_of_arguments_is_refused() {
    let error = hql::check("fn inc(n: Int) : Int = n + 1\ninc(1, 2)")
        .unwrap_err()
        .to_string();
    assert!(error.contains("takes 1 arguments"), "{error}");
    let unannotated = hql::check("f = c => c").unwrap_err().to_string();
    assert!(
        unannotated.contains("types its parameters"),
        "{unannotated}"
    );
    let result = hql::check("fn f(n: Int) : String = n")
        .unwrap_err()
        .to_string();
    assert!(result.contains("declared to return String"), "{result}");
}

#[test]
fn every_name_the_binary_provides_follows_its_kind() {
    use hql::naming::{Kind, departure};
    for extension in hql::extensions::catalogue(&hql::extensions::Config::default()) {
        assert_eq!(departure(Kind::Extension, extension.name), None);
        for step in extension.steps {
            // A step named for a type is that type's constructor, so it is
            // named as the type is: `Logger(stderr, Text, name = "q")`.
            let kind = if hql::types::named(step.name, &[]).is_some() {
                Kind::Type
            } else {
                Kind::Function
            };
            assert_eq!(departure(kind, step.name), None, "step {}", step.name);
        }
    }
    // Every name `lib/std/` declares passes: checking it reports no warning.
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    let outcome = hql::check_reported(&library::source(), &vault, hql::reporting::Mode::Strict);
    let warnings: Vec<_> = outcome
        .reports
        .iter()
        .map(|report| report.message.clone())
        .collect();
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn a_name_off_its_kinds_convention_is_a_warning_at_the_name() {
    let vault = hql::vault::Vault::empty();
    for (source, name, message) in [
        (
            "birdCards = 1\nbirdCards",
            "birdCards",
            "a binding, written in snake_case",
        ),
        (
            "fn get_size(xs: List<Int>) : Int = xs | size\n1",
            "fn get_size",
            "without a `get_` prefix",
        ),
        (
            "type book { title : String }\n1",
            "type book",
            "a type, written in PascalCase",
        ),
        (
            "type Book { Title : String }\n1",
            "Title : String",
            "a field, written in snake_case",
        ),
        (
            "[1] | map(N => N)",
            "N => N",
            "a parameter, written in snake_case",
        ),
    ] {
        let outcome = hql::run(source, &vault, hql::reporting::Mode::Strict);
        assert!(!outcome.failed(), "{source}: a warning never fails");
        let warning = outcome
            .reports
            .iter()
            .find(|report| report.message.contains(message))
            .unwrap_or_else(|| panic!("{source}: {:?}", outcome.reports));
        assert!(
            source[warning.span.clone()].starts_with(name),
            "{source}: {:?}",
            warning.span
        );
    }
}

/// HQL declares and an extension implements: a standard module written as
/// external declarations names exactly the functions its extension provides,
/// so neither side can drift from the other.
#[test]
fn every_external_declaration_has_an_implementation_and_every_implementation_a_declaration() {
    let catalogue = hql::extensions::catalogue(&hql::extensions::Config::default());
    let mut checked = 0;
    for (name, contract) in library::contracts().expect("std parses") {
        let declared: Vec<&str> = contract
            .iter()
            .map(|declaration| declaration.name.as_str())
            .collect();
        let provider = catalogue
            .iter()
            .find(|provider| provider.provides.contains(&name))
            .unwrap_or_else(|| {
                panic!(
                    "lib/{}/mod.hql declares externally, and no extension provides `{name}`",
                    library::path(name)
                )
            });
        let mut implemented: Vec<&str> = provider.steps.iter().map(|step| step.name).collect();
        let mut declared = declared;
        implemented.sort_unstable();
        declared.sort_unstable();
        declared.dedup();
        assert_eq!(declared, implemented, "module `{name}`");
        checked += 1;
    }
    assert!(
        checked >= 10,
        "only {checked} modules are declared externally"
    );
}

#[test]
fn a_generic_record_holds_its_fields_to_the_parameter_as_finally_bound() {
    assert!(
        refused(
            "type Box<T> { f : T -> Int, x : T }\nb = Box(f = (n: Int) => n + 1, x = \"s\")\nb"
        )
        .contains("`f` of `Box`")
    );
    // A parameter a field's function takes is not covariant.
    assert!(
        refused("type Box<T> { f : T -> Int }\nb : Box<Data> = Box(f = (n: Int) => n + 1)\nb")
            .contains("declared Box<Data>")
    );
    for source in [
        "type Box<T> { x : T }\nb : Box<Data> = Box(x = 1)\nb.x",
        "type Sink<T> { f : T -> Int }\ns : Sink<Int> = Sink(f = (d: Data) => 1)\ng = s.f\ng(3)",
    ] {
        assert_eq!(
            eval(source).map(|value| value.to_string()),
            Ok("1".to_owned()),
            "{source}"
        );
    }
}
