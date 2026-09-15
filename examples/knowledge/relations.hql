// status: proposed
// feature: knowledge operation
// implementation: pending
// environment: knowledge-v1
// expected-type: Knowledge
// expected: WorksOn claims with their identities and provenance
// note: Typed semantic filtering preserves Knowledge; an eventual materialization to List[Relation] is separate.

knowledge
| relations[WorksOn]
