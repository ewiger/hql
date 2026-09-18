# An unresolved reference warns and yields absence

Recorded 2026-09-18 from a user decision, resolving `INC-15`.

- [Doc](../wiki/hql/types/doc-type.hmd) said a warning, following HMD; the
  corpus said a hard error. The decision is the warning, because a forward link
  to a document nobody has written yet is ordinary and
  [link-op](../wiki/hql/operators/link-op.hmd) permits it.
- `[[name]]` is therefore `Option[Card]` **by type**, not only when it fails.
  Evaluating an unresolved one yields absence and queues a warning; the exit
  status does not change.
- Absence propagates through a field — `[[nowhere]].title` is
  `Option[String]` — and a pipeline step applied to absence is one applied to
  nothing rather than a failure. That lifting is a stopgap: there is no `match`
  and no sum-type syntax yet, so it is how absence is currently consumed.
  Replacing it with an explicit form is open, and it is the one place the
  implementation is weaker than the design asks: absence is meant to be an
  ordinary sum type consumed by an exhaustive `match` — see
  [no traits](no-traits.md).
- This needed a warning channel, which the language did not have. See
  [reporting](reporting-modes.md).

Implemented in `src/evaluation.rs` and `src/checker.rs`. Tests in
`tests/vault.rs`.
