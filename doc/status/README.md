# Status

`doc/status/` is the project's **audit layer**: findings about the knowledge base
itself rather than about the language. It is descriptive, not normative — nothing
here declares a decision. A decision belongs in `doc/proposals/` or a
`doc/models/` document, a small and still-undigested one in `doc/memory/`, and
the work that carries it out in `doc/issues/`.

| File | Scope |
| --- | --- |
| [inconsistencies.md](inconsistencies.md) | Contradictions and stale claims across `doc/**` and `examples/**`, addressed as `INC-NN` |
| [consolidation.md](consolidation.md) | Renames and removals a settled decision implies but that are not yet carried through, addressed as `CON-NN` |

Findings carry stable identifiers so a commit, an issue card or a proposal can
point at one without restating it. A finding is removed when the contradiction is
gone, not when a decision about it is recorded elsewhere, and its identifier is
not reused afterwards.
