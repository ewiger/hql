// status: proposed
// feature: program result
// implementation: pending
// environment: knowledge-v1
// expected-type: Hmd
// expected: alice body only
// note: Preferred bare typed and inferred bindings retain intermediate values. Program returns only the final Hmd value.

alice : Card = resolve([[alice]])

title : String = alice.title
fm = alice.frontmatter

alice.body
