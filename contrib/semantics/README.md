# hql-semantics

The producer for an HQL embedding index. It reads a vault, embeds each card
with a language model, and writes an SQLite index that the `hql` binary reads.

The split is deliberate: the model does not belong in the Rust binary, and the
binary never invokes this tool. `cargo test` runs against the committed index
and needs no Python at all. See
[HQL-0002](../../doc/proposals/HQL-0002/README.md) for the schema, the text
contract and what is out of scope.

## Use

```sh
python3 cli.py build   --vault ../../tests/fixtures/birds \
                       --out   ../../tests/fixtures/birds/.hql/index.sqlite \
                       --built-at 2026-09-18T00:00:00+00:00
python3 cli.py verify  --vault ../../tests/fixtures/birds \
                       --index ../../tests/fixtures/birds/.hql/index.sqlite
python3 cli.py inspect --index ../../tests/fixtures/birds/.hql/index.sqlite
```

`--offline` refuses the network and is what CI uses; the model must already be
in the local cache. `--built-at` pins the build stamp, so rebuilding an
unchanged vault produces a byte-identical file and a regenerated index is not a
diff nobody can review.

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

## Requirements

`transformers` and `torch`, for the model; `pytest`, to run the tests.

```sh
pip install -r requirements.txt
pytest
```
