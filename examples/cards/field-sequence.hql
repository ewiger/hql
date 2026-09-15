// status: proposed
// feature: last expression
// implementation: pending
// environment: knowledge-v1
// expected-type: Hmd
// expected: alice body including heading and links
// note: Only Hmd is returned. Earlier unbound String/Frontmatter values are evaluated and discarded, never implicitly printed.

alice : Card = resolve([[alice]])

alice.title
alice.frontmatter
alice.body
