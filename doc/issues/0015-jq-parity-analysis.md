# 0015 — jq parity analysis: data formats, types and operations

Status: backlog

jq is the reference tool for querying and reshaping JSON. HQL already has the
pipeline shape jq has, and a `Data` type that is the JSON/YAML data model
([0006](0006-scalar-and-data-union.md),
[data-type](../wiki/hql/types/data-type.hmd)). What it lacks is a way to
treat a JSON, YAML or CSV document as a query source, and much of the
vocabulary jq uses on one.

This issue is an **analysis**, not an implementation. It decides which jq
features HQL can support well, names the roadblocks that stand in the way, and
puts the work in an order. Its outcome is a proposal and follow-up issues;
no code under `src/` changes here.

## Questions the analysis answers

1. **Input and output formats.** What it takes to read JSON, YAML and CSV as
   a query source, from a file, from stdin, or as a binding beside `cards`
   ([0008](0008-cards-as-a-vault-binding.md)), and to write them back out.
   Today JSON is output only, through the `json` presentation step and
   `--format json`. The binary already depends on `serde_json` and
   `serde_yaml_ng`. CSV output is a natural fit for a `table`, and CSV input
   is a natural source of records.
   - **Pretty printing for free.** `jq .` is the identity program, yet it is
     the most common use of jq: any JSON fed in comes back indented and
     coloured. HQL's identity does the same over every input it understands,
     each rendered in its own format: a JSON, YAML or CSV document, a Markdown
     document, a single card, a query over many cards, or the whole vault.
     Markdown in gives pretty-printed Markdown out, and so on for each format.
     jq's output switches carry over where they mean something: compact
     (`-c`), raw strings (`-r`), sorted keys (`-S`), and colour only on a
     terminal. The analysis decides whether this is the default presentation
     of a value or a CLI mode.
   - **Where the input comes from.** Three sources, decided together:
     - **Standard input**, when `hql` runs as a command-line tool or an
       interpreter, which is how `jq .` is used.
     - **A path opened from inside a query.** HQL has no way to open a file
       today. The [data-type](../wiki/hql/types/data-type.hmd) card sketches
       `yaml.load("config.yaml")`. The analysis weighs whether a per-format
       loader or one `open` that picks the format from the file is better.
     - **The vault**, through `cards` and `--vault`, as now.
2. **Separate names, separate types.** Whether each format gets its own
   operation and its own type (`json`, `yaml`, `csv`; a CSV row is not a
   `Data` tree), and how they fit the presentation rules of
   [HQL-0008](../proposals/HQL-0008/README.md): reading a format is
   computation, writing one is presentation.
3. **Type system roadblocks.** Where `Data` is too weak or too strict to work
   with as freely as jq does:
   - Going from `Data` to the member a value actually is (`x : Int`) is
     refinement, which is still design.
   - Key order: `Data::Map` is a `BTreeMap`, so a round trip sorts keys.
   - `null` against `Empty` against `Option<T>`; non-finite floats; integers
     beyond `i64`.
   - Typing a heterogeneous array, and checking a document against a declared
     schema (`typed(as: …)` over records).
4. **Operations and operators.** Which jq features map onto HQL, which need
   a new form, and which do not belong. At least:
   - Path access: `.a.b`, `.[0]`, slices, optional access `.a?`, and
     recursive descent `..`.
   - Iteration and construction: `.[]`, object and array construction,
     `to_entries` / `from_entries`, `keys`, `has`.
   - Aggregation: `reduce`, `group_by`, `unique_by`, `min_by`, `max_by`,
     `add`. These overlap with `group` / `reduce` in
     [0014](0014-next-feature-bundle.md) and
     [HQL-0004](../proposals/HQL-0004/README.md).
   - Updates: `|=`, `del`, `setpath`. In a functional, typed language these
     return a new tree.
   - Strings: interpolation, `split`, `join`, regex `test` / `capture`, and
     formatters such as `@csv`, `@tsv` and `@base64`.
   - Alternatives and errors: `//`, `try` / `catch`, `empty`.
   - Streaming large inputs (`--stream`), against `Stream<T>` in
     [HQL-0006](../proposals/HQL-0006/README.md).
5. **What HQL adds.** Where static checking, the knowledge graph and retrieval
   give a better answer than jq, so the analysis argues for parity *and more*
   and not for a jq clone.

## Method

- Survey the jq manual and sort every feature into one of three groups:
  *supported now*, *supportable* (with the missing type or operation named),
  *out of scope* (with a reason).
- For each supportable feature, write a jq one-liner and the HQL program it
  would become as an example under `examples/` with `status: proposed`. The
  examples are the design corpus.
- Rank the roadblocks by how many examples each one unblocks.

## Done when

- A feature matrix (jq feature, HQL status, missing piece) exists in a new
  proposal under `doc/proposals/`.
- The examples above are committed with `status: proposed`.
- The main roadblocks are listed in priority order, starting with the type
  system, and each becomes a backlog issue or a step of an existing one.
- Format input and output, JSON, YAML and CSV at least, has its own follow-up
  issue. That issue includes the HQL equivalent of `jq .`: pretty printing
  any input, whether from stdin, a path or the vault, in its own format with
  no query written.
