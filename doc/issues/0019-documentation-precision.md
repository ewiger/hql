# 0019 — Documentation states semantics, not intuition

Status: doing
Related: the "Documentation site" section of [stack.md](../stack.md), which
holds the rule; [0015](0015-jq-parity-analysis.md) and
[0016](0016-sql-parity-analysis.md), which name the operations HQL adopts
from jq and SQL

The published prose often replaces a language or runtime property with a
friendly paraphrase. The landing page said:

> Every step is checked before anything runs, and every node in the answer
> records *why* it is there.

That sentence fuses two unrelated properties, static type checking before
evaluation and the presence a graph stores per node, and names neither.
"Step", "anything runs", "answer" and "why" are placeholders; the sentence
survives having its nouns replaced with "thing", which is the test for a
sentence with no semantic payload.

## The rule

A descriptive sentence answers at least one of: **what operation** happens,
**on what value**, **in what phase**, **with what types** in and out, **with
what observable result**. A sentence that answers none is cut. Introduce the
technical term first, state its observable behaviour second, and give at most
one line of intuition last. Headings name the term, not the slogan.

Language mechanisms are not described with anthropomorphic verbs:

| Not this | This |
| --- | --- |
| knows, understands | declares, stores, compares embeddings, matches text |
| figures out, decides | resolves, infers, checks, evaluates |
| checks everything | type-checks the complete program before evaluation |
| records *why* | stores a presence per node; carries its retrieval |
| finds what something is about | ranks cards by embedding similarity |

The same boundary matters for the coming Event, Scope and agent work: the
language has semantics, an agent has behaviour, and the prose must not blur
them.

## Canonical terms

Each term is defined as the implementation has it.

- **Static type checking.** The complete program is type-checked before
  evaluation. A type error is reported with its byte range and the program
  is not evaluated.
- **Pipeline step.** `xs | f(a)` is `f(xs, a)`. A step consumes the
  previous step's value and returns a new typed value; the output type of
  one step must be accepted by the next.
- **Lexical retrieval.** `lexical(query)` ranks cards by textual match with
  the query. It needs no index.
- **Semantic retrieval.** `semantic(query)` compares an embedding of the
  query with the embeddings in the vault's index and returns cards ranked by
  similarity. The index is built ahead of time; the binary reads it.
- **Retrieval provenance.** A hit carries its card, its score and the
  `Retrieval` that produced it: query, index, model, model revision, metric,
  and whether the search was approximate (`src/search.rs`).
- **Node presence.** A `Graph` stores one presence per node: `member` for a
  node projected from a collection, `retrieved <score>` for a hit, and
  `expanded from <seed> at depth <n>` for a node reached by traversal
  (`src/graph.rs`). It records the seed and the distance, not a predecessor
  edge, so the prose must not claim that a path can be reconstructed.
- **Extension.** A registered provider of typed steps. The implemented
  extensions are `core`, `graph`, `present`, `lexical` and `semantic`;
  extensions over SQL databases, JSON and YAML documents, message queues and
  REST APIs are design direction, and are described as such.

## Work

1. **The rule.** Written into the "Documentation site" section of
   `doc/stack.md`. Done.
2. **The public pages.** `doc/index.md`, `doc/public/introduction.md`,
   `doc/public/retrieval.md`, `doc/public/install.md`, `doc/public/tour.md`,
   `doc/public/reference.md`, `README.md`, the `site_description` in
   `mkdocs.yml`, and the opening of `doc/wiki/hql.hmd`. Done.
3. **The public wiki cards.** Every card with `nav: visibility: public`.
   Start with the `knows` hits: `doc/wiki/hql/types/card-type.hmd` (the
   heading "A card is a document that knows where it is", and lines 68, 162,
   286), `doc/wiki/hql/type-system.hmd` (lines 168, 376, 531),
   `doc/wiki/hql/types/data-type.hmd:80`, `doc/wiki/hql/types/graph-type.hmd:16`,
   `doc/wiki/hql/operators/link-op.hmd:133`, `doc/wiki/hql/fields.hmd:85`,
   `doc/wiki/hql/design-status.hmd:66`; and `doc/wiki/hql/semantic-search.hmd:139`
   ("carrying why each node is present").
4. **The Reference models in the nav.** `doc/models/domain/graphs.md` lines
   11-12 and 45 (`knows`); the tagline in
   `doc/models/requirements/bootstrap.md:9`.
5. **Headings.** Site-wide, a heading names the term: `Static type checking`,
   `Node presence`, `Semantic retrieval`, `Pipeline evaluation`.

## Done when

- No public page or card uses *why* for provenance; *knows*, *understands*
  or *figures out* for a language mechanism; or a slash compound such as
  "authored/storage" in place of two sentences.
- The opening sentence of every public page names an operation, a value, a
  phase or a type.
- Every integration beyond the five implemented extensions is marked as
  design direction.
- `uv run mkdocs build --strict` passes.
