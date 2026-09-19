# hql-semantics

The producer for an HQL embedding index. It reads a vault, embeds each card
with a language model, and writes an SQLite index that the `hql` binary reads.

The split is deliberate: the model does not belong in the Rust binary, and the
binary never invokes this tool. `cargo test` runs against the committed index
and needs no Python at all. See
[HQL-0002](../../doc/proposals/HQL-0002/README.md) for the schema, the text
contract and what is out of scope, and
[the requirements](../../doc/models/requirements/semantic-search.md) for the
chain a query depends on.

`--vault` wants a vault: a directory with `.hmd/` at its root. A
`.hmd/config.toml` inside it is optional and its defaults are assumed where it
is absent. This tool does not check that yet, and a directory that is not a
vault reports "no documents" rather than saying so —
[issue 0011](../../doc/issues/0011-wire-the-semantic-search-toolchain.md) is that
work.

## Use

```sh
python3 cli.py build   --vault ../../tests/fixtures/birds \
                       --out   ../../tests/fixtures/birds/.hql/index.sqlite \
                       --built-at 2026-09-18T00:00:00+00:00 \
                       --query "night hunting birds" \
                       --query "birds swimming underwater catching fish" \
                       --query "birds weaving hanging nests"
python3 cli.py verify  --vault ../../tests/fixtures/birds \
                       --index ../../tests/fixtures/birds/.hql/index.sqlite
python3 cli.py inspect --index ../../tests/fixtures/birds/.hql/index.sqlite
```

`--offline` refuses the network and is what CI uses; the model must already be
in the local cache. `--built-at` pins the build stamp, so rebuilding an
unchanged vault produces a byte-identical file and a regenerated index is not a
diff nobody can review.

`--query` is repeatable and embeds a question into the index. The `hql` binary
has no model, so it cannot embed a document *or* a question: `semantic` looks a
query up, and one the index does not hold is a failure naming this command
rather than a ranking of nothing. That is a real limit — an index answers the
questions it was built for — and it is the price of keeping the model out of the
binary.

`verify` exits 1 when the index no longer describes the vault: a card whose text
has changed since the build, a card missing from either side, or a vector that
is not the declared length.

## The text contract

The text embedded here and the text hashed in `src/document.rs` must be
byte-identical. A mismatch has no symptom — the index would score text nobody
wrote — so `vault.py` reimplements the loader's header parser rather than
reaching for a YAML library, whose idea of the same document would differ in
ways nothing would report. A change to `Document::text()` changes both in one
commit, and `tests/semantics.rs` fails until it does.

The text is the title, then the authored header's strings in key order, then the
body. `title` is the only key filtered out, because it is written once at the
front. Where a document sits — `name`, `path`, `format` — is a field of `Doc`
rather than a header entry, so there is nothing to filter: a card that moves
between directories does not change its score, and a header key someone wrote
called `path` is authored text like any other.

## Requirements

`transformers` and `torch`, for the model; `pytest`, to run the tests. The
HyperMarkDown CLI is a dependency in practice — it owns root discovery and
`.hmd/config.toml` — but is not declared or used yet; see
[issue 0011](../../doc/issues/0011-wire-the-semantic-search-toolchain.md).

```sh
pip install -r requirements.txt
pytest
```
