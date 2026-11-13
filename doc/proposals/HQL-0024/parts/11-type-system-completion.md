# Part 11: Type system completion

- **Source**: the [type system card](../../../wiki/hql/type-system.hmd) and
  the goals of the first HQL-0024 stub; the draft does not cover this part
- **Depends on**: [traits](05-traits.md) for two items below
- **Needed by**: nothing else in this proposal

## Scope

The constructs the type system card leaves undecided or designed-but-unbuilt.
They were the whole of HQL-0024 before the draft widened it, and they remain in
scope: a module and receiver model is specified against a type system that
closes.

## Items

Each item names its card status and what, if anything, the draft changes.

| Item | Card status | Draft |
| --- | --- | --- |
| Is narrowing nominal or structural? | undecided; `type A : B` is declared, a `Data` region earns a type by established structure | silent |
| The call-site spelling of refinement and validation | undecided ([type interpretation](../../../wiki/hql/type-interpretation.hmd) has `is`, `of_type`, the asserting binding) | silent |
| Diagnosis of conflicting parents in a supertype set | designed: `{Int, String}` refused, `{Scalar, Orderable}` admitted; the message is not fixed | silent |
| The shape of a parameterized capability registration, `Renderable<T>` | decided: an implementation of `Encoder<T>` for the format, a generic trait ([HQL-0027](../../HQL-0027/README.md)) | a parameterized trait with an implementation per type, if traits cover capabilities ([part 5](05-traits.md)) |
| How much refinement survives an extension, a cell or a host boundary | undecided | silent; the host boundary ([part 10](10-host-boundary.md)) fixes where the boundary is, not what crosses it |
| May a subtype change a field's optionality? | undecided ([fields](../../../wiki/hql/fields.hmd)) | silent |
| Constructor parameters `F<_>` in the checker | designed, unbuilt; `filter` keeps its container, `map` and `sort` return a `List` ([issue 0018](../../../issues/archive/0018-operator-synonymy-audit.md)) | the receiver signatures of [part 4](04-receiver-modules.md) return the pattern's constructor, which this item has to reconcile |
| Record values and the conversions that need them | implemented: a record declares its shape, an incomplete literal fails at construction ([HQL-0018](../../HQL-0018/README.md)), and `is`, `of_type` and a type arm read a `Data` tree as a declared record by its shape ([HQL-0016](../../HQL-0016/README.md)) | silent |
| `Result<T, E>` and `match` arms over it | designed beside `Option`, not declared ([`lib/std/core/mod.hql`](../../../../lib/std/core/mod.hql), [match](../../../wiki/hql/operators/match-op.hmd)) | silent |
| Function types beyond four parameters | implemented up to four as `Fn0` to `Fn4` | silent |

## What stands

- `:` is the one relation symbol: inhabitation, subtyping and bounds. The
  draft's `T : Eq` keeps it.
- No field visibility ([fields](../../../wiki/hql/fields.hmd)). The draft's
  "visibility" is a different thing: which modules a source unit can see,
  controlled by the prelude and `import` ([part 2](02-imports.md)). The two
  do not meet, and the card's "no `pub`, `protected` or `private`" is unchanged
  by anything in the draft.
- Variance from immutability, angle brackets for generics, one-letter type
  parameters, `->` for function types, `fn`, `:=` and `where`.

## Work units

One per item in the table, in the order given; the two that depend on part 5
wait for its decision. Each lands with the checker change and the test that
pins it, and amends the type system card so that the card has no "designed,
not in the checker" sentence left for that item.
