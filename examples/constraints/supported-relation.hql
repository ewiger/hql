// status: design-question
// feature: structural evidence constraint
// implementation: pending
// environment: knowledge-v1
// expected-type: Satisfaction[SupportedRelation(r)]
// expected: satisfied: the Evidence section contains a link
// note: This demonstrates only the stated structural predicate, not truth or evidential sufficiency of the claim. Missing sections, failure value and satisfaction types remain open.

constraint SupportedRelation(r: Relation) =
    r.section("Evidence")
    | links
    | nonEmpty

r = resolve([[relations/alice-hypermarkdown]]) as WorksOn
r satisfies SupportedRelation
