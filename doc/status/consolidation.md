# Consolidation TODO

Opened 2026-09-17 on branch `feat/lang-design`; pruned 2026-09-18 on
`feat/semantic-search`.

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
| CON-06 | Title precedence: derived heading or authored entry | needs a decision |
| CON-07 | Precedence between derived, authored and contributed header layers | half decided |
| CON-08 | Whether a card requires a vault | needs a decision |

Closed by the corpus being absent: `CON-01`, `CON-03`, `CON-05`, `CON-11`,
`CON-12` and `CON-13` were each carried through `doc/**`, `src/` and `std/`, and
their only outstanding residue was `examples/`, which is in no branch. They are
deleted here and reopen only if the corpus returns — see `INC-25`.

**These three items are misfiled.** A consolidation item is work implied by a
decision already made; all three below are decisions not yet made, which is what
`doc/proposals/` is for. Promoting them to proposals, or to issue cards, would
empty this file.

## CON-06 — Title precedence

Needs a decision. A document can state a title in its header and open with a
different first heading. Which wins, and whether the loser stays reachable, is
recorded as open in [Doc](../wiki/hql/types/doc-type.hmd). `name` and `path`
have no equivalent problem, because nothing authored can contradict them.

## CON-07 — Header layer precedence

**Half decided 2026-09-18** by the implementation. For a *header*, derived paths
— `name`, `path`, `format` — win over anything authored, because nothing
authored can contradict where a document is. For *metadata*, the question is
sidestepped rather than answered: an extension contributes under a key it owns
(`metadata.search.*`, `metadata.git.*`), so an authored entry and a contributed
one cannot collide. See [the command line](../models/behavior/cli.md).

What remains open is an authored entry contradicting a derived one on a path
neither obviously owns, such as `title` — which is `CON-06`. Whether a
contributed entry may overwrite an authored one is a third question, and an
extension that contributes only type knowledge raises none of them. Recorded as
open in [Doc](../wiki/hql/types/doc-type.hmd) and
[Card](../wiki/hql/types/card-type.hmd).

## CON-08 — Does a card require a vault

Needs a decision. Card-ness no longer follows from the file format, but
[Card](../wiki/hql/types/card-type.hmd) still narrows on two conditions, the
second being a place in a vault. Naming and `downlinks` do need a namespace;
whether a document outside one can nonetheless represent a piece of knowledge is
unresolved.
