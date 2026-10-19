//! The `semantic` extension: ranking against vectors a language model made.
//!
//! The model is not in this binary. `contrib/semantics/` embeds a vault ahead
//! of time into an SQLite index, and this module reads it. A query is read
//! from the index too. One the index lacks may be embedded by the index's own
//! `http` embedder, as HQL-0022 allows, through the AI harness's recordings,
//! and never by any other model: inventing a vector would be the silent
//! wrongness the whole split exists to avoid.
//!
//! The index is untrusted input. Everything read from it is checked: the schema
//! version, the vault it was built for, the dimension count, and the length of
//! every vector.

use super::collections::{elements, wrong};
use super::spec::{ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern as P};
use super::{EvalCx, Operation, Purity, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::document::Card;
use crate::execution::Arg as TypedArg;
use crate::search::{self, Retrieval};
use crate::types::Value;
use crate::vault::Vault;
use crate::warnings::Warning;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The only schema this reader knows.
pub const SCHEMA_VERSION: i64 = 1;
/// The largest vector width accepted, so a corrupt header cannot ask for an
/// allocation the size of the file.
const MAX_DIMENSIONS: i64 = 8192;
/// The command that produces an index, named by every failure that needs one.
const BUILD: &str = "hql-semantics build";
/// The only metric this reader computes. `dot` over unit-length vectors is
/// cosine similarity, and an index declaring anything else is refused rather
/// than scored by an arithmetic it did not ask for.
const METRIC: &str = "cosine";

/// What a vault's `hql.toml` says about its index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Semantics {
    /// The index path, relative to the vault root.
    pub index: Option<String>,
    /// The `http` embedder that filled it, which may also embed a query the
    /// index holds no vector for.
    pub embedder: Option<Embedder>,
}

/// An `http` embedder from `[semantics.embedder]`: an OpenAI-shaped
/// `/v1/embeddings` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Embedder {
    /// The model id sent, which must be the index's.
    pub model: String,
    /// Its pinned revision, which must be the index's.
    pub revision: String,
    /// Where requests are sent.
    pub endpoint: String,
    /// The NAME of the variable holding the key.
    pub api_key_env: Option<String>,
}

impl Semantics {
    /// Read `[semantics]` from a vault's configuration file.
    #[must_use]
    pub fn read(root: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(root.join(crate::reporting::CONFIG_FILE)) else {
            return Self::default();
        };
        let Ok(parsed) = toml::from_str::<toml::Value>(&text) else {
            return Self::default();
        };
        let section = parsed.get("semantics");
        let table = section.and_then(|section| section.get("embedder"));
        let text = |key: &str| {
            table
                .and_then(|table| table.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        };
        // Only an `http` embedder can run here: a `local` one is a checkpoint
        // in the producer's process.
        let embedder = (text("provider").as_deref() == Some("http")).then(|| Embedder {
            model: text("model").unwrap_or_default(),
            revision: text("revision").unwrap_or_default(),
            endpoint: text("endpoint").unwrap_or_default(),
            api_key_env: text("api_key_env"),
        });
        Self {
            index: section
                .and_then(|section| section.get("index"))
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            embedder,
        }
    }
}

/// Every step the `semantic` extension provides.
pub(crate) static STEPS: &[Operation] = &[Operation {
    name: "semantic",
    spec: OperationSpec {
        parameters: &[TypeParameter {
            name: "T",
            bound: Some(crate::types::TypeConstructor("Doc")),
        }],
        input: P::Collection(&P::Var("T")),
        arguments: &[ArgumentSpec::value(
            "query",
            P::Type(crate::types::TypeConstructor("String")),
        )],
        output: O::Type(P::Apply(
            crate::types::TypeConstructor("Ranking"),
            &[P::Type(crate::types::TypeConstructor("Card"))],
        )),
    },
    summary: "Rank a corpus against a query by the meaning a model found in both.",
    purity: Purity::Pure,
    check: Some(check_semantic),
    eval: eval_semantic,
}];

fn eval_semantic(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "query").ok_or_else(|| wrong("semantic", "a query", &span))?;
    let Value::Str(query) = cx.evaluate(&argument.value)? else {
        return Err(wrong("semantic", "a text query", &span));
    };
    let elements = elements(&input, "semantic", &span)?;
    let cards: Vec<Arc<Card>> = elements.iter().filter_map(Value::as_card).collect();
    if cards.len() != elements.len() {
        return Err(wrong("semantic", "a collection of cards", &span));
    }

    let path = configured(cx.vault(), &span)?;
    let index = Index::open(&path, cx.vault(), &span)?;
    let asked = match index.query(&query, &span) {
        Ok(asked) => asked,
        Err(missing) => match &cx.vault().semantics.embedder {
            Some(embedder) => index.embed(cx.vault(), embedder, &query, &span)?,
            None => return Err(missing),
        },
    };

    let mut scored = Vec::new();
    let mut skipped = 0_usize;
    for card in cards {
        let Some(vector) = index.vectors.get(card.name()) else {
            // Absent from the index is absent from the ranking. Scoring it
            // zero would be indistinguishable from indexed and unrelated.
            skipped += 1;
            continue;
        };
        if vector.digest != digest(&card.document.text()) {
            // A stale vector is a wrong answer, not an impossible one, so it
            // ranks and says so. Skipping it would make an edited card vanish,
            // which a reader cannot tell from irrelevance.
            cx.warn(Warning::new(
                span.clone(),
                format!(
                    "`{}` has changed since the index was built, so its score is stale: \
                     rerun `{BUILD}`",
                    card.name()
                ),
            ));
        }
        let score = dot(&asked, &vector.values);
        scored.push((Value::Card(card), score));
    }
    if skipped > 0 {
        cx.warn(Warning::new(
            span.clone(),
            format!(
                "{skipped} card{} absent from the index and so from this ranking: rerun `{BUILD}`",
                if skipped == 1 { " is" } else { "s are" }
            ),
        ));
    }

    Ok(Value::Ranking(Arc::new(search::rank(
        scored,
        crate::types::TypeRef::CARD,
        Retrieval {
            query: query.to_string(),
            index: path.display().to_string(),
            model: index.model,
            revision: index.revision,
            metric: index.metric,
            // Exact brute force over every vector the corpus handed us.
            approximate: false,
        },
    ))))
}

/// One link of the semantic-search chain, as `hql doctor` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// The part checked: `vault`, `config`, `index`, `model`, `queries` or `cards`.
    pub part: &'static str,
    /// Whether the part is there and usable.
    pub ok: bool,
    /// What was found, or what is missing and the command to run next.
    pub detail: String,
}

/// Walk vault, configuration, index, model and query set in order, stopping
/// at the first part that is missing, so the last check names what to do.
///
/// A card the index has no current vector for is reported after the chain
/// holds: queries still answer, and the ranking says which cards it skipped.
#[must_use]
pub fn doctor(vault: &Vault) -> Vec<Check> {
    let mut checks = Vec::new();
    let mut check = |part, ok, detail: String| {
        checks.push(Check { part, ok, detail });
        ok
    };
    if !check(
        "vault",
        vault.is_present(),
        if vault.is_present() {
            format!("{} ({} cards)", vault.root.display(), vault.cards.len())
        } else {
            "no vault: pass --vault <directory>".to_owned()
        },
    ) {
        return checks;
    }
    let config = vault.root.join(crate::reporting::CONFIG_FILE);
    let path = match configured(vault, &(0..0)) {
        Ok(path) => {
            check(
                "config",
                true,
                format!("{} names {}", config.display(), path.display()),
            );
            path
        }
        Err(error) => {
            check("config", false, error.message());
            return checks;
        }
    };
    let index = match Index::open(&path, vault, &(0..0)) {
        Ok(index) => {
            check(
                "index",
                true,
                format!("{} ({} card vectors)", path.display(), index.vectors.len()),
            );
            index
        }
        Err(error) => {
            check("index", false, error.message());
            return checks;
        }
    };
    check(
        "model",
        true,
        format!(
            "{} at {}, {} dimensions, {}",
            index.model, index.revision, index.dimensions, index.metric
        ),
    );
    let queries: i64 = index
        .connection
        .query_row("SELECT count(*) FROM query", [], |row| row.get(0))
        .unwrap_or(0);
    if !check(
        "queries",
        queries > 0,
        if queries > 0 {
            format!("{queries} query vectors")
        } else {
            format!("the index holds no query vectors: rerun `{BUILD} --query \"…\"`")
        },
    ) {
        return checks;
    }
    let missing = vault
        .cards
        .iter()
        .filter(|card| {
            index
                .vectors
                .get(card.name())
                .is_none_or(|vector| vector.digest != digest(&card.document.text()))
        })
        .count();
    check(
        "cards",
        missing == 0,
        if missing == 0 {
            "every card has a current vector".to_owned()
        } else {
            format!(
                "{missing} card{} no current vector and rank{} as stale or not at all: rerun `{BUILD}`",
                if missing == 1 { " has" } else { "s have" },
                if missing == 1 { "s" } else { "" }
            )
        },
    );
    checks
}

/// The index path a vault configures, resolved inside the vault root.
fn configured(vault: &Vault, span: &Range<usize>) -> Result<PathBuf, Diagnostic> {
    let named = vault.semantics.index.as_deref().ok_or_else(|| {
        Diagnostic::name(
            span.clone(),
            format!(
                "`semantic` needs an index: name one with `[semantics] index` in \
                 {}, and build it with `{BUILD}`",
                crate::reporting::CONFIG_FILE
            ),
        )
    })?;
    within(&vault.root, Path::new(named)).ok_or_else(|| {
        Diagnostic::name(
            span.clone(),
            format!("the configured index `{named}` is outside the vault"),
        )
    })
}

/// A vault-relative path, refused if it climbs out of the vault.
fn within(root: &Path, named: &Path) -> Option<PathBuf> {
    if named.is_absolute() {
        return None;
    }
    let mut resolved = root.to_path_buf();
    for component in named.components() {
        match component {
            std::path::Component::Normal(part) => resolved.push(part),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(resolved)
}

/// One card's vector, and the text it was computed from.
struct Vector {
    digest: String,
    values: Vec<f32>,
}

/// An opened index, checked.
struct Index {
    model: String,
    revision: String,
    metric: String,
    dimensions: usize,
    vectors: BTreeMap<String, Vector>,
    connection: Connection,
}

impl Index {
    fn open(path: &Path, vault: &Vault, span: &Range<usize>) -> Result<Self, Diagnostic> {
        let fail = |message: String| Diagnostic::runtime(span.clone(), message);
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| {
                fail(format!(
                    "the index `{}` cannot be read ({error}): build it with `{BUILD}`",
                    path.display()
                ))
            })?;

        let (version, built_for): (i64, String) = connection
            .query_row("SELECT schema_version, vault FROM index_meta", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|error| fail(format!("the index has no readable metadata: {error}")))?;
        if version != SCHEMA_VERSION {
            return Err(fail(format!(
                "unknown index schema: {version}, and this reader knows {SCHEMA_VERSION}"
            )));
        }
        // An index built for another corpus ranks cards it has never seen.
        if !same_vault(&built_for, &vault.root) {
            return Err(fail(format!(
                "the index was built for `{built_for}` and this vault is `{}`: rerun `{BUILD}`",
                vault.root.display()
            )));
        }

        let (model, revision, dimensions, metric, normalized): (String, String, i64, String, i64) =
            connection
                .query_row(
                    "SELECT id, revision, dimensions, metric, normalized FROM model",
                    [],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .map_err(|error| fail(format!("the index names no model: {error}")))?;
        if dimensions <= 0 || dimensions > MAX_DIMENSIONS {
            return Err(fail(format!(
                "the index declares {dimensions} dimensions, and at most {MAX_DIMENSIONS} are read"
            )));
        }
        // Scoring is `dot`, which is cosine only for unit-length vectors. With
        // several embedders able to fill an index, the producer that normalises
        // and the reader that relies on it are no longer one decision, so the
        // precondition is checked rather than assumed. Normalising here instead
        // would be this binary computing vectors, which it does not do.
        if normalized == 0 {
            return Err(fail(format!(
                "the index declares its vectors are not normalized, and scoring is a \
                 dot product, which is {METRIC} only at unit length: rebuild it with \
                 `{BUILD}`"
            )));
        }
        // Reporting one metric through `Retrieval` while computing another is
        // the misdescribed evidence this extension exists to avoid.
        if metric != METRIC {
            return Err(fail(format!(
                "the index declares the metric `{metric}` and this reader computes \
                 `{METRIC}`"
            )));
        }
        let dimensions = usize::try_from(dimensions).unwrap_or_default();

        let mut statement = connection
            .prepare("SELECT name, content_hash, embedding FROM card")
            .map_err(|error| fail(format!("the index has no cards: {error}")))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|error| fail(format!("the index cannot be walked: {error}")))?;
        let mut vectors = BTreeMap::new();
        for row in rows {
            let (name, digest, blob) =
                row.map_err(|error| fail(format!("the index cannot be walked: {error}")))?;
            let values = floats(&blob, dimensions).ok_or_else(|| {
                fail(format!(
                    "`{name}` has a {}-byte vector where {} were declared",
                    blob.len(),
                    dimensions * 4
                ))
            })?;
            vectors.insert(name, Vector { digest, values });
        }
        drop(statement);

        Ok(Self {
            model,
            revision,
            metric,
            dimensions,
            vectors,
            connection,
        })
    }

    /// The vector for a query, which the producer computed.
    fn query(&self, text: &str, span: &Range<usize>) -> Result<Vec<f32>, Diagnostic> {
        let blob: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT embedding FROM query WHERE text = ?1",
                [text],
                |row| row.get(0),
            )
            .ok();
        let blob = blob.ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                format!(
                    "the index holds no vector for `{text}`, and this binary has no model to \
                     make one: rerun `{BUILD} --query \"{text}\"`"
                ),
            )
        })?;
        floats(&blob, self.dimensions).ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                format!(
                    "the vector for `{text}` is not {} bytes",
                    self.dimensions * 4
                ),
            )
        })
    }
}

impl Index {
    /// Embed a query the index holds no vector for, with the embedder that
    /// built it, through the harness's recordings and mode.
    fn embed(
        &self,
        vault: &Vault,
        embedder: &Embedder,
        text: &str,
        span: &Range<usize>,
    ) -> Result<Vec<f32>, Diagnostic> {
        let fail = |message: String| Diagnostic::runtime(span.clone(), message);
        // A vector from another model lies in another geometry, and its dot
        // product with these is a number that means nothing.
        if embedder.model != self.model || embedder.revision != self.revision {
            return Err(fail(format!(
                "the index was built by `{}` at `{}`, and [semantics.embedder] names `{}` at \
                 `{}`: a query embedded by another model cannot be scored against it",
                self.model, self.revision, embedder.model, embedder.revision
            )));
        }
        let model = crate::extensions::ai::transport::Model {
            api: crate::extensions::ai::transport::Api::Embeddings,
            endpoint: embedder.endpoint.clone(),
            model: embedder.model.clone(),
            api_key_env: embedder.api_key_env.clone(),
            max_tokens: 0,
            temperature: None,
            session: None,
        };
        let body = serde_json::json!({"input": [text], "model": embedder.model});
        let response = crate::extensions::ai::transport::send(
            &vault.root,
            &vault.ai,
            &model,
            &body,
        )
        .map_err(|message| {
            fail(format!(
                "the index holds no vector for `{text}`, and embedding it failed: {message}; \
                     or rerun `{BUILD} --query \"{text}\"`"
            ))
        })?;
        let values: Vec<f32> = response
            .pointer("/data/0/embedding")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| fail("the embeddings response holds no `data[0].embedding`".to_owned()))?
            .iter()
            .map(|value| value.as_f64().map(|number| number as f32))
            .collect::<Option<_>>()
            .ok_or_else(|| fail("an embedding holds a value that is not a number".to_owned()))?;
        if values.len() != self.dimensions {
            return Err(fail(format!(
                "the embedder returned {} dimensions and the index holds {}",
                values.len(),
                self.dimensions
            )));
        }
        // Scoring is a dot product, which is cosine only at unit length.
        let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm == 0.0 {
            return Err(fail("the embedder returned a zero vector".to_owned()));
        }
        Ok(values.into_iter().map(|value| value / norm).collect())
    }
}

/// An index built for a vault names it as the producer was given it, which may
/// be spelled differently from the path this run was given.
fn same_vault(built_for: &str, root: &Path) -> bool {
    let tail = |text: &str| {
        Path::new(text)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    built_for == root.to_string_lossy()
        || tail(built_for) == root.file_name().map(|n| n.to_string_lossy().into_owned())
}

/// Little-endian `f32`s, or `None` when the blob is not exactly that many.
fn floats(blob: &[u8], dimensions: usize) -> Option<Vec<f32>> {
    if blob.len() != dimensions * 4 {
        return None;
    }
    // `as_chunks` gives `[u8; 4]` directly, so the width is in the type rather
    // than in four index expressions that could disagree with it.
    Some(
        blob.as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(f32::from_le_bytes)
            .collect(),
    )
}

/// Cosine similarity, which is the dot product for unit-length vectors.
fn dot(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum()
}

/// The sha256 of a card's indexed text, lowercase hex, as the producer writes.
pub(crate) fn digest(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn check_semantic(
    cx: &mut dyn super::CheckCx,
    _: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    configured(cx.vault(), &span)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vault_relative_path_resolves_and_an_escaping_one_does_not() {
        let root = Path::new("/vault");
        assert_eq!(
            within(root, Path::new(".hql/index.sqlite")),
            Some(PathBuf::from("/vault/.hql/index.sqlite"))
        );
        assert_eq!(within(root, Path::new("../elsewhere.sqlite")), None);
        assert_eq!(within(root, Path::new("/etc/passwd")), None);
    }

    #[test]
    fn a_blob_is_read_only_at_the_declared_length() {
        assert_eq!(floats(&[0, 0, 0, 0], 1), Some(vec![0.0]));
        assert_eq!(floats(&[0, 0, 0], 1), None);
        assert_eq!(floats(&[0, 0, 0, 0, 0], 1), None);
    }

    #[test]
    fn an_http_embedder_is_read_for_the_queries_the_index_lacks() {
        // `[semantics.embedder]` says which model fills an index. This binary
        // reads an `http` one only to embed a query the index lacks; what a
        // score reports still comes from the index, never from this table.
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join(crate::reporting::CONFIG_FILE),
            "[semantics]\n\
             index = \".hql/index.sqlite\"\n\
             \n\
             [semantics.embedder]\n\
             provider = \"http\"\n\
             model = \"text-embedding-3-small\"\n\
             revision = \"2024-01-25\"\n\
             endpoint = \"https://api.openai.com/v1/embeddings\"\n\
             api_key_env = \"OPENAI_API_KEY\"\n\
             dimensions = 1536\n",
        )
        .expect("the configuration");
        assert_eq!(
            Semantics::read(directory.path()),
            Semantics {
                index: Some(".hql/index.sqlite".to_owned()),
                embedder: Some(Embedder {
                    model: "text-embedding-3-small".to_owned(),
                    revision: "2024-01-25".to_owned(),
                    endpoint: "https://api.openai.com/v1/embeddings".to_owned(),
                    api_key_env: Some("OPENAI_API_KEY".to_owned()),
                }),
            }
        );
    }

    #[test]
    fn the_digest_is_the_hash_the_producer_writes() {
        assert_eq!(
            digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
