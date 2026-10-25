# 0017 — `graph` is two operations under one name

Status: done
Decisions: [HQL-0008](../../proposals/HQL-0008/README.md), its section
"`graph` is a presentation"
Related: step 6 of [0013](../0013-typed-pipeline-operations.md)

`graph` has two jobs under one name, as the
[presentation](../../wiki/hql/presentation.hmd) card says in its section
"`graph` is doing two jobs":

- **a projection** — turn cards or knowledge into a `Graph` value;
- **a view** — render something *as* a graph, the way `table` renders it as
  rows.

The implementation has only the projection. So the front-page example reads as
"render as a graph, then render as a table", which makes no sense:

```hql
import semantic

cards
| semantic("night hunting birds")
| take(3)
| expand(depth = 1)
| graph
| table
```

`expand` already returns a `Graph`, so the `graph` step does nothing. With and
without it the type is `Graph` and the `table` output is identical, as checked
on the birds vault.

## Resolution

The projection is the `Graph` constructor, written `Graph(xs)` or `xs | Graph`;
the view is formatting, `| d2`, `| mermaid` or `| format(Markdown)`, through
the renderers of [HQL-0017](../../proposals/HQL-0017/README.md). `cards | graph`
is refused with a diagnostic naming `Graph`, `table` is retired, and `expand`
refuses depth `0`. No example or test pipes a graph into a presenter.
