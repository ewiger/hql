# Traits, impls and the std-lib corpus

Recorded 2026-09-17 from a user instruction to populate `examples/std-lib/` with
the advanced part of the language: traits and implementations as the replacement
for interfaces, `Card` implemented by `HmdCard`, `DataCard` and `MdCard`, type
refinement, `Option`, and functions — with no pipeline syntax yet.

- Traits are the chosen mechanism for a required surface. A trait is not a
  supertype and no value has the type `Card`; implementors provide the surface.
- `Card` is implemented by three unrelated representations. `DataCard` is the
  most general (a JSON or YAML document: a header with no prose body and no
  links), `MdCard` adds body and sections with empty uplinks, `HmdCard` adds
  references. The format condition lives in an implementation, which is what
  [card-ness as a knowledge notion](extension-boundary.md) requires.
- `HmdCard` returns as the name of a concrete implementation. That is a different
  role from the `HmdCard` alias for the `Card` type dropped as CON-01, and the
  earlier resolution is not reopened by it.
- Typing is strict, not duck typing. A record declares its shape and an
  incomplete literal fails at construction; a `Data` tree earns a stronger type
  by established structure, and the access is checked only where the evidence
  holds. Refinement never converts the value.
- Absence is `Option`, declared as an ordinary sum type. There is no null, and
  exhaustive `match` is what replaces the null check.
- Two statements in the wiki cards now conflict with this direction: [core](../wiki/hql/core.hmd)
  and [design status](../wiki/hql/design-status.hmd) keep "Rust-like trait
  machinery" out of the design, and [Card](../wiki/hql/types/card-type.hmd)
  declares `Card <: Doc` as a narrowing. Both readings are held as corpus
  alternatives (`std-lib/trait-card.hql`, `std-lib/card-narrowing-alternative.hql`)
  and the wiki cards are deliberately left unedited until the choice is recorded.
- std-lib cases use angle-bracket generics, following the wiki cards rather than
  the square brackets of the older corpus. The divergence stays unresolved.
- `examples/fixtures/cards/notes/plain.md` is a new fixture: the one plain
  Markdown document, so a Markdown card can be shown without an HMD body.
- `examples/cards/frontmatter.hql` had been deleted in the working tree while its
  index row remained, which broke the corpus count assertion. Restored from HEAD.

See [std-lib walkthrough](../../examples/std-lib/README.md) and
[corpus direction](corpus-direction.md).
