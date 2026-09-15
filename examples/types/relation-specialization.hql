// status: design-question
// feature: Relation Card types
// implementation: pending
// environment: knowledge-v1
// expected-type: Option[Role]
// expected: Some(architect)
// note: Relation-as-Card is preferred. Declaration brackets, optional ?, Role enum, as validation and narrowing syntax are unresolved; as is not assumed to bypass validation.

type Relation[S, T] <: Card {
    source: Ref[S]
    target: Ref[T]
}

type WorksOn <: Relation[Person, Project] {
    since: Date?
    role: Role?
}

r = resolve([[relations/alice-hypermarkdown]]) as WorksOn
r.role
