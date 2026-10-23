// Ranking a corpus against a query.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.
//
// A score is evidence for the proposition that a value is relevant to a query,
// never relevance itself, which is why every hit carries the retrieval that
// produced it: the index, model and metric, held as an open tree.
import core

type Hit<T> {
    value     : T
    score     : Float
    query     : String
    retrieval : Data
}

// A concrete retrieval satisfies the abstract positional Seq contract.
// It adds what makes the order meaningful — the query it is relative to, and
// the retrieval that produced it.
type Ranking<T> : Seq<Hit<T>> {
    hits      : List<Hit<T>>
    query     : String
    retrieval : Data
}
