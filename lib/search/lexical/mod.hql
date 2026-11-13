// Every declaration is external: the `lexical` extension implements it.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.

/// Lexical retrieval: a corpus ranked by the words it shares with a query,
/// a hashed bag of words compared by cosine. Offline, deterministic and
/// exact; it models no meaning, so a query and a document that share no
/// word score zero. Imported where it is used: `import lexical`.
module lexical

import core
import doc
import retrieval

/// The cards of `self` ranked by the words their text shares with
/// `query`, as a `Ranking<Card>`. The text of each, and `query`, is a
/// hashed bag of its words of two or more letters or digits, lowercased,
/// and the two are compared by cosine. A card that shares no word with
/// `query` scores zero and is not a hit; the rest are in descending order
/// of score. The retrieval names the model `hql.hashbag.v1` and the metric
/// `cosine`, and is not approximate.
fn lexical<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
