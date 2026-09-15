// status: proposed
// feature: relation construction
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Link]
// expected: ordinary links from the Relation Card; only the final expression is returned
// note: Relation is a typed Card specialization with identity and prose. The candidate as operation must validate before endpoint projection; no property bag attached to an HMD link.

r = resolve([[relations/alice-hypermarkdown]]) as WorksOn

r.source
r.target
r.frontmatter.since
r.sections
r.links
