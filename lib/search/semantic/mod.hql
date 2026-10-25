// Every declaration is external: the `semantic` extension implements it.
//
// See doc/wiki/hql/semantic-search.hmd and doc/proposals/HQL-0002/README.md.

/// Semantic retrieval: a corpus ranked against a query by vectors a
/// language model made ahead of time, read from the index that
/// `[semantics] index` in hql.toml names. The query is looked up in the
/// index too; one the index does not hold is refused. Imported where it is
/// used: `import semantic`.
module semantic

import core
import doc
import retrieval

/// The cards of `self` ranked against `query` by the cosine of their
/// vectors in the vault's index, as a `Ranking<Card>` in descending order
/// of score; a card scoring zero or less is not a hit. `query` must be in
/// the index. A card the index holds no vector for is left out of the
/// ranking, with a warning; one changed since the index was built is
/// scored, with a warning that its score is stale.
fn semantic<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
