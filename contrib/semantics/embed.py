"""Turn text into vectors with a language model.

The model is named and its revision pinned, because a `Retrieval` claims both
and a claim that is not pinned is not a claim. Pooling is the mean over the
tokens the attention mask keeps, followed by L2 normalisation, which is what
the sentence-transformers wrapper around this checkpoint does; reproducing it
here keeps the dependency to `transformers` and `torch`.
"""

from __future__ import annotations

from dataclasses import dataclass

DEFAULT_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
DEFAULT_REVISION = "1110a243fdf4706b3f48f1d95db1a4f5529b4d41"
MAX_TOKENS = 256
METRIC = "cosine"


@dataclass(frozen=True)
class Model:
    """A loaded checkpoint, and what it reports about itself."""

    id: str
    revision: str
    dimensions: int
    _tokenizer: object
    _model: object

    def embed(self, texts: list[str]) -> list[list[float]]:
        import torch

        vectors: list[list[float]] = []
        for start in range(0, len(texts), 16):
            batch = texts[start : start + 16]
            encoded = self._tokenizer(
                batch,
                padding=True,
                truncation=True,
                max_length=MAX_TOKENS,
                return_tensors="pt",
            )
            with torch.no_grad():
                output = self._model(**encoded)
            mask = encoded["attention_mask"].unsqueeze(-1).float()
            pooled = (output.last_hidden_state * mask).sum(1) / mask.sum(1).clamp(min=1e-9)
            pooled = torch.nn.functional.normalize(pooled, p=2, dim=1)
            vectors.extend(row.tolist() for row in pooled)
        return vectors


def load(model_id: str = DEFAULT_MODEL, revision: str = DEFAULT_REVISION,
         offline: bool = False) -> Model:
    """Load a checkpoint, refusing the network when `offline` is asked for."""
    import os

    if offline:
        os.environ["HF_HUB_OFFLINE"] = "1"
        os.environ["TRANSFORMERS_OFFLINE"] = "1"
    from transformers import AutoModel, AutoTokenizer

    tokenizer = AutoTokenizer.from_pretrained(model_id, revision=revision)
    model = AutoModel.from_pretrained(model_id, revision=revision)
    model.eval()
    return Model(
        id=model_id,
        revision=revision,
        dimensions=int(model.config.hidden_size),
        _tokenizer=tokenizer,
        _model=model,
    )
