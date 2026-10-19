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
| `hql render <file>` | build a document from its cells ([chapter 11](11-documents.md)) |
| `hql builtins` | list every step, by the extension that provides it, with its signature |
| `hql config` | the settings in force, and where each came from |
| `hql doctor` | check the semantic-search chain and name the first missing part |

`--vault <dir>` names the vault, `--to <format>` the output format, and
`--envelope json` wraps the answer for another program.

## The REPL

`hql repl` keeps its bindings from one input to the next. The arrow keys edit a
line and recall earlier inputs, from a history kept between sessions. An input
goes on over several lines while a bracket is open, or while the next line
opens with `|`, so every program in this book can be pasted in as it is
written. `:type` asks for a type, and `:quit` leaves:

```text
$ hql repl
hql 0.4.0. `:help` for commands, `:quit` to leave.
hql> shelf = [412, 280]
()
hql> shelf | sum
692
hql> :type shelf
List<Int>
hql> :quit
```

## What a step takes

`hql builtins` lists every step with its signature, one line per form:

```text
$ hql builtins
core 0.4.0 (always available)
  size
    Collection<T> | size<T>() -> Int
    Count all element occurrences.
  contains
    Collection<T> | contains<T>(value: T (positional)) -> Bool
    Whether a value occurs in the collection.
  …
```

A step from an extension that the program has not imported is listed too, so
the import it needs can be looked up. `import lexical` brings in a step;
`lexical.lexical(…)` names one with its extension, when two extensions provide
the same name. `graph` and `format` are imported for every program unless the
vault's `hql.toml` says `prelude = false`.

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

## Logging from a program

A program reports on itself with a logger. `import logging` provides one; it
writes structured events to an endpoint of its choosing, apart from the result:

```text
$ hql --vault examples/tutorial 'import logging
log = Logger(stderr, Text, name = "books")
books = cards | filter(c => c.metadata.category == "book")
log | info("selected", {category: "book"})
books | size'
INFO books selected {category: "book"}
7
```

`--log-level debug` lowers every logger's threshold, and `--log-level warn`
raises it, without changing the program.

`--envelope json` carries every report, whatever the level, as data.

## Settings

The report mode and level come from the command line, then the vault's
`hql.toml`, then the environment (`$HQL_REPORT`, `$HQL_REPORT_LEVEL`), then the
defaults. `hql config` says which applies, and why:

```text
$ hql --vault examples/tutorial config
report mode: strict (from the default)
report level: warning (from the default)
vault: examples/tutorial (14 cards)
```

## What is next

So far every program read a vault or its own values.
[Chapter 13](13-reading-and-writing.md) reads files and standard input, and
writes files and standard output.
