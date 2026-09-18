# Prompt — issue 0001

Paste the block below into a new session, in this repository. It is the
instruction; [0001-real-embeddings.md](0001-real-embeddings.md) is the detail it
refers to.

---

Work on issue 0001. Start by reading `doc/issues/0001-real-embeddings.md` in
full, and `doc/memory/` as `CLAUDE.md` requires. Create the branch
`feat/semantics` off `feat/lang-design` before touching anything.

**Settle the five Open Questions at the end of the issue with me before writing
any code.** Ask them as one batch with your recommendation for each; do not
guess and do not start while they are open.

The job, in three commits, in this order:

1. **`tests/fixtures/birds/`** — a vault of at least 40 bird articles with a
   real taxonomy: order cards, family cards, species cards, at least four
   relation cards (`metadata.knowledge.type: Relation` with
   `knowledge.relation.source`/`.target`), at least one whose endpoints
   deliberately do not resolve, `metadata.status` on a useful subset, `[[links]]`
   in bodies including one forward link to a document that does not exist.
   Real encyclopedic prose, three to eight sentences per article — the articles
   are the corpus a model will embed, so templated text leaves nothing to
   measure. Leave `tests/fixtures/vault` alone; the suite pins behaviour
   against it.

2. **`contrib/semantics/`** — a standalone Python CLI
   (`hql-semantics build|verify|inspect`) that reads a vault and writes the
   SQLite index whose schema the issue pins exactly. Default model
   `sentence-transformers/all-MiniLM-L6-v2`, with the revision pinned, vectors
   L2-normalised, and that fact recorded in the database.

3. **The rename and the consumer** — the current hashed-bag matcher becomes
   `lexical(query)`, unchanged except in name, keeping `hql.hashbag.v1` as its
   reported model. `semantic(query)` reads the embedding index, configured per
   vault under `[semantics] index = "..."` in the `hql.toml` the reporting layer
   already reads.

Constraints that are not negotiable, because getting them wrong is the failure
this issue exists to prevent:

- **The embedded text must be byte-identical between Python and Rust.** Today
  that is `Document::text()` in `src/document.rs`: title, then every string in
  the header, then the body, joined by single spaces. Pin the contract in both
  implementations and prove it with a golden test over every card in the birds
  vault. A mismatch silently indexes the wrong text.
- **`cargo test` must pass with no Python installed.** Commit the prebuilt index
  at `tests/fixtures/birds/.hql/index.sqlite` (~61 KB) and have the Rust tests
  read it. `pytest` covers the Python side and runs separately.
- **`semantic` without an index is an error** naming the missing index and the
  command that builds one. Never fall back to `lexical` — that restores the
  confusion being removed.
- **`Retrieval` reports what actually ran**: the real model id, its revision,
  the metric computed, and `approximate: false`, because scoring is exact brute
  force over stored vectors. Add `revision` to the type.
- **Staleness is detected**: Rust recomputes the content hash and compares, and
  a mismatch goes into the report queue rather than passing silently.
- **A card missing from the index is absent from the ranking**, with a warning
  counting how many were skipped — never scored zero, which is indistinguishable
  from indexed-and-unrelated.

The acceptance criterion is a measurement, so design the articles for it:
write the owl articles using "nocturnal", "after dark", "low light", and never
the phrase "hunt at night". Then `lexical("birds that hunt at night")` finds
nothing useful and `semantic("birds that hunt at night")` surfaces the owls.
Produce at least three such query pairs and pin them as tests. If those tests
pass, the issue worked; if they are weak, the rest does not compensate.

Out of scope, and say so in the memory note: approximate or ANN search, top-k
pushdown, chunking, `recall` over knowledge, any write path or effect, and any
embedding computed inside the Rust binary.

Finish by updating every file listed under "Documentation to update" in the
issue — including closing `INC-23` in `doc/status/inconsistencies.md` and adding
a `doc/memory/` note in the style of `doc/memory/cli-host.md`. Leaving `doc/`
stale is how `INC-15` and `CON-05` happened. Then move the card to `done` in
`doc/issues/kanban.yaml`.

Gates before you call it finished: `cargo test` passes with no Python present,
`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean,
`hql-semantics verify` reports the committed index clean, and
`hql --vault tests/fixtures/birds eval` answers a paraphrase query correctly
where `lexical` does not.
