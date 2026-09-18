# 0006 — `Scalar`, and `Data` as a union

Status: done
Branch: `feat/scalar-data-union`

`Data` was declared as a bare name, `type Data`, with its shape described only
in the prose of [data-type](../wiki/hql/types/data-type.hmd). The binary agreed
with the declaration by saying as little: `Data` had no parent, no members, and
nothing was a `Data` except a tree literal. So `tree : Data = [1, 2]` was
refused, although a list of integers is exactly what a YAML or JSON sequence is.

HQL is to declare the leaves and the tree:

```hql
abstract type Scalar

type Bool   : Scalar
type Int    : {Scalar, Orderable}
type Float  : {Scalar, Orderable}
type String : {Scalar, Orderable}
type Date   : {Scalar, Orderable}

type Data = union {
    Scalar,
    Seq<Data>,
    Map<String, Data>
}
```

`Data` is where a union is genuinely needed. A supertype set says a type is
every parent at once; a tree is *one* of a leaf, a sequence of trees, or a
string-keyed map of trees. That is the data model of YAML and JSON, which is
what makes `Data` the type for querying them.

## Scope

- **Several parents in the registry.** `TypeDefinition` holds `parents`, and
  ancestry is walked as a directed acyclic graph in which an ancestor reached
  twice is one ancestor. `Int : {Scalar, Orderable}` needs it, and the
  single-parent field could not say it.
- **`TypeKind::Union`.** A type narrows a union by narrowing one of its
  members, so `Int`, `List<List<Int>>` and `SortedMap<String, Float>` are
  `Data`, and `Set<Int>` and `Map<Int, Int>` are not. A union's members may
  apply the union itself; a bare self-member and an empty union are refused.
- **`Scalar` and `Date`** join the built-in types. `Date` is declared so a
  schema can name it, and no literal produces one.
- **`type Name = union { A, B }`** in the parser and the declaration checker:
  members exist, appear once, and are not the union itself; a built-in union is
  restated member for member. `union` is contextual and stays an ordinary name.
- **Supertype sets of contracts.** Only two concrete types can share nothing,
  so `{Scalar, Orderable}` holds and `{Int, String}` is still refused.
- `std/core.hql`, the [data-type](../wiki/hql/types/data-type.hmd) and
  [type-system](../wiki/hql/type-system.hmd) cards, the grammar in
  [expressions](../models/behavior/expressions.md), and a runnable
  [tree example](../../examples/birds/queries/tree.hql).
- **Not a change of carrier.** A tree literal is still `Value::Data`, and
  viewing a list or a map as `Data` converts nothing.
- **Not leaf refinement.** Going from `Data` to the member a value actually is —
  `header.age : Int` — stays with refinement, which is still design.
- **`[1, "owl"]` is still refused.** A list literal becomes a `List<Data>` only
  when an element already is one. Whether unrelated leaves should join to a tree
  on their own is a separate decision.

## Done when

- `cargo test` passes, and every file under `std/` still parses and checks.
- `hql check std/core.hql` prints `Unit`.
- Tests pin the membership of `Data` in both directions, the scalar parents,
  diamond ancestry, and the refusals of a malformed union.
- A program the checker accepts through a `Scalar` or `Data` view also
  evaluates.

## Validation

- `cargo test --offline --locked`: 148 tests pass.
- `cargo clippy --offline --locked --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `hql check` prints `Unit` for every file under `std/`.
- `hql run examples/birds/queries/tree.hql` prints a `List<Data>` of six trees.

Evaluating a list literal re-infers its element type from runtime values, which
cannot see a static view. This issue makes the tree case hold; the general
repair is [0007](0007-checked-element-type-at-runtime.md).
