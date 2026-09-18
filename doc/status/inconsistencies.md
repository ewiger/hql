# Inconsistencies across doc/

First audited 2026-09-17 on branch `feat/lang-design`; re-verified 2026-09-18
against commit `ac9e4c5` on `feat/semantic-search`. Files under `src/types/` were
being edited while the audit ran, so every claim about `src/` here was checked
against that commit rather than the working tree.

Each finding carries a stable identifier of the form `INC-NN`. The identifier is
an address, not a priority: it is there so a commit, an issue card or a proposal
can point at a row without restating it. Identifiers are never reused.

**Resolved findings are deleted, not struck through.** The current state is the
finding list; git history is the record of how it got here. A finding is removed
when the contradiction is gone, not when a decision about it is recorded
elsewhere.

**Reading rule used.** The wiki cards under [doc/wiki/hql/](../wiki/hql/) are the
current source of truth, with [doc/models/](../models/) as the longer argument
behind them.

## Status

**Kind** says what has to happen: a `contradiction` needs a decision, `stale`,
`broken` and `cosmetic` need only an edit.

| ID | Finding | Kind | Where |
| --- | --- | --- | --- |
| INC-26 | Seven links point at the semantic-search model, which was deleted | broken | root README, two models, both proposals, one card |
| INC-27 | Eight wikilinks and the card table point at the `orderable` card, which was deleted | broken | seven HQL cards |
| INC-29 | `Orderable` ancestry is stated three ways by the cards, the binary and `std/` | contradiction | [type-system.hmd](../wiki/hql/type-system.hmd), [std-lib.hmd](../wiki/hql/std-lib.hmd), `std/`, `src/types/builtin.rs` |
| INC-28 | `examples/` is described as a record of superseded alternatives; the restored corpus is a runnable notebook | contradiction | two cards, two models, [std/README.md](../../std/README.md) |
| INC-30 | Contributed metadata lands in the header in one card and in `card.metadata` everywhere else | contradiction | [doc-type.hmd](../wiki/hql/types/doc-type.hmd), [data-type.hmd](../wiki/hql/types/data-type.hmd) |
| INC-13 | The card's slug rule contradicts the HMD spec it cites, and 83 links rest on autodiscovery | contradiction | [hypermarkdown.hmd](../wiki/hypermarkdown.hmd) |
| INC-31 | The collection model is two `.md` files inside a wiki of `.hmd` cards | contradiction | [types/collections/](../wiki/hql/types/collections/) |
| INC-25 | One link still points into a corpus path that does not exist | broken | [knowledge.hmd](../wiki/hql/knowledge.hmd) |
| INC-12 | Three links point at renamed or nonexistent cards | broken | hypermarkdown, issues README, kanban.yaml |
| INC-32 | The generics model cites a closed consolidation item and says the corpus is absent | stale | [bind-vs-apply-in-generic-types.md](../models/behavior/bind-vs-apply-in-generic-types.md) |
| INC-33 | Issue 0004 is `backlog` in its file and `todo` on the board | stale | [0004](../issues/0004-collection-order-vocabulary.md), [kanban.yaml](../issues/kanban.yaml) |
| INC-20 | The Kanban README's worked example does not match the real board | stale | [issues/README.hmd](../issues/README.hmd) |

`INC-04`, `INC-06`, `INC-08`, `INC-16`, `INC-17` and `INC-18` were findings
against files of the earlier corpus — `examples/fixtures/`, `examples/graphs/`,
`examples/errors/` and its case counts. The corpus restored in `8aa265e` contains
none of those files and none of those contradictions, so the six are deleted.

## INC-26 — Links to the deleted semantic-search model

`doc/models/domain/semantic-search.md` was removed in `99fdc52`. Seven links still
point at it:

| Where | Count |
| --- | --- |
| [README.md](../../README.md), under **Project knowledge** | 1 |
| [graphs.md](../models/domain/graphs.md) | 1 |
| [knowledge.md](../models/domain/knowledge.md) | 1 |
| [HQL-0001](../proposals/HQL-0001/README.md) | 1 |
| [HQL-0002](../proposals/HQL-0002/README.md) | 2 |
| [semantic-search.hmd](../wiki/hql/semantic-search.hmd) | 1 |

`HQL-0002` is an accepted proposal that names the model as the argument it rests
on, so this is more than link hygiene: the longer argument behind the
[semantic search card](../wiki/hql/semantic-search.hmd) is no longer in the tree.

**Resolution needed:** restore the model, or repoint the seven links at the card
and accept that the card is now the whole argument.

## INC-27 — Links to the deleted `orderable` card

`doc/wiki/hql/types/orderable.hmd` was removed in the same commit. `[[orderable]]`
still appears eight times: twice each in
[collections.hmd](../wiki/hql/collections.hmd) and
[design-status.hmd](../wiki/hql/design-status.hmd), and once each in
[type-system.hmd](../wiki/hql/type-system.hmd),
[card-type.hmd](../wiki/hql/types/card-type.hmd),
[semantic-search.hmd](../wiki/hql/semantic-search.hmd) and
[lexical.hmd](../wiki/hql/extensions/lexical.hmd).

These are not forward links. The design-status card table lists `orderable` as an
existing card with a scope — "`Orderable`, the card's own key, and the `take`
contract" — and the semantic-search card sends the reader there "for the tie
rule", which is now stated nowhere. The **Orderable** section of the
[collection model](../wiki/hql/types/collections/collection-types.md) covers the
law type but not the card's own key, the `take` contract or the tie rule.

## INC-29 — `Orderable` ancestry, stated three ways

| Source | What it says |
| --- | --- |
| [type-system.hmd](../wiki/hql/type-system.hmd) | "`Card : Orderable` is an ordinary narrowing" |
| `src/types/builtin.rs` | `Doc : Orderable`, which `Card` inherits; `Hit : Orderable` as well |
| [std/doc.hql](../../std/doc.hql), [std/knowledge.hql](../../std/knowledge.hql), [std/retrieval.hql](../../std/retrieval.hql) | `type Doc { … }`, `type Card : Doc`, `type Hit<T> { … }` — no `Orderable` parent on any of the three |

The comment above `abstract type Orderable` in [std/core.hql](../../std/core.hql)
says a card is orderable because it has a name, yet no declaration in `std/`
makes it so. Every file in `std/` still checks, which contradicts
[std-lib.hmd](../wiki/hql/std-lib.hmd): "`std/` is not a second opinion about the
type lattice; it is the lattice written down … a file that drifts from the binary
now fails". Omitting a parent the binary holds is drift the check does not catch.

**Resolution needed:** decide whether the ordering key belongs to `Doc` or to
`Card`, write that parent in `std/`, and either make the agreement check cover a
missing parent or narrow the card's claim to what is checked.

## INC-28 — What `examples/` is

The corpus restored in `8aa265e` describes itself in
[examples/README.md](../../examples/README.md) as HyperMarkDown cards paired with
runnable HQL programs, each checked by `tests/corpus.rs` against a declared type,
result or diagnostic. Every query is marked `implementation: implemented`.

Five documents describe a different thing:

| Where | Claim |
| --- | --- |
| [std-lib.hmd](../wiki/hql/std-lib.hmd) | the corpus "records what was considered and may keep a withdrawn spelling beside a current one" |
| [std/README.md](../../std/README.md) | `examples/` holds "design inputs, explicitly allowed to keep superseded alternatives" |
| [design-direction.hmd](../wiki/design-direction.hmd) | the corpus "records current design inputs", followed by directions the birds notebook does not exercise |
| [program-values.md](../models/behavior/program-values.md) | the no-visible-result value is "called Unit in the corpus"; the corpus never names it |
| [architecture.md](../models/domain/architecture.md) | cites the corpus for a design whose implementation is deferred |

The links resolve, so nothing reports this. The std-lib argument depends on it:
the contrast between `std/` and the corpus is that only one of them may hold a
withdrawn spelling, and the corpus now holds none.

**Resolution needed:** decide whether `examples/` is a design-input corpus that
may run ahead of the implementation or a set of runnable examples that may not,
then bring the five descriptions into line.

## INC-30 — Where contributed metadata lives

[doc-type.hmd](../wiki/hql/types/doc-type.hmd), under **Three layers, one tree**,
assembles the *header* from derived, authored and contributed entries, and
[data-type.hmd](../wiki/hql/types/data-type.hmd) repeats it: one `Data` tree lets
derived entries "sit beside authored keys and extension-contributed ones".

[fields.hmd](../wiki/hql/fields.hmd) and
[card-type.hmd](../wiki/hql/types/card-type.hmd) say otherwise: `header.metadata`
is what the author wrote and nothing else, and contributions are assembled into
`card.metadata`, which fields.hmd calls "a different field, not a renaming". The binary agrees
with the second pair — `src/vault.rs` inserts `lexical.model` and
`lexical.indexed` into the card's metadata, not the header.

[options-in-types-and-fields.md](../models/behavior/options-in-types-and-fields.md)
noticed this under **What this does not settle** and said it belonged here; it
was never filed.

## INC-13 — The card's slug rule contradicts the HMD specification

[hypermarkdown.hmd](../wiki/hypermarkdown.hmd) states the rule for a `[[ ]]`
link as naming "a `.hmd` file anywhere under `doc/wiki/` — subfolders included —
by filename alone". The
[language specification](https://hypermarkdown.org/wiki/hmd-lang-spec/) the card
cites in its own opening line defines something different, and the difference is
not cosmetic.

Under **Grammar**, a page reference is absolute, relative or unqualified, and an
unqualified reference is either a *bare name* of one segment or an *unqualified
path* of two or more. The spec is explicit that the two "are collectively
unqualified references and use the same search phases". So `[[hql/knowledge]]`
is ordinary grammar rather than a departure from the rule, and the card's
"by filename alone" both describes one phase of four and forbids a spelling the
format provides.

**What the wiki actually does.** The spec's resolution algorithm, run over every
wikilink under `doc/wiki/` with this repository's effective configuration — no
`.hmd/config.toml` exists, so the root is `doc/wiki`, autodiscovery is on and its
mode is `both`, and no card declares an import:

| Outcome | Links |
| --- | --- |
| Resolved on the spine — pure path arithmetic from the card outward | 160 |
| Resolved only by the autodiscovery sweep | 83 |
| Unresolved, all of them `[[orderable]]` and `[[kanban]]` | 10 |
| Ambiguous, escaping the root, or malformed | 0 |

Nothing is ambiguous, and the 76 path-qualified links are not the defect. They
are load-bearing. Two filenames occur twice under `doc/wiki/`: `knowledge.hmd` in
`hql/` and in `hql/extensions/`, and `hypermarkdown.hmd` at the wiki root and in
`hql/extensions/`. A bare `[[knowledge]]` resolves by the spine walk to whichever
is nearer the card that writes it — the extensions card from inside
`hql/extensions/`, the HQL one from elsewhere under `hql/` — but written in
[hmd-integration.hmd](../wiki/hmd-integration.hmd), whose spine reaches neither,
it falls to the sweep and matches both, which is the `HMD002` error. That card
writes `[[hql/knowledge]]`, and the spec prescribes exactly that: "The remedy is
to qualify the link, not to memorise a tie-break."

**The real fragility is the 83.** Those resolve only in the spec's last phase,
which it describes as "a convenience … allowed to fail loudly" — `[[card-type]]`
written in `hql/`, for instance, whose spine never enters `hql/types/`. They hold
today only because autodiscovery defaults on. Adding a `.hmd/config.toml` that
sets `autodiscovery = false`, or a single card opting out with
`use: [no_autodiscovery]`, unresolves all 83 at once; the spec also warns that
adding a wildcard import may silently redirect a reference that had resolved this
way. Nothing in the repository records that dependency.

**The scope rule is sound.** The card's second rule — anything outside
`doc/wiki/` uses ordinary Markdown links, never `[[ ]]` — matches the spec's
namespace containment, which addresses nothing outside the namespace root and
reports a target that escapes it as `HMD003`.
[doc/issues/README.hmd](../issues/README.hmd) still breaks it with
`[[hyper-markdown]]`, which is also a dead slug; see `INC-12`.

**Resolution needed:** correct the card's rule to the spec's three reference
forms and its phase order, rather than applying a house spelling the format does
not have. Then decide whether the wiki depends on autodiscovery deliberately —
qualifying those 83 links, or pinning `autodiscovery = true` in a config file
with a comment saying what rests on it. The one genuine violation to fix is the
`[[ ]]` link in `doc/issues/`.

## INC-31 — The collection model is not a card

[collection-types.md](../wiki/hql/types/collections/collection-types.md) and
[collection.md](../wiki/hql/types/collections/collection.md) sit under
`doc/wiki/hql/types/`, but [the wiki README](../wiki/README.md) and the project
instructions both say the wiki holds `.hmd` cards, one concept per file. Being
`.md` puts them outside the graph by the format's own rule, not merely by
convention: the binding step of the
[language specification](https://hypermarkdown.org/wiki/hmd-lang-spec/) appends
`.hmd` to a reference and otherwise looks for a directory's `index.hmd`, so no
wikilink can name a `.md` file at all. Neither is therefore reachable:
[collections.hmd](../wiki/hql/collections.hmd) and
[type-system.hmd](../wiki/hql/type-system.hmd) reach the first by a relative
Markdown link, nothing under `doc/` links the second, and the
[design-status](../wiki/hql/design-status.hmd) card table lists neither. The cards
that link the first call it "the collection model", which is the vocabulary of
`doc/models/`. The second is not written as a card at all: it opens on a quoted
question — "So what laws does collection has?" — and answers in the first
person, which is a conversation pasted in rather than a statement of what is
true.

**Resolution needed:** make them `.hmd` cards and list them, or move them to
`doc/models/`. `std/collections.hql`, `std/core.hql` and
[examples/birds/README.md](../../examples/birds/README.md) name the current path.

## INC-25 — A link into a corpus path that does not exist

`examples/` is back, so seven of the eight links this finding recorded resolve.
One does not: [knowledge.hmd](../wiki/hql/knowledge.hmd) closes by pointing at
"presentation alternatives" in `examples/presentation/knowledge-views.hql`, and
the restored corpus has no `presentation/` folder.

## INC-12 — Broken links

| Where | Link | Problem |
| --- | --- | --- |
| [doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd) | `[[kanban]]`, twice in prose and once as the slug rule's own example | No `kanban.hmd` card exists anywhere under `doc/wiki/` |
| [doc/issues/README.hmd](../issues/README.hmd) | `[[hyper-markdown]]` | The card is `hypermarkdown`; see also INC-13 |
| [doc/issues/kanban.yaml](../issues/kanban.yaml) | comment "Convention: doc/wiki/kanban.hmd" | That file does not exist |

The `kanban` link may be a deliberate forward link — hypermarkdown says forward
links are fine — but the card closes with a "See" pointer to it, which reads as a
pointer to something written, and `kanban.yaml` names the card as an existing
convention.

## INC-32 — The generics model describes a superseded status

[bind-vs-apply-in-generic-types.md](../models/behavior/bind-vs-apply-in-generic-types.md)
says carrying the `<>` spelling through every file "is tracked as `CON-13`" and
that "the example corpus is not in this working tree and is the outstanding
residue". `CON-13` is closed, the corpus is in the tree, and it carries no
square-bracket generic. The same passage cites `INC-06` as quoting
`Option<List[Card]>`, and **What this does not decide** cites `INC-05`; both
findings are deleted, so the citations lead nowhere.

## INC-33 — Issue 0004 disagrees with the board

[doc/issues/README.hmd](../issues/README.hmd) says a card's column mirrors the
`Status:` line inside its issue file.
[0004](../issues/0004-collection-order-vocabulary.md) reads `Status: backlog`;
[kanban.yaml](../issues/kanban.yaml) holds it under `todo`. The issue file also
ends on a TODO asking for a complete rewrite, which suggests `backlog` is the
truthful one.

## INC-20 — The Kanban example does not match the board

[doc/issues/README.hmd](../issues/README.hmd) shows a sample board with
`last_id: 1` and one card in `todo` naming a file `0001-first-issue.md` that does
not exist; the real [kanban.yaml](../issues/kanban.yaml) has `last_id: 5` and
five cards, and `0001` is the extension mechanism. The README example is
illustrative, but nothing says so.

## Verified consistent

Worth recording, because these were checked and hold:

- The `:` subtype operator is carried through. No `<:` survives as syntax in
  `src/`, `std/`, `tests/`, `examples/` or the cards; it appears only where
  [type-system.hmd](../wiki/hql/type-system.hmd) and
  [issue 0005](../issues/0005-colon-subtype-operator.md) explain its absence, and
  in [the conversation record](../conversations/sets-n-subtypes.md), which says
  up front that it predates the change.
- The restored corpus agrees with the vocabulary. Every expectation type in
  `examples/birds/queries/` — `Data`, `Seq<String>`, `List<String>`,
  `List<List<String>>`, `Map<String, Int>` — is declared in `std/`, and
  [the birds README](../../examples/birds/README.md) infers `cards` as
  `Set<Card>`.
- A `Ranking<Card>` is accepted where a `Seq<Hit<Card>>` is annotated, as
  `type Ranking<T> : Seq<Hit<T>>` in [std/retrieval.hql](../../std/retrieval.hql)
  declares.
- An indexer contributes its embedding and index membership, and never a score.
  `metadata.lexical` carries what an extension knows about a card, and a score
  exists only on a `Hit`, beside the `Retrieval` that produced it. A retrieval
  extension contributes under a key named after itself, which is `lexical` in
  both the binary and the cards.
- The `uplinks` / `downlinks` rename is complete. No `backlinks` survives except
  where the rename is explained, and the direction sense ("uplinks outgoing,
  downlinks incoming") is stated identically in
  [Doc](../wiki/hql/types/doc-type.hmd), [HQL knowledge](../wiki/hql/knowledge.hmd)
  and [the link operator](../wiki/hql/operators/link-op.hmd).
- Generics are written `<>` in `doc/**`, in `src/`, in `std/` and in `examples/`.
  The old `Name[A, B]` spelling appears only where
  [the generics model](../models/behavior/bind-vs-apply-in-generic-types.md)
  explains it and where `tests/core.rs` pins it as an error.
- `cards : Set<Card>` is stated consistently across the cards, the models and
  `src/`, and `cards | take(2)` checks as `List<Card>`. Which type supplies the
  ordering `take` needs is `INC-29`.
- [expression behavior](../models/behavior/expressions.md) matches `src/`: at
  most 255 additions in one expression, `i64`/`f64`, no widening, and
  overflow-as-evaluation-error all hold against `src/parser.rs` and
  `src/evaluation.rs`.
- Every source path a document names — `src/…rs` in the cards, the proposals,
  the issues and [stack.md](../stack.md) — exists after the `src/types/` split.
- The four done issues read `Status: done` and sit in the board's `done` column;
  both proposals read `accepted`.
- A card without YAML frontmatter is well formed. The header is optional for an
  HMD document — see the [language specification](https://hypermarkdown.org/wiki/hmd-lang-spec/)
  — so [hypermarkdown.hmd](../wiki/hypermarkdown.hmd) having none is not a defect.
- Every relative Markdown link in `doc/`, `std/`, `examples/` and the root README
  resolves, apart from those in `INC-25` and `INC-26` and the deliberate
  placeholders in [doc/proposals/TEMPLATE.md](../proposals/TEMPLATE.md).
