# hql-semantics

The producer for an HQL embedding index. It reads a vault, embeds each card
with a language model, and writes an SQLite index that the `hql` binary reads.
Which model does the embedding is the vault's choice — see
[Embedders](#embedders).

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
has changed since the build, a card missing from either side, a vector that is
not the declared length, or a vector that is not unit-length.

## Embedders

Two, because they are the two shapes a model can be reached in rather than the
two vendors anyone happens to use.

| Provider | The model runs | Needs |
| --- | --- | --- |
| `local` (default) | in this process | `transformers`, `torch`, the checkpoint cached |
| `http` | anywhere else | `endpoint`, `model`, `revision` — and no `torch` at all |

`http` speaks one OpenAI-shaped wire format — `POST` a
`{"input": [...], "model": "..."}` body, receive
`{"data": [{"index": N, "embedding": [...]}]}` — which a hosted provider and a
model server on `localhost` both implement. It uses only the standard library,
so reaching a hosted embedder costs this tool no dependency.

A vault declares its embedder in `hql.toml`, beside the index the embedder
fills. The whole table is optional and its absence means `local`.

```toml
[semantics]
index = ".hql/index.sqlite"

[semantics.embedder]
provider    = "http"
model       = "text-embedding-3-small"
revision    = "2024-01-25"
endpoint    = "https://api.openai.com/v1/embeddings"
api_key_env = "OPENAI_API_KEY"
dimensions  = 1536
```

Every `build` flag overrides the matching field, so a one-off build need not edit
the vault:

```sh
python3 cli.py build --vault ../../tests/fixtures/birds \
                     --out /tmp/index.sqlite \
                     --provider http \
                     --model nomic-embed-text \
                     --revision v1.5 \
                     --endpoint http://localhost:11434/v1/embeddings \
                     --query "night hunting birds"
```

Endpoints that fit: OpenAI, Voyage, Cohere, and a local `llama.cpp`, `vLLM`,
Ollama or text-embeddings-inference server. **There is no Anthropic embeddings
endpoint to configure** — Claude models are text-in, text-out.

`revision` is required for `http` and the tool refuses without it. A hosted
endpoint offers nothing to derive it from, and `Retrieval.revision` claims to
pin the numbers a score is, so whoever configures the vault states what they are
calling. An unpinnable claim is better refused than invented.

### Keys

`api_key_env` names the environment variable holding the key. It never holds the
key: `hql.toml` is committed, and
[`tests/fixtures/birds/hql.toml`](../../tests/fixtures/birds/hql.toml) already
is. A literal `api_key` in the table is refused, and the refusal says where the
key goes. An unset variable is refused before a single card is read, so the
failure is the variable rather than an HTTP 401 a hundred requests later.

Note that an `http` embedder sends card text to whatever `endpoint` names. That
is what it is for, and it is a reason for the embedder to be declared in a file
that gets reviewed rather than passed as a flag somebody forgets.

### Why every vector is normalised here

The Rust reader scores with a dot product, which is cosine similarity *only* for
unit-length vectors. So this tool L2-normalises every vector whatever a provider
returned and whatever it claims, `store.write` refuses a vector that is not unit
length, and the reader refuses an index declaring `normalized = 0`.

The case that makes this more than bookkeeping: asking a provider for fewer
dimensions than its model computes — OpenAI's `dimensions` parameter — returns a
*prefix* of a unit vector, which is not itself a unit vector. Stored as one, it
would make every score involving that card quietly wrong. `--offline` refuses the
network, and since the network is the whole of the `http` provider, the two
together are refused by name.

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

Python 3.11 or newer, for `tomllib` in the standard library.

`transformers` and `torch` are needed by the `local` provider only — the `http`
provider reaches a model over the standard library, so indexing through one needs
neither. `pytest` runs the tests, which need no model and no network: a fake
transport answers the `http` provider and a fake model fills an index, so what is
under test is this tool's own arithmetic and refusals. The
HyperMarkDown CLI is a dependency in practice — it owns root discovery and
`.hmd/config.toml` — but is not declared or used yet; see
[issue 0011](../../doc/issues/0011-wire-the-semantic-search-toolchain.md).

```sh
pip install -r requirements.txt
pytest
```
