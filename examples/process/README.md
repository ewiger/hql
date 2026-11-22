# Process: child processes

The cases follow [HQL-0064](../../doc/proposals/HQL-0064/index.hmd), the
companion note [child processes](../../doc/proposals/HQL-0064/parts/02-child-processes.hmd).
`command(program, args)` from `io.process` is a value that starts nothing.
`source` and `tap` start it and read its standard output, `target` writes
its standard input, and `run` waits for it and gives a `Result`: `ok` with
what it wrote when it exits `0`, `err(CommandFailure)` with a `reason`
otherwise. The cases name Unix programs, `printf`, `wc`, `sh`, `sleep` and
`yes`, and run on Unix.

| Case | What it shows | Result |
| --- | --- | --- |
| [source](queries/source-lines.hql) | every line of a child's standard output | `["wren", "kite"]` |
| [run](queries/run-ok.hql) | a child given input, and what it wrote | `2` |
| [exited](queries/run-exited.hql) | a non-zero status, as `err` | `exited 3` |
| [not found](queries/run-not-found.hql) | a program not in `PATH` | `not-found` |
| [timed out](queries/run-timed-out.hql) | a child terminated at `timeout_ms` | `timed-out` |
| [tap](queries/tap-take.hql) | a child that never ends, read until `take` ends it | `3` |
| [target](queries/target-step.hql) | one document written to a child's standard input | `()` |
| [not imported](queries/not-imported.hql) | `command` without `io.process` | a check error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/process/queries/run-exited.hql
```
