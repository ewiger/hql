# TODO: a list the vault writes

Start at [the board](wiki/todo.hmd). Four task cards carry a `status` and a
`priority`, and three architecture-decision cards a `status` and a `date`. The
board's `hql#eval` cells select from them, so the lists on it are queries
rather than text somebody keeps up to date. Only `wiki/` is the vault.

From the repository root:

```sh
cargo build --locked
target/debug/hql --vault examples/todo/wiki render examples/todo/wiki/todo.hmd
target/debug/hql --vault examples/todo/wiki run examples/todo/queries/todo-list.hql
target/debug/hql --vault examples/todo/wiki run examples/todo/queries/adr-overview.hql
target/debug/hql run examples/todo/queries/invalid-string-plus.hql
```

| Query | What it shows | Status | Result |
| --- | --- | --- | --- |
| [todo list](queries/todo-list.hql) | Open tasks, ordered by an `Int` read out of `priority` | runs | Fix broken backlinks, Write HQL documentation, Publish HyperMarkdown 0.4.2 announcement |
| [ADR overview](queries/adr-overview.hql) | Decisions ordered by a `String` read out of `date`, projected to records | runs | Event execution model, Reduce and merge, Stream IO model |
| [string plus](queries/invalid-string-plus.hql) | `+` on two strings | type error |  |

Rendered, the board reads:

```markdown
## Still to do

- Fix broken backlinks
- Write HQL documentation
- Publish HyperMarkdown 0.4.2 announcement

## Architecture decisions

- {date: "2026-10-10", status: "accepted", title: "Event execution model"}
- {date: "2026-10-18", status: "accepted", title: "Reduce and merge"}
- {date: "2026-10-19", status: "proposed", title: "Stream IO model"}
```

A metadata leaf is an `Option<Data>`, and `Data` is not
[`Orderable`](../../doc/wiki/hql/types/orderable.hmd), so a leaf cannot be a
sort key as it stands: `sort(by = c => c.metadata.priority)` is refused. Each
query reads the leaf with a typed `match` pattern, `some(Int p)` or
`some(String d)`, which yields a value that is. A list of strings renders as a
Markdown bullet list and a list of records as one bullet per record; a
`- [ ]` task line or a table is not a rendering HQL produces, and `+` does not
join strings, so a checkbox prefix cannot be built in the query either.

Reading the keys through a declared partial record instead, so that
`sort(by = c => c.metadata.priority)` checks without a `match`, is open as
[issue 0027](../../doc/issues/0027-typed-metadata-sort-keys.md) on the
`todo` column of the board.
