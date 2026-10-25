# HQL-0045: The error model: failures as data, diagnostics as events, and the run-scoped diagnostic channel

**Status**: drafted
**Created**: 2026-10-25
**Source**: [HQL-0033](../HQL-0033/README.md) (errors as values),
[HQL-0032 section 28](../HQL-0032/README.md#28-validation-and-diagnostic-transport-are-separate)
(validation and diagnostic transport are separate), [HQL-0021 section 10](../HQL-0021/README.md#10-channels)
(channels), [HQL-0044](../HQL-0044/README.md) (the phases), the
[error-handling card](../../wiki/hql/error-handling.hmd), the
[reporting model](../../models/behavior/reporting.md), `src/diagnostics.rs`,
`src/reporting.rs`, `src/warnings.rs`, `src/io.rs` and `emit` in `src/cli.rs`

## Companion notes

- [Errors as values](../../wiki/hql/error-handling.hmd) — the card this record
  corrects: its `Report` entry becomes the `Diagnostic` of section 3, and its
  report becomes a presentation.
- [Compilation](../../wiki/hql/compilation.hmd) — the phases a diagnostic names.
- [Channels](../HQL-0021/README.md#10-channels) — the transport a diagnostic
  travels on.

## Abstract

A failure in HQL is one of two things, and nothing else. A failure the program
elected to keep inside its dataflow is **data**: an `err(e)` of a
`Result<T, E>`, an `Invalid` with its `Violation`s, an ordinary value that
`map_err`, `errs()`, `fork` and `sink` handle as they handle any value. Every
other failure is described by a **diagnostic**: a structured event, produced by
the loader, the parser, the checker, the planner, the evaluator or the
renderer, that names the **phase** it occurred in, the **kind** of problem,
its **severity** and its **disposition** for the work that produced it. These
four axes are independent; none is inferred from another. The runtime owns a
**diagnostic channel**, `Channel<Diagnostic>`, scoped to the run: it is the
FIFO of [HQL-0021](../HQL-0021/README.md), with the same endpoint machinery,
and it exists before evaluation begins, so that every phase writes into it.
Diagnostics reach a person or a tool through ordinary transport: a format
renders them and an endpoint delivers them, `stderr` as text by default, and a
host's configuration changes that wiring without a reporting architecture of
its own. A **report** is a presentation of the channel's contents, not a
runtime entity; `Report`, `Reports` and `Warning` are removed. A fatal
diagnostic ends its producer only after the channel's consumers are drained.
Whether a producer continues after a diagnostic is the producer's, by phase,
and `collect` asks for continuation where the producer can give it. A program
neither raises a diagnostic nor reads the channel it runs in.

## Motivation

The knowledge base holds two models of where a failure goes, both marked in
force. [HQL-0032 section 28](../HQL-0032/README.md#28-validation-and-diagnostic-transport-are-separate)
separates validation from "diagnostic transport" and lists memory channels
among the destinations, as a normative rule that HQL-0033 never superseded.
HQL-0033 and the card say a failure is a value or a report entry, that "no
program intercepts a report entry", and never mention a channel. Each is
right about something: the first about transport, the second about the
boundary between data and diagnostics. Neither says what a diagnostic is.

The implementation shows the cost. A `Diagnostic` carries a stage and a
message; `Report::of` then derives a severity from the stage, so a contract
violation found while executing is a `failure` and a data failure raised
during execution is stamped `type`, the label of a check. `Warning` is a
second type on a parallel path that meets `Diagnostic` only as `Report` rows.
`Report` has fields for a code, a trace and a found value that nothing fills.
A `Channel<LogEvent>` named `cell.logs` is seeded for every rendered cell and
read by nothing; `vault::warnings()` has no caller, so a document the loader
skips is reported nowhere. `collect` is consulted only while checking. A
program's `Logger` writes to any endpoint through `io`, while the language's
own diagnostics write to `stderr` through a renderer of their own, and the
two are interleaved on one stream with nothing to tell them apart.

The terms have drifted with the code. "Diagnostic" names a fail-fast stop in
HQL-0033, a `Violation` payload in HQL-0032, a transport in its section 28,
and the program's log events in the reporting model. Three severity ladders,
`debug..failure`, `Trace..Error` and `Violation.severity`, are stated with no
relation between them. The time dimension, that a type error exists before
the program could run and a division by zero only while it does, is a
`stage` word on the diagnostic and nowhere a rule.

This record corrects the model rather than extending it: one vocabulary, one
diagnostic type with its axes made explicit, one transport that already
exists, and the lifecycle that makes the transport usable.

## Goals

- One definition of each term: failure, diagnostic, channel, format, target,
  result.
- The four axes of a diagnostic as fields, each set by its producer.
- The diagnostic channel as the one transport, reusing HQL-0021's channels
  and endpoints, with the lifetime that lets every phase write into it.
- The lifecycle of a fatal diagnostic, so that a diagnostic written just
  before the run ends is delivered.
- Continuation decided per phase by the producer.
- HQL-0032 section 28 reconciled with HQL-0033: transport is separate from
  validation, and transport is this channel.

## Non-goals

- The rendered text of a diagnostic beyond the fields it carries. A
  verbosity, a layout and a palette are presentation and follow this record.
- A construct for a program to raise a diagnostic. There is none, and this
  record adds none.
- A program reading the channel of the run it is in; section 5 says why it
  cannot.
- The program's log. A `LogEvent` written through a `Logger` is the
  program's, on the endpoint the program names ([HQL-0021](../HQL-0021/README.md)).
- Lifecycle hooks, `on failure` events and dataset-level rules of HQL-0032;
  `Message<T>` and dead-letter policy of HQL-0033. They stand as designed.

## Specification

### 1. Terminology

Each term names one thing. A sentence that uses one for another is wrong.

| Term | Meaning |
| --- | --- |
| **failure** | an operation did not produce the value its signature promises |
| **data failure** | a failure the program keeps as a value: `err(e)`, `Invalid`, `Violation` |
| **diagnostic** | a structured event describing a failure, or a fact about one, that is not data; produced by a phase, never by a program |
| **phase** | when a diagnostic occurred: `load`, `parse`, `check`, `plan`, `execute`, `render` |
| **kind** | what the problem is, by name |
| **severity** | how serious the diagnostic is for the person reading it |
| **disposition** | what the diagnostic means for the work of the phase that produced it |
| **diagnostic channel** | the run-scoped `Channel<Diagnostic>` every phase writes into |
| **format** | a rendering of a diagnostic: `DiagnosticText`, `Json` |
| **target** | the endpoint a rendered diagnostic is delivered to |
| **report** | a presentation of `Seq<Diagnostic>`: the channel's contents rendered in a format, with a summary |
| **log event** | an event the program writes through a `Logger`; not a diagnostic |

In one line: errors are produced; diagnostics describe them; the channel
transports diagnostics; formats render them; targets deliver them; results
represent the failures a program elects to handle as values.

### 2. Two sides of a failure

A failure is data or it is a diagnostic. The call site decides, and nothing
moves a failure from one side to the other.

```text
                        failure occurs
                              │
               ┌──────────────┴──────────────┐
          failure is data              failure is a diagnostic
               │                             │
     Result / Invalid / Violation        Diagnostic
               │                             │
        an ordinary value             the diagnostic channel
               │                             │
      map / filter / fork / sink        format, then target
                                             │
                                 stderr / file / IDE / telemetry
```

- `x | validate(T)` MUST produce `Result<T, Invalid>`; the failure is data
  ([HQL-0033](../HQL-0033/README.md)).
- `x | T` MUST produce a diagnostic where `x` does not read as `T`: the
  operation requires `x` to satisfy `T`, and inability to do so is a failure
  of the execute phase. The diagnostic MAY carry the same `Violation`s as
  payload; they are its context, not its identity.
- A data failure MUST NOT become a diagnostic by any operation: `sink`,
  `target`, `errs()` and `match` over an `Invalid` write and read a value.
  A diagnostic MUST NOT become a value: no operation reads the channel of
  the run it is in (section 5).
- `err(e)`, `validate`, `require`, `warn`, `validate f with V` and `retry`
  are the program's constructs for a data failure. The language has no
  construct that produces a diagnostic; a diagnostic is what a phase
  produces when it cannot do what the program asked.
- A `Violation` with `severity = "warning"` is data, as the rest of the
  `Invalid` is; it is never a diagnostic.

### 3. The diagnostic

A `Diagnostic` is one record. The four axes are fields, each set by the
producer that knows it; a field that does not apply is absent.

```text
phase        load | parse | check | plan | execute | render
kind         a kebab-case name, stable across releases
severity     debug | info | warning | error | failure
disposition  warning | recoverable | fatal
message      one sentence for a person; nothing a field carries is formatted into it
origin       what was being read: a path, <expression>, <repl:N>, a document cell
span         { start, end }: zero-based, half-open UTF-8 byte range in origin
labels       [ { origin?, span, text } ]: secondary places, each with a note
help         [ String ]: what to write instead; a nearest name
expected     a type, or the shape a step needed
found        the value or type that was there, as data
element      { step, position, of, value }: the element a step was working on
bindings     { name: value }: the bindings in scope at the span
trace        [ { kind: step | call, name, origin, span, position?, element? } ], outermost first
source       { path, line?, key? }: where the data came from
violations   [ Violation ]: for a contract read, what validate would have given
```

- `phase` MUST be the [HQL-0044](../HQL-0044/README.md) phase of the
  producer. A failure found while executing is `execute`, whatever it is
  about. `plan` is the backend's refusal of a pipeline; `render` is a fault
  of a document rather than of a cell.
- `kind` MUST be set on every diagnostic. The set is published with the
  reference. Kinds of this record: `unclosed-collection`,
  `unexpected-token`, `unknown-name`, `unknown-step`, `unknown-field`,
  `type-mismatch`, `non-exhaustive-match`, `missing-implementation`,
  `duplicate-implementation`, `unsatisfied-bound`, `ambiguous-call`,
  `import-cycle`, `shape-mismatch`, `contract-violation`,
  `division-by-zero`, `numeric-overflow`, `io-failure`, `source-refused`,
  `unresolved-link`, `skipped-document`, `backend-refusal`,
  `duplicate-cell`, `always-true-test`, `shadowed-step`, `stale-index`.
- `severity` MUST be set by the producer and MUST NOT be derived from the
  phase or the disposition. `phase = execute, kind = contract-violation,
  severity = error` and `phase = check, kind = type-mismatch, severity =
  error` are both well-formed, and so is `phase = execute, kind =
  division-by-zero, severity = failure`.
- `disposition` says what the producer did next: `warning`, the work stands;
  `recoverable`, the unit of work the diagnostic is about, a statement, an
  element, a document, is lost and the producer went on; `fatal`, the
  producer stopped. It is the producer's statement of fact, not a policy.
- `message` MUST NOT carry what a field carries. A nearest name is `help`;
  a second place is a label; a value is `found` or `element`.
- `Diagnostic` is a type of the standard library, declared in HQL, so that
  a diagnostic read back from a file with `| Diagnostic` is the same record
  that was written. It is constructed by no program.

### 4. The time dimension

A run has two lifetimes, and they are not the same.

```text
run
├── diagnostic infrastructure      exists for the whole run
│     the diagnostic channel, its consumers, the exit status
├── load
├── parse
├── check
├── plan
├── execute                        HQL evaluation: values, streams, the program's channels
└── render
```

- The diagnostic channel MUST be opened before `load` and closed after the
  last phase. A `load`, `parse`, `check` or `plan` diagnostic is written
  into it by a phase that ran before any HQL evaluated.
- The program's channels, `Channel<T>(name)`, MUST live for `execute` and
  for one cell of `render`, as [HQL-0021](../HQL-0021/README.md#10-channels)
  has them. A program that failed `check` never opened one.
- A diagnostic is therefore never a variable of the program. `source(errors)
  | sink(stderr)` cannot report a type error of the program it is written
  in: that program did not start.

### 5. The diagnostic channel

The runtime owns one `Channel<Diagnostic>` per run. It is a channel of
[HQL-0021](../HQL-0021/README.md#10-channels) in every respect that record
states: typed, FIFO, in memory, written in order; the one difference is its
scope, the run rather than the evaluation.

- Every phase MUST write its diagnostics into the channel, in the order it
  found them: the loader's `skipped-document`, the parser's failure, the
  checker's errors and warnings, the planner's refusal, the evaluator's
  failures and warnings, the renderer's faults. No phase MUST write a
  diagnostic anywhere else.
- A diagnostic MUST be written once. Filtering by severity, rendering and
  delivery are the consumers'; none removes a diagnostic from the channel.
- The host MUST read the channel at the end of the run, as HQL-0021 lets a
  host read every channel at the end of a scope, and MAY `tap` it while the
  run proceeds, for a consumer that delivers each diagnostic as it is
  written.
- The program MUST NOT read or write the channel of the run it is in. It
  holds no name for it, and it is not among the program's channels. A later
  run MAY read what an earlier one delivered to a file, `source(file(p),
  JsonLines) | Diagnostic`, as it reads any data.
- `hql render` MUST give each cell's diagnostics the cell as `origin`, and
  MUST write them into the run's channel; `cell.logs` stays the cell's own
  `Channel<LogEvent>` and is unrelated.

### 6. Format and target

A diagnostic reaches a person or a tool through the endpoint machinery of
HQL-0021. There is no reporting architecture beside it.

- `DiagnosticText` is a format: one diagnostic as the heading, the source
  line and caret, its labels, `help`, `expected` and `found`, its element and
  trace. `Json` writes the record with every field present. A format is a
  presentation and is specified with the others ([HQL-0017](../HQL-0017/README.md),
  [HQL-0024 part 7](../HQL-0024/parts/07-formats-and-codecs.md)).
- A target is an endpoint: `stderr`, `stdout`, a file, a network endpoint.
- The host's default wiring MUST be, in effect, `diagnostics |
  DiagnosticText | stderr`, with the result on `stdout`, and the program
  MUST NOT have to say so.
- The host's configuration changes the wiring and nothing else:
  `--diagnostics-format text|json` chooses the format; the shown level,
  `--report-level` today, is a filter on the consumer by severity; the
  envelope, `--envelope json`, carries the channel's contents under
  `diagnostics` beside the value. These are settings of the default
  endpoint, resolved through the layers the host already has, the query,
  the vault's `hql.toml`, the environment, the default.
- A report is `Seq<Diagnostic>` rendered: the channel's contents in a
  format, followed by a summary count per severity when there is more than
  one. The word names the presentation and no entity.

### 7. The lifecycle of a fatal diagnostic

A fatal diagnostic ends its producer, and the channel is useful only if what
was written is delivered first.

```text
running
   │
diagnostic written            the producer writes it into the channel
   │
producer stopping             disposition = fatal: this phase ends; later phases do not start
   │
consumers drained             every consumer has delivered every diagnostic written
   │
channels closed               the program's channels, then the diagnostic channel
   │
run ended                     the exit status is decided
```

- The run MUST NOT end before every consumer of the diagnostic channel has
  delivered every diagnostic written into it.
- A fatal diagnostic MUST NOT discard what was written before it. The
  diagnostics a phase produced before stopping are in the channel in order.
- Reading the channel never resumes the computation. Reporting is not
  exception handling.

### 8. Continuation, by phase

Whether a producer continues after a diagnostic is the producer's. `collect`
asks every producer for as much continuation as it can give; `strict` asks
each to stop at its first `error`. Neither changes what a diagnostic is.

| Phase | After an `error` the producer | Under `collect` |
| --- | --- | --- |
| `load` | skips the document and continues | the same: a skipped document is `recoverable` under both |
| `parse` | stops: nothing after an unparseable source has a structure | the same; a parse failure is `fatal` |
| `check` | stops at the first | checks every statement; a failed one binds its name so the next is reported on its own fault |
| `plan` | stops: a refused pipeline has no plan | the same |
| `execute` | stops at the first | MAY continue past a `recoverable` failure of one element or one statement, where the step's semantics allow; a `fatal` failure, an I/O failure or an overflow, stops under both |
| `render` | stops at the failing cell | renders every cell, each reported against its own fault |

- A producer MUST set `disposition` to what it did. Under `collect` an
  element failure the evaluator stepped past is `recoverable`; the same
  failure under `strict` is `fatal`.
- The exit status MUST be `1` when the channel holds a diagnostic at or above
  the fail level, `error` unless configured, and `0` otherwise. A `warning`
  never changes it unless the fail level says so.

### 9. What this corrects

- `Report`, `Reports` and `Warning` are removed. `Diagnostic` is the one
  type; a warning is a diagnostic with `severity = warning` and
  `disposition = warning`.
- The `stage` field is removed; `phase` and `kind` carry what it conflated.
- The severity derived in `Report::of` is removed; each producer states its
  own.
- HQL-0032 section 28 stands with its destinations named: the transport is
  the diagnostic channel, and "memory channels", "output log", "IDE
  diagnostics" and "telemetry" are targets of it. HQL-0033's rule that a
  program intercepts no entry stands as section 5: the program does not
  read the channel of its own run.
- The card's "entry" is the `Diagnostic` of section 3; its "report" is the
  presentation of section 6. The card is rewritten when this record is
  accepted.
- The loader's warnings reach the channel. `cell.logs` is left as it is.
- `hql repl` gives each line typed its own origin, `<repl:N>`, with the
  line counted in what was typed and not in the replayed bindings.

## Backwards Compatibility

- The JSON form keeps `severity`, `message` and `span`; `stage` is replaced
  by `phase` and `kind`, and `reports` in the envelope by `diagnostics`.
  A reader of the envelope that ignores unknown keys and does not read
  `stage` is unaffected.
- The text form keeps its first two lines, the heading and the location.
  The heading names the phase and the kind, `check error[type-mismatch]:`,
  where it named the stage. A message loses what moves into a field: "did
  you mean `map`?" becomes `help`, "at byte 71" a label.
- A data failure found while executing is `phase = execute`; it was stage
  `type`.
- `--report strict|collect` and `--report-level` keep their names and their
  meaning in this record; `--diagnostics-format` is added. Renaming the
  first two is left to the host's model.
- The library API changes: `Outcome.reports: Reports` becomes
  `Outcome.diagnostics: Vec<Diagnostic>`, the channel's contents in order.
- A run over a vault with a skipped document gains a warning it did not
  report before; a warning does not change the exit status.

## Reference Implementation

Each unit lands with its tests and with the card's sentences corrected for
what it builds.

1. `Diagnostic` as the one record of section 3, with its four axes, replacing
   the enum of `src/diagnostics.rs` and absorbing `src/warnings.rs` and the
   `Report` of `src/reporting.rs`; `kind` and `severity` set at every
   producer (`src/parser.rs`, `src/checker.rs`, `src/evaluation.rs`,
   `src/extensions/*.rs`, `src/transclude.rs`, `src/backends/plan.rs`).
2. The diagnostic channel: a run-scoped `Channel<Diagnostic>` on the `io`
   host, opened in `run_on` before loading and closed after the last phase,
   written by every producer; `Outcome.diagnostics` is its contents
   (`src/io.rs`, `src/lib.rs`).
3. The loader writes `skipped-document` (`src/vault.rs`).
4. The lifecycle of section 7: consumers drained before the run ends
   (`src/lib.rs`, `src/cli.rs`).
5. `DiagnosticText` as a format and `Json` of a `Diagnostic` with every
   field, replacing `src/render.rs`; `--diagnostics-format`; the default
   wiring in `emit` (`src/extensions/format.rs`, `src/cli.rs`).
6. `Diagnostic` declared in `lib/std/`, readable with `| Diagnostic`.
7. Execution context: `element`, `bindings` and the `call` frames of `trace`
   recorded by the evaluator when it writes (`src/evaluation.rs`).
8. Provenance: `source` on a diagnostic about a value read from a card
   (`src/document.rs`, `src/knowledge.rs`).
9. `collect` through `execute` for the steps whose semantics allow it,
   `disposition` set accordingly (`src/evaluation.rs`).
10. `hql repl` origins `<repl:N>` (`src/cli.rs`).
11. The error-handling card, the reporting model and `cli.md` rewritten to
    the terminology of section 1.

## Test Plan

`tests/reporting.rs` and `tests/cli.rs` MUST include:

- for each phase, one diagnostic with its `phase`, `kind`, `severity` and
  `disposition`, compared in the JSON form;
- a contract read `x | T` under `execute` with `severity = error`, and a
  division by zero with `severity = failure`, neither derived from the
  phase;
- the channel's order: a check warning, then an execute failure, delivered
  in that order under `--diagnostics-format json`;
- a fatal diagnostic delivered before the run ends, with the diagnostics
  before it intact;
- a vault with a malformed header writing `skipped-document` and keeping
  the rest;
- `collect` stepping past a recoverable element failure in `map` with
  `disposition = recoverable`, and stopping at an overflow;
- `x | validate(T) | errs()` producing values and no diagnostic;
- `hql repl` reporting the second line typed as `<repl:2>:1`.

```bash
cargo test --locked --test reporting --test cli
```

## Changelog

- 2026-10-25: first draft, a host-owned `Report` with its own queue,
  destinations and formats; withdrawn the same day
- 2026-10-25: rewritten from the error model: failures as data or
  diagnostics, four axes, the run-scoped diagnostic channel on HQL-0021's
  endpoints, the lifecycle of a fatal diagnostic, continuation by phase
