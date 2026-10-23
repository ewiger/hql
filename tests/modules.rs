//! Import units, the program module and declaration visibility: each module
//! is a closure over its own imports, and a `private` declaration is never
//! exported (HQL-0028).

use hql::{check_in, reporting::Mode, run, vault::Vault};
use std::path::Path;

/// A module root holding each `(name, source)` file, and a vault that
/// discovers modules under it.
fn workspace(files: &[(&str, &str)]) -> (tempfile::TempDir, Vault) {
    let root = tempfile::tempdir().unwrap();
    for (name, source) in files {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let mut vault = Vault::empty();
    vault
        .extensions
        .module_roots
        .push(root.path().to_path_buf());
    (root, vault)
}

fn refused(files: &[(&str, &str)], program: &str) -> String {
    let (_root, vault) = workspace(files);
    match check_in(program, &vault) {
        Err(diagnostic) => diagnostic.message(),
        Ok(inferred) => panic!("{program}: expected a refusal, got {inferred}"),
    }
}

fn shown(files: &[(&str, &str)], program: &str) -> String {
    let (_root, vault) = workspace(files);
    let outcome = run(program, &vault, Mode::Strict);
    assert!(!outcome.failed(), "{program}: {:?}", outcome.reports);
    outcome.value.expect("a value").to_string()
}

const HELPERS: &str = "\
module m
private fn twice(n: Int) : Int = n * 2
fn quadruple(n: Int) : Int = twice(twice(n))
limit = 10
private secret = 42
";

#[test]
fn a_private_function_is_used_inside_its_module_and_reached_by_no_importer() {
    let files = [("m.hql", HELPERS)];
    assert_eq!(
        shown(&files, "import m\n[m.quadruple(3), m.limit]"),
        "[12, 10]"
    );
    assert!(refused(&files, "import m\nm.twice(3)").contains("no function `twice`"));
    assert!(refused(&files, "import m\nm.secret").contains("no function `secret`"));
    assert!(refused(&files, "import m\ntwice(3)").contains("not bound"));
}

#[test]
fn a_private_receiver_function_joins_no_importers_receiver_lookup() {
    let files = [(
        "text.hql",
        "module text for String\n\
         private fn yell(self) : String = self.upper()\n\
         fn shout(self) : String = fmt(\"{}!\", self.yell())\n",
    )];
    assert_eq!(shown(&files, "import text\n\"hi\".shout()"), "HI!");
    assert!(refused(&files, "import text\n\"hi\".yell()").contains("no function `yell`"));
    assert!(refused(&files, "import text\ntext.yell(\"hi\")").contains("no function `yell`"));
}

#[test]
fn a_private_type_is_named_by_no_importer_and_no_public_signature() {
    let files = [(
        "shapes.hql",
        "module shapes\nprivate type Point { x : Int, y : Int }\n\
         fn norm1(x: Int, y: Int) : Int = sum_of(Point(x = x, y = y))\n\
         private fn sum_of(p: Point) : Int = p.x + p.y\n",
    )];
    assert!(refused(&files, "import shapes\nshapes.norm1(1, 2)").contains("not bound"));
    let files = [(
        "shapes.hql",
        "module shapes\nprivate type Point { x : Int, y : Int }\n\
         private fn sum_of(p: Point) : Int = p.x + p.y\n\
         fn norm1(x: Int, y: Int) : Int = sum_of(Point(x = x, y = y))\n",
    )];
    assert_eq!(shown(&files, "import shapes\nshapes.norm1(1, 2)"), "3");
    assert!(
        refused(&files, "import shapes\nPoint(x = 1, y = 2)")
            .contains("`Point` is private to the module `shapes`")
    );
    assert!(
        refused(&files, "import shapes\nfn f(p: Point) : Int = 1\n1")
            .contains("private to the module `shapes`")
    );
    for leaking in [
        "module leak\nprivate type Point { x : Int }\nfn origin() : Point = Point(x = 0)\n",
        "module leak\nprivate type Point { x : Int }\nfn x_of(p: Point) : Int = p.x\n",
        "module leak\nprivate type Point { x : Int }\ntype Line { from : Point, to : Point }\n",
        "module leak\nprivate trait Shape { fn area(self) : Float }\nfn big<T : Shape>(s: T) : Bool = true\n",
    ] {
        let files = [("leak.hql", leaking)];
        let message = refused(&files, "import leak\n1");
        assert!(
            message.contains("is public and its signature mentions"),
            "{message}"
        );
    }
}

#[test]
fn a_private_trait_is_not_exported() {
    let files = [(
        "sized.hql",
        "module sized\nprivate trait Sized { fn width(self) : Int }\n\
         type Box { w : Int }\n\
         impl Sized for Box { fn width(self) : Int = self.w }\n\
         fn box_width(b: Box) : Int = b.width()\n",
    )];
    assert_eq!(
        shown(&files, "import sized\nsized.box_width(Box(w = 3))"),
        "3"
    );
    assert!(refused(&files, "import sized\nfn f<T : Sized>(x: T) : Int = 1\n1").contains("Sized"));
}

#[test]
fn private_is_refused_on_an_impl_and_in_the_program_module() {
    let message = refused(
        &[],
        "trait T { fn f(self) : Int }\ntype B { x : Int }\nprivate impl T for B { fn f(self) : Int = 1 }\n1",
    );
    assert!(message.contains("an `impl` is never private"), "{message}");
    let message = refused(&[], "private fn f() : Int = 1\nf()");
    assert!(
        message.contains("`private` belongs to a module"),
        "{message}"
    );
    // `private` is recognized by position, so it is no reserved word.
    let (_root, vault) = workspace(&[]);
    assert_eq!(
        check_in("private = 1\nprivate + 1", &vault)
            .unwrap()
            .to_string(),
        "Int"
    );
}

#[test]
fn internal_is_reserved() {
    let files = [("m.hql", "module m\ninternal fn f() : Int = 1\n")];
    let message = refused(&files, "import m\n1");
    assert!(message.contains("`internal` is reserved"), "{message}");
}

#[test]
fn a_module_sees_no_host_binding() {
    let vault_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault");
    let root = tempfile::tempdir().unwrap();
    for (name, source) in [
        (
            "reads_cards.hql",
            "module reads_cards\nfn count() : Int = cards | size\n",
        ),
        (
            "reads_vault.hql",
            "module reads_vault\nfn named() : String = vault.name\n",
        ),
        (
            "reads_stdin.hql",
            "module reads_stdin\nfn line() : String = stdin | first\n",
        ),
    ] {
        std::fs::write(root.path().join(name), source).unwrap();
    }
    let mut vault = hql::vault::load(&vault_root).unwrap();
    vault
        .extensions
        .module_roots
        .push(root.path().to_path_buf());
    // The program module holds them.
    assert!(check_in("[cards | size, vault.cards | size]", &vault).is_ok());
    for module in ["reads_cards", "reads_vault", "reads_stdin"] {
        let message = check_in(&format!("import {module}\n1"), &vault)
            .unwrap_err()
            .message();
        assert!(
            message.contains("is not bound in a module"),
            "{module}: {message}"
        );
    }
}

#[test]
fn an_import_is_visible_in_the_importing_unit_only() {
    // The program imports `regex`; the module it imports does not, so the
    // module does not see it.
    let files = [(
        "words.hql",
        "module words\nfn has_digit(s: String) : Bool = regex.is_match(s, \"[0-9]\")\n",
    )];
    let message = refused(&files, "import regex\nimport words\n1");
    assert!(message.contains("regex"), "{message}");
    // A module's own import is its own: the program does not see it.
    let files = [(
        "words.hql",
        "module words\nimport regex\nfn has_digit(s: String) : Bool = regex.is_match(s, \"[0-9]\")\n",
    )];
    assert_eq!(
        shown(&files, "import words\nwords.has_digit(\"a1\")"),
        "true"
    );
    assert!(refused(&files, "import words\nregex.is_match(\"a1\", \"[0-9]\")").contains("regex"));
}

#[test]
fn two_imports_declaring_one_receiver_function_are_ambiguous() {
    let files = [
        (
            "a.hql",
            "module a for String\nfn tidy(self) : String = \"a\"\n",
        ),
        (
            "b.hql",
            "module b for String\nfn tidy(self) : String = \"b\"\n",
        ),
        // `trim` is the prelude's `string.trim` as well.
        (
            "c.hql",
            "module c for String\nfn trim(self) : String = \"c\"\n",
        ),
    ];
    let message = refused(&files, "import a\nimport b\n\"x\".tidy()");
    assert!(
        message.contains("`a.tidy` and `b.tidy`") && message.contains("ambiguous"),
        "{message}"
    );
    // Import order never decides.
    assert!(refused(&files, "import b\nimport a\n\"x\".tidy()").contains("ambiguous"));
    assert_eq!(
        shown(&files, "import a\nimport b\n[a.tidy(\"x\"), b.tidy(\"x\")]"),
        "[\"a\", \"b\"]"
    );
    let message = refused(&files, "import c\n\" x \".trim()");
    assert!(message.contains("`c.trim` and `string.trim`"), "{message}");
    assert_eq!(
        shown(&files, "import c\n[c.trim(\" x \"), string.trim(\" x \")]"),
        "[\"c\", \"x\"]"
    );
}

#[test]
fn a_units_own_declaration_wins_over_an_import_in_every_spelling() {
    let files = [
        (
            "a.hql",
            "module a for String\nfn tidy(self) : String = \"a\"\n",
        ),
        (
            "own.hql",
            "module own for String\nimport a\n\
             fn tidy(self) : String = \"own\"\n\
             fn both(s: String) : List<String> = [s.tidy(), \" x \".trim()]\n\
             fn trim(self) : String = \"mine\"\n",
        ),
    ];
    // Inside `own`, its `tidy` wins over `a.tidy`; its `trim` is declared
    // below the use, so the use is refused rather than reaching the
    // prelude's.
    let message = refused(&files, "import own\n1");
    assert!(message.contains("declared by this unit below"), "{message}");
    let files = [
        (
            "a.hql",
            "module a for String\nfn tidy(self) : String = \"a\"\n",
        ),
        (
            "own.hql",
            "module own for String\nimport a\n\
             fn tidy(self) : String = \"own\"\n\
             fn trim(self) : String = \"mine\"\n\
             fn both(s: String) : List<String> = [s.tidy(), \" x \".trim(), a.tidy(s)]\n",
        ),
    ];
    assert_eq!(
        shown(&files, "import own\nown.both(\"s\")"),
        "[\"own\", \"mine\", \"a\"]"
    );
    // A program's own function wins as a call and a step, and a receiver
    // call of its name reaches no import's in its place.
    let program = "fn lower(s: String) : String = \"own\"\n";
    assert_eq!(
        shown(&[], &format!("{program}[lower(\"A\"), \"A\" | lower]")),
        "[\"own\", \"own\"]"
    );
    assert!(refused(&[], &format!("{program}\"A\".lower()")).contains("this unit's own function"));
    // Declared below its use, it is refused rather than the import taken.
    assert!(
        refused(&[], &format!("x = lower(\"A\")\n{program}x"))
            .contains("declared by this unit below")
    );
}

#[test]
fn the_import_graph_is_acyclic() {
    let files = [
        ("a.hql", "module a\nimport b\nfn f() : Int = 1\n"),
        ("b.hql", "module b\nimport c\nfn g() : Int = 1\n"),
        ("c.hql", "module c\nimport a\nfn h() : Int = 1\n"),
    ];
    let message = refused(&files, "import a\n1");
    assert!(
        message.contains("circular: a imports b imports c imports a"),
        "{message}"
    );
}

#[test]
fn a_parent_importing_its_child_is_warned_and_still_runs() {
    let files = [
        (
            "shapes/circle.hql",
            "module shapes.circle\nfn area(r: Int) : Int = 3 * r * r\n",
        ),
        (
            "shapes/mod.hql",
            "module shapes\nimport shapes.circle\nfn unit_area() : Int = circle.area(1)\n",
        ),
    ];
    let (_root, vault) = workspace(&files);
    let outcome = run("import shapes\nshapes.unit_area()", &vault, Mode::Strict);
    assert_eq!(outcome.value.expect("a value").to_string(), "3");
    let messages: Vec<_> = outcome.reports.iter().map(|r| r.message.clone()).collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("imports its own child `shapes.circle`")),
        "{messages:?}"
    );
    // A child importing its parent is the direction the tree allows.
    let files = [
        ("shapes/mod.hql", "module shapes\nfn unit() : Int = 1\n"),
        (
            "shapes/circle.hql",
            "module shapes.circle\nimport shapes\nfn area(r: Int) : Int = 3 * r * r * shapes.unit()\n",
        ),
    ];
    let (_root, vault) = workspace(&files);
    let outcome = run("import shapes.circle\ncircle.area(1)", &vault, Mode::Strict);
    assert!(outcome.reports.is_empty(), "{:?}", outcome.reports);
}

#[test]
fn a_module_file_opens_with_the_name_its_position_gives() {
    let program = "import shapes.circle\ncircle.area(1)";
    assert_eq!(
        shown(
            &[(
                "shapes/circle.hql",
                "// comments may come first\nmodule shapes.circle\nfn area(r: Int) : Int = 3 * r * r\n",
            )],
            program,
        ),
        "3"
    );
    for (source, expected) in [
        (
            "fn area(r: Int) : Int = 3 * r * r\n",
            "write `module shapes.circle`",
        ),
        (
            "module circle\nfn area(r: Int) : Int = 3 * r * r\n",
            "`module circle` names another module: this file is the module `shapes.circle`",
        ),
        (
            "import math\nmodule shapes.circle\nfn area(r: Int) : Int = 3 * r * r\n",
            "before any other statement",
        ),
        (
            "module shapes.circle\nmodule shapes.circle\nfn area(r: Int) : Int = 3 * r * r\n",
            "declares its name once",
        ),
        (
            "module for String\nfn area(r: Int) : Int = 3 * r * r\n",
            "`module string for String`",
        ),
    ] {
        let message = refused(&[("shapes/circle.hql", source)], program);
        assert!(message.contains(expected), "{source}: {message}");
    }
}
