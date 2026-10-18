//! `Scalar` and `Data`: which values are leaves, and which are trees.
//!
//! `Data` is a union, so a type is a `Data` by being one of its members rather
//! than by declaring it as a parent. These tests pin the membership, the
//! multi-parent ancestry the scalars need, and the registry rules behind both.

use hql::types::{
    TypeConstructor, TypeDefinition, TypeError, TypeKind, TypeRef, TypeSystem, builtin, collections,
};
use hql::{check, eval};

fn map(key: TypeRef, value: TypeRef) -> TypeRef {
    collections::MAP.apply([key, value])
}

#[test]
fn every_scalar_is_a_leaf_and_only_the_ordered_ones_are_orderable() {
    for scalar in [
        TypeRef::BOOL,
        TypeRef::INT,
        TypeRef::FLOAT,
        TypeRef::STR,
        TypeRef::DATE,
    ] {
        assert!(scalar.is(&TypeRef::SCALAR), "{scalar}");
        assert!(scalar.is_concrete(), "{scalar}");
    }
    for ordered in [TypeRef::INT, TypeRef::FLOAT, TypeRef::STR, TypeRef::DATE] {
        assert!(ordered.is_orderable(), "{ordered}");
    }
    // Being a leaf and having an order are separate contracts.
    assert!(!TypeRef::BOOL.is_orderable());
    assert!(!TypeRef::SCALAR.is(&TypeRef::ORDERABLE));
    assert!(!TypeRef::ORDERABLE.is(&TypeRef::SCALAR));
    // A card is orderable without being a leaf.
    assert!(!TypeRef::CARD.is(&TypeRef::SCALAR));
    assert!(!TypeRef::SCALAR.is_concrete());
}

#[test]
fn data_is_a_leaf_a_sequence_of_trees_or_a_string_keyed_map_of_trees() {
    let system = builtin::system().expect("the built-in registry");
    let definition = system
        .definition(TypeRef::DATA.constructor)
        .expect("a declaration of Data");
    assert_eq!(
        definition.kind,
        TypeKind::Union(vec![
            TypeRef::SCALAR,
            TypeRef::seq(TypeRef::DATA),
            map(TypeRef::STR, TypeRef::DATA),
        ])
    );

    for tree in [
        TypeRef::DATA,
        TypeRef::SCALAR,
        TypeRef::INT,
        TypeRef::BOOL,
        TypeRef::seq(TypeRef::INT),
        TypeRef::list(TypeRef::STR),
        // Membership recurses through the argument: a list of lists of maps.
        TypeRef::list(TypeRef::list(map(TypeRef::STR, TypeRef::FLOAT))),
        map(TypeRef::STR, TypeRef::list(TypeRef::DATA)),
        collections::SORTED_MAP.apply([TypeRef::STR, TypeRef::INT]),
        TypeRef::list(TypeRef::NEVER),
    ] {
        assert!(tree.is(&TypeRef::DATA), "{tree}");
    }
}

#[test]
fn what_no_member_admits_is_not_data() {
    for other in [
        // A set has no positions, and a tree's children have them.
        TypeRef::set(TypeRef::INT),
        // A tree's keys are strings.
        map(TypeRef::INT, TypeRef::INT),
        TypeRef::list(TypeRef::CARD),
        TypeRef::CARD,
        TypeRef::EDGE,
        TypeRef::UNIT,
        TypeRef::optional(TypeRef::INT),
    ] {
        assert!(!other.is(&TypeRef::DATA), "{other}");
    }
    // Membership runs one way: a tree is not thereby a leaf or a sequence.
    assert!(!TypeRef::DATA.is(&TypeRef::SCALAR));
    assert!(!TypeRef::DATA.is(&TypeRef::seq(TypeRef::DATA)));
    assert!(!TypeRef::DATA.is_concrete());
}

#[test]
fn a_binding_may_view_a_member_as_data_without_converting_it() {
    assert_eq!(check("tree : Data = 1\ntree"), Ok(TypeRef::DATA));
    assert_eq!(check("tree : Data = [1, 2]\ntree"), Ok(TypeRef::DATA));
    assert_eq!(
        check("tree : Data = Map([\"owl\"], [ [1, 2] ])\ntree"),
        Ok(TypeRef::DATA)
    );
    assert_eq!(check("leaf : Scalar = true\nleaf"), Ok(TypeRef::SCALAR));
    // The value is the value it was: a view changes what is known, not what
    // is held.
    assert_eq!(
        eval("tree : Data = [1, 2]\ntree").unwrap().to_string(),
        "[1, 2]"
    );

    for (source, reason) in [
        ("tree : Data = {1, 2}", "its value is Set<Int>"),
        ("tree : Data = Map([1], [1])", "its value is Map<Int, Int>"),
        ("leaf : Scalar = [1]", "its value is List<Int>"),
    ] {
        let refusal = check(source).expect_err(source).message();
        assert!(refusal.contains(reason), "{source}: {refusal}");
    }
}

#[test]
fn a_tree_beside_leaves_makes_a_sequence_of_trees() {
    let source = "[{ species: \"owl\" }, \"owl\", 3]";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::DATA)));
    assert_eq!(
        eval(source).unwrap().to_string(),
        "[{species: \"owl\"}, owl, 3]"
    );
}

#[test]
fn unrelated_trees_are_held_as_data_wherever_they_stand() {
    // Any JSON array can be written: leaves of different types are trees.
    for source in [
        "[1, \"owl\"]",
        "[\"owl\", 1]",
        "[1, \"owl\", { a: 1 }]",
        "[{ a: 1 }, 1, \"owl\"]",
        "[1, 2.5, true]",
        "[1, [2]]",
    ] {
        assert_eq!(check(source), Ok(TypeRef::list(TypeRef::DATA)), "{source}");
        assert_eq!(
            eval(source).expect(source).type_of(),
            TypeRef::list(TypeRef::DATA),
            "{source}"
        );
    }
    assert_eq!(check("{1, \"owl\"}"), Ok(TypeRef::set(TypeRef::DATA)));
    // Arguments meet first, which says more than the tree they both are.
    assert_eq!(
        check("[ [1], [\"owl\"] ]"),
        Ok(TypeRef::list(TypeRef::list(TypeRef::DATA)))
    );
    assert_eq!(check("tree : Data = [1, \"owl\"]\ntree"), Ok(TypeRef::DATA));
    assert_eq!(
        eval("{ tags: [1, \"owl\", [true, 2.5]] }")
            .unwrap()
            .to_string(),
        "{tags: [1, \"owl\", [true, 2.5]]}"
    );
    // Related types still meet where they always did.
    assert_eq!(check("[1, 2]"), Ok(TypeRef::list(TypeRef::INT)));
}

#[test]
fn what_is_not_a_tree_still_has_nothing_in_common_with_one() {
    // A set has no positions, so it is no tree, and neither is absence.
    for source in ["[1, {2}]", "[{2}, 1]", "[\"owl\", Map([1], [1])]"] {
        let refusal = check(source).expect_err(source).message();
        assert!(
            refusal.contains("have no common type"),
            "{source}: {refusal}"
        );
    }
    // The mistake of mixing leaves now surfaces where an order was wanted.
    let refusal = check("[1, \"owl\"] | sort")
        .expect_err("an unordered tree")
        .message();
    assert!(refusal.contains("Data has none"), "{refusal}");
    // A common type is for holding values together; it does not make two concrete
    // types one, so a supertype set of them is still uninhabitable.
    assert_eq!(TypeRef::INT.common_type(&TypeRef::STR), Some(TypeRef::DATA));
    assert_eq!(TypeRef::INT.declared_common_type(&TypeRef::STR), None);
    assert!(check("type Impossible : {Int, String}").is_err());
}

#[test]
fn a_list_checked_through_a_view_also_evaluates() {
    // A view is static, so the leaf is still an `Int` when the list is built.
    // What the checker accepted, evaluation must not refuse.
    let source = "count : Scalar = 3\ntree : Data = [\"dusk\"]\n[count, \"owl\", true, tree]";
    assert_eq!(check(source), Ok(TypeRef::list(TypeRef::DATA)));
    let value = eval(source).expect("a checked program evaluates");
    assert_eq!(value.to_string(), "[3, owl, true, [dusk]]");
    assert_eq!(value.type_of(), TypeRef::list(TypeRef::DATA));
}

#[test]
fn an_ancestor_reached_along_two_paths_is_one_ancestor() {
    let scalar = |name, kind, parents: &[&'static str]| TypeDefinition {
        constructor: TypeConstructor(name),
        kind,
        parameters: vec![],
        parents: parents.iter().copied().map(TypeConstructor).collect(),
        applied: vec![],
    };
    let mut system = TypeSystem::new();
    for definition in [
        scalar("Root", TypeKind::Abstract, &[]),
        scalar("Left", TypeKind::Abstract, &["Root"]),
        scalar("Right", TypeKind::Abstract, &["Root"]),
        scalar("Both", TypeKind::Concrete, &["Left", "Right"]),
    ] {
        system.declare(definition).expect("a coherent declaration");
    }
    let both = TypeRef::from(TypeConstructor("Both"));
    for ancestor in ["Both", "Left", "Right", "Root"] {
        let ancestor = TypeRef::from(TypeConstructor(ancestor));
        assert_eq!(system.is_subtype(&both, &ancestor), Ok(true), "{ancestor}");
    }
    let left = TypeRef::from(TypeConstructor("Left"));
    let right = TypeRef::from(TypeConstructor("Right"));
    assert_eq!(system.is_subtype(&left, &right), Ok(false));

    // Every parent must already exist, which is what keeps ancestry acyclic.
    assert_eq!(
        system.declare(scalar("Orphan", TypeKind::Concrete, &["Root", "Nowhere"])),
        Err(TypeError::UnknownConstructor(TypeConstructor("Nowhere")))
    );
}

#[test]
fn a_union_needs_members_that_exist_and_are_not_itself() {
    let union = |name, members: Vec<TypeRef>| TypeDefinition {
        constructor: TypeConstructor(name),
        kind: TypeKind::Union(members),
        parameters: vec![],
        parents: vec![],
        applied: vec![],
    };
    let mut system = collections::type_system().expect("the collection hierarchy");
    assert_eq!(
        system.declare(union("Nothing", vec![])),
        Err(TypeError::EmptyUnion(TypeConstructor("Nothing")))
    );
    let itself = TypeRef::from(TypeConstructor("Loop"));
    assert_eq!(
        system.declare(union("Loop", vec![itself])),
        Err(TypeError::SelfMember(TypeConstructor("Loop")))
    );
    // A refused union leaves nothing behind, so the name is still free.
    let missing = TypeRef::from(TypeConstructor("Nowhere"));
    assert_eq!(
        system.declare(union("Tree", vec![missing])),
        Err(TypeError::UnknownConstructor(TypeConstructor("Nowhere")))
    );
    let tree = TypeRef::from(TypeConstructor("Tree"));
    assert_eq!(
        system.declare(union("Tree", vec![TypeRef::seq(tree.clone())])),
        Ok(())
    );
    assert_eq!(
        system.is_subtype(&TypeRef::list(TypeRef::list(tree.clone())), &tree),
        Ok(true)
    );
    // A union is what its members are, so it is never a value's outer type.
    assert_eq!(
        system.validate_concrete(&tree),
        Err(TypeError::AbstractType(tree))
    );
}

#[test]
fn a_key_of_a_tree_is_optional_and_a_missing_one_is_absence() {
    let tree = "tree = {genre: \"novel\", pages: 412, note: {}}\n";
    // `field? : T` is exactly `Option<T>`: reading a key says it may be missing.
    assert_eq!(
        check(&format!("{tree}tree.genre")),
        Ok(TypeRef::optional(TypeRef::DATA))
    );
    // One `Option`, however deep the path goes.
    assert_eq!(
        check(&format!("{tree}tree.note.author")),
        Ok(TypeRef::optional(TypeRef::DATA))
    );
    assert_eq!(
        eval(&format!("{tree}tree.genre")).unwrap().to_string(),
        "\"novel\""
    );
    assert_eq!(
        eval(&format!("{tree}tree.isbn")).unwrap().to_string(),
        "none"
    );
    assert_eq!(
        eval(&format!("{tree}tree.note.author"))
            .unwrap()
            .to_string(),
        "none"
    );
    // Equality looks through the `Option`: absence equals nothing present.
    assert_eq!(
        eval(&format!("{tree}tree.genre == \"novel\""))
            .unwrap()
            .to_string(),
        "true"
    );
    assert_eq!(
        eval(&format!("{tree}tree.isbn == \"novel\""))
            .unwrap()
            .to_string(),
        "false"
    );
}

#[test]
fn a_tree_is_read_through_its_map_alternative_and_a_leaf_is_no_map() {
    let tree = "tree = {genre: \"novel\", pages: 412}\n";
    assert_eq!(
        check(&format!("{tree}get(tree, \"genre\")")),
        Ok(TypeRef::optional(TypeRef::DATA))
    );
    assert_eq!(
        eval(&format!("{tree}get(tree, \"genre\")"))
            .unwrap()
            .to_string(),
        "\"novel\""
    );
    assert_eq!(
        eval(&format!("{tree}get(tree, \"isbn\")"))
            .unwrap()
            .to_string(),
        "none"
    );
    // A map key of a tree is a String.
    assert!(check(&format!("{tree}get(tree, 3)")).is_err());
    // A leaf viewed as a map is a failed mapping, reported as a type error
    // rather than passed off as absence.
    for source in [
        format!("{tree}tree.pages.unit"),
        format!("{tree}get(tree.pages, \"unit\")"),
    ] {
        let refusal = eval(&source).expect_err(&source);
        assert_eq!(refusal.stage(), "type", "{source}");
        assert!(
            refusal.message().contains("not Data of shape int"),
            "{refusal:?}"
        );
    }
}

#[test]
fn an_applied_parent_receives_transformed_arguments() {
    // `Ranking<T> : Seq<Hit<T>>`: the parent's argument is built from the
    // parameter rather than forwarded, and subtyping follows it.
    let card = TypeRef::CARD;
    let ranking = TypeRef::ranking(card.clone());
    assert!(ranking.is(&TypeRef::seq(TypeRef::hit(card.clone()))));
    assert!(ranking.is(&TypeRef::collection(TypeRef::hit(card.clone()))));
    assert!(ranking.is(&TypeRef::seq(TypeRef::hit(TypeRef::DOC))));
    assert!(!ranking.is(&TypeRef::seq(card.clone())));
    assert!(!TypeRef::seq(TypeRef::hit(card.clone())).is(&ranking));
    assert_eq!(ranking.element(), Some(TypeRef::hit(card.clone())));
    assert!(ranking.is_sequence());
    // Narrowing the elements cannot keep the ranking: its argument is the
    // ranked value, not the hit.
    assert_eq!(
        ranking.with_element(TypeRef::hit(card.clone())),
        TypeRef::list(TypeRef::hit(card.clone()))
    );
    assert_eq!(
        TypeRef::set(TypeRef::INT).with_element(TypeRef::STR),
        TypeRef::set(TypeRef::STR)
    );

    let mut system = collections::type_system().expect("the collection hierarchy");
    let wrong_arity = TypeDefinition {
        constructor: TypeConstructor("Pairs"),
        kind: TypeKind::Concrete,
        parameters: vec![],
        parents: vec![],
        applied: vec![collections::SEQ.apply([TypeRef::INT, TypeRef::INT])],
    };
    assert!(matches!(
        system.declare(wrong_arity),
        Err(TypeError::IncompatibleParent { .. })
    ));
}

#[test]
fn a_tree_leaf_is_read_by_its_shape_as_the_scalar_it_holds() {
    use hql::types::TypeRef;
    for (source, expected) in [
        ("tree = {pages: 412}\ntree.pages | is(Int)", "true"),
        ("tree = {pages: 412}\ntree.pages | is(String)", "false"),
        (
            "tree = {pages: 412, title: \"Dune\"}\n[tree.pages, tree.title] | somes | of_type(Int)",
            "[412]",
        ),
        (
            "tree = {pages: 412}\nmatch tree.pages { some(Int n) => n + 1, _ => 0 }",
            "413",
        ),
        (
            "tree = {title: \"Dune\"}\nmatch tree.title { some(Int n) => n, _ => 0 }",
            "0",
        ),
    ] {
        assert_eq!(hql::eval(source).unwrap().to_string(), expected, "{source}");
    }
    let read = "tree = {pages: 412}\n[tree.pages] | somes | of_type(Int)";
    assert_eq!(hql::check(read), Ok(TypeRef::list(TypeRef::INT)));
    assert_eq!(
        hql::eval(read).unwrap().type_of(),
        TypeRef::list(TypeRef::INT)
    );
}

#[test]
fn a_type_arm_covers_its_subject_and_refuses_a_type_that_cannot_fit() {
    assert_eq!(
        hql::eval("match 3 { Int n => n * 2 }").unwrap().to_string(),
        "6"
    );
    let never = hql::check("match 3 { String s => 1, _ => 0 }").unwrap_err();
    assert!(never.to_string().contains("never fits"), "{never}");
}

#[test]
fn a_ranking_reports_the_element_type_it_was_built_from() {
    use hql::search::{Retrieval, rank};
    use hql::types::{TypeRef, Value};
    let retrieval = Retrieval {
        query: "q".to_owned(),
        index: "i".to_owned(),
        model: "m".to_owned(),
        revision: "r".to_owned(),
        metric: "cosine".to_owned(),
        approximate: false,
    };
    let ranking = rank(
        vec![
            (Value::Int(7), 0.5),
            (Value::Int(3), 0.9),
            (Value::Int(1), 0.0),
        ],
        TypeRef::INT,
        retrieval,
    );
    assert_eq!(ranking.hits.len(), 2, "a zero score is no answer");
    assert_eq!(ranking.hits[0].value, Value::Int(3));
    let value = Value::Ranking(std::sync::Arc::new(ranking));
    assert_eq!(value.type_of(), TypeRef::ranking(TypeRef::INT));
    let first = value.elements().unwrap().remove(0);
    assert_eq!(first.type_of(), TypeRef::hit(TypeRef::INT));
}
