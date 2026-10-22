# 0039 — A `Table` shape, and a review of rendering and presentation

Status: backlog
Related: [HQL-0017](../proposals/HQL-0017/README.md),
[formats and codecs](../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[presentation](../wiki/hql/presentation.hmd),
[the ADR status page](../status/ADR-status.hmd)

HQL-0017 separates what a value is (a shape type: `Graph`, `Table`, `Tree`)
from the syntax it is written in (a format), and names a renderer for each
shape in each format. `Table` was deferred until record values existed.
Records exist now; `Table` does not: `Table(xs)` is a name error.

The table renderers `aligned`, `pipe`, `rows` and `records` are registered
in `src/extensions/format.rs`, but `by_name` applies them to any value, so a
table is written only by naming a renderer:
`xs | format(as = Markdown, renderer = "pipe")`. Without one, a list of
records in `Markdown` is a bullet list. Column order cannot be stated: an
anonymous record literal's fields are written sorted by name, in every
table renderer.

HQL-0024 part 7 has since redefined a format as a `Format` value with
`Encoder<T>` impls, and a renderer as one of a format's encoders. HQL-0017's
shape side, renderer table and configuration were not restated against that
model.

## Scope

- `Table` declared, built as `Table(xs, columns = …)`: one row per record,
  `columns` naming the fields in order.
- A table written in each format by the format's encoder for `Table`:
  `Table(xs) | markdown` a pipe table, `| csv` rows, `| json` records,
  `| text` aligned columns, with no renderer named.
- The table renderers refused for a value that is not a `Table`.
- A review of rendering and presentation as a whole: shapes, formats,
  encoders, renderers, `renderer =`, the vault's `[format]` configuration
  and the presentation context, stated in one current model across
  HQL-0017, HQL-0024 part 7 and the presentation card.

## Done when

- `Table(xs, columns = [...])` writes its columns in the stated order in
  `Markdown`, `Csv`, `Json` and `Text`.
- The ADR status page builds its table with `Table`, not a renderer name.
- One document states the presentation model, and the others point to it.
