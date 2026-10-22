# HQL-0029: Braces: `{}` is the empty set, `{:}` the empty Data

**Status**: implemented
**Created**: 2026-10-22
**Source**: [set and Data](../../wiki/hql/set-vs-data.hmd)

## Abstract

Braces hold either values or `key: value` entries, and the colon is what tells
them apart. Braces without a colon denote a set; braces containing key/value
entries denote `Data`. The empty set is `{}`, a `Set<Never>`, and the empty
`Data` is `{:}`. Neither empty form takes its meaning from the type expected
of it.

## Motivation

`{1, 2}` is a `Set<Int>` and `{name: "Alice"}` is `Data`, but `{}` was an empty
`Data`, so the set literal had no empty form: an empty set was written
`Set<Int>([])`, and `s : Set<Int> = {}` was refused as a `Data`. Making `{}`
an empty set where a `Set` is expected and `Data` elsewhere would give one
literal two meanings chosen by context. A colon in the empty `Data` keeps the
rule the non-empty forms already follow.

## Goals

- one rule for every brace literal, empty or not, stated by its own syntax
- an empty set literal that a declared set type accepts
- printing that reads back: the empty set prints `{}`, the empty `Data` `{:}`

## Non-goals

- an empty set symbol such as `∅`: the rule completes the existing `{1, 2}`
  syntax instead of adding a one-off token
- structural record types; `{name: "Alice"}` stays `Data`

## Specification

### 1. The brace grammar

```text
set   ::= "{" (expression ("," expression)* ","?)? "}"
data  ::= "{" ":" "}" | "{" key ":" expression ("," key ":" expression)* "}"
```

```hql
{}                        // Set<Never>, the empty set
{1, 2, 3}                 // Set<Int>
{"a", "b"}                // Set<String>
{:}                       // Data, the empty record
{name: "Alice"}           // Data
{name: "Alice", age: 42}  // Data
```

- Braces without `:` MUST denote a set, and braces containing `key: value`
  entries MUST denote `Data`.
- `{}` MUST be a `Set<Never>`. As every `Set<Never>`, it is accepted where a
  `Set<T>` is declared: `x : Set<Int> = {}`.
- `{:}` MUST be the empty `Data`: `d : Data = {:}`.
- `d : Data = {}` MUST be a type error: `{}` already has a definite meaning.
- Where a data literal is the only thing the grammar admits, a link's
  annotation `a -> b {…}`, `{}` MUST be refused as a syntax error naming `{:}`.

### 2. Printing

- An empty `Data` MUST print as `{:}`, alone or nested: `{a: {:}}`.
- An empty `Set<Never>` MUST print as `{}`. An empty set of a known element
  type MUST print as `Set<T>([])`, so the text reads back with that type.

## Backwards Compatibility

Breaking: `{}` written for an empty `Data` is now an empty set. Every such use
in `std/`, the examples, the tests and the documentation is written `{:}`:
`reduce({:}, merge)`, the logging operations' `data: Data = {:}` and
`Edge(…, data = {:})`. JSON's `{}` is unaffected.

## Security Considerations

None.

## Deployment / Activation

Takes effect in the parser, with no configuration.

## Reference Implementation

- `src/parser.rs`: `{}` parses as a set literal and `{:}` as a data literal;
  a data-only position refuses `{}`
- `src/data.rs`, `src/types/display.rs`: `{:}` and `{}` printed
- `std/collections.hql`, `std/io/logging.hql`, `std/io/net.hql`: `{:}`
- [the braces examples](../../../examples/braces/README.md)

## Test Plan

- `tests/data.rs` checks the type of each form above, `x : Set<Int> = {}`,
  `d : Data = {:}`, the refusal of `d : Data = {}` and of `{}` as a link's
  annotation, and how each prints
- the corpus runs `examples/braces/`

## Open Questions

None.

## Changelog

- 2026-10-22: drafted and implemented
