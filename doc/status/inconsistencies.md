# Inconsistencies across doc/

First audited 2026-09-17 on branch `feat/lang-design`; re-verified 2026-09-18
against the working tree of `feat/semantic-search`.

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
| INC-25 | Eight documents link into `examples/`, which exists in no branch | broken | wiki, models, proposals, root README |
| INC-13 | The wiki breaks its own two rules about wikilink spelling and scope | contradiction | [hypermarkdown.hmd](../wiki/hypermarkdown.hmd) and the HQL cards |
| INC-12 | Four links point at renamed or nonexistent cards | broken | wiki README, hypermarkdown, issues README, kanban.yaml |
| INC-23 | An indexer is said to contribute "embeddings or a semantic-search score"; a score is not a per-card fact | contradiction | [card-type.hmd](../wiki/hql/types/card-type.hmd) |
| INC-20 | The Kanban README's worked example does not match the real board | stale | [issues/README.hmd](../issues/README.hmd) |

Blocked, not open: `INC-04`, `INC-06`, `INC-08`, `INC-16`, `INC-17` and `INC-18`
are all findings *about* the example corpus. See **The absent corpus** below.

## INC-25 — Documents link into a corpus that is not in the repository

`examples/` is absent from the working tree and from every branch —
`feat/lang-design`, `feat/semantics`, `feat/semantic-search` and `main` all have
zero files under that path. Eight relative links point into it anyway, so each
one is broken:

| Where | What it points at |
| --- | --- |
| [README.md](../../README.md) | the example corpus |
| [design-direction.hmd](../wiki/design-direction.hmd) | the corpus |
| [design-status.hmd](../wiki/hql/design-status.hmd) | the corpus |
| [knowledge.hmd](../wiki/hql/knowledge.hmd) | the corpus |
| [std-lib.hmd](../wiki/hql/std-lib.hmd) | `examples/README.md`, to argue `std/` is *not* the corpus |
| [program-values.md](../models/behavior/program-values.md) | a worked case |
| [architecture.md](../models/domain/architecture.md) | the corpus |
| [HQL-0002](../proposals/HQL-0002/README.md) | corpus cases |

This is the finding to decide first, because six other findings and six
consolidation items are bookkeeping about the same absence.

## The absent corpus

`INC-04` (`cards : Set<Card>` versus `List<Card>` at a third site), `INC-06`
(mixed generic brackets in one type), `INC-08` (case counts disagreeing three
ways), `INC-16` (`validate` with two signatures), `INC-17` (the Person fixture
contradicting its README) and `INC-18` (expectation types no vocabulary defines)
were all findings against files under `examples/`. Nothing can be verified or
fixed while the corpus is absent, and none of them is evidence about the current
repository.

They keep their identifiers so that a sweep of a restored corpus can point at
them. If `INC-25` is decided against restoring the corpus, they are deleted
along with it.

## INC-13 — The wiki breaks its own linking rules

[hypermarkdown.hmd](../wiki/hypermarkdown.hmd) states two rules about the wiki
graph that the wiki itself breaks:

1. **"The slug names a `.hmd` file … by filename alone (subfolders included)."**
   Sixty-odd links across the HQL cards are path-qualified instead —
   `[[hql/knowledge]]` twenty-two times, `[[hql/design-status]]` eleven,
   `[[hql/collections]]` ten, and so on — while sibling links in the same files
   use bare slugs (`[[card-type]]`, `[[pipes]]`). Both forms would resolve under
   a by-filename rule; the mixture means the rule is being ignored rather than
   applied, and a reader cannot tell which spelling is canonical.
2. **"Anything outside `doc/wiki/` … uses ordinary relative Markdown links,
   never `[[ ]]`."** [doc/issues/README.hmd](../issues/README.hmd) is outside
   `doc/wiki/` and uses `[[hyper-markdown]]`.

**Resolution needed:** decide whether wiki subfolders take path-qualified slugs,
then apply it once; and either extend the `[[ ]]` space to `doc/issues/` or
convert that one link.

## INC-12 — Broken links

| Where | Link | Problem |
| --- | --- | --- |
| [doc/wiki/README.md](../wiki/README.md) | `hyper-markdown.hmd` | The file is `hypermarkdown.hmd` |
| [doc/wiki/hypermarkdown.hmd](../wiki/hypermarkdown.hmd) | `[[kanban]]`, three times | No `kanban.hmd` card exists anywhere under `doc/wiki/` |
| [doc/issues/README.hmd](../issues/README.hmd) | `[[hyper-markdown]]` | Same rename; see also INC-13 |
| [doc/issues/kanban.yaml](../issues/kanban.yaml) | comment "Convention: doc/wiki/kanban.hmd" | That file does not exist |

`[[kanban]]` may be a deliberate forward link — hypermarkdown says forward links
are fine — but the card closes with "See [[kanban]]", which reads as a pointer to
something written, and `kanban.yaml` names the card as an existing convention.

## INC-23 — An indexer cannot contribute a score

[Card](../wiki/hql/types/card-type.hmd) lists, among the entries an extension may
contribute, "an indexer supplying embeddings or a semantic-search score". The two
are not the same kind of thing.

An embedding is a property of the card: it exists whether or not anyone ever
searches, it goes stale when the card's content changes, and the assembled
`card.metadata` layer is the right home for it. A score exists only relative to a
query. Storing one in a per-card layer means either one privileged query or a
number whose name no longer describes it.

The fix is to split the sentence, keeping embeddings, chunk identity and index
membership as contributions and moving the score into the retrieval result, where
it travels with the query and index revision that produced it. The
[semantic search model](../models/domain/semantic-search.md) already states the
correction; the card has not been edited to match.

## INC-20 — The Kanban example does not match the board

[doc/issues/README.hmd](../issues/README.hmd) shows a sample board with
`last_id: 1` and one card in `todo` naming a file `0001-first-issue.md` that does
not exist; the real [kanban.yaml](../issues/kanban.yaml) has `last_id: 3` and
three live cards. The README example is illustrative, but nothing says so.

## Verified consistent

Worth recording, because these were checked and hold:

- The `uplinks` / `downlinks` rename is complete. No `backlinks` survives except
  where the rename is explained, and the direction sense ("uplinks outgoing,
  downlinks incoming") is stated identically in
  [Doc](../wiki/hql/types/doc-type.hmd), [HQL knowledge](../wiki/hql/knowledge.hmd)
  and [the link operator](../wiki/hql/operators/link-op.hmd).
- Generics are written `<>` in `doc/**`, in `src/` and in `std/`. Nothing carries
  the old `Name[A, B]` spelling.
- `cards : Set<Card>` is stated consistently across the cards, the models and
  `src/`; `Orderable` supplies the ordering `take` needs.
- [expression behavior](../models/behavior/expressions.md) matches `src/`: the
  256-literal bound, `i64`/`f64`, no widening, and overflow-as-evaluation-error
  all hold against `src/parser.rs` and `src/evaluation.rs`.
- A card without YAML frontmatter is well formed. The header is optional for an
  HMD document — see the [language specification](https://hypermarkdown.org/wiki/hmd-lang-spec/)
  — so [hypermarkdown.hmd](../wiki/hypermarkdown.hmd) having none is not a defect.
- Every relative Markdown link in `doc/` resolves, apart from those in `INC-12`,
  `INC-25` and the deliberate placeholders in
  [doc/proposals/TEMPLATE.md](../proposals/TEMPLATE.md).
