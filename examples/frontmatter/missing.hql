// status: design-question
// feature: optional field
// implementation: pending
// environment: knowledge-v1
// expected-type: Option[String]
// expected: None
// note: Option under an explicit optional nickname schema is one candidate; missing-field diagnostics and Unknown remain alternatives.

resolve([[bob]]).frontmatter.nickname
