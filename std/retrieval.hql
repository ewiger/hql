// Ranking a corpus against a query.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.
//
// A score is evidence for the proposition that a value is relevant to a query,
// never relevance itself, which is why Retrieval is not optional on a Hit.

type Query
type Score
type Metric
type IndexRef
type ModelRef

type Retrieval {
    query       : Query
    index       : IndexRef
    model       : ModelRef
    metric      : Metric
    approximate : Bool
}

type Hit<T> {
    value      : T
    score      : Score
    provenance : Retrieval
}

// Narrows Seq the way Relation narrows Edge: it adds what makes the order
// meaningful — the query it is relative to, and the rule that broke ties.
type Ranking<T> <: Seq<Hit<T>>
