// status: proposed
// feature: conflicting knowledge
// implementation: pending
// environment: knowledge-v1
// expected-type: Knowledge
// expected: both active and ended WorksOn claims, with different identities and evidence
// note: Knowledge preserves conflicting assertions. A graph projection must not silently choose one or merge away provenance.

knowledge
| between([[alice]], [[hypermarkdown]])
