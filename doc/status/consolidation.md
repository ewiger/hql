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
| CON-09 | Knowledge is day-one design, not future work | open |
| CON-10 | Links to `first-milestone`, which no longer exists | open |

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

The type cards now treat knowledge as planned from the start rather than
deferred. Two files still carry the older framing:
[knowledge model](../models/domain/knowledge.md) opens "all related syntax and
runtime support are future work", and
[architecture](../models/domain/architecture.md) says its declarations "guide
future work".

Both statements are true about the *implementation*, which supports Int, Bool
and addition. The item is to make them say that, rather than reading as though
the knowledge design itself were postponed.

## CON-10 — Links to a deleted card

`first-milestone.hmd` was removed but is still linked from
[design status](../wiki/hql/design-status.hmd), [HQL](../wiki/hql.hmd) and
[design direction](../wiki/design-direction.hmd). Unresolved links are warnings
in HMD rather than errors, so this is a cleanup: repoint each link or restore the
card. Related to `INC-12`, which lists four other broken links.
