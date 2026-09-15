// status: design-question
// feature: explicit declaration
// implementation: pending
// environment: knowledge-v1
// expected-type: Unit
// expected: unit value; one proposed knowledge contribution
// note: Historical structured-HQL alternative, not the preferred persistent rich-relation representation. Prefer Relation Cards. Assertion effect/identity and duplicate policy remain open.
// alternatives: design/declaration-arrow.hql, design/declaration-pipeline.hql

declare relatedTo([[alice]], [[bob]]) {
    kind = daughter,
    confidence = 0.9,
    source = [[family-record]]
}
