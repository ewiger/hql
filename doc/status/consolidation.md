# Consolidation TODO

Opened 2026-09-17 on branch `feat/lang-design`.

This file tracks **consolidation work**: renames and removals that a settled
decision implies but that have not yet been carried through every file. It is a
work index, not a decision record. A decision belongs in `doc/proposals/` or
`doc/memory/`; a contradiction still awaiting one belongs in
[inconsistencies.md](inconsistencies.md) as an `INC-NN` finding.

Items carry stable `CON-NN` identifiers so a commit can point at one without
restating it. An item is removed when the work is finished everywhere, not when
the decision behind it is recorded, and its identifier is not reused.

## Index

| ID | Item | State |
| --- | --- | --- |
| CON-01 | Drop `HmdCard`; the type is `Card` | done, residue |
| CON-03 | `Hmd` is a format, not the parsed body type | done, residue |
| CON-05 | `cards` ordering: `List<Card>` or `Set<Card>` | open, needs decision |
| CON-06 | Title precedence: derived heading or authored entry | open, needs decision |
| CON-07 | Precedence between derived, authored and contributed header layers | open, needs decision |
| CON-08 | Whether a card requires a vault | open, needs decision |
| CON-09 | Knowledge is day-one design, not future work | done |
| CON-10 | Links to `first-milestone`, which no longer exists | done |
| CON-11 | `typed Relation` over cards becomes `typed RelationCard` | open, corpus residue |
| CON-12 | `Card` splits into `ConceptCard` and `RelationCard` | done, corpus residue |

## CON-01 — Drop HmdCard

**Decided.** The type is `Card`, with no alias and no `Hmd` prefix. The prefix
named a condition the type no longer has: card-ness is a knowledge property, not
a file format, so nothing about a card follows from its file being `.hmd`. See
[extension boundary](../memory/extension-boundary.md).

Done in [Card](../wiki/hql/types/card-type.hmd),
[design status](../wiki/hql/design-status.hmd),
[HMD integration](../wiki/hmd-integration.hmd) and the extension-boundary memory.
`INC-03` is struck as resolved.

Residue: [inconsistencies.md](inconsistencies.md) still contains the name where a
finding quotes text that used it. Those are quotations of a former state and are
deliberately left, since rewriting them would falsify the audit record.

## CON-03 — Hmd is a format, not a type

**Decided.** `Hmd` names the dialect that `header.format` reports. The parsed
body value is `HmdContent`, and `Content` is a document body's type before its
dialect is established; `HmdContent <: Content` is the usual narrowing.

Done in [Doc](../wiki/hql/types/doc-type.hmd).

Residue, in the corpus and its index:

| Where | What it says |
| --- | --- |
| [cards/body.hql](../../examples/cards/body.hql) | `expected-type: Hmd` |
| [cards/field-sequence.hql](../../examples/cards/field-sequence.hql) | `expected-type: Hmd` |
| [examples/README.md](../../examples/README.md) | "type: Hmd" in the case table |

`Content` and `HmdContent` are the names this consolidation used. They were not
chosen by a decision record, and confirming or replacing them is part of closing
this item.

## CON-05 — cards ordering

Needs a decision, and it is about whether vault order is observable rather than
about a spelling. [Card](../wiki/hql/types/card-type.hmd) says `List<Card>`;
[HQL collections](../wiki/hql/collections.hmd) says `Set<Card>`;
[fixture environments](../../examples/fixtures/README.md) assumes root-relative
path order. Tracked as `INC-04`.

## CON-06 — Title precedence

Needs a decision. A document can state a title in its header and open with a
different first heading. Which wins, and whether the loser stays reachable, is
recorded as open in [Doc](../wiki/hql/types/doc-type.hmd). `name` and `path`
have no equivalent problem, because nothing authored can contradict them.

## CON-07 — Header layer precedence

Needs a decision, and not a single rule. A header is assembled from derived,
authored and contributed entries. An author overriding a derived `title` is
reasonable; an author overriding a derived `path` is not. Whether a contributed
entry may overwrite an authored one is a third question, and an extension that
contributes only type knowledge raises none of them. Recorded as open in
[Doc](../wiki/hql/types/doc-type.hmd) and [Card](../wiki/hql/types/card-type.hmd).

## CON-08 — Does a card require a vault

Needs a decision, raised by the extension-boundary memory. Card-ness no longer
follows from the file format, but [Card](../wiki/hql/types/card-type.hmd) still
narrows on two conditions, the second being a place in a vault. Naming and
`downlinks` do need a namespace; whether a document outside one can nonetheless
represent a piece of knowledge is unresolved.

## CON-09 — Knowledge is day-one design

**Done 2026-09-18.** Both files now separate the settled design from the deferred
implementation. [Knowledge model](../models/domain/knowledge.md) opens "preferred
conceptual model, designed from day one rather than deferred… the *implementation*
is a bootstrap evaluator", and [architecture](../models/domain/architecture.md)
says the declarations "are settled design, not deferred work; what is deferred is
the *implementation*". [Graphs](../models/domain/graphs.md) was reworded to
match. Closes the second half of `INC-07`.

## CON-10 — Links to a deleted card

**Done 2026-09-18.** All three `[[first-milestone]]` links are repointed at
[bootstrap requirements](../models/requirements/bootstrap.md), which is what the
card described. `INC-12`'s four other broken links are untouched and still open.

## CON-11 — typed Relation becomes typed RelationCard

`cards | typed Relation | graph` selects nothing, because a card is never a
`Relation`. The idiom is `cards | typed RelationCard | graph` — see `INC-22`.

Done throughout `doc/`. Residue: the example corpus is not present in this
working tree, and [corpus direction](../memory/corpus-direction.md) records that
its rows still carry the old spelling. Closing this item means sweeping
`examples/` when it is back.

## CON-12 — Card splits into ConceptCard and RelationCard

**Decided.** `Card <: Doc` is the base representation type;
`ConceptCard <: {Card, Concept}` is the node-occupying default and
`RelationCard <: Card` is the exception, declared by
`metadata.knowledge.type == Relation`. `Card <: Concept` is withdrawn. See
[card family](../memory/card-family.md).

Done in the knowledge and graph models and in the Card, Doc, Graph, type-system,
knowledge, pipes, link-op, design-status and design-direction cards.

Residue: the example corpus, same reason as `CON-11`. Also unswept are
`examples/std-lib/`, where `HmdCard`, `MdCard` and `DataCard` implement `Card` —
that direction is unaffected, since they are representations of the base type,
but the cases should say which kind they produce.
