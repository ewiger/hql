# Semantic-search requirements

Status: `lexical` is implemented and `semantic` reads a committed index; the
toolchain that produces one is not wired together — see
[issue 0011](../../issues/0011-wire-the-semantic-search-toolchain.md).

Semantic search is the one HQL feature whose answer depends on things outside
the binary: a vault, an index, and a model that computed it. These are its
requirements, stated as what has to be true before a `semantic` query can
answer, and who is responsible for each.

## A card requires a vault, and a vault is a `.hmd/` directory

A card is a document with a place in a namespace, and the namespace is HMD's.
The marker is `.hmd/` at the vault root, which is what `hmd` walks up to find.

`.hmd/config.toml` inside it is **not** required to read a vault: where it is
absent, its defaults are assumed, and they are the values `hmd init` writes —
the wiki root, the namespace name, autodiscovery and its mode. A vault is
therefore usable before anyone has configured it, and configuring it later
changes behaviour rather than enabling it.

The directory itself is required, and increasingly so. It is where vault state
lives, it is what makes a root discoverable from a working directory rather than
supplied by hand, and it is the only thing that distinguishes a vault from a
folder of Markdown. Working with `.hmd` cards without one fails semantic search
first and more later, so the long-term requirement is the directory, with the
config file remaining optional.

## Three parts, and what each is responsible for

| Part | Responsibility | May assume |
| --- | --- | --- |
| `hmd` (Python, package `hypermarkdown`) | the vault: root discovery, `.hmd/config.toml`, the namespace | nothing about HQL |
| `contrib/semantics/` (Python) | the index: read the vault, embed each card and each query, write `.hql/index.sqlite` | `hmd` is installed |
| `hql` (Rust) | the query: score a stored query against stored vectors | nothing but the index file |

The dependency runs one way. A script around semantic search may assume the
`hmd` CLI is installed and treat it as a dependency, which is the default case
for `HmdCard`s. The Rust binary may assume neither Python nor `hmd`: it consumes
a committed index, and `cargo test` must keep running with no Python present.

## What must hold before a query can answer

1. The vault exists and is one — `.hmd/` is present.
2. `hql.toml` names an index, and the file it names exists.
3. The index describes *this* vault: every card present on both sides, and every
   content hash still matching. `cli.py verify` is what says so.
4. The index holds the query. The binary has no model, so it cannot embed a
   question; a query the index does not carry is an error naming the command
   that would add it, never a fallback to `lexical`.

A failure of any of these names the part that failed. Semantic search never
degrades into lexical search silently, because a worse answer that looks like an
answer is the failure mode this design exists to avoid.

## The text contract

The text the producer embeds and the text `Document::text()` hashes must be
byte-identical, or the index scores text nobody wrote and every score is quietly
wrong. This is why `contrib/semantics/vault.py` reimplements the loader's header
parser instead of using a YAML library, and why a change to either side moves
both in one commit. The indexed text is the title, then the authored header's
strings in key order, then the body; where a document sits is a field rather
than a header entry and is not indexed at all, so moving a card between
directories does not change its score.

## Out of scope

The model in the Rust binary, embedding at query time, multiple indexes, hybrid
retrieval and approximate search. See
[HQL-0002](../../proposals/HQL-0002/README.md) for the schema and the full list,
and [semantic search](../../wiki/hql/semantic-search.hmd) for what a `Hit`
carries.
