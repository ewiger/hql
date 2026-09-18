# 0003 — Parse and check `type` declarations

Status: backlog
Branch: `feat/type-declarations`

`std/` holds the standard library's declarations as HQL source, and the parser
cannot read a single line of it:

```text
$ hql eval 'type Link<S, T> <: Edge<S, T>'
syntax failure: unexpected character
```

Until this lands, `std/` is text nothing verifies — which is the state
`doc/status/` exists to catch. The folder is worth having anyway, because the
declarations need one home rather than being scattered through prose, but the
gap is real and belongs on the board rather than in a comment.

## Scope

- The `type` statement: `type Name`, `type Name <: Parent`,
  `type Name <: {A, B}`, record bodies, and generic parameters in both
  positions — `Set<T>`, `type Edge<S, T> { … }`.
- Optional fields, `metadata? : Data`, per `doc/wiki/hql/fields.hmd`.
- Checking a declaration: a parent must exist, a supertype set must not join
  domains, and a declaration must not conflict with a built-in type of the same
  name.
- **Not evaluation.** A declaration yields `Unit`. Constructing a value of a
  declared record type is separate work.

## Done when

- Every file under `std/` parses and checks, pinned by a test that walks the
  folder — so a file added there cannot be silently ignored.
- `hql check std/graph.hql` prints `Unit`.
- `doc/models/behavior/expressions.md` documents the statement.
- `doc/wiki/hql/std-lib.hmd` stops describing `std/` as unloadable.
