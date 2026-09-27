"""Which embedder a vault declares, and what it refuses to declare.

Parsing only: nothing here loads a model or opens a socket.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import config  # noqa: E402

BIRDS = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "birds"

REMOTE = """
[semantics]
index = ".hql/index.sqlite"

[semantics.embedder]
provider    = "http"
model       = "text-embedding-3-small"
revision    = "2024-01-25"
endpoint    = "https://api.openai.com/v1/embeddings"
api_key_env = "OPENAI_API_KEY"
dimensions  = 1536
"""


def vault(tmp_path, text=None):
    if text is not None:
        (tmp_path / config.CONFIG_FILE).write_text(text, encoding="utf-8")
    return tmp_path


def test_a_vault_with_no_config_file_is_local(tmp_path):
    # A vault is usable before it is configured.
    assert config.read(vault(tmp_path)) == config.Embedder()
    assert config.read(vault(tmp_path)).provider == config.LOCAL


def test_a_vault_that_names_only_its_index_is_local(tmp_path):
    # What every existing vault looks like, the committed fixture included.
    declared = config.read(vault(tmp_path, '[semantics]\nindex = ".hql/index.sqlite"\n'))
    assert declared == config.Embedder()


def test_the_committed_birds_vault_stays_local():
    assert config.read(BIRDS).provider == config.LOCAL


def test_every_field_is_read(tmp_path):
    declared = config.read(vault(tmp_path, REMOTE))
    assert declared == config.Embedder(
        provider="http",
        model="text-embedding-3-small",
        revision="2024-01-25",
        endpoint="https://api.openai.com/v1/embeddings",
        api_key_env="OPENAI_API_KEY",
        dimensions=1536,
    )
    assert declared.remote


def test_a_key_written_into_the_committed_file_is_refused_and_told_where_it_goes(tmp_path):
    text = '[semantics.embedder]\nprovider = "http"\napi_key = "sk-secret"\n'
    with pytest.raises(config.ConfigError) as raised:
        config.read(vault(tmp_path, text))
    said = str(raised.value)
    assert "api_key_env" in said, "a refusal must say where the key goes"
    assert "sk-secret" not in said, "a refusal must not echo the key"


def test_an_unknown_key_is_refused_by_name(tmp_path):
    # A typo that silently selects a default is how a vault comes to be indexed
    # by a model nobody chose.
    text = '[semantics.embedder]\nprovidor = "http"\n'
    with pytest.raises(config.ConfigError, match="`providor`"):
        config.read(vault(tmp_path, text))


def test_an_unknown_provider_is_refused_by_name(tmp_path):
    text = '[semantics.embedder]\nprovider = "anthropic"\n'
    with pytest.raises(config.ConfigError, match="anthropic"):
        config.resolve(config.read(vault(tmp_path, text)))


@pytest.mark.parametrize(
    "text",
    [
        '[semantics.embedder]\nprovider = ""\n',
        '[semantics.embedder]\nmodel = "  "\n',
        "[semantics.embedder]\nmodel = 7\n",
        "[semantics.embedder]\ndimensions = 0\n",
        "[semantics.embedder]\ndimensions = -1\n",
        "[semantics.embedder]\ndimensions = true\n",
        '[semantics.embedder]\ndimensions = "1536"\n',
    ],
)
def test_a_field_of_the_wrong_shape_is_refused(tmp_path, text):
    with pytest.raises(config.ConfigError):
        config.read(vault(tmp_path, text))


def test_unreadable_toml_is_refused_rather_than_defaulted(tmp_path):
    # An absent file means the default; a broken one is a different thing.
    with pytest.raises(config.ConfigError, match="TOML"):
        config.read(vault(tmp_path, "[semantics.embedder\n"))


def test_http_needs_an_endpoint_and_a_model():
    with pytest.raises(config.ConfigError, match="`endpoint`"):
        config.check(config.Embedder(provider="http", model="m", revision="r"))
    with pytest.raises(config.ConfigError, match="`model`"):
        config.check(config.Embedder(provider="http", endpoint="http://x", revision="r"))


def test_http_needs_a_pinned_revision():
    # `Retrieval.revision` claims to pin the numbers a score is, and an endpoint
    # offers nothing to derive it from, so it is stated or refused.
    with pytest.raises(config.ConfigError, match="revision"):
        config.check(
            config.Embedder(provider="http", model="m", endpoint="http://x/v1/embeddings")
        )


def test_local_needs_neither_model_nor_revision():
    # `embed` supplies the pinned checkpoint it defaults to.
    assert config.check(config.Embedder()) == config.Embedder()


def test_a_flag_overrides_one_field_and_leaves_the_rest(tmp_path):
    declared = config.read(vault(tmp_path, REMOTE))
    resolved = config.resolve(declared, model="text-embedding-3-large", dimensions=3072)
    assert resolved.model == "text-embedding-3-large"
    assert resolved.dimensions == 3072
    assert resolved.endpoint == declared.endpoint
    assert resolved.api_key_env == declared.api_key_env
    assert resolved.revision == declared.revision


def test_a_flag_left_unset_overrides_nothing(tmp_path):
    declared = config.read(vault(tmp_path, REMOTE))
    assert config.resolve(declared, **dict.fromkeys(config.FIELDS)) == declared


def test_a_flag_can_switch_a_declared_remote_vault_to_local(tmp_path):
    declared = config.read(vault(tmp_path, REMOTE))
    resolved = config.resolve(declared, provider="local")
    assert not resolved.remote
