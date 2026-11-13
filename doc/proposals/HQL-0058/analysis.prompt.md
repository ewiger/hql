# Prompt: how search, storage and memory are wired

Paste the block below into a new session in this repository. It produces a
report and changes no code. Run it before revising
[HQL-0058](README.md), and again after any change to `semantic`, `lexical`,
the index or `remember`.

---

Analyse how search, storage and AI memory are wired together in HQL, as the
code stands, and report where the wiring is absent. Write the report to
`doc/proposals/HQL-0058/analysis.md`. Change nothing else.

Read `doc/memory/` first, as `CLAUDE.md` requires, then `doc/stack.md`.
Ignore `.grem/**`. Treat documents as claims and code as fact: where a card or
a proposal says something the code does not do, the code is what you report,
and the disagreement is a finding.

## The question

AI memory is one loop over one corpus:

```text
corpus -> restrict -> rank -> combine -> context -> run -> write -> index -> corpus
```

For each arrow, establish what exists, what type crosses it, and what is lost
crossing it.

## What to read

- Search: `lib/std/retrieval/`, `lib/search/`, the `search` extension under
  `src/extensions/`, the cards `semantic-search`, `semantic` and `lexical`
  under `doc/wiki/hql/`, and HQL-0002.
- Storage: the index reader, `contrib/semantics/`, `[semantics]` in the
  fixtures' `hql.toml`, and the endpoint records HQL-0012, HQL-0035 and
  HQL-0046 for how any other store is named, typed and written.
- Memory and runs: `lib/std/ai/`, the `ai` extension, HQL-0022, and the Learn
  chapters on retrieval, agents and runs, and agents inside documents.
- Attributes and links: how `filter` reads `card.metadata`, how `expand` and
  `Graph` traverse, and the `fields`, `card-type` and `knowledge` cards.
- Planning: the optimizer rule in the `pipes` card, and HQL-0055 and HQL-0056
  for what a step may be reordered or pushed into.

## Method

1. **Inventory.** List every step that takes a corpus and returns a narrower
   or ordered one: its module, its signature, its purity, what it reads from
   outside the program, and the test that pins it. Run `hql builtins` and
   read the declarations; do not rely on the cards.
2. **Compose in pairs.** For each ordered pair of kinds among attribute,
   lexical, semantic and graph, write the shortest program that combines
   them and run it against `tests/fixtures/birds/` or `examples/tutorial`.
   Record for each: does it check, does it run, what is the result type, and
   does each result still say how it was found. A pair that can only be
   written by sorting records by hand is recorded as not composable.
3. **Trace one round trip.** Take the document
   `examples/notebook/next-meeting.hmd`. Follow one value from the ranking,
   through `Context`, `run` and `remember`, to the next render. At each step
   name the file and function, the fields that carry provenance in and out,
   and the first step at which the remembered card cannot be found by
   meaning. State exactly which command a person must run today to make it
   findable.
4. **Storage.** State what is stored, by whom, in what unit, and when it goes
   stale: card vectors, query vectors, recordings, memory cards. For each,
   name its writer and its reader, and whether HQL can write it. List every
   condition under which `semantic` warns or fails, with the message.
5. **Order and equivalence.** For `filter` before and after each ranking
   step, state whether the results are equal and why, from the scoring code.
   State what the optimizer rule permits and forbids for each.
6. **Compare with HQL-0058.** For each numbered rule area of its
   Specification, mark it: already true, contradicted by the code, buildable
   from existing steps in HQL source, or needing native work. Name the file
   each one lands in.

## The report

- One table per arrow of the loop: what exists, the type that crosses, what
  is lost, the evidence as a file and line or a program and its output.
- The pair table of step 2, complete, including the pairs that work.
- The round trip of step 3 as a numbered list.
- A list of findings, most consequential first. Each is one sentence stating
  the defect, then the program or file that shows it. Do not propose syntax
  in a finding.
- Last, a short section of proposed examples: programs that should run and do
  not, each written as it would appear under `examples/`, with the result it
  should give. These are the cases the proposal is revised against.

Report only what you ran or read. Where something could not be run, say what
stopped it. Do not add open questions; state what is unknown as a finding
with what would settle it.
