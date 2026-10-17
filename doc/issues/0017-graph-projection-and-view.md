# 0017 — `graph` is two operations under one name

Status: backlog
Decisions: [HQL-0008](../proposals/HQL-0008/README.md), its section
"`graph` is a presentation"
Related: step 6 of [0013](0013-typed-pipeline-operations.md)

`graph` has two jobs under one name, as the
[presentation](../wiki/hql/presentation.hmd) card says in its section
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

## Work

1. **The split.** HQL-0008 decides that `graph` becomes the presentation and
   the projection gets a distinct name. Its open question on that name
   (`to_graph`, `as_graph`, `project_graph`) is settled here or in HQL-0008.
   This is step 6 of 0013; do it there if 0013 gets to it first.
2. **The examples.** Done: the redundant `| graph` after `expand` is gone from
   `README.md`, `doc/index.md` and `doc/public/retrieval.md`, and each
   program prints what its page shows. No test pins `| graph | table`. The
   projection's own uses in `tests/vault.rs` and
   `tests/extension_contracts.rs` move to its new name with step 1.

## Done when

- No example or test pipes a `graph` result into `table` or another
  presentation.
- The projection and the view have different names, and `hql builtins` lists
  the view under presentation.
- The presentation card no longer lists "whether `graph` is the projection or
  the view" as open.
