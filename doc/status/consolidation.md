# Consolidation TODO

Opened 2026-09-17 on branch `feat/lang-design`; re-verified 2026-09-18 against
commit `ac9e4c5` on `feat/semantic-search`.

This file tracks **consolidation work**: renames and removals that a settled
decision implies but that have not yet been carried through every file. It is a
work index, not a decision record. A decision belongs in `doc/proposals/` or a
`doc/models/` document; a contradiction still awaiting one belongs in
[inconsistencies.md](inconsistencies.md) as an `INC-NN` finding.

Items carry stable `CON-NN` identifiers so a commit can point at one without
restating it. **A finished item is deleted**, not kept as a record that it was
done; git history is that record. Identifiers are not reused.

## Index

The index is empty: no consolidation item is outstanding.

`CON-01`, `CON-03`, `CON-05`, `CON-11`, `CON-12`, `CON-13` and `CON-14` were
closed with `examples/` as their only outstanding residue. The corpus restored in
`8aa265e` was checked for each — `Hmd` as a body type, `List<Card>` for `cards`,
`typed Relation`, square-bracket generics — and carries none of them, so the
seven stay deleted.

The file stays although the index is empty, because the identifiers are
addresses a commit may point at. The next item takes `CON-15`.
