"""The index the Rust reader is handed."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import store  # noqa: E402

BIRDS = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "birds"


def test_vectors_round_trip_as_little_endian_f32():
    vector = [0.0, 1.0, -0.5]
    blob = store.pack(vector)
    assert len(blob) == len(vector) * 4
    assert store.unpack(blob) == vector


def test_the_committed_index_pins_its_schema_and_its_model():
    index = store.read(BIRDS / ".hql" / "index.sqlite")
    model_id, revision, dimensions, metric, normalized = index["model"]
    version, vault_name, _, _ = index["meta"]
    assert version == store.SCHEMA_VERSION
    assert vault_name == "tests/fixtures/birds"
    assert model_id == "sentence-transformers/all-MiniLM-L6-v2"
    assert len(revision) == 40, "the model revision is pinned, not just its name"
    assert (dimensions, metric, normalized) == (384, "cosine", 1)


def test_every_vector_is_the_declared_length_and_unit_long():
    index = store.read(BIRDS / ".hql" / "index.sqlite")
    dimensions = index["model"][2]
    for name, _, _, blob in index["cards"]:
        assert len(blob) == dimensions * 4, name
        norm = sum(value * value for value in store.unpack(blob)) ** 0.5
        assert abs(norm - 1.0) < 1e-5, name
