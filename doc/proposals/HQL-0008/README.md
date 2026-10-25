# HQL-0008: Presentation is separate from computation

**Status**: superseded by [HQL-0017](../HQL-0017/README.md)
**Created**: 2026-09-28
**Source**: [presentation](../../wiki/hql/presentation.hmd)

[HQL-0017](../HQL-0017/README.md) supersedes this record: presentation is the
`format` operation over format types, and the section "Presenting into a
document" is carried by `hql render` and HQL-0017's `hmd` presentation context.

## Abstract

**Presentation** is the umbrella term for turning a pipeline's result into a
form a consumer reads. A presentation may be written as a step, `| table`, but
it is not an operation: it is registered separately, it is always the last step
of a pipeline, and nothing downstream reads its result. The initial
presentations are `table`, `graph` and `markdown`, beside the existing `json`
and `text`. A presentation of a finite value renders it once; a presentation of
a `Stream<T>` renders incrementally as values arrive.

## Motivation

- The host already has a `Presentation` type and a `present` extension
  (`table`, `json`, `text`), and the [presentation](../../wiki/hql/presentation.hmd)
  card already says a presenter is terminal. But presenters are registered as
  ordinary steps, beside `map` and `filter`, so nothing in the host type tells a
  planner or a help page that they are a different kind of thing.
- [HQL-0005](../HQL-0005/README.md) gives a step two possible targets, an
  operation or a presentation. This record is the second half of that split.
- Once streams exist ([HQL-0006](../HQL-0006/README.md)), a presentation can be
  asked to render a value that never ends. That case needs a rule.
- The name of the category has been unsettled. "Visualization" excludes
  `markdown` and `text`, which are presentations and are not visual.
  "Representation" is the natural word for how a value is held in memory, and
  the runtime carrier discussion in [queries](../../models/behavior/queries.md)
  needs it for that.

## Non-goals

- **No new presentations beyond `markdown`.** `tree`, `value` and `empty` stay
  listed on the card as intended, not specified.
- **No presentation arguments.** Column selection for `table` and the like are
  an open question.
- **No persistence.** Writing a presentation to a file stays an explicit host
  effect, as the card already says.

## Specification

### The category

- The category MUST be called **Presentation**, and a registered member of it a
  presentation. It MUST NOT be called Visualization or Representation in `doc/`,
  `src/` or `hql builtins` output.
- A presentation MUST accept its input and produce a `Presentation` value. Its
  input type MAY be any type; presenting is where the element type stops
  mattering, which is why nothing may read the result.
- A presentation MUST NOT change the value it presents. It decides only how the
  value is rendered.

### Presentations are not operations

- Presentations MUST be registered in their own table, distinct from
  operations, so that the host type of what a step invokes says which kind it
  is.
- A presentation MUST be the last step of its pipeline. A step after a
  presentation MUST be a check error naming both steps; the current rule
  ("nothing pipes out of it") becomes a diagnostic rather than a convention.
- Presentations carry no shape. A planner places a presentation in its own
  stage at the end of the plan, after every operation.
- A presentation MUST be pure. Rendering to a terminal or a file is performed
  by the host after the pipeline returns.
- `hql builtins` MUST list presentations under their own heading, as
  [HQL-0005](../HQL-0005/README.md) requires.

### The initial presentations

| Presentation | Input | Renders |
| --- | --- | --- |
| `table` | any | rows and columns |
| `graph` | a graph, a card, or a collection of cards or hits | a graph view |
| `markdown` | any | Markdown text |
| `json` | any | structured data |
| `text` | any | plain text |

- `markdown` MUST render records as a Markdown table, sequences as a list, and
  a `Doc` or `Card` as its own Markdown body.
- The spelling `table` and `table()` MUST both parse and mean the same step.

### `graph` is a presentation

Today `graph` is registered in the graph extension as a projection that returns
a `Graph` value. The [presentation](../../wiki/hql/presentation.hmd) card
records that it is doing two jobs.

- The step `graph` MUST become the presentation.
- The projection from cards to a `Graph` value MUST get a distinct name, so that
  a pipeline that continues after it (`expand`, `uplinks`) invokes an operation
  and not a presentation. The name is an open question.

### Presenting a stream

- A presentation of a finite value MUST render it once, after the pipeline has
  produced it.
- A presentation of a `Stream<T>` MUST render incrementally: each value, or each
  row, is rendered as it arrives, and the presentation is complete only if the
  stream ends. `table` over a stream appends rows; `markdown` over a stream
  appends list items.
- The corpus pins presentation results only over finite inputs, since an
  unbounded presentation has no final text to compare.

### Presenting into a document

`hql render` writes a cell's output into a Markdown or HyperMarkDown document
as part of the document, not inside a fence. Issue
[0020](../../issues/archive/0020-render-as-compilation.md) carries the command.

- A host that writes into a document MUST produce Markdown: the output is
  spliced into the document as written.
- In a document, `table` MUST render as a Markdown pipe table, with a header
  row and a delimiter row.
- A cell without a presentation MUST be presented with `markdown`.
  [Knowledge](../../wiki/hql/knowledge.hmd) is still excluded from that default,
  as the [presentation](../../wiki/hql/presentation.hmd) card requires.
- A presentation the query chose MUST be written in its document form: `table`
  as a pipe table, `json` as a fenced `json` block, and `text` and `markdown`
  as they are.

## Backwards Compatibility

`table`, `json` and `text` keep their names and output. `markdown` is new.
The `graph` projection changes name, which changes every program that pipes a
`graph` result into a further step; those programs get a diagnostic naming the
new operation. A pipeline that ends in `graph` is unaffected in what it shows.

## Reference Implementation

- `src/extensions/mod.rs` — a separate presentation table on `Extension`, and
  the check that a presentation is last
- `src/extensions/present.rs` — `markdown`; `table`, `json`, `text` moved to the
  presentation table
- `src/extensions/graph.rs` — `graph` split into the presentation and the
  renamed projection
- `src/types/display.rs` — Markdown rendering, and incremental rendering of a
  stream
- `src/transclude.rs` — the document host writes Markdown output in place of,
  or after, each cell
- `doc/wiki/hql/presentation.hmd` — rewritten to state the rules above and drop
  the resolved open items

## Test Plan

Unit tests MUST include:

- `cards | table | size` is a check error naming `table` and `size`
- `table` and `table()` produce the same result
- `markdown` over a list of records produces a Markdown table
- a presentation over an unbounded test source followed by nothing renders
  rows incrementally and `take` upstream bounds it
- `hql builtins` lists `table`, `graph`, `markdown`, `json` and `text` under
  the presentation heading and not under operations

## Open Questions

- What is the projection from cards to a `Graph` value called once `graph` is
  the presentation? Candidates: `to_graph`, `as_graph`, `project_graph`.
- May a presentation take arguments (`table(columns = ...)`,
  `graph(depth = 2)`)? The prompt that produced this record writes `table(...)`;
  the card lists it as open.
- Is `Presentation` one opaque type, or does each presentation return a named
  subtype (`Table`, `Markdown`)?
- Outside a document, does a cell without a presentation keep a host default
  chosen by result type, as the card describes, and does a `Stream<T>` result
  default to `table`?

## Changelog

- 2026-09-28: drafted
- 2026-10-18: presenting into a document
- 2026-10-19: superseded by HQL-0017, which implements presentation as the
  `format` operation and carries "Presenting into a document" as `hql render`
  in the `hmd` presentation context
