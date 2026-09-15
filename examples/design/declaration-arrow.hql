// status: design-question
// feature: declaration alternative
// implementation: pending
// environment: alice-cell-v1
// expected-type: Unit
// expected: one candidate knowledge contribution
// note: Historical HQL declaration alternative, not HMD link syntax. Rich persistent relations now prefer Cards; implicit subject/effect details remain open.
// alternatives: declarations/assertion.hql, design/declaration-pipeline.hql

relatedTo -> [[bob]] {
    kind = daughter,
    confidence = 0.9,
    source = [[family-record]]
}
