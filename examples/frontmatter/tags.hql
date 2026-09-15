// status: proposed
// feature: reserved frontmatter type
// implementation: pending
// environment: knowledge-v1
// expected-type: Bool
// expected: true
// note: Candidate projection preserves validated HMD tags as a sequence of strings.

card : Card = resolve([[alice]])
card.frontmatter.tags
| contains("researcher")
