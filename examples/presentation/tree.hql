// status: proposed
// feature: tree presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: hierarchical presentation of the projects namespace
// note: Hierarchical input such as a namespace, document structure or AST. The presenter does not flatten the value; compare the collection reading in realistic/namespace.hql.
// alternatives: presentation/table.hql

root.projects
| tree
