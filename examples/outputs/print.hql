// status: design-question
// feature: explicit output effect
// implementation: pending
// environment: knowledge-v1
// expected-type: Unit
// expected: unit program result; three explicit output effects
// note: print is a proposed effect, not implicit evaluation behavior. Serialization of Frontmatter and Hmd is unresolved.

print(alice.title)
print(alice.frontmatter)
print(alice.body)
