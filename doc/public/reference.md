# Reference

These pages describe what the binary implements. Where a model or card
describes language the binary does not run yet, its status line says so.

| Page | Covers |
| --- | --- |
| [Expressions](../models/behavior/expressions.md) | The grammar the parser accepts, and how expressions are checked |
| [Command line](../models/behavior/cli.md) | `hql`'s commands, options, exit codes and configuration |
| [Reporting](../models/behavior/reporting.md) | Warnings, errors and failures, and the `strict` and `collect` modes |
| [Queries](../models/behavior/queries.md) | How a query is sourced and traversed |
| [Program values](../models/behavior/program-values.md) | What a program returns, and what the host does with it |
| [Design status](../wiki/hql/design-status.hmd) | Every design card, and the decisions still open |

## Commands at a glance

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql builtins` | list the steps |
| `hql config` | the reporting mode in force, and where it came from |

`-` reads standard input wherever a file is taken. Success is exit `0`, a
language or I/O failure `1`, a usage error `2`.

## The standard library

HQL's types are declared in HQL itself, under
[`std/`](https://github.com/ewiger/hql/tree/main/std): `core`, `collections`,
`doc`, `graph`, `knowledge` and `retrieval`. The checker reads those
declarations rather than assuming them.
