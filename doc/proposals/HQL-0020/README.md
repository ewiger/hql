# HQL-0020: The vault and its namespace

- **Status**: accepted
- **Created**: 2026-10-18
- **Source**: [issue 0010](../../issues/0010-yield-and-bind.md),
  [HQL-0012](../HQL-0012/README.md), [card type](../../wiki/hql/types/card-type.hmd)

## Abstract

`vault` is the current vault, a value of type `VaultSource` that the host
binds. A vault is not itself a sequence of cards: it is an endpoint whose
fields are what a program reads from it. `vault.cards` is its cards, a keyed,
lazy `Seq<Card>` in the order of a sorted walk; `vault.knowledge` is its
knowledge universe; `vault.graph` is the part of that knowledge that is
concepts and relations, its knowledge graph; `vault.name` and `vault.root` are
its namespace name and root. The standard library binds the three a program
uses most: `cards = vault.cards`, `knowledge = vault.knowledge` and
`graph = vault.graph`. A vault is one namespace: `vault.cards` is every card in
it, whichever module a program sits in, and never a card of another vault.
Another vault is reached as a named source of kind `vault`, with fields of its
own, and is never merged into `cards`. This replaces HQL-0012's binding of
`cards` to the vault itself and its order by card name.

## Motivation

- A vault has more than one thing to offer a program: its cards, its knowledge
  graph, its name and root. HQL-0012 first made the vault the sequence of its
  cards, which left nowhere to put the rest. `vault.graph` needs a vault with fields.
- [Issue 0010](../../issues/0010-yield-and-bind.md) decides that a vault yields
  cards in the order a sorted walk reaches them, so that a lazy source can
  stream them. HQL-0012 first ordered a vault by card name, which a lazy source
  cannot do without reading every card first.
- The [card type](../../wiki/hql/types/card-type.hmd) card leaves two questions
  on `cards`: what it means inside a nested module, and whether imported
  namespaces contribute to it.

## Non-goals

- **No namespace initialization.** `init.hql` and what a namespace imports stay
  with the [extensions](../../wiki/hql/extensions.hmd) card.
- **No module values.** A module's cards are selected by `path`; a value for a
  module, with fields of its own, is not added here.
- **No change to wikilink resolution.** `[[ ]]` resolves as
  [hyper-markdown](../../wiki/hypermarkdown.hmd) says, inside one vault.

## Specification

### 1. The vault type

```hql
abstract type VaultSource : DataSource {
    name  : String                       // the namespace name, from .hmd/config.toml
    root  : String                       // the vault root
    cards     : KeyedSource<String, Card>  // every card, keyed by card name
    knowledge : Knowledge                  // concepts, relations, propositions, evidence
    graph     : KnowledgeGraph             // the knowledge as concepts and relations
}
```

- `VaultSource` MUST NOT be a `Seq<Card>` itself. A vault is an endpoint that
  holds sources, as a [HQL-0012](../HQL-0012/README.md) `Catalog` holds tables,
  except that its members are typed fields rather than names looked up with
  `member`.
- `vault.cards` MUST be a `KeyedSource<String, Card>`: `get(vault.cards,
  "alice")` gives `Option<Card>`.
- `vault.knowledge` MUST be the knowledge universe of the vault's cards, as
  [`std/knowledge.hql`](../../../std/knowledge.hql) declares it: concepts,
  relations, propositions and evidence, with their provenance.
- `vault.graph` MUST equal `KnowledgeGraph(vault.cards)`
  ([HQL-0019](../HQL-0019/README.md)): the part of the knowledge that is
  concepts and the relations between them. It is narrower than
  `vault.knowledge`, which also holds propositions and evidence that are not
  edges. Each MUST be built at most once per program.
- `name` and `root` MUST be the values `.hmd/config.toml` gives or defaults to,
  as the card type card describes.
- The concrete types of HQL-0012's source kinds, `Vault`, `GitVault` and
  `RemoteVault`, narrow `VaultSource` and keep these fields.

### 2. The current vault and its bindings

```hql
cards     = vault.cards
knowledge = vault.knowledge
graph     = vault.graph
```

- The host MUST bind `vault` to the current vault: the one `--vault` names, or
  for `hql render`, the vault containing the document. Without one, `vault` is
  unbound, and the checker says that it needs `--vault`.
- The standard library MUST make the three bindings above. The host seeds
  `vault` alone; `cards` and `graph` are bindings in `std/vault.hql`, and
  `knowledge` joins them with `vault.knowledge`. They are ordinary
  bindings, not keywords: a program reads `cards` and `graph` as it reads any
  name, and both are unbound when `vault` is.
- A binding records how to start traversing its value, never a position
  reached, so two uses of `cards` in one program see the same cards, by
  HQL-0012's stability rule.

### 3. Order

- `vault.cards` MUST yield its cards in the order a sorted walk reaches them:
  the entries of each directory sorted by name, depth-first, each card yielded
  as it is reached. The order is defined by the walk and never by the
  filesystem, so `cards | take(5)` answers the same on every machine.
- This order is total, so it satisfies HQL-0012's order rule for a finite
  source.

### 4. One vault is one namespace

- `vault.cards` MUST be every card in the vault, whichever module the program
  or the document sits in. A module's cards are a `filter` on `c.path`, the
  card's path relative to the vault root.
- `vault.cards` MUST NOT contain a card of another vault. Its horizon is a
  wikilink's: what `[[ ]]` cannot reach, `cards` does not hold.

### 5. Other vaults

```toml
[sources.field_notes]
kind = "vault"
root = "../field-notes"
```

```hql
field_notes = source("field_notes")
field_notes.cards | take(5)
field_notes.graph
```

- Another vault MUST be reached as a named source of kind `vault`, by
  HQL-0012's configuration, with the same fields as the current one.
- Its cards MUST NOT be merged into `cards`. A program that wants both writes
  both, and joins them by [HQL-0013](../HQL-0013/README.md) when it relates
  them.
- A wikilink in one vault never resolves into another; a relation between
  cards of two vaults is a join's result, not a link.

## Backwards Compatibility

Breaking against HQL-0012, which is drafted and not implemented: `cards` is
`vault.cards`, not `vault`, and a vault's order is the sorted walk, not card
name. `cards` was a `Set<Card>` the host seeded; it is `vault.cards`, a lazy
`Seq<Card>` in walk order, as [issue 0010](../../issues/0010-yield-and-bind.md)
carries. `knowledge` is not bound until `vault.knowledge` exists. `vault`,
`graph` and the vault fields are new names.

## Reference Implementation

- `std/sources.hql` — `VaultSource` with its fields
- `std/vault.hql` — `Vault` with its fields, and the bindings
  `cards = vault.cards`, `knowledge = vault.knowledge` and `graph = vault.graph`
- `src/traversal.rs`, `src/evaluation.rs` — the sorted walk, yielding cards
  lazily
- `src/vault.rs` — `name`, `root`, `cards` and `graph` read from the loaded
  vault
- `src/checker.rs` — the vault fields
- `doc/wiki/hql/types/card-type.hmd` — the section on `cards`, answering its
  two open questions
- `doc/wiki/hql/naming.hmd` — `vault`, `cards` and `graph` among the seeded
  and library bindings

## Test Plan

Unit tests MUST include:

- `cards | take(5)` gives the same five cards over two copies of a fixture
  vault created in different file orders
- `get(vault.cards, "alice")` is `some` for a card and `none` otherwise
- `vault.graph` and `vault.knowledge` each built once when read twice in one
  program
- a proposition with evidence in a fixture vault is in `knowledge` and is no
  edge of `graph`
- a program in a nested module sees every card of the vault in `cards`
- a second vault configured as a source: `field_notes.cards` holds its cards
  and `cards` holds none of them
- without `--vault`, `vault`, `cards`, `knowledge` and `graph` are unbound
  with the message naming `--vault`

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-18: drafted
- 2026-10-19: accepted; `vault : Vault` with `name`, `root`, `graph` and
  `cards`, `cards = vault.cards` as a lazy `Seq<Card>` in sorted-walk order,
  `graph = vault.graph` and a missing vault naming `--vault` are implemented,
  and keyed `vault.cards`, the `knowledge` binding and other vaults as sources
  are not yet
- 2026-10-22: other vaults are read as sources of kind `vault`,
  `source("field_notes")` a `Vault` with its own `cards`, `graph`, `name` and
  `root`, its cards never merged into `cards`
  ([HQL-0012](../HQL-0012/README.md), [issue 0040](../../issues/0040-configured-data-sources.md));
  `std/io/mod.hql` declares `VaultSource : DataSource`. `Vault` does not yet
  narrow `VaultSource`, and keyed `vault.cards` and `knowledge` are not
  built
