// status: design-question
// feature: frontmatter field
// implementation: pending
// environment: knowledge-v1
// expected-type: Unknown
// expected: Alice at runtime; no static String guarantee
// note: Unknown is a placeholder for the uncommitted partial-schema policy; see types/schema-refinement.hql.

card : Card = resolve([[alice]])
card.frontmatter.name
