# Prompt — issues 0001 and 0002

Paste the block below into a new session, in this repository.

---

Two pieces of work, in order. The designs are recorded as ADRs; the issues hold
the sequence.

- [HQL-0001](../proposals/HQL-0001/README.md) — the extension mechanism, carried
  out by [issue 0001](0001-extension-mechanism.md)
- [HQL-0002](../proposals/HQL-0002/README.md) — semantic retrieval over a real
  embedding index, carried out by [issue 0002](0002-semantic-retrieval.md)

Read `doc/memory/` first, as `CLAUDE.md` requires, then both proposals and both
issues in full.

**Before writing any code, settle the Open Questions in HQL-0001 with me** —
four of them — and move its status to `accepted`. Ask them as one batch with a
recommendation for each. Do the same for HQL-0002 before starting issue 0002, not
before issue 0001.

Then: branch `feat/extensions` off `feat/lang-design` and carry out issue 0001.
The sequence there is five commits, and the second one matters most — after
introducing the registry, the whole suite must pass with no test edited. If a
test needs changing at that point, the registry changed behaviour and should not
have.

When that lands, branch `feat/semantics` off it and carry out issue 0002.

Do not build `lexical` and `semantic` as two more arms of the `match` in
`src/checker.rs` and `src/evaluation.rs`. That is what HQL-0001 exists to
prevent, and it is the reason these are two issues rather than one.

The measurable outcome for issue 0002, which the corpus must be written for:
`lexical("birds that hunt at night")` finds nothing useful and
`semantic("birds that hunt at night")` finds the owls, because the owl articles
say "nocturnal" and "after dark" and never "hunt at night". Three such pairs,
asserted by card name. If they are weak, the rest of the issue does not
compensate.

Finish each issue by updating the documentation its "Done when" section lists,
and moving its card to `done` in `doc/issues/kanban.yaml`. Leaving `doc/` stale
is how `INC-15` and `CON-05` happened.
