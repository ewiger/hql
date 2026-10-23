# Dataflow: fork and shared producers

The cases follow [HQL-0037](../../doc/proposals/HQL-0037/README.md): a
program is a logical dataflow graph, and `fork` is its broadcast fan-out.
`xs | fork { a = …, b = … }` gives every branch the same input, and its
value is the record of the branches' results, `{a: A, b: B}`, each field
typed by its own branch pipeline.

| Case | What it shows | Result |
| --- | --- | --- |
| [record of branches](queries/fork-record.hql) | three branches of different types over one list | `{small: [1, 2], large: [30, 40, 50], count: 5}` |
| [destructuring](queries/fork-destructure.hql) | `{ early, late } = … \| fork { … }` and `parts.early` | the names before and after 1950 |
| [over a stream](queries/fork-stream.hql) | each branch of a `Stream` is a stream in the upstream's order | `[[2, 4, 6], [1, 3, 5]]` |
| [branch named twice](queries/invalid-fork-branch-twice.hql) | a branch name is written once | a syntax error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/dataflow/queries/fork-record.hql
```
