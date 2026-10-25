# 0037 — I/O: typed decoding, and HQL-0021 accepted

Status: done
Related: [HQL-0021](../../proposals/HQL-0021/README.md),
[formats and codecs](../../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[HQL-0012](../../proposals/HQL-0012/README.md),
[0029](../0029-binary-formats-and-codecs.md),
[presentation](../../wiki/hql/presentation.hmd),
[the I/O examples](../../../examples/io/README.md)

HQL-0021's endpoints, `source`, `tap`, `target`, `sink`, channels, `net`,
logging and the command line are built. Its section "Typed decoding" was
not: `s : Seq<Sighting> = source(file("birds.json"))` was refused as a
`Seq<Data>`. The specification also stated a layout and a format model the
implementation had moved past: `lib/std/io/init.hql`, and no `JsonLines`.

## Scope

- The expected type of a read decodes each document as a declared record:
  checked in `src/checker.rs`, decoded in `src/extensions/io.rs` through
  `Value::decoded`, a live read one document at a time.
- The proposal states what shipped: `lib/std/io/mod.hql`, `JsonLines` and
  `Lines` as formats, `.jsonl` implying `JsonLines`, the `effectful fn`
  headers, and the remaining work.

## Done when

- `tests/io.rs` reads JSON Lines, a JSON file, a CSV file and standard input
  into a declared record, through a binding and a function result, and
  refuses a row that does not decode, naming the document and the field.
- The [I/O examples](../../../examples/io/README.md) hold a JSON Lines and a CSV
  case.
- HQL-0021 is accepted, implemented in part, with configured sources by id
  and `max_bytes` stated as waiting for HQL-0012.

## Decisions taken while implementing

- **The expected type is the declared type of the whole value.** A read
  decodes as a record when it is the whole value of an annotated binding or
  the whole body of a function with a declared result, written as a call or
  as `file(p) | source`. A read inside a larger expression has no expected
  type of its own; `| of_type(Sighting)` selects records there.
- **No type argument at the call.** HQL writes type arguments at a call only
  for a constructor, `Channel<LogEvent>("logs")`; `source<Sighting>(…)` is
  not added, and the binding's annotation is the one place the type is
  written.
- **Decoding is reading by shape.** The section said "by the type's
  constructor"; a tree becomes a record by the same reading `match`,
  `is` and `of_type` use (`Value::interpreted`, now `Value::decoded` with
  the reason it fails): required fields at their types, optional fields
  absent without an entry, other entries ignored.
- **Only a CSV cell is parsed.** A CSV cell is text whatever it holds, so it
  is read as the `Int`, `Float` or `Bool` its field declares, and an empty
  cell at an optional non-`String` field is absent. A `Json`, `JsonLines` or
  `Yaml` leaf keeps its written type: `"3"` is not an `Int`.
- **What decodes is decided at check time, what fails at run time.** A read
  is retyped only when its document type may be the expected element
  (`program::may_be`): a declared record from `Data`, or a type that narrows
  `Data`. A `Seq<Sighting>` over `Lines` keeps `Seq<String>` and the binding
  refuses it. A document that does not decode is an evaluation error naming
  the endpoint, the document's position from 1, and the field's dotted path.
- **`Logger` keeps one declared form.** A logger on a
  `Channel<LogEvent>` omits the format; `lib/std/io/logging/mod.hql` declares the
  form with a format and says the format is omitted for a channel, and the
  proposal now shows the one header.

## Left open

- `source("id")`, `tap("id")` and `max_bytes` are built
  ([issue 0040](0040-configured-data-sources.md)).
