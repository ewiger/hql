# Consolidation TODO

Opened 2026-09-17 on branch `feat/lang-design`; re-verified 2026-09-18 against
commit `ac9e4c5` on `feat/semantic-search`.

This file tracks **consolidation work**: renames and removals that a settled
decision implies but that have not yet been carried through every file. It is a
work index, not a decision record. A decision belongs in `doc/proposals/` or a
`doc/models/` document; a contradiction still awaiting one belongs in
[inconsistencies.md](inconsistencies.md) as an `INC-NN` finding.

Items carry stable `CON-NN` identifiers so a commit can point at one without
restating it. **A finished item is deleted**, not kept as a record that it was
done; git history is that record. Identifiers are not reused.

## Index

| ID | Item | State |
| --- | --- | --- |
| CON-14 | `HmdCard` survives in the type-system card | residue |
| CON-06 | Title precedence: derived heading or authored entry | needs a decision |
| CON-07 | Precedence between derived, authored and contributed header layers | half decided |
| CON-08 | Whether a card requires a vault | needs a decision |

`CON-01`, `CON-03`, `CON-05`, `CON-11`, `CON-12` and `CON-13` were closed with
`examples/` as their only outstanding residue. The corpus restored in `8aa265e`
was checked for each — `HmdCard`, `Hmd` as a body type, `List<Card>` for `cards`,
`typed Relation`, square-bracket generics — and carries none of them, so the six
stay deleted. `CON-14` is what that check found outside the corpus.

**`CON-06`, `CON-07` and `CON-08` are misfiled.** A consolidation item is work
implied by a decision already made; those three are decisions not yet made, which
is what `doc/proposals/` is for. Promoting them to proposals, or to issue cards,
would leave this file holding only real residue.

## CON-14 — `HmdCard` in the type-system card

The decision `CON-01` carried was that there is no `HmdCard`: the type is `Card`,
and [std/knowledge.hql](../../std/knowledge.hql) declares `type Card : Doc` and
nothing beneath it for the format. [type-system.hmd](../wiki/hql/type-system.hmd)
still uses `type HmdCard : Card` three times as its worked example of subtyping,
under **Subtyping**. `CON-01` was closed as carried through `doc/**`; this site
was missed. Any real pair from `std/` — `ConceptCard : Card`, `Card : Doc` —
makes the same point.

## CON-06 — Title precedence

Needs a decision. A document can state a title in its header and open with a
different first heading. Which wins, and whether the loser stays reachable, is
recorded as open in [Doc](../wiki/hql/types/doc-type.hmd). `name` and `path`
have no equivalent problem, because nothing authored can contradict them.

## CON-07 — Header layer precedence

**Half decided 2026-09-18** by the implementation. For a *header*, derived paths
— `name`, `path`, `format` — win over anything authored, because nothing
authored can contradict where a document is. For *metadata*, the question is
sidestepped rather than answered: an extension contributes under a key named
after itself (`metadata.lexical.*`), so an authored entry and a contributed one
cannot collide. Both halves are in the code rather than in a document:
`src/document.rs` writes the derived paths after parsing the authored header, and
`src/vault.rs` inserts the contributed key.

What remains open is an authored entry contradicting a derived one on a path
neither obviously owns, such as `title` — which is `CON-06`. Whether a
contributed entry may overwrite an authored one is a third question, and an
extension that contributes only type knowledge raises none of them. Recorded as
open in [Doc](../wiki/hql/types/doc-type.hmd) and
[Card](../wiki/hql/types/card-type.hmd). Which tree a contributed entry lands
in — the header or `card.metadata` — is itself contradicted across the cards; see
`INC-30`.

## CON-08 — Does a card require a vault

Needs a decision. Card-ness no longer follows from the file format, but
[Card](../wiki/hql/types/card-type.hmd) still narrows on two conditions, the
second being a place in a vault. Naming and `downlinks` do need a namespace;
whether a document outside one can nonetheless represent a piece of knowledge is
unresolved. [The command line](../models/behavior/cli.md) answers for the host
only: without `--vault`, `cards`, `downlinks` and `expand` say they need one.
