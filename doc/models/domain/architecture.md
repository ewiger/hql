# Core architecture

## Implemented boundary

```text
source string → lexer/parser → private AST → checker → typed execution IR
                                                        ↓
                                                    evaluation → Value
                    └────────────── diagnostics ──────────────────────┘
CLI: arguments + file I/O + output, calling the public library API
```

`lexer.rs` and `parser.rs` produce private syntax with source byte ranges.
`checker.rs` resolves imports, step identities, fields, constructors, annotations,
and expression types. `execution.rs` lowers those checked facts into a private
execution tree. Failed checking produces no executable program, including in
collect mode. `evaluation.rs` consumes only that tree; it does not resolve imports
or infer collection element types again. Runtime failures such as overflow,
negative counts, missing index data, and unequal map lengths remain fallible.

Collection literals and lambda results retain their checked element types,
including empty `map` results. Left-associated addition chains lower to a flat
fold while retaining overflow spans. Extension evaluators receive typed arguments;
their structural `StepSpec` drives checking and generated help. Dependent hooks
add source requirements or ordering constraints after ordinary checking.

Runtime values, cards, graph values, and retrieval provenance share immutable
storage through `Arc`. Runtime and extension carriers are checked for `Send + Sync`.
Execution is still sequential: a planner, parallel scheduling, and backend
pushdown are future consumers of the typed execution boundary.

## Source and knowledge assembly

```text
VaultSource → source snapshot → DocumentStore → card and knowledge assembly
                                                     ↓
                                           extension metadata contributions
                                                     ↓
                                              Vault → HQL execution
```

`sources.rs` owns filesystem traversal and reading behind `VaultSource`.
`document.rs` assigns names, parses headers and references, and reports bad
documents. `knowledge.rs` assembles effective metadata, verifies relation
endpoints, and builds link and relation edges. The extension registry dispatches
metadata contributions; lexical metadata is owned by the lexical extension.
`vault.rs` coordinates these stages and exposes their aggregate to HQL.

YAML syntax belongs to the YAML library. Conversion to HQL `Data` is fallible:
unsupported keys, tags, and numbers are diagnosed rather than reinterpreted.
Invalid documents are skipped with a warning containing their source path.
Memory sources use the same naming, parsing, and knowledge assembly as files.
See [the stack](../../stack.md) and [data values](../data/values.md).

## Corpus design refinements

[Program values](../behavior/program-values.md) records last-expression results,
lexical cell environments and host-controlled presentation. The
[knowledge model](knowledge.md) distinguishes structural links, relation cards,
Knowledge, Graph and HmdGraph. These are settled design, not deferred work; what
is deferred is the remaining *implementation*, including first-class functions
and generated query sources. See the [corpus](../../../examples/README.md).
