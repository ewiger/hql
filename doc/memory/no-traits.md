# HQL has no traits; a trait is a Rust mechanism

Recorded 2026-09-18 from a user correction, replacing the earlier
`trait-direction` note, which had chosen traits as HQL's mechanism for a
required surface and flagged that it contradicted the wiki cards.

- **The contradiction is now decided against traits.** [core](../wiki/hql/core.hmd),
  [design status](../wiki/hql/design-status.hmd), [pipes](../wiki/hql/types/pipes.hmd)
  and [extensions](../wiki/hql/extensions.hmd) all keep Rust-like trait
  machinery out of the design, and they were right. The earlier note held both
  readings "until the choice is recorded"; this is that record.
- A trait is a **host-language** mechanism. An extension is written in Rust and
  may use traits internally, as any Rust program does. None of that reaches
  HQL's surface — the boundary [extensions](../wiki/hql/extensions.hmd) already
  draws, and the general rule in [stack scope](stack-scope.md): "Rust does it
  this way" is never an argument for an HQL feature.

## What replaces a trait, depending on what was being said

Three different things had been collapsed into one mechanism, and separating
them is why none of them needs traits.

- **Ancestry** — the value genuinely *is* the thing. Ordinary `<:`.
  `Card <: Doc`, `ConceptCard <: {Card, Concept}`, and `Card <: Orderable`,
  because a card carries an ordering key of its own: its name. `Orderable` is a
  supertype, not a bound and not a capability, which is exactly how the user
  phrased it when it was decided — see [orderable cards](orderable-cards.md).
- **Capability** — something can be *done with* the value, by someone else. An
  [extension](../wiki/hql/extensions.hmd) registers `render(Graph) -> Html`.
  [Type system](../wiki/hql/type-system.hmd) already states this as "capability
  is not ancestry"; dropping traits removes the competing answer, so what
  remains is whether a capability is a registration or a type, and it is a
  registration.
- **A shared shape** — several types happen to have the same fields. That is a
  [partial schema](../wiki/hql/fields.hmd) over an open `Data` tree, established
  by refinement where the evidence holds, not a contract anyone implements.

## What survives from the earlier note

- **Typing is strict, not duck typing.** A record declares its shape and an
  incomplete literal fails at construction; a `Data` tree earns a stronger type
  by established structure, checked only where the evidence holds. Refinement
  never converts the value. This is unchanged, and it is *why* no trait is
  needed: the checker asks about structure it can see.
- **Absence is `Option`, an ordinary sum type**, with no null and exhaustive
  `match` in place of a null check. A sum type is not a trait, so this was never
  part of the contradiction. It is still unimplemented — see
  [absence and warnings](absence-and-warnings.md) for the lifting used instead.

## What is withdrawn

- `Card` implemented by `DataCard`, `MdCard` and `HmdCard` as a trait with three
  implementors. The format axis is not a type axis at all:
  [extension boundary](extension-boundary.md) removed the format condition from
  `Card`, `header.format` reports the dialect (`CON-03`), and the kinds a card
  has are `ConceptCard` and `RelationCard` — see [card family](card-family.md).
- The bracket divergence the earlier note left open. Generics are written `[]`;
  see [the CLI host](cli-host.md).
- The corpus alternatives `std-lib/trait-card.hql` and
  `std-lib/card-narrowing-alternative.hql` no longer stand as live alternatives.
  The corpus is not in this working tree, so sweeping them is residue alongside
  `CON-11`.
