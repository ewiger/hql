"""The SQLite index: schema version 1, and nothing the reader cannot check.

The Rust side only ever reads this file, and treats it as untrusted input, so
everything it must verify is written explicitly: the schema version, the
dimension count, the vault the index was built for, and a hash of the text each
vector was computed from.
"""

from __future__ import annotations

import sqlite3
import struct
from datetime import datetime, timezone
from pathlib import Path

SCHEMA_VERSION = 1
TOOL_VERSION = "hql-semantics 0.1.0"

SCHEMA = """
CREATE TABLE model (
    id          TEXT    PRIMARY KEY,
    revision    TEXT    NOT NULL,
    dimensions  INTEGER NOT NULL,
    metric      TEXT    NOT NULL,
    normalized  INTEGER NOT NULL
);

CREATE TABLE index_meta (
    schema_version INTEGER NOT NULL,
    vault          TEXT    NOT NULL,
    built_at       TEXT    NOT NULL,
    tool_version   TEXT    NOT NULL
);

CREATE TABLE card (
    name         TEXT PRIMARY KEY,
    path         TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding    BLOB NOT NULL
);
"""


def pack(vector) -> bytes:
    """A vector as little-endian f32, which is what the reader expects."""
    return struct.pack("<%df" % len(vector), *vector)


def unpack(blob: bytes) -> list[float]:
    return list(struct.unpack("<%df" % (len(blob) // 4), blob))


def write(out: Path, vault_name: str, model, rows, built_at=None) -> None:
    """Write a whole index. `rows` is (name, path, content_hash, vector)."""
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()
    connection = sqlite3.connect(out)
    try:
        connection.executescript(SCHEMA)
        connection.execute(
            "INSERT INTO model VALUES (?, ?, ?, ?, ?)",
            (model.id, model.revision, model.dimensions, "cosine", 1),
        )
        stamp = built_at or datetime.now(timezone.utc).replace(microsecond=0).isoformat()
        connection.execute(
            "INSERT INTO index_meta VALUES (?, ?, ?, ?)",
            (SCHEMA_VERSION, vault_name, stamp, TOOL_VERSION),
        )
        connection.executemany(
            "INSERT INTO card VALUES (?, ?, ?, ?)",
            [(name, path, digest, pack(vector)) for name, path, digest, vector in rows],
        )
        connection.commit()
    finally:
        connection.close()


def read(path: Path) -> dict:
    """Everything an index holds, for `verify` and `inspect`."""
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        model = connection.execute(
            "SELECT id, revision, dimensions, metric, normalized FROM model"
        ).fetchone()
        meta = connection.execute(
            "SELECT schema_version, vault, built_at, tool_version FROM index_meta"
        ).fetchone()
        cards = connection.execute(
            "SELECT name, path, content_hash, embedding FROM card ORDER BY name"
        ).fetchall()
    finally:
        connection.close()
    return {"model": model, "meta": meta, "cards": cards}
