// Module `lexical`: a corpus ranked by the words it shares with a query, a
// hashed bag of words compared by cosine. Offline, deterministic and exact; it
// models no meaning, so a query and a document that share no word score zero.
// Imported where it is used. Every declaration is external: the `search`
// extension implements it.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.
module lexical
import core
import doc
import retrieval

fn lexical<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
