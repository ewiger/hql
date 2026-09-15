// status: design-question
// feature: partial schema
// implementation: pending
// environment: knowledge-v1
// expected-type: Unknown
// expected: 1200 at runtime; schema not declared
// note: Known reserved fields coexist with custom fields; Unknown is not yet a language type.

resolve([[project-x]]).frontmatter.budget
