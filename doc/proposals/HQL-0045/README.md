# HQL-0045: Structured reports: every diagnostic an entry, rendered at a verbosity, written to a destination

**Status**: drafted
**Created**: 2026-10-25
**Source**: [the report](../../wiki/hql/error-handling.hmd#the-report) in the
error-handling card, [HQL-0033](../HQL-0033/README.md) section 33 (reporting
is separate from handling), [HQL-0044](../HQL-0044/README.md) (the phases),
`src/reporting.rs`, `src/render.rs` and `emit` in `src/cli.rs`

## Companion notes

- [Errors as values](../../wiki/hql/error-handling.hmd) — the card whose
  designed entry fields this record specifies and schedules.
- [Compilation](../../wiki/hql/compilation.hmd) — the phases an entry names.

## Abstract

Every diagnostic a run produces, from loading the vault to the last step a
backend executes, is an **entry** of one structured type, `Report`, queued in
the run's report in the order it was found. An entry names its phase, its
stage, a stable `code`, a one-sentence `message`, and the context that applies
to it: secondary spans, help, expected and found, the element and bindings an
execution failure was working on, a trace of pipeline steps and function
frames, the data's provenance, and for a contract violation the `Invalid`. A
host renders the queue at one of three **verbosities**, `brief`, `normal` and
`full`, and writes it to a configured **destination** as text or as JSON
lines; the data form always carries every field. A handled failure is a value and
never an entry. The message never carries what a field
carries: no byte offset, value or suggestion is formatted into the sentence.

## Motivation

The card designs a structured entry and the implementation builds four of its
fields. What a reader sees today is one sentence and a caret.

| Gap | Today |
| --- | --- |
| No code | `code` is set only on `hql check`'s signature lines; no error has one |
| No execution context | `division by zero` does not say which element, which value or which step |
| `trace`, `found`, `shown` never filled | the fields exist on `Report` and the renderer prints them, but nothing sets them |
| Phase mislabelled | a data failure raised while executing reports stage `type`, the label of a check |
| No provenance | a failure over a card does not name the card |
| A secondary place flattened into the message | a duplicate `impl` says "at byte 71" |
| A suggestion inside the message | "did you mean `map`?" is part of the sentence, not a field |
| Loader warnings dropped | a document the vault skips is reported nowhere; `vault::warnings` has no caller |
| A handled failure printed flat | `validate(T) \| errs()` prints every `Invalid` as one line of constructor syntax on stdout |
| REPL lines miscounted | an entry in `hql repl` counts the replayed bindings, so the second line typed is reported as line 3 |
| One verbosity, one destination | text on stderr, or the whole run as one `--envelope json` document |

## Non-goals

- Severity or fail level per phase. An entry names its phase, so a later
  record MAY configure `fail` per phase; this one does not.
- A program reading or writing the report. The report is the host's: no
  program intercepts an entry or adds one ([HQL-0033](../HQL-0033/README.md)).
  A handled failure goes where the program sends it, with an ordinary sink.
- The program's own logging. A `LogEvent` written through a `Logger` stays
  the program's, on the endpoint it names
  ([HQL-0021](../HQL-0021/README.md)).
- Numeric codes. A code is a name.

## Current reports

Every example below is the output of the current binary, stderr unless noted.

Syntax, `xs = [1, 2` followed by `xs`:

```text
syntax failure: expected `]` after collection elements, found `xs`
 --> syntax.hql:2:1
  |
2 | xs
  | ^^
```

Name, a misspelt step:

```text
name error: `mapp` is not a pipeline step; did you mean `map`?
 --> name.hql:2:6
  |
2 | xs | mapp(x => x + 1)
  |      ^^^^^^^^^^^^^^^^
```

Type, a field that may be missing added to a string:

```text
type error: addition requires two Int, two Float or two Decimal operands, or two values of a type implementing `Add`, not Option<Data>
 --> type.hql:2:18
  |
2 | books | map(b => b.pages + "p")
  |                  ^^^^^^^
```

Execution, `[4, 0, 2] | map(x => 10 / x)`; nothing says the second element
was `0`:

```text
evaluation failure: division by zero
 --> div.hql:1:22
  |
1 | [4, 0, 2] | map(x => 10 / x)
  |                      ^^^^^^
```

Execution over a vault, reported as a `type` error although `hql check`
accepts the program; no card is named:

```text
type error: `unit` needs a map here, not Data of shape int
 --> <expression>:1:63
  |
1 | cards | filter(c => c.metadata.category == "book") | map(c => c.metadata.pages.unit)
  |                                                               ^^^^^^^^^^^^^^^^^^^^^
```

A fail-fast contract read; the span is the whole pipeline and the element is
not named:

```text
evaluation failure: this does not read as User: field `age`: "forty" is not Int
 --> failfast.hql:6:1
  |
6 | [{name: "ada", age: 36}, {name: "bob", age: "forty"}, {name: "cy", age: -1}] | User
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The same records through `validate(User) | errs()`, on **stdout**, as one
line:

```text
[Invalid(contract = "User", value = {age: "forty", name: "bob"}, violations = [Violation(contract = "User", path = ["age"], kind = "type", rule = "User.age.type", text = "\"forty\" is not Int", expected = "Int", actual = "forty", severity = "error", message = "field `age`: \"forty\" is not Int")]), Invalid(contract = "User", value = {age: -1, name: "cy"}, violations = [Violation(contract = "User", path = [], kind = "rule", rule = "User.1", text = "require age >= 0", expected = none, actual = none, severity = "error", message = "`require age >= 0` does not hold")])]
```

The data form, `--envelope json`, holds no more than the text:

```json
{
  "message": "division by zero",
  "severity": "failure",
  "span": { "end": 27, "start": 21 },
  "stage": "evaluation"
}
```

## Specification

### 1. The entry

An entry is a record of one type, `Report`, written as data in the shape
below. A field that does not apply
is absent, never empty.

```text
severity   debug | info | warning | error | failure
phase      load | parse | check | execute | render
stage      syntax | name | type | evaluation | program | load | render
code       kebab-case name, stable across releases
message    one sentence, for a person
origin     the program's origin: a path, <expression>, <repl:N>, a document cell
span       { start, end }: zero-based, half-open UTF-8 byte range in origin
labels     [ { origin?, span, text } ]: secondary places, each with a note
help       [ String ]: what to write instead, a nearest name
expected   a type, or the shape a step needed
found      the value or type that was there, as data
element    { step, position, of, value }: the element a step was working on
bindings   { name: value }: the parameters and bindings in scope at the span
trace      [ { kind: step | call, name, origin, span, position?, element? } ],
           outermost first
source     { path, line?, key? }: where the data came from
invalid    the Invalid that validate gives for the same value
```

- `phase` MUST be the [HQL-0044](../HQL-0044/README.md) phase that found the
  entry. A failure found while executing is `phase: execute` and
  `stage: evaluation`, whatever the failure is about; `type` and `name` are
  stages of `check` only.
- `code` MUST be set on every entry the language produces. A code names the
  kind of problem, not the message: `division-by-zero`, `numeric-overflow`,
  `unknown-step`, `unknown-name`, `type-mismatch`, `non-exhaustive-match`,
  `missing-implementation`, `duplicate-implementation`, `unsatisfied-bound`,
  `shape-mismatch`, `contract-violation`, `unresolved-link`,
  `skipped-document`. The list of codes is published on the reference site.
- `message` MUST NOT format a field into itself. A secondary place is a
  `label`, a suggestion is `help`, a value is `found` or `element`.
- `trace` holds function frames as well as pipeline steps: a function with a
  body, a recursive one included, is a `call` frame at the call's span.
- An entry's values are the values themselves, as data. Shortening a value
  for a person is the renderer's, at a verbosity, never the entry's.

### 2. One queue for every phase

The report is the run's one queue of entries, oldest first.

- Every phase MUST write into it: the loader's warnings, the parser's
  failure, every check entry, every execution entry, and a host's render
  entries. A host MUST NOT write a diagnostic of its own around the queue.
- `strict` and `collect` keep their meaning ([the card](../../wiki/hql/error-handling.hmd#strict-and-collect)):
  `collect` lists every check entry, and once elementwise execution is built,
  every failing element.
- An entry is written once. Rendering, filtering by level and choosing a
  destination are views of the queue; none removes an entry from it.

### 3. Verbosity

A host renders each entry at one of three verbosities.

| Verbosity | Shows |
| --- | --- |
| `brief` | one line: `origin:line:column: stage severity[code]: message`, with the element in brackets where there is one |
| `normal` | the default: the heading, the source line and caret, every label, `help`, `expected` and `found`, the element, and the trace's steps |
| `full` | `normal`, and the phase and backend, the bindings, every trace frame with its place, the data's source, and `invalid` with each violation |

- At `brief` and `normal` a value is shortened to one line of at most 120
  characters; at `full` it is written whole, one field per line.
- The data form, JSON, MUST carry every field at every verbosity.
- Verbosity and `level` are separate: `level` decides which severities are
  shown, verbosity how much of each.

### 4. Destination and format

The report is written to an endpoint, in a format, as a result is.

| Setting | Values | Default |
| --- | --- | --- |
| `to` | `stderr`, `stdout`, a path | `stderr` |
| `format` | `text`, `json-lines` | `text` |

- `json-lines` writes each entry as one JSON document, in queue order, as it
  is found; it is for a tool reading a running program.
- When `to` is `stdout`, the result is written first and the report after
  it, so a value and its report are never interleaved.
- `--envelope json` keeps its shape: one document holding the type, the
  value and the whole queue under `reports`, each entry with every field.

### 5. Configuration

The settings take the precedence the report's others take: the query, the
vault's `hql.toml`, the environment, the default.

| Setting | Flag | `[report]` key | Environment |
| --- | --- | --- | --- |
| verbosity | `--report-verbosity brief\|normal\|full` | `verbosity` | `HQL_REPORT_VERBOSITY` |
| destination | `--report-to stderr\|stdout\|<path>` | `to` | `HQL_REPORT_TO` |
| format | `--report-format text\|json-lines` | `format` | `HQL_REPORT_FORMAT` |

```toml
# hql.toml at the vault root
[report]
mode      = "collect"
level     = "warning"
verbosity = "full"
to        = ".hql/report.jsonl"
format    = "json-lines"
```

`hql config` prints each setting in force and the layer that supplied it.

### 6. An `Invalid` printed as a result

An `Invalid` or a `Violation` written as a result in `Text` is presented in
the report's layout at the host's verbosity, one block per value, rather than
as one line of constructor syntax. `--to hql` writes the literal, and `Json`
writes the data, as for any value.

## Per phase

What each phase reports, how `strict` and `collect` treat it, and its entry at
the three verbosities. Each example is the current case above, reported to
this specification.

### Load

The loader reads the vault and its configuration before the program is
parsed. A document it cannot read is skipped and is a `warning`, `phase:
load`, `code: skipped-document`, with `source` naming the file; the run goes
on without it under either mode. A configuration that cannot be read is a
`failure`.

```text
brief   wiki/bad.md:2:8: load warning[skipped-document]: the header is not valid YAML; the document is skipped
```

```text
load warning[skipped-document]: the header is not valid YAML; the document is skipped
 --> wiki/bad.md:2:8
  |
2 | title: [unclosed
  |        ^ this sequence is never closed
  = help: close it with `]`, or quote the title
```

### Parse

The parser stops at the first failure, under both modes: nothing after an
unreadable source can be examined. One entry, `severity: failure`.

```text
brief   syntax.hql:2:1: syntax failure[unclosed-collection]: expected `]` after collection elements, found `xs`
```

```text
syntax failure[unclosed-collection]: expected `]` after collection elements, found `xs`
 --> syntax.hql:2:1
  |
1 | xs = [1, 2
  |      - the list opens here
2 | xs
  | ^^ `xs` begins a new statement
  = help: close the list with `]` at the end of line 1
```

### Check

The checker reports `name` and `type` entries, `severity: error`, and its
warnings. `strict` stops at the first; `collect` checks every statement, a
failed one binding its name so the next is reported on its own fault. No part
of a program with a check error executes.

```text
brief   name.hql:2:6: name error[unknown-step]: `mapp` is not a pipeline step
```

```text
name error[unknown-step]: `mapp` is not a pipeline step
 --> name.hql:2:6
  |
2 | xs | mapp(x => x + 1)
  |      ^^^^ no step is named `mapp`
  = help: `map`, the nearest step
```

A secondary place is a label, not a byte offset in the sentence:

```text
type error[duplicate-implementation]: `Named` is already implemented for Dog in this program
  --> traits.hql:11:1
   |
 7 | impl Named for Dog {
   | ------------------ the first implementation
...
11 | impl Named for Dog {
   | ^^^^^^^^^^^^^^^^^^ a second
   = help: a trait has one implementation per type; remove one
```

The type example at `full`, with `expected` and `found`:

```text
type error[type-mismatch]: addition needs two operands of one type implementing `Add`
 --> type.hql:2:18
  |
2 | books | map(b => b.pages + "p")
  |                  ^^^^^^^   --- String
  |                  |
  |                  Option<Data>
  = expected: two Int, two Float, two Decimal, or two of a type implementing `Add`
  = found:    Option<Data> + String
  = help:     `b.pages` may be missing; take it apart with `match`, or give a default
  = phase:    check
```

### Execute

An execution entry depends on the values: `phase: execute`, `stage:
evaluation`, `severity: failure` for an arithmetic or I/O failure and
`error` for data that does not read. `strict` ends at the first; `collect`
continues elementwise once that is built. These are the entries that need the
most context, and today carry the least.

```text
brief   div.hql:1:22: evaluation failure[division-by-zero]: division by zero [map, element 2: 0]
```

```text
evaluation failure[division-by-zero]: division by zero
 --> div.hql:1:22
  |
1 | [4, 0, 2] | map(x => 10 / x)
  |                      ^^^^^^ the divisor is 0
  = element: 2 of 3 in map: 0
  = trace:   map (step 1), element 2
```

```text
evaluation failure[division-by-zero]: division by zero
 --> div.hql:1:22
  |
1 | [4, 0, 2] | map(x => 10 / x)
  |                      ^^^^^^ the divisor is 0
  = element:  2 of 3 in map: 0
  = bindings: x = 0
  = trace:
      1. map, step 1       div.hql:1:13   element 2 of 3
      2. 10 / x            div.hql:1:22
  = phase:    execute, backend localhost
```

Over a vault, the entry names the card and the key in its header, and is an
execution entry, not a `type` one:

```text
evaluation error[shape-mismatch]: `unit` needs a map, and this `pages` is an Int
 --> <expression>:1:63
  |
1 | cards | filter(c => c.metadata.category == "book") | map(c => c.metadata.pages.unit)
  |                                                               ^^^^^^^^^^^^^^^^^^^^^
  = expected: Map<String, Data>
  = found:    112
  = source:   books/a-room-of-ones-own.hmd:8, metadata.pages
  = trace:    filter (step 1) → map (step 2), element 1 of 7: a-room-of-ones-own
```

A function frame in the trace, for a failure inside a body:

```text
evaluation failure[numeric-overflow]: `x + n` leaves the Int range
 --> overflow.hql:2:19
  |
2 | [1, 2] | map(x => x + n)
  |                   ^^^^^ 1 + 9223372036854775807
  = element:  1 of 2 in map: 1
  = bindings: x = 1, n = 9223372036854775807
  = trace:
      1. map, step 1       overflow.hql:2:10   element 1 of 2
      2. x + n             overflow.hql:2:19
```

A fail-fast contract read names the element and spans the read, with each
violation; `invalid` is what `validate` gives:

```text
evaluation error[contract-violation]: element 2 does not read as User
 --> failfast.hql:6:80
  |
6 | [{name: "ada", age: 36}, {name: "bob", age: "forty"}, {name: "cy", age: -1}] | User
  |                                                                                ^^^^
  = element:   2 of 3: {name: "bob", age: "forty"}
  = violation: age, type (User.age.type): "forty" is not Int
  = help:      read it with `validate(User)` to keep the failure as a value
```

### A handled failure

A `Result` is a value and is not in the report. `validate(User) | errs()` as
a result, presented by section 6:

```text
Invalid User: {name: "bob", age: "forty"}
  = violation: age, type (User.age.type): "forty" is not Int
Invalid User: {name: "cy", age: -1}
  = violation: rule (User.1): `require age >= 0` does not hold
```

### Render

`hql render` reports each cell's entries with the cell as `origin`, a label
at the fence, and `phase: render` for a fault in the document rather than in
a cell: a duplicate cell id, a cell reading one that does not exist, a write
a document may not make.

### Data form

The division example as one JSON line, every field present at every
verbosity:

```json
{"severity":"failure","phase":"execute","stage":"evaluation","code":"division-by-zero","message":"division by zero","origin":"div.hql","span":{"start":21,"end":27},"labels":[{"span":{"start":26,"end":27},"text":"the divisor is 0"}],"element":{"step":"map","position":2,"of":3,"value":0},"bindings":{"x":0},"trace":[{"kind":"step","name":"map","origin":"div.hql","span":{"start":12,"end":28},"position":1,"element":2},{"kind":"call","name":"10 / x","origin":"div.hql","span":{"start":21,"end":27}}]}
```

## Backwards Compatibility

- Every entry keeps `severity`, `stage`, `message` and `span`; the fields
  added are absent where they do not apply, so a reader of the JSON that
  ignores unknown keys is unaffected.
- A message loses what moves into a field: "did you mean `map`?" moves to
  `help`, "at byte 71" to a label. A test or tool matching a whole message
  changes with it.
- A data failure raised while executing changes from `stage: type` to `stage:
  evaluation`, and its heading from `type error` to `evaluation error`.
- The default text gains lines (`= element:`, `= help:`). `--report-verbosity
  brief` gives one line per entry.
- A run over a vault with a skipped document gains a warning it did not
  report before. A warning never changes the exit status.

## Reference Implementation

Each work unit lands with its tests and amends the card's "designed"
sentences for what it builds.

1. `Report` gains `phase`, `origin`, `labels`, `help`, `expected`, `element`,
   `bindings` and `source`; `trace` gains `call` frames. `Diagnostic` carries
   a `code`, labels and help, and its constructors take them
   (`src/diagnostics.rs`, `src/reporting.rs`).
2. Codes for every diagnostic the parser and checker raise; suggestions and
   secondary places move out of messages (`src/parser.rs`,
   `src/checker.rs`).
3. Execution context: the evaluator records the step, element position and
   value, bindings and frames when it raises; a data failure is raised as
   `stage: evaluation` (`src/evaluation.rs`).
4. Provenance: a card's path and header line reach an entry about a value
   read from it (`src/document.rs`, `src/knowledge.rs`).
5. The loader's warnings enter the queue, `code: skipped-document`
   (`src/vault.rs`, `run_on` in `src/lib.rs`).
6. The renderer at three verbosities, labels and multi-line `full`
   (`src/render.rs`); `brief` as one line.
7. Destination and format, `json-lines`, and the configuration layers
   (`src/cli.rs`, `src/reporting.rs`, `hql config`).
8. An `Invalid` presented in the report layout (`src/extensions/format.rs`).
9. `hql repl` origins `<repl:N>`, the line counted in what was typed.

## Test Plan

`tests/reporting.rs` and `tests/cli.rs` MUST include:

- for each phase, one entry with its `phase`, `stage` and `code`, compared
  in the JSON form;
- each example of the "Per phase" section rendered at `brief`, `normal` and
  `full` and compared to the text above;
- the division example's `element` and `bindings`, and the overflow
  example's frames;
- a vault with a malformed header reporting `skipped-document` and keeping
  the rest;
- `--report-to stdout` writing the result before the report;
- `hql repl` reporting the second line typed as `<repl:2>:1`.

```bash
cargo test --locked --test reporting --test cli
```

## Changelog

- 2026-10-25: drafted
