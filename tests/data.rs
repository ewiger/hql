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
    // Leaves alone still need a type in common: nothing asked for a tree.
    assert!(check("[1, \"owl\"]").is_err());
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
