// Module `semantic`: a corpus ranked against a query by vectors a language
// model made ahead of time, read from the index `[semantics] index` in hql.toml
// names. The query is looked up in the index too; one the index does not hold
// is refused. Imported where it is used. Every declaration is external: the
// `search` extension implements it.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.
module semantic
import core
import doc
import retrieval

fn semantic<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
