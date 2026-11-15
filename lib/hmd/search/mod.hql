// See doc/proposals/HQL-0059/index.hmd and doc/wiki/hql/semantic-search.hmd.

/// Retrieval over a vault's cards: `hmd.search.lexical` ranks by the words a
/// card shares with a query, `hmd.search.semantic` by vectors from the
/// vault's index. `hmd`'s prelude imports both.
module hmd.search

import core
