# 0011 — Wire the semantic-search toolchain together

Status: todo
Branch: `feat/semantic-toolchain`

Semantic search in HQL has three parts that no single thing installs, configures
or checks. A query fails at whichever part is missing, and the diagnostic names
that part rather than the gap:

| Part | What it is | What it owns |
| --- | --- | --- |
| `hmd` | the HyperMarkDown Python CLI, package `hypermarkdown` | the vault: `.hmd/`, the namespace root, `hmd init` |
| `contrib/semantics/` | the Python producer in this repository | the index: reads the vault, embeds each card, writes `.hql/index.sqlite` |
| `hql` | the Rust binary | the query: reads the index, never builds one, has no model |

The work is to make those three a toolchain rather than three tools that happen
to agree.

## What is missing

**Nothing establishes a vault.** `contrib/semantics/cli.py` takes `--vault
<DIR>` and walks it; it never asks whether the directory is a vault. A typo
produces "no documents", not "that is not a vault". `hmd` already answers the
question — it finds a project by walking up for `.hmd/`, falling back to
`.git/` — and the producer should ask it rather than reimplement the walk.

**The producer does not depend on `hmd`.** `contrib/semantics/requirements.txt`
names `transformers`, `torch` and `pytest`. Working with `HmdCard`s — the
default case — means the `hypermarkdown` package is a real dependency, both for
root discovery and for reading `.hmd/config.toml`. It is not declared, so the
producer cannot use it.

**Configuration is split across two files and one of them is optional.** A
vault's HMD side is `.hmd/config.toml` and its HQL side is `hql.toml`, which is
what names the index. Nothing states how the two relate or which of them a
producer reads.

**Nothing checks the chain end to end.** `cli.py verify` checks an index against
a vault. Nothing checks that a vault is a vault, that the index it names in
`hql.toml` is the one that was built, or that `hql semantic` can answer at all.

## What to do

1. Declare `hypermarkdown` a dependency of `contrib/semantics/` and use its root
   discovery: `--vault` resolves through the same walk `hmd` does, and a
   directory that is not a vault is refused by name.
2. Read `.hmd/config.toml` where it exists and assume its defaults where it does
   not, matching `hmd init`'s written defaults. A vault is usable before it is
   configured; `.hmd/` itself is what has to be there.
3. State, in one place, which file owns what: `.hmd/config.toml` is the
   namespace, `hql.toml` is the index and the extension imports.
4. Add a `doctor`-style check that walks the whole chain — vault, config, index,
   model, query set — and names the first part that is missing, so a failed
   `semantic` query has one command to run next.
5. Give `hmd`'s absence a diagnostic of its own. The Rust binary must keep
   needing neither Python nor `hmd`; this is the producer's dependency, not the
   consumer's, and `cargo test` still runs against the committed index.

## Why it is one card

Each part is small and none of them is useful alone: declaring the dependency
without using the root discovery changes nothing, and the check in step 4 is
what makes the rest visible to someone whose query just failed.

See [the semantic-search requirements](../models/requirements/semantic-search.md),
[HQL-0002](../proposals/HQL-0002/README.md) and
[semantic search](../wiki/hql/semantic-search.hmd).
