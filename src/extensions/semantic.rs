//! The `semantic` extension: ranking against vectors a language model made.
//!
//! The model is not in this binary and never will be. `contrib/semantics/`
//! embeds a vault ahead of time into an SQLite index, and this module reads it.
//! That is why a query must be in the index too: a consumer that cannot embed a
//! document cannot embed a question either, and inventing a vector for one
//! would be the silent wrongness the whole split exists to avoid.
//!
//! The index is untrusted input. Everything read from it is checked: the schema
//! version, the vault it was built for, the dimension count, and the length of
//! every vector.

use super::collections::{elements, wrong};
use super::{CheckCx, EvalCx, Purity, Step, collection, named_or_first};
use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::document::Card;
use crate::search::{self, Retrieval};
use crate::types::Type;
use crate::values::Value;
use crate::vault::Vault;
use crate::warnings::Warning;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The only schema this reader knows.
pub const SCHEMA_VERSION: i64 = 1;
/// The largest vector width accepted, so a corrupt header cannot ask for an
/// allocation the size of the file.
const MAX_DIMENSIONS: i64 = 8192;
/// The command that produces an index, named by every failure that needs one.
const BUILD: &str = "hql-semantics build";

/// What a vault's `hql.toml` says about its index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Semantics {
    /// The index path, relative to the vault root.
    pub index: Option<String>,
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
        Self {
            index: parsed
                .get("semantics")
                .and_then(|section| section.get("index"))
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
        }
    }
}

/// Every step the `semantic` extension provides.
pub(crate) static STEPS: &[Step] = &[Step {
    name: "semantic",
    signature: "cards | semantic(\"night hunting birds\")",
    summary: "Rank a corpus against a query by the meaning a model found in both.",
    purity: Purity::Pure,
    check: check_semantic,
    eval: eval_semantic,
}];

fn check_semantic(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "semantic", &span)?;
    if !element.is(&Type::Doc) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!("`semantic` ranks documents, not {element}"),
        ));
    }
    let argument = named_or_first(arguments, "query").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`semantic` needs a query: `semantic(\"…\")`")
    })?;
    let query = cx.infer(&argument.value)?;
    if query != Type::Str {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("a query is text, not {query}"),
        ));
    }
    // The index is named by the vault, so a program that cannot reach one is
    // wrong before it runs rather than empty after it.
    configured(cx.vault(), &span)?;
    Ok(Type::Ranking(Box::new(Type::Card)))
}

fn eval_semantic(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "query").ok_or_else(|| wrong("semantic", "a query", &span))?;
    let Value::Str(query) = cx.evaluate(&argument.value)? else {
        return Err(wrong("semantic", "a text query", &span));
    };
    let elements = elements(&input, "semantic", &span)?;
    let cards: Vec<Rc<Card>> = elements.iter().filter_map(Value::as_card).collect();
    if cards.len() != elements.len() {
        return Err(wrong("semantic", "a collection of cards", &span));
    }

    let path = configured(cx.vault(), &span)?;
    let index = Index::open(&path, cx.vault(), &span)?;
    let asked = index.query(&query, &span)?;

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
        scored.push((card, score));
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

    Ok(Value::Ranking(Rc::new(search::rank(
        scored,
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

        let (model, revision, dimensions, metric): (String, String, i64, String) = connection
            .query_row(
                "SELECT id, revision, dimensions, metric FROM model",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| fail(format!("the index names no model: {error}")))?;
        if dimensions <= 0 || dimensions > MAX_DIMENSIONS {
            return Err(fail(format!(
                "the index declares {dimensions} dimensions, and at most {MAX_DIMENSIONS} are read"
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
    Some(
        blob.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
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
    fn the_digest_is_the_hash_the_producer_writes() {
        assert_eq!(
            digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
