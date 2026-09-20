# HQL-0003: A graph example corpus

**Status**: drafted
**Created**: 2026-09-20
**Source**: [the graph domain](../../models/domain/graphs.md)

## Abstract

A new example vault at `examples/graph/` brings the graph extension under the
corpus harness. It pairs a small hand-written wiki — concept cards in a
two-level hierarchy, one relation card whose endpoints resolve and one whose
endpoints do not — with runnable queries covering `uplinks`, `downlinks`,
`expand`, `graph`, and the `->` link operator. Every query carries the corpus
header the harness already requires, so its inferred type and its result are
pinned and checked by `cargo test`. Graph-valued cases declare `expected-json`
rather than `expected`, because a graph's `Display` collapses it to node and
edge counts. Nothing in the language, the extension, or the harness changes.

## Motivation

The graph extension is the largest declared surface in the project with no
example behind it. `examples/` holds one vault, `birds/`, and every query in it
is about collection types; the README describes it as a collection notebook and
says so.

Everything graph-shaped that runs today runs somewhere the corpus cannot see:

- [`nearest.hql`](../../../tests/fixtures/queries/nearest.hql) and
  [`kinds.hql`](../../../tests/fixtures/queries/kinds.hql) sit under
  `tests/fixtures/`, which `load_cases` skips explicitly. They have no corpus
  header, no pinned type, and no README that links them.
- `uplinks`, `downlinks`, `expand` and `graph` otherwise appear only as source
  strings inside [`tests/extension_contracts.rs`](../../../tests/extension_contracts.rs),
  which asserts that each step runs at its declared result type and says nothing
  about what it returns.
- Two assertions in [`tests/birds.rs`](../../../tests/birds.rs) count links in a
  fixture vault. They are the only checks that traversal produces the right
  edges, and they check a count.
- The link operator is lexed, parsed and evaluated — `->` builds an `Edge` with
  no vault at all — and no example writes one.

Three things follow, and they are why this is worth a record rather than a
commit.

**The design corpus does not cover the domain.** Examples are where HQL features
are designed; a domain with no runnable example is a domain whose open questions
never meet a value. [The graph domain](../../models/domain/graphs.md) closes on
six open questions — variance, endpoint resolution, horizon, merge policy,
whether `Graph` is one type or a family, weights — and not one of them is
currently pressed by anything a reader can run.

**Observable graph output is unpinned.** A `Graph` renders as
`graph of N nodes and M edges`, its table renders node presence through
`Presence::label`, and its JSON carries `nodes[].presence` as that same label
string. Those are the only ways a graph is ever seen, and no test fixes any of
them against a stated expectation. `expand` exists precisely so a subgraph does
not claim its neighbours matched; that guarantee is a label in a JSON document
nobody asserts on.

**The two-graph split is invisible.** The document graph is over `Card` and the
knowledge graph over `Concept`, so a relation card is a node in one and an edge
in the other. That is the central claim of the domain model, and demonstrating
it needs a vault holding both kinds of card — which `examples/birds/wiki` does
not.

## Goals

- Every graph step has a runnable example whose type and result are checked.
- The `ConceptCard` / `RelationCard` distinction is visible in a result, not
  only in prose.
- A change to graph rendering, presence labelling, or edge traversal fails
  `cargo test` with a named example rather than passing silently.

## Non-goals

- **No new steps, syntax or types.** The harness asserts
  `implementation: implemented` for every corpus case, so this proposal can only
  exercise what exists. A traversal, subgraph or path vocabulary stays undesigned.
- **No write path.** `->` yields an `Edge` value. It does not add an edge to a
  vault, and the example MUST NOT imply that it does.
- **No renderer.** `render(Graph) -> Html` is not part of this.
- **No `graph.nodes` / `graph.edges` field access.** Whether a graph exposes its
  parts as collections is open in the domain model; an example would settle it
  by accident.
- **No move of the existing fixture queries.** `nearest.hql` and `kinds.hql`
  stay where the tests that use them expect them.

## Specification

### 1. Location and shape

The example is a sibling of `birds/`, with the same three-part layout.

```text
examples/graph/
  README.md          the index; every query MUST be linked from it
  wiki/              the vault, and the only directory loaded as one
  queries/*.hql      corpus cases
```

- The vault root MUST be `examples/graph/wiki`, so the README and the query
  files do not become cards. This matches `birds/` and is what the
  `vault: graph/wiki` header resolves to, since `Case::vault` joins the header
  value onto `examples/`.

### 2. What the vault holds

The vault is written for the traversals, not for prose. It MUST contain:

- **Concept cards in a two-level hierarchy**, so that a card has both an
  outgoing and an incoming direction and `expand(depth = 1)` reaches something
  a reader can name. A parent with three children is the minimum that makes
  `downlinks | count` a number worth pinning.
- **One resolving relation card** — `metadata.knowledge.type: Relation` with a
  `relation.source` and `relation.target` that both name cards in the vault.
- **One non-resolving relation card**, declaring the same shape over names that
  are absent. It demonstrates the rule the type system rests on: an authored key
  does not mint a type, so the card stays a `ConceptCard` and its failed claim
  survives in its metadata rather than being discarded.

Card bodies SHOULD stay short. This vault is read by traversals, and any lexical
or semantic step run against it is out of scope.

### 3. Corpus headers

Each query file MUST open with the header block `load_cases` parses — `// key: value`
lines, then a blank line, then the source:

```text
// status: valid-now
// feature: <what the case shows>
// implementation: implemented
// environment: knowledge-v1
// vault: graph/wiki
// expected-type: <the exact inferred type, as it prints>
// expected-json: <the result as JSON>
```

- Graph-valued cases MUST declare `expected-json`, not `expected`. A graph's
  `Display` is `graph of 4 nodes and 3 edges`, which would pin the counts and
  nothing else; the JSON carries each node's presence label and each induced
  edge, which is the part worth protecting.
- Edge-valued and scalar cases MAY use `expected`, whose form is fixed: an edge
  prints as `source -> target`, a list as `[a -> b, c -> d]`.
- `expected-type` MUST be the type as it prints, because the harness compares
  `ty.to_string()` to the header verbatim.

### 4. The required cases

Each row is one file under `queries/`. Result forms follow the specification
above; exact values are fixed against the vault when it is written.

| Query | What it shows | Type |
| --- | --- | --- |
| `uplinks` | the edges leaving one card, addressed by `[[name]]` | `List<Edge>` |
| `downlinks` | the edges entering a card, which needs a vault | `List<Edge>` |
| `expand` | a seed plus its neighbours, each node labelled with why it is present | `Graph` |
| `graph` | a filtered collection of cards projected whole | `Graph` |
| `kinds` | the relation card that resolves beside the one that does not | a collection of cards |
| `link` | `[[a]] -> [[b]] { .. }` as a value | `Edge` |

- The `link` case MUST declare `environment: pure` and MUST NOT declare a
  `vault`, which the harness enforces as an exclusive pair. The operator
  resolves nothing: it takes the two names as written. Stating that in a header
  is the cheapest available proof that authoring a link is not a vault write.
- The `expand` case MUST assert a node whose presence is `expanded from <seed> at depth 1`
  alongside one that is `member`. That contrast is the whole reason `Presence`
  exists.
- The `kinds` case MUST show the non-resolving relation card appearing as a
  `ConceptCard`. A card's JSON carries `kind`, so the expectation can state it.

### 5. The index

- `examples/graph/README.md` MUST link every case as `](queries/<name>.hql)`.
  The harness walks up from each case to the nearest `README.md` and asserts
  that exact substring, so a case added without an index entry fails.
- It SHOULD carry the same table-and-commands shape as
  [`examples/birds/README.md`](../../../examples/birds/README.md): the build
  command, one `hql --vault examples/graph/wiki run …` line per query, and a
  table of query, what it shows, and result.
- [`examples/README.md`](../../../examples/README.md) MUST gain a sentence
  naming this vault and what it covers, since it currently describes `birds/` as
  the only example.

### 6. What the example must not claim

The domain model's open questions MUST NOT be answered by an example that
happens to run:

- no `graph.nodes` or `graph.edges` access,
- no query relying on a merge policy for two edges over the same pair,
- no traversal past the vault horizon,
- no case that would need `Graph<Card, Link> : Graph<Doc, Edge>` to check.

Where a case comes close to one of these, its comment SHOULD name the open
question instead of demonstrating an answer.

## Backwards Compatibility

Additive. No source file, step, type or harness behavior changes, and existing
examples are untouched.

The one real consequence is intended: graph output becomes pinned. After this,
changing `Presence::label`, the graph JSON shape, or which edges `induced`
returns breaks a named example rather than passing. Any such change must then
update the example in the same commit, which is the point.

## Reference Implementation

- `examples/graph/wiki/*.hmd` — the cards specified above.
- `examples/graph/queries/*.hql` — the six cases.
- `examples/graph/README.md` — the index the harness requires.
- `examples/README.md` — one added paragraph.

No Rust changes. The cases are collected by the existing
[`tests/corpus.rs`](../../../tests/corpus.rs) walker, which picks up any `.hql`
file under `examples/` that is not the `answer.hql` smoke fixture.

## Test Plan

The existing harness supplies the coverage; this proposal supplies the cases.
`corpus_metadata_and_index_cover_every_case` checks the headers and the README
link, and `valid_now_cases_check_and_evaluate` checks each case twice — once
through `check_in`, once through `run` — and asserts the two agree before
comparing the result to the header.

```bash
cargo build --locked
cargo test --test corpus
cargo test
```

Each query MUST also run from the command line exactly as the README prints it,
including the `--vault` argument, since a README command that does not run is
the failure mode an index is supposed to prevent.

## Open Questions

These MUST be resolved before this moves from drafted to accepted.

- Should the vault be a fresh subject, or the taxonomy already written in
  `tests/fixtures/birds`? The fixture has the hierarchy and the relation cards
  ready, but it is built to satisfy a lexical-retrieval constraint — no card may
  share a word with the queries asked of it — and an example vault inherits that
  constraint silently.
- Does `expected-json` for a `Graph` pin a float? `Presence::label` formats a
  retrieved score as `{:.4}`, so a case combining retrieval with `expand` would
  encode a score in a label string. Keeping retrieval out of this example avoids
  it; whether that is a rule or an accident should be decided.
- Should `nearest.hql`, which is the best existing demonstration of
  retrieval-then-expansion, gain a corpus-headed counterpart here, given that it
  would import `lexical` and so reintroduce the float above?
- Is a sixth `graph` case redundant beside `expand`, since both return `Graph`
  and differ only in how nodes got there?

## Changelog

- 2026-09-20: drafted
