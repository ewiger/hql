# The `[[..]]` wiki-link operator

Recorded 2026-09-18 from a user instruction: the `[[..]]` operator is enough on
its own, so no function wraps it.

- `[[..]]` resolves a name in the current namespace and yields what it
  addresses. The binding is `alice = [[alice]]`. Defined in
  [link-op](../wiki/hql/operators/link-op.hmd), the first card under
  `doc/wiki/hql/operators/`.
- The result type follows the address granularity: `Doc`, `Section`, `Block`.
  Card-ness stays a checked narrowing; a reference cannot establish it.
- A reference obtained from a value at runtime is a separate situation. What
  turns one into its target, and whether `Ref[T]` survives, are open.
- How a failed resolution is carried is the largest open question, because an
  operator must produce a value while HMD deliberately treats an unresolved link
  as a warning.

Carried through the wiki cards, the models and the memory notes; the wrapper
spelling is gone from `doc/`. See [corpus direction](corpus-direction.md) for the
binding forms.
