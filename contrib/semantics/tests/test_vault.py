"""The reader must see a vault exactly as `src/vault.rs` sees it."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import store  # noqa: E402
import vault  # noqa: E402

BIRDS = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "birds"


def test_header_splits_at_the_closing_fence():
    authored, body = vault.split_header("---\ntitle: A\n---\n# A\n\nbody\n")
    assert authored == "title: A\n"
    assert body == "# A\n\nbody\n"


def test_a_document_without_a_header_is_all_body():
    authored, body = vault.split_header("# A\n\nbody\n")
    assert authored == ""
    assert body == "# A\n\nbody\n"


def test_nested_maps_and_scalars_parse_as_the_loader_parses_them():
    header = vault.parse_header(
        "title: A\n"
        "metadata:\n"
        "  status: todo\n"
        "  knowledge:\n"
        "    type: Relation\n"
        "tags: [one, two]\n"
        "count: 3\n"
        "open: true\n"
    )
    assert header == {
        "title": "A",
        "metadata": {"status": "todo", "knowledge": {"type": "Relation"}},
        "tags": ["one", "two"],
        "count": 3,
        "open": True,
    }


def test_the_indexed_text_leaves_out_where_a_document_sits():
    # `name`, `path` and `format` say where a card is, not what it says; a card
    # that moves between directories must not thereby change its score.
    document = vault.Document(
        name="a",
        path=Path("somewhere/a.hmd"),
        title="A title",
        header={"title": "A title", "path": "somewhere/a.hmd", "status": "todo"},
        body="the body",
    )
    assert document.text() == "A title todo the body"


def test_the_title_falls_back_to_the_first_heading():
    documents = {document.name: document for document in vault.load(BIRDS)}
    assert documents["barn-owl"].title == "Barn owl"
    assert len(documents) >= 40


def test_the_committed_index_describes_the_committed_corpus():
    # The golden test from the Python side: every card's text hashes to what
    # the index says it hashed to when the vectors were computed.
    index = store.read(BIRDS / ".hql" / "index.sqlite")
    indexed = {name: digest for name, _, digest, _ in index["cards"]}
    documents = vault.load(BIRDS)
    assert set(indexed) == {document.name for document in documents}
    for document in documents:
        assert indexed[document.name] == document.content_hash(), document.name
