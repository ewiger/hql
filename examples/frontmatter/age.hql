// status: design-question
// feature: frontmatter field
// implementation: pending
// environment: knowledge-v1
// expected-type: Unknown
// expected: 42 at runtime; no static Int guarantee
// note: Custom age is not a reserved HMD field. Do not infer a global schema from one card.

card : Card = resolve([[alice]])
card.frontmatter.age
