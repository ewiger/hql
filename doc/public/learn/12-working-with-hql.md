# 12. Working with `hql`

The language is half of what you use; the other half is the command and what
it says back. This chapter is about the tools: the commands, the reports, and
the messages `hql` writes when a program is wrong.

## The commands

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate a program given as an argument |
| `hql run <file>` | check and evaluate a program in a file; `-` reads standard input |
| `hql check <file>` | write the program's type, and the signature of every alias, without evaluating |
| `hql repl` | read, evaluate and print, keeping bindings between lines |
| `hql render <file>` | build a document from its cells ([chapter 18](18-documents.md)) |
| `hql builtins` | list every step, by the provider that implements it, with its signature |
| `hql config` | the settings in force, and where each came from |
| `hql doctor` | check the semantic-search chain and name the first missing part |

`--vault <dir>` names the vault, `--to <format>` the output format, and
`--envelope json` wraps the answer for another program.

## The REPL

`hql repl` keeps its bindings from one input to the next. The arrow keys edit a
line and recall earlier inputs, from a history kept between sessions. An input
goes on over several lines while a bracket is open, or while the next line
opens with `|`, so every program in this book can be pasted in as it is
written. `:type` asks for the type of an expression, or, given a function's
name, prints the standard library's declarations of it, one per overload.
`:quit` leaves, as do `quit`, `exit` and Ctrl-D:

```text
$ hql repl
hql 0.5.1. `:help` for commands, `:quit` to leave.
hql> shelf = [412, 280]
()
hql> shelf | sum
692
hql> :type shelf
List<Int>
hql> :type range
fn range(start: Int, end: Int, step: Int = 1) : List<Int>    // std/seq
hql> :quit
```

## What a step takes

`hql builtins` lists every step with its signature, one line per form:

```text
$ hql builtins
core 0.5.0 (always available)
  size
    Collection<T> | size<T>() -> Int
    Count all element occurrences.
  contains
    Collection<T> | contains<T>(value: T (positional)) -> Bool
    Whether a value occurs in the collection.
  …
```

A function of a module that the program has not imported is listed too, so
the import it needs can be looked up. `import lexical` makes a module visible;
`lexical.lexical(…)` names a function with its module, when two visible modules
declare one name and the input does not tell them apart. The modules of
`lib/std/prelude.hql`, `string`, `seq`, `math`, `option`, `result`, `graph`,
`format` and `io` among them, are visible in every program unless the vault's
`hql.toml` says `prelude = false`.

## Reports

A run produces a value **and** a queue of reports: warnings that change
nothing, errors that make a result wrong, failures that make one impossible.
Every report names where it points, by line and column.

By default `hql` stops at the first error. `--report collect` reports every
error it can find:

```text
$ hql --report collect eval 'a = nosuchthing
b = alsomissing'
name error: `nosuchthing` is not bound
 --> <expression>:1:5
  |
1 | a = nosuchthing
  |     ^^^^^^^^^^^

name error: `alsomissing` is not bound
 --> <expression>:2:5
  |
2 | b = alsomissing
  |     ^^^^^^^^^^^

2 reports: 0 failure, 2 error, 0 warning
```

A misspelled step suggests the nearest one, and a retired name says what
replaced it:

```text
$ hql eval '[1, 2] | mapp(n => n)'
name error: `mapp` is not a pipeline step; did you mean `map`?
```

A warning never changes the exit status. Names have one casing per kind —
`snake_case` for bindings, functions and fields, `PascalCase` for types — and
a name off its kind's convention is a warning where it is written:

```text
$ hql eval 'bookCount = 7
bookCount'
7
type warning: `bookCount` is a binding, written in snake_case
 --> <expression>:1:1
  |
1 | bookCount = 7
  | ^^^^^^^^^
```

`--envelope json` carries every report, whatever the level, as data. A
program that wants to report on itself, apart from its result, writes to a
logger: the [logging page](09-io/logging.md) of chapter 9.

## Settings

The report mode and level come from the command line, then the vault's
`hql.toml`, then the environment (`$HQL_REPORT`, `$HQL_REPORT_LEVEL`), then the
defaults. `hql config` says which applies, and why:

```text
$ hql config
report mode: strict (from the default)
report level: warning (from the default)
backend: localhost (from the default)
vault: none
```

`backend` is the execution backend of [chapter 13](13-pipelines.md), and
`localhost` is the only one. Run inside a vault, or with `--vault`, the last
line names it and counts its cards.

## What is next

Every chapter so far was about a step. [Chapter 13](13-pipelines.md) is about
the pipeline the steps form: what `|` does, when a program is checked, where
it runs, and the systems a pipeline will connect.
