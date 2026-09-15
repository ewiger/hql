// status: proposed
// feature: knowledge operation
// implementation: pending
// environment: knowledge-v1
// expected-type: Knowledge
// expected: alice-to-bob link and declared relation, kept distinct
// note: Evidence origins: link, declaration, frontmatter, structure. Do not silently equate evidence with truth.

knowledge
| between([[alice]], [[bob]])
