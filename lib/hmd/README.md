# `lib/hmd/` — the hmd package

HyperMarkdown in HQL: the builtin package `hmd`, beside `std`
([HQL-0059](../../doc/proposals/HQL-0059/index.hmd)). Its contracts are HQL
files its internal extension, `hmd`, implements where they declare without a
body.

**Status: parsed and checked.** `tests/declarations.rs` walks this folder as
it walks [`lib/std/`](../std/README.md), so a file added here cannot be left
out of the binary.

A run with no vault and no import of `hmd` has none of this. `hmd` is
active in a run that has a vault, found at or above the program or named
with `--vault`, and in a run that imports it, with `import hmd`,
`--import hmd` or `[modules] import = ["hmd"]`, and then:

- [`prelude.hql`](prelude.hql) applies to every unit of the run, importing
  `hmd.vault`, `hmd.search`, `hmd.search.lexical` and `hmd.search.semantic`;
- the host binds `vault`, and the program module binds `cards` and `graph`
  over it;
- a document reference, `@[[name]]`, resolves against the vault.

`hql render` and `hql::vault::load` are hosts: they import `hmd` themselves.

| Module | Declares | |
| --- | --- | --- |
| [`hmd`](mod.hql) | nothing yet | the package's root |
| [`hmd.vault`](vault/mod.hql) | `Vault` | the vault a program reads |
| [`hmd.search`](search/mod.hql) | nothing yet | retrieval over a vault's cards |
| [`hmd.search.lexical`](search/lexical/mod.hql) | `lexical<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>` | the words a card shares with the query, a hashed bag of words compared by cosine |
| [`hmd.search.semantic`](search/semantic/mod.hql) | `semantic<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>` | vectors a language model made ahead of time, read from the index the vault's `.hmd/config.toml` names |

The retrievals import `core`, `doc` and `retrieval` from the standard
package, which declares `Card`, `Doc`, `Ranking` and `Hit`
([semantic search](../../doc/wiki/hql/semantic-search.hmd),
[HQL-0002](../../doc/proposals/HQL-0002/index.hmd)).

```hql
import hmd
cards | lexical("night hunting birds") | take(3)
```

A diagnostic about one of these modules names it by package,
`hmd/search.lexical`, as it names `std/core`.
