# 11. Advanced data typing

[Chapter 2](../02-data-types/data.md) read JSON-shaped trees as `Data`, and
[chapter 10](../10-the-type-system/index.md) declared records of the
program's own. This chapter is about the step between them: how a tree that
arrived from outside becomes a value the program has a type for, and how a
program states what it requires of that value beyond its shape. The
[structural examples](https://github.com/ewiger/hql/tree/main/examples/structural)
have one program per case.

## Decide at the boundary

Data from outside a program has no type the checker can see. A JSON file, a
message, a document's frontmatter or an API response decodes to `Data`, a
tree whose leaves could hold anything. A program may carry the tree as it is
and test each leaf where it is used, as the `match` arms of chapter 2 do.
That works, and it spreads one decision over every place the data is read.

HQL's answer is a **type boundary**: one step, written `tree | T`, that reads
the tree as a declared type `T` and either succeeds, with a value the checker
knows every field of, or does not. After the boundary the program is ordinary
typed code. Before it, the data is `Data`, and nothing is assumed about it.

```hql
type Point = {
    x: Int
    y: Int
}

d : Data = {x: 1, y: 2}
p = d | Point
p.x + p.y
```

```text
3
```

The rule the chapter rests on is this: a type is inferred while the
information to infer it is present, and where structure is dynamic or
absent, the program writes an explicit boundary rather than letting anything
be guessed. The boundary is where a decision about the data is made, once,
instead of being postponed to every reader of the value.

## Four kinds of structural type

A record type declared with fields reads a tree in one of four ways,
depending on two independent choices. The first is whether the type is
**closed**, admitting exactly its declared fields, or **open**, requiring its
declared fields and keeping whatever else the tree holds. The second is
whether it is a plain type, which states a shape, or a **contract**, which
states a shape and rules the values must hold.

| | Plain | Contract |
| --- | --- | --- |
| Closed | `type A = {…}` | `type C : TypeContract = {…}` |
| Open | `type B : Data = {…}` | `type D : DataContract = {…}` |

Whether a field is optional, `email?: String`, is a third choice, made per
field, and has no bearing on the other two: a closed type with optional
fields is still closed.

## Typing is not mapping

Reading a tree as a type changes what the program knows about the value. It
does not change the value. An open view over a tree with twelve fields still
holds twelve fields beneath the three it declares, and the same tree read as
another view sees the others. Changing the value, renaming a field, trimming
a string or dropping what a consumer does not need, is a `map`, written
where the change is paid for. The two are kept apart on purpose: a reader of
`raw | UserData` knows nothing was altered, and a reader of `map(…)` knows
nothing was checked.

## Contracts are more than schemas

A schema says what a value looks like: field names, types, optionality,
nesting. A contract says that and what the value must mean: that an `id` is
not empty, that an account that is closed says when, that an `age` lies in a
range, that an `email` is one. The rules are written in the type, beside the
fields they constrain, and hold wherever a value of the type is made. A
schema can be exported from a contract; the rules are what the export loses.

## The pages

| Page | What it covers |
| --- | --- |
| [Open and closed types](open-and-closed.md) | `type A = {…}` against `type B : Data = {…}`, what each reads, what each refuses, and optional fields |
| [Type contracts](type-contracts.md) | `TypeContract`: a closed type with rules, for the program's canonical values |
| [Data contracts](data-contracts.md) | `DataContract`: an open contract for data from outside, defaults, presence and implication |
| [Validators](validators.md) | a check with a name, `validate f with Email`, and the standard validators |
| [Validation as a value](validation-as-a-value.md) | `validate(T)`, `Result<T, Invalid>`, every violation at once, and routing the rejected |
| [Contracts and schemas](contracts-and-schemas.md) | what a schema states, what a contract adds, and where each lives |
