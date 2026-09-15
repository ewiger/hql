# Core architecture

## Implemented boundary

```text
source string → parser/scanner → private AST → type checking → evaluation → Value
                                      └──────── diagnostics ────────────────┘
CLI: arguments + file I/O + output, calling only hql::check and hql::eval
```

`parser.rs` owns the tiny scanner and parser together; a separate lexer adds
no useful boundary yet. `ast.rs` carries source byte ranges. `types.rs` owns
static checking, `values.rs` runtime results, `evaluation.rs` checked arithmetic,
and `diagnostics.rs` structured failures. All modules have working consumers.
The AST remains private so callers cannot bypass limits or construct malformed
syntax. The library accesses neither the filesystem nor HyperMarkDown.

## Intended integration boundary (not implemented)

```text
HyperMarkDown documents
    ↓ existing HMD parser + resolver, behind an adapter
 typed Card / document model
    ↓ HQL core
 values / cards / graphs / HMD / diagnostics / transformations
```

A HyperMarkDown document resolves naturally to a `Card`. The host adapter will
supply typed document structure, frontmatter, namespaces, and resolved links /
relations. Knowledge graphs are a future value domain. Do not make string
search over raw HMD the core model. Host parsing/resolution and language
semantics must remain separable, allowing future non-HMD data sources.

The concrete Card schema, host interface, serialization, and evaluation effects
are unresolved; no speculative traits or Card stubs are introduced now.
See [integration evidence](../../wiki/hmd-integration.hmd) and
[bootstrap value data](../data/values.md).
