# `lib/search/` — the search package

The builtin package for ranking a vault's cards against a query, written as
HQL contracts its internal extension, `search`, implements.

**Status: parsed and checked.** `hql check lib/search/lexical/mod.hql` prints
`Unit`, and `tests/declarations.rs` walks this folder as it walks
[`lib/std/`](../std/README.md), so a file added here cannot be left out of the
binary.

The package holds one folder per module, the module's declarations in its
`mod.hql`, and no prelude: each module is imported where it is used.

| Module | Declares | Ranks by |
| --- | --- | --- |
| [`lexical`](lexical/mod.hql) | `lexical<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>` | the words a card shares with the query, a hashed bag of words compared by cosine |
| [`semantic`](semantic/mod.hql) | `semantic<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>` | vectors a language model made ahead of time, read from the configured index |

Both import `core`, `doc` and `retrieval` from the standard package, which
declares `Ranking` and `Hit`
([semantic search](../../doc/wiki/hql/semantic-search.hmd),
[HQL-0002](../../doc/proposals/HQL-0002/README.md)).

```hql
import lexical
cards | lexical("night hunting birds") | take(3)
```

A diagnostic about one of these modules names it by package, `search/lexical`,
as it names `std/core`.
