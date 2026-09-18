"""hql-semantics: build, verify and inspect an HQL embedding index.

The Rust binary never invokes this tool, and `cargo test` never needs it. The
producer is Python because the model is; the consumer is Rust because the
language is.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parent))

import embed
import store
import vault as vault_reader


def build(args) -> int:
    root = Path(args.vault)
    documents = vault_reader.load(root)
    if not documents:
        print(f"{root}: no documents", file=sys.stderr)
        return 1
    model = embed.load(args.model, args.revision, offline=args.offline)
    vectors = model.embed([document.text() for document in documents])
    queries = sorted(set(args.query or []))
    query_rows = list(zip(queries, model.embed(queries))) if queries else []
    rows = [
        (
            document.name,
            document.path.relative_to(root).as_posix(),
            document.content_hash(),
            vector,
        )
        for document, vector in zip(documents, vectors)
    ]
    out = Path(args.out)
    store.write(out, root.as_posix(), model, rows, query_rows, built_at=args.built_at)
    print(
        f"{out}: {len(rows)} cards, {len(query_rows)} queries, "
        f"{model.dimensions} dimensions, {model.id}"
    )
    return 0


def verify(args) -> int:
    """Exit 1 when the index no longer describes the vault."""
    root = Path(args.vault)
    index = store.read(Path(args.index))
    if index["meta"][0] != store.SCHEMA_VERSION:
        print(f"unknown index schema: {index['meta'][0]}", file=sys.stderr)
        return 1
    documents = {document.name: document for document in vault_reader.load(root)}
    indexed = {row[0]: row for row in index["cards"]}
    problems = []
    for name, document in sorted(documents.items()):
        row = indexed.get(name)
        if row is None:
            problems.append(f"{name}: absent from the index")
            continue
        if row[2] != document.content_hash():
            problems.append(f"{name}: stale, the text has changed since the build")
    for name in sorted(set(indexed) - set(documents)):
        problems.append(f"{name}: in the index and not in the vault")
    dimensions = index["model"][2]
    for name, _, _, blob in index["cards"]:
        if len(blob) != dimensions * 4:
            problems.append(f"{name}: embedding is {len(blob)} bytes, not {dimensions * 4}")
    for text, blob in index["queries"]:
        if len(blob) != dimensions * 4:
            problems.append(f"query {text!r}: embedding is the wrong length")
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"{args.index}: clean, {len(indexed)} cards")
    return 0


def inspect(args) -> int:
    index = store.read(Path(args.index))
    model_id, revision, dimensions, metric, normalized = index["model"]
    version, vault_name, built_at, tool = index["meta"]
    print(f"schema        {version}")
    print(f"vault         {vault_name}")
    print(f"built at      {built_at}")
    print(f"tool          {tool}")
    print(f"model         {model_id}")
    print(f"revision      {revision}")
    print(f"dimensions    {dimensions}")
    print(f"metric        {metric}")
    print(f"normalized    {bool(normalized)}")
    print(f"cards         {len(index['cards'])}")
    print(f"queries       {len(index['queries'])}")
    for text, _ in index["queries"]:
        print(f"              {text}")
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="hql-semantics", description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    builder = commands.add_parser("build", help="embed a vault into an index")
    builder.add_argument("--vault", required=True)
    builder.add_argument("--out", required=True)
    builder.add_argument("--model", default=embed.DEFAULT_MODEL)
    builder.add_argument("--revision", default=embed.DEFAULT_REVISION)
    builder.add_argument("--offline", action="store_true",
                         help="refuse the network; what CI uses")
    builder.add_argument("--query", action="append", default=[],
                         help="a query to embed into the index; repeatable, "
                              "because the reader cannot embed one itself")
    builder.add_argument("--built-at", default=None,
                         help="pin the build stamp, so a rebuild is byte-identical")
    builder.set_defaults(run=build)

    verifier = commands.add_parser("verify", help="check an index against a vault")
    verifier.add_argument("--vault", required=True)
    verifier.add_argument("--index", required=True)
    verifier.set_defaults(run=verify)

    inspector = commands.add_parser("inspect", help="what an index says about itself")
    inspector.add_argument("--index", required=True)
    inspector.set_defaults(run=inspect)

    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    raise SystemExit(main())
