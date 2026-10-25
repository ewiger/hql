// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.

/// Retrieval: a corpus ranked against a query. A score is evidence for the
/// proposition that a value is relevant to a query, never relevance itself,
/// so every hit carries the retrieval that produced it: the index, model
/// and metric, held as an open tree.
module retrieval

import core

/// One ranked value: the `value`, its `score` against `query`, and the
/// `retrieval` that produced it, a tree holding `index`, `model`,
/// `revision`, `metric`, `approximate` and `query`.
type Hit<T> {
    value     : T
    score     : Float
    query     : String
    retrieval : Data
}

/// The hits of one retrieval, as a `Seq<Hit<T>>` in descending order of
/// score, ties ordered by name, with the `query` they are relative to and
/// the `retrieval` that produced them. A value that scores zero is not a
/// hit.
type Ranking<T> : Seq<Hit<T>> {
    hits      : List<Hit<T>>
    query     : String
    retrieval : Data
}
