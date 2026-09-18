# Inconsistencies across doc/ and examples/

Audited 2026-09-17 against the working tree of branch `feat/lang-design`.
Scope: every file under `doc/` and `examples/`, cross-checked against
`README.md`, `src/` and `tests/`.

Each finding carries a stable identifier of the form `INC-NN`. The identifier is
an address, not a priority: it is there so a commit, an issue card or a proposal
can point at a row without restating it. Identifiers are never reused — a
resolved finding leaves its row struck from the table rather than renumbered.

**Reading rule used.** The newest wiki cards under
[doc/wiki/hql/](../wiki/hql/) — in particular [Card](../wiki/hql/types/card-type.hmd),
[Doc](../wiki/hql/types/doc-type.hmd) and [Data](../wiki/hql/types/data-type.hmd) —
are treated as the current source of truth. The example corpus is a design *input*
and is explicitly allowed to preserve superseded alternatives, so a corpus file
disagreeing with a card is only a finding when the corpus does not label the
disagreement, or when a doc claims the corpus says something it does not.

## Status

**State** is `open` unless the repository already names the conflict as an
undecided question, which is `tracked`. **Kind** says what has to happen:
a `contradiction` needs a decision, `stale` and `broken` need only an edit.

| ID | Finding | Kind | State | Where |
| --- | --- | --- | --- | --- |
| INC-01 | A chat transcript is pasted into the Card card, reversing its own decisions | contradiction | open | [card-type.hmd](../wiki/hql/types/card-type.hmd) |
| INC-02 | `header` / `frontmatter` / `metadata` / `fm` name one concept four ways | contradiction | open | core, program-values, both corpus READMEs |
| ~~INC-03~~ | ~~The `Card` / `HmdCard` alias rests on one sentence in one card~~ | ambiguity | resolved | [card-type.hmd](../wiki/hql/types/card-type.hmd) |
| INC-04 | `cards : Set<Card>` versus `List[Card]` has a third site the tracked question omits | contradiction | tracked | [fixtures/README.md](../../examples/fixtures/README.md) |
| INC-05 | `Seq<T>` exists only in the wiki; `List` is the corpus's only sequence | stale | open | collections, corpus |
| INC-06 | `Option<List[Card]>` mixes both generic bracket styles in one type | stale | open | [graphs/path.hql](../../examples/graphs/path.hql) |
| INC-07 | Two files still say the evaluator supports only Int, Bool and addition | stale | open | [hql.hmd](../wiki/hql.hmd), [knowledge.md](../models/domain/knowledge.md) |
| INC-08 | Corpus case counts disagree three ways, and no test catches it | stale | open | [README.md](../../README.md), [examples/README.md](../../examples/README.md) |
| INC-09 | A standard library is both a bootstrap non-goal and a wiki card | ambiguity | open | [bootstrap.md](../models/requirements/bootstrap.md), [std-lib.hmd](../wiki/hql/std-lib.hmd) |
| INC-10 | `refinement-of-types.hmd` is zero bytes, untracked and unlinked | stub | open | [refinement-of-types.hmd](../wiki/hql/refinement-of-types.hmd) |
| INC-11 | The design-status card table omits two existing cards | stale | open | [design-status.hmd](../wiki/hql/design-status.hmd) |
| INC-12 | Four links point at renamed or nonexistent cards | broken | open | wiki README, hypermarkdown, issues README, kanban.yaml |
| INC-13 | The wiki breaks its own two rules about wikilink spelling and scope | contradiction | open | [hypermarkdown.hmd](../wiki/hypermarkdown.hmd) and the HQL cards |
| INC-14 | A directory is linked as though it were a card | stale | open | [hql.hmd](../wiki/hql.hmd) |
| INC-15 | Unresolved reference: a warning in the card, a hard error in the corpus | contradiction | open | [doc-type.hmd](../wiki/hql/types/doc-type.hmd), [unresolved-card.hql](../../examples/errors/unresolved-card.hql) |
| INC-16 | `validate` is used with two incompatible signatures, neither labelled | contradiction | open | `types/`, `design/`, `constraints/` |
| INC-17 | The Person fixture module contradicts the schema its own README states | contradiction | open | [people.hql](../../examples/fixtures/modules/people.hql), [fixtures/README.md](../../examples/fixtures/README.md) |
| INC-18 | Corpus expectations name a dozen types no vocabulary defines | contradiction | open | [core.hmd](../wiki/hql/core.hmd), corpus expectations |
| INC-19 | Prompt context leaked into a design card, documenting files that never existed | stale | open | [design-status.hmd](../wiki/hql/design-status.hmd) |
| INC-20 | The Kanban README's worked example does not match the real board | stale | open | [issues/README.hmd](../issues/README.hmd), [kanban.yaml](../issues/kanban.yaml) |
| INC-21 | Typos, missing frontmatter, and an empty proposals index | cosmetic | open | several |

## INC-01 — A chat transcript is pasted into the Card card

[Card](../wiki/hql/types/card-type.hmd) ends, after a horizontal rule following
"See [[doc-type]] … for what is still undecided", with roughly ninety lines of
raw dialogue beginning "Yes. That changes the model in an important way." It is
unsigned, addresses the reader as "your relation idea", and is not written in
the card register the wiki declares in [doc/wiki/README.md](../wiki/README.md).

It is not merely stylistic: the transcript **reverses the card's own decision**
three ways.

| The card's body says | The appended transcript says |
| --- | --- |
| `card.header : Data`, and `header` is "deliberately not `frontmatter`" | `card.frontmatter : Data?` is the authored YAML, and `card.metadata : Data` is the effective tree |
| The header "is not optional… no `Data?` to unwrap" | `Data?`, explicitly optional |
| `cards : List<HmdCard>` | `cards : List<Card>` |

So the same file now offers three competing names for one concept — `header`,
`metadata`, `frontmatter` — and the "not optional" argument is stated and then
silently withdrawn. Nothing else in the repository uses `card.metadata`.

**Resolution needed:** decide whether the layered `metadata` / `frontmatter`
split supersedes the single non-optional `header`, record that as a proposal or
a `doc/memory/` note, then rewrite the card in its own voice. The transcript
should not survive in either case — the `.hmd/**`-backed `metadata.state` idea
and the "refinement rather than inheritance" reading of `Relation <: Card` are
worth keeping, but as prose the card owns. Resolving this decides INC-02.

## INC-02 — header / frontmatter / metadata / fm

[Card](../wiki/hql/types/card-type.hmd) and [Data](../wiki/hql/types/data-type.hmd)
both settle on `card.header : Data` and reject a declared `Frontmatter` record.
Four other places still use the older vocabulary, and none of them flag it:

- [HQL core](../wiki/hql/core.hmd), value-vocabulary table: lists `Frontmatter`
  as a member of the Documents family, alongside `Card`, `Section` and `Link`.
  The same table omits `Doc` and `Data` entirely, although both now have cards.
- [Program values](../models/behavior/program-values.md): the worked program
  binds `fm = alice.frontmatter` and its prose names "title, frontmatter and body".
- [examples/README.md](../../examples/README.md), "Read these first": states the
  contract as "title → String, frontmatter → Frontmatter, body → Hmd".
- [Fixture environments](../../examples/fixtures/README.md), field conventions:
  "frontmatter is a Frontmatter value" and "`fm` is candidate shorthand".

The corpus cases follow the old spelling throughout — `.frontmatter` in
[cards/frontmatter.hql](../../examples/cards/frontmatter.hql) and the whole
`frontmatter/` group, `.fm` in [outputs/pure-update.hql](../../examples/outputs/pure-update.hql),
`_.fm` in [realistic/namespace.hql](../../examples/realistic/namespace.hql).
`.header` appears in no example. The corpus is entitled to lag, but the two
READMEs above describe a contract, not an alternative, so they contradict the
cards outright.

**Resolution needed:** state in [HQL design status](../wiki/hql/design-status.hmd)
that `header : Data` supersedes `Frontmatter`, fix the core vocabulary table,
and reword the two READMEs to present `frontmatter` as the corpus's older
spelling rather than as the type.

## ~~INC-03 — Card versus HmdCard~~

**Resolved 2026-09-17.** `HmdCard` is dropped; `Card` is the type, with no alias.
The `Hmd` prefix named a condition the type no longer has, card-ness having been
settled as a knowledge property rather than a file format. The name survives in
this file only where a finding quotes the former text.

The finding was: [Card](../wiki/hql/types/card-type.hmd) opened by declaring
`HmdCard` the type and `Card` a short spelling of it, then used `HmdCard` in its
own declarations while every other card, model and example used `Card`. That was
coherent but load-bearing on one sentence in one card.

## INC-04 — cards : Set<Card> versus List[Card]

[HQL design status](../wiki/hql/design-status.hmd), under decisions still needed,
records this as a two-way conflict between [Card](../wiki/hql/types/card-type.hmd)
(`List<HmdCard>`, ordered) and [HQL collections](../wiki/hql/collections.hmd)
(`Set<Card>`, unordered), and [Card](../wiki/hql/types/card-type.hmd) carries a
blockquote saying the same. Good — but there is a **third site** neither mentions:

[Fixture environments](../../examples/fixtures/README.md) states "`cards` is an
ordered collection of all fixture Cards in root-relative path order", and four
corpus cases depend on that order as their expected value —
[collections/all.hql](../../examples/collections/all.hql),
[collections/titles.hql](../../examples/collections/titles.hql),
[pipelines/inline.hql](../../examples/pipelines/inline.hql) and
[pipelines/multiline.hql](../../examples/pipelines/multiline.hql). Deciding for
`Set<Card>` therefore invalidates expectations, not just spellings. The tracked
question should say so.

## INC-05 — Seq exists only in the wiki

[HQL collections](../wiki/hql/collections.hmd) introduces `Seq<T>` and notes that
"no List alias or automatic conversion has been decided", yet `List` is the only
sequence type the corpus uses (about thirty cases) and
[design-direction](../wiki/design-direction.hmd) still lists `List[T]` under
"type vocabulary to leave room for". `Seq` appears nowhere outside the wiki.

## INC-06 — Mixed generic brackets in one type

Bracket style is unsettled by design, but one case mixes both spellings in a
single type: [graphs/path.hql](../../examples/graphs/path.hql) expects
`Option<List[Card]>`. Whatever is decided, that is a typo in either direction.

## INC-07 — Stale "Int, Bool and addition" after Float landed

`Float` is implemented — see [expression behavior](../models/behavior/expressions.md)
and [bootstrap decisions](../memory/bootstrap.md) — and most files say so. Two
do not:

- [HQL overview](../wiki/hql.hmd): "evaluates integer and Boolean literals and
  integer addition".
- [Knowledge model](../models/domain/knowledge.md), status line: "The working
  evaluator still supports only Int, Bool and addition."

## INC-08 — Corpus case counts disagree three ways

| Source | Claim |
| --- | --- |
| [README.md](../../README.md), example corpus | 108 design cases |
| [examples/README.md](../../examples/README.md), opening line | 125 small design cases |
| [examples/README.md](../../examples/README.md), index preamble | 47 design-question, 14 invalid, 58 proposed, 6 valid-now (= 125) |
| Working tree | **126** (47 design-question, 14 invalid, **59** proposed, 6 valid-now) |

The index table itself is correct and complete: 126 rows, 126 files, no orphan on
either side. Only the prose totals are stale, and `tests/corpus.rs` does not
catch them — it asserts row count equals case count, never the sentences. Adding
that assertion would keep this from recurring.

## INC-09 — The standard library is a card and a non-goal

[Bootstrap requirements](../models/requirements/bootstrap.md) lists "standard
library" among the explicit bootstrap non-goals, while
[doc/wiki/hql/std-lib.hmd](../wiki/hql/std-lib.hmd) exists as a card. The card is
three lines and is linked from nothing. Not a contradiction of substance — a
non-goal may still have a design card — but the card currently asserts a scope it
does not describe.

## INC-10 — An empty card

[doc/wiki/hql/refinement-of-types.hmd](../wiki/hql/refinement-of-types.hmd) is
**zero bytes**, untracked, and unlinked. Refinement is discussed across
[Card](../wiki/hql/types/card-type.hmd), [Data](../wiki/hql/types/data-type.hmd)
and the transcript in INC-01, so the topic is real and the placeholder is a
forward link — but an empty card breaks the wiki's "a card defines a single
thing" contract in [doc/wiki/README.md](../wiki/README.md).

## INC-11 — The design-status card table is incomplete

Neither `std-lib` (INC-09) nor `refinement-of-types` (INC-10) appears in the card
table in [HQL design status](../wiki/hql/design-status.hmd), which otherwise
claims to enumerate the HQL cards.

## INC-12 — Broken links

| Where | Link | Problem |
| --- | --- | --- |
| [doc/wiki/README.md](../wiki/README.md) | `hyper-markdown.hmd` | Renamed to `hypermarkdown.hmd` in this branch |
| [doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd) | `[[kanban]]`, three times | No `kanban.hmd` card exists anywhere under `doc/wiki/` |
| [doc/issues/README.hmd](../issues/README.hmd) | `[[hyper-markdown]]` | Same rename; see also INC-13 |
| [doc/issues/kanban.yaml](../issues/kanban.yaml) | comment "Convention: doc/wiki/kanban.hmd" | That file does not exist |

`[[kanban]]` may be a deliberate forward link — [doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd)
says forward links are fine — but it closes the card with "See [[kanban]]", which
reads as a pointer to something written, and `kanban.yaml` names the card as an
existing convention.

## INC-13 — The wiki breaks its own linking rules

[doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd) states two rules about the
wiki graph that the wiki itself breaks:

1. **"The slug names a `.hmd` file … by filename alone (subfolders included)."**
   Thirty links across the HQL cards are path-qualified instead — `[[hql/core]]`,
   `[[hql/knowledge]]`, `[[hql/design-status]]` and so on — while sibling links in
   the same files use bare slugs (`[[card-type]]`, `[[pipes]]`). Both forms would
   resolve under a by-filename rule; the mixture means the rule is being ignored
   rather than applied, and a reader cannot tell which spelling is canonical.
2. **"Anything outside `doc/wiki/` … uses ordinary relative Markdown links, never
   `[[ ]]`."** [doc/issues/README.hmd](../issues/README.hmd) is outside `doc/wiki/`
   and uses `[[hyper-markdown]]`.

**Resolution needed:** decide whether wiki subfolders take path-qualified slugs,
then apply it once; and either extend the `[[ ]]` space to `doc/issues/` or
convert that one link.

## INC-14 — A directory linked as a card

[HQL overview](../wiki/hql.hmd) links a *directory* as `[doc/wiki/hql/**](hql)`,
which is neither a wikilink to a card nor a link to a file. It sits between the
two rules in INC-13 rather than breaking either.

## INC-15 — Unresolved reference: error or warning?

[Doc](../wiki/hql/types/doc-type.hmd), under addressing, is emphatic: "HMD treats
an unresolved link as a warning, deliberately… HQL must not turn that into a hard
error by accident."

[errors/unresolved-card.hql](../../examples/errors/unresolved-card.hql) is a case
with status `invalid` expecting the diagnostic `UnresolvedCard` at the `resolve`
stage. Its note — "Fixture deliberately has no matching card" — presents this as
settled, and the case is not marked `design-question`.

These may both be right: resolution failing is not the same event as a body link
being unresolved during a link check. But nothing in either file says so, and
[HQL design status](../wiki/hql/design-status.hmd) lists "reference failures" as
open while the corpus row reads as decided. Say which of the two it is.

## INC-16 — validate has two signatures

Three cases use the name with incompatible shapes, and none cross-references the
others as alternatives:

- [types/schema-refinement.hql](../../examples/types/schema-refinement.hql),
  [design/custom-types-a.hql](../../examples/design/custom-types-a.hql) and
  [design/custom-types-b.hql](../../examples/design/custom-types-b.hql):
  `validate[Person](card)` — one argument, type applied.
- [constraints/predicate-alternative.hql](../../examples/constraints/predicate-alternative.hql):
  `validate(alice, adult)` — two arguments, predicate passed, returning
  `Validation[Person]`.

[Card](../wiki/hql/types/card-type.hmd) uses only the first form. The second is a
genuine alternative and is worth keeping, but it should be labelled as one — it
currently reads as the same function used inconsistently.

## INC-17 — Person: subtype or intersection

[Fixture environments](../../examples/fixtures/README.md) declares the fixture
schema as "Person <: Card with `name:String`, `age:Int`, and optional
`nickname:String`". The fixture module that the `modules-v1` profile points at,
[modules/people.hql](../../examples/fixtures/modules/people.hql), declares
`type Person = Card & { name: String, age: Int }` — the structural-intersection
form, and without `nickname`. The corpus keeps both readings deliberately as
[custom-types-a](../../examples/design/custom-types-a.hql) versus
[custom-types-b](../../examples/design/custom-types-b.hql), so the *fixture* silently
picking one while its README states the other is the inconsistency, not the
existence of two forms. The missing optional `nickname` matters because
[frontmatter/missing.hql](../../examples/frontmatter/missing.hql) expects
`Option[String]` / `None` on the strength of it.

## INC-18 — Types named in expectations that no vocabulary defines

Expected types in the corpus reference names absent from the vocabulary in
[HQL core](../wiki/hql/core.hmd) and from every type card: `Unknown`
(four `frontmatter/` cases), `Validation[T]`, `Satisfaction[T]`, `Visualization`,
`Diagram[D2]`, `Origin`, `Provenance`, `Edge`, `Role`, `Record` and `GraphLike`.
Most are openly speculative and labelled `design-question`, which is fine. Two are
not: [frontmatter/tags.hql](../../examples/frontmatter/tags.hql) is `proposed`,
and [knowledge/provenance.hql](../../examples/knowledge/provenance.hql) is
`proposed` while expecting `List[(Origin, Option[Float], Provenance)]` — a tuple
type that no card mentions at all. Either demote them or give the vocabulary a
home; [HQL core](../wiki/hql/core.hmd) says the list is "the current design names",
which the corpus contradicts by using a dozen more.

## INC-19 — Prompt context leaked into a design card

[HQL design status](../wiki/hql/design-status.hmd), under architecture sources,
opens: "The supplied context names `docs/language-design.md`,
`docs/knowledge-model.md` and `docs/decisions.md`. Those paths are absent from this
repository."

"The supplied context" is an artifact of how the card was produced, not a fact
about HQL, and the sentence documents three files that have never existed. The
useful half — the list of sources that *are* real — should stay; the first two
sentences should go.

## INC-20 — The Kanban example does not match the board

[doc/issues/README.hmd](../issues/README.hmd) shows `last_id: 1` with a sample
card in `todo`; the real [kanban.yaml](../issues/kanban.yaml) has `last_id: 0`
and empty columns. The README example is illustrative, but nothing says so, and
it names a file `0001-first-issue.md` that does not exist.

## INC-21 — Smaller items

- [HQL overview](../wiki/hql.hmd): "Markdown docuemnts" (typo); "closer to rust"
  (lowercase, beside capitalised Kusto, Scala, Haskell, Lean).
- [doc/wiki/hql/std-lib.hmd](../wiki/hql/std-lib.hmd) and
  [doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd) carry no YAML
  frontmatter, unlike every other card in the wiki.
- [doc/proposals/README.md](../proposals/README.md) has an index table with a
  header row and no rows, and no proposal has been written — consistent, but it
  means every "record this as a proposal" resolution above starts from empty.
- `doc/status/` (this folder) is not mentioned in the knowledge-base list in
  [CLAUDE.md](../../CLAUDE.md).

## Verified consistent

Worth recording, because these were checked and hold:

- The corpus index is complete in both directions — 126 rows, 126 files, no
  orphans — and `cargo test --locked` passes, 15 tests across 6 suites.
- The `uplinks` / `downlinks` rename is complete. No `backlinks` survives except
  in the two places that explain the rename, and the direction sense ("uplinks
  outgoing, downlinks incoming") is stated identically in
  [Doc](../wiki/hql/types/doc-type.hmd), [HQL knowledge](../wiki/hql/knowledge.hmd),
  [corpus direction](../memory/corpus-direction.md),
  [fixtures](../../examples/fixtures/README.md) and both realistic cases.
- The fixture link graph supports its expected values exactly. Alice's uplink is
  `bob`; her downlinks are `carol`, `family-record` and `research/graph-notes` —
  the two relation cards point at her from YAML, not from a body link, and are
  correctly excluded per the stated "authored body links only" scope.
- [expression behavior](../models/behavior/expressions.md) matches `src/`: the
  256-literal bound, `i64`/`f64`, no widening, and overflow-as-evaluation-error
  all hold against `src/parser.rs` and `src/evaluation.rs`.
- Every relative Markdown link in `doc/` and `examples/` resolves, apart from
  those in INC-12 and the deliberate placeholders in
  [doc/proposals/TEMPLATE.md](../proposals/TEMPLATE.md).
