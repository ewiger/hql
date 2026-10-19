//! How a request reaches a model: the vault's model table, the host's mode,
//! and the recordings that let a document answer again without a network.
//!
//! A request body is the unit of everything here. It is hashed in canonical
//! form to name its recording, so the same request answers the same way under
//! `replay`, and a recording whose request does not hash to its name is
//! refused rather than trusted.

use serde_json::Value as J;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The largest recording the reader accepts: 16 MiB.
const RECORDING_LIMIT: u64 = 16 * 1024 * 1024;

/// Whether a model request may reach the network, decided by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Answer from recordings only; a miss is a failure.
    #[default]
    Replay,
    /// Answer from a recording when one exists, else send and record.
    Record,
    /// Always send, never record.
    Live,
}

impl Mode {
    /// The mode a command-line word names.
    ///
    /// # Errors
    /// Names the three modes when the word is none of them.
    pub fn parse(word: &str) -> Result<Self, String> {
        match word {
            "replay" => Ok(Self::Replay),
            "record" => Ok(Self::Record),
            "live" => Ok(Self::Live),
            other => Err(format!(
                "unknown --ai mode `{other}`; expected replay, record or live"
            )),
        }
    }

    /// Whether this mode may write to the vault.
    #[must_use]
    pub fn writes(self) -> bool {
        self != Self::Replay
    }
}

/// The wire format a model is reached in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// The Anthropic Messages API.
    Messages,
    /// An OpenAI-shaped `/v1/chat/completions`.
    ChatCompletions,
}

impl Api {
    /// The word `hql.toml` writes for this API.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::ChatCompletions => "chat-completions",
        }
    }
}

/// One `[ai.models.<key>]` table.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The wire format.
    pub api: Api,
    /// Where requests are sent.
    pub endpoint: String,
    /// The model id sent in each request.
    pub model: String,
    /// The NAME of the variable holding the key, never the key.
    pub api_key_env: Option<String>,
    /// The most tokens one response may hold.
    pub max_tokens: i64,
    /// Sampling temperature, when the vault fixes one.
    pub temperature: Option<f64>,
}

/// What a vault's `hql.toml` says under `[ai]`, and the mode the host chose.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Models by key.
    pub models: BTreeMap<String, Model>,
    /// Where recordings live, relative to the vault root.
    pub recordings: PathBuf,
    /// Where `remember` writes, relative to the vault root.
    pub memory: Option<PathBuf>,
    /// Set by the host, never read from the file.
    pub mode: Mode,
    /// A refusal found while reading the table, reported at first use.
    pub problem: Option<String>,
}

impl Settings {
    /// Read `[ai]` from a vault's `hql.toml`. A missing file or table means
    /// no models; a malformed one is kept as a problem for first use.
    #[must_use]
    pub fn read(root: &Path) -> Self {
        let mut settings = Self {
            recordings: PathBuf::from(".hql/runs"),
            ..Self::default()
        };
        let Ok(text) = std::fs::read_to_string(root.join("hql.toml")) else {
            return settings;
        };
        let Ok(parsed) = toml::from_str::<toml::Value>(&text) else {
            return settings;
        };
        let Some(section) = parsed.get("ai").and_then(toml::Value::as_table) else {
            return settings;
        };
        if let Err(problem) = settings.parse(section) {
            settings.problem = Some(problem);
        }
        settings
    }

    fn parse(&mut self, section: &toml::Table) -> Result<(), String> {
        for (key, value) in section {
            match key.as_str() {
                "recordings" => {
                    self.recordings =
                        PathBuf::from(value.as_str().ok_or("`[ai] recordings` must be a path")?);
                }
                "mode" => {
                    return Err("`[ai] mode` is not read from hql.toml: whether a program \
                                may reach a model is the host's decision; pass --ai"
                        .to_owned());
                }
                "memory" => {
                    let table = value.as_table().ok_or("`[ai.memory]` must be a table")?;
                    for (key, value) in table {
                        if key != "dir" {
                            return Err(format!("unknown key `{key}` in [ai.memory]"));
                        }
                        self.memory = Some(PathBuf::from(
                            value.as_str().ok_or("`[ai.memory] dir` must be a path")?,
                        ));
                    }
                }
                "models" => {
                    let table = value.as_table().ok_or("`[ai.models]` must be a table")?;
                    for (name, model) in table {
                        let model = model
                            .as_table()
                            .ok_or_else(|| format!("`[ai.models.{name}]` must be a table"))?;
                        self.models.insert(name.clone(), parse_model(name, model)?);
                    }
                }
                other => return Err(format!("unknown key `{other}` in [ai]")),
            }
        }
        Ok(())
    }

    /// The model a key names.
    ///
    /// # Errors
    /// Names the key and the table it is missing from.
    pub fn model(&self, key: &str) -> Result<&Model, String> {
        if let Some(problem) = &self.problem {
            return Err(problem.clone());
        }
        self.models
            .get(key)
            .ok_or_else(|| format!("no model `{key}`: declare [ai.models.{key}] in hql.toml"))
    }
}

fn parse_model(name: &str, table: &toml::Table) -> Result<Model, String> {
    let text = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    for key in table.keys() {
        match key.as_str() {
            "api" | "endpoint" | "model" | "api_key_env" | "max_tokens" | "temperature" => {}
            "api_key" => {
                return Err(format!(
                    "[ai.models.{name}] may not hold a key: hql.toml is committed. \
                     Name the environment variable that holds it with `api_key_env`"
                ));
            }
            other => return Err(format!("unknown key `{other}` in [ai.models.{name}]")),
        }
    }
    let api = match text("api").as_deref() {
        Some("messages") => Api::Messages,
        Some("chat-completions") => Api::ChatCompletions,
        Some(other) => {
            return Err(format!(
                "unknown api `{other}` in [ai.models.{name}]; expected messages or chat-completions"
            ));
        }
        None => return Err(format!("[ai.models.{name}] needs `api`")),
    };
    Ok(Model {
        api,
        endpoint: text("endpoint").ok_or_else(|| format!("[ai.models.{name}] needs `endpoint`"))?,
        model: text("model").ok_or_else(|| format!("[ai.models.{name}] needs `model`"))?,
        api_key_env: text("api_key_env"),
        max_tokens: table
            .get("max_tokens")
            .and_then(toml::Value::as_integer)
            .unwrap_or(4096),
        temperature: table.get("temperature").and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|n| n as f64))
        }),
    })
}

/// The canonical text of a request: keys sorted, no whitespace.
#[must_use]
pub fn canonical(body: &J) -> String {
    fn sorted(value: &J) -> J {
        match value {
            J::Object(map) => {
                let ordered: BTreeMap<&String, J> = map
                    .iter()
                    .map(|(key, value)| (key, sorted(value)))
                    .collect();
                J::Object(
                    ordered
                        .into_iter()
                        .map(|(key, value)| (key.clone(), value))
                        .collect(),
                )
            }
            J::Array(items) => J::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    sorted(body).to_string()
}

/// The name a request's recording is stored under.
#[must_use]
pub fn key(body: &J) -> String {
    let digest = Sha256::digest(canonical(body).as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Send a request, or answer it from its recording, as the mode allows.
///
/// # Errors
/// A recording miss under `replay`, an unreadable or mismatched recording, a
/// missing credential, or a transport failure.
pub fn send(root: &Path, settings: &Settings, model: &Model, body: &J) -> Result<J, String> {
    let key = key(body);
    let directory = root.join(&settings.recordings);
    if !directory.starts_with(root) || settings.recordings.is_absolute() {
        return Err("`[ai] recordings` must resolve inside the vault".to_owned());
    }
    let path = directory.join(format!("{key}.json"));
    if settings.mode != Mode::Live
        && let Some(recorded) = recorded(&path, &key)?
    {
        return Ok(recorded);
    }
    if settings.mode == Mode::Replay {
        return Err(format!(
            "no recording for this request ({key}); run with --ai record"
        ));
    }
    let response = post(model, body)?;
    if settings.mode == Mode::Record {
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let recording = serde_json::json!({ "request": body, "response": response });
        let text = serde_json::to_string_pretty(&recording).map_err(|error| error.to_string())?;
        std::fs::write(&path, text + "\n")
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(response)
}

fn recorded(path: &Path, key: &str) -> Result<Option<J>, String> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(None);
    };
    if metadata.len() > RECORDING_LIMIT {
        return Err(format!("{}: a recording is at most 16 MiB", path.display()));
    }
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let recording: J =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let (Some(request), Some(response)) = (recording.get("request"), recording.get("response"))
    else {
        return Err(format!(
            "{}: a recording holds a request and a response",
            path.display()
        ));
    };
    if self::key(request) != key {
        return Err(format!(
            "{}: the recorded request does not hash to the recording's name",
            path.display()
        ));
    }
    Ok(Some(response.clone()))
}

fn post(model: &Model, body: &J) -> Result<J, String> {
    let key = match &model.api_key_env {
        Some(variable) => Some(std::env::var(variable).map_err(|_| {
            format!("the environment variable `{variable}` named by api_key_env is not set")
        })?),
        None => None,
    };
    let mut request = ureq::post(&model.endpoint).set("content-type", "application/json");
    request = match (model.api, &key) {
        (Api::Messages, Some(key)) => request
            .set("x-api-key", key)
            .set("anthropic-version", "2023-06-01"),
        (Api::Messages, None) => request.set("anthropic-version", "2023-06-01"),
        (Api::ChatCompletions, Some(key)) => request.set("authorization", &format!("Bearer {key}")),
        (Api::ChatCompletions, None) => request,
    };
    match request.send_json(body) {
        Ok(response) => response
            .into_json::<J>()
            .map_err(|error| format!("{}: unreadable response: {error}", model.endpoint)),
        // The body says why; the credential never appears in it.
        Err(ureq::Error::Status(code, response)) => Err(format!(
            "{} answered {code}: {}",
            model.endpoint,
            response.into_string().unwrap_or_default()
        )),
        Err(error) => Err(format!("{}: {error}", model.endpoint)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(text: &str) -> Settings {
        let mut settings = Settings::default();
        let table: toml::Table = toml::from_str(text).unwrap();
        if let Err(problem) = settings.parse(table["ai"].as_table().unwrap()) {
            settings.problem = Some(problem);
        }
        settings
    }

    #[test]
    fn a_key_written_into_the_file_is_refused_by_naming_where_it_goes() {
        let read = settings(
            "[ai.models.default]\napi = \"messages\"\nendpoint = \"x\"\nmodel = \"m\"\napi_key = \"sk\"",
        );
        assert!(read.model("default").unwrap_err().contains("api_key_env"));
    }

    #[test]
    fn the_mode_is_never_read_from_the_file() {
        let read = settings("[ai]\nmode = \"live\"");
        assert!(read.model("default").unwrap_err().contains("--ai"));
    }

    #[test]
    fn an_unknown_api_and_an_unknown_key_are_refused_by_name() {
        let read = settings("[ai.models.m]\napi = \"grpc\"\nendpoint = \"x\"\nmodel = \"m\"");
        assert!(read.model("m").unwrap_err().contains("grpc"));
        let read = settings(
            "[ai.models.m]\napi = \"messages\"\nendpoint = \"x\"\nmodel = \"m\"\nmax_token = 1",
        );
        assert!(read.model("m").unwrap_err().contains("max_token"));
    }

    #[test]
    fn a_request_hashes_the_same_whatever_its_key_order() {
        let one =
            serde_json::json!({"model": "m", "messages": [{"role": "user", "content": "hi"}]});
        let two =
            serde_json::json!({"messages": [{"content": "hi", "role": "user"}], "model": "m"});
        assert_eq!(key(&one), key(&two));
        assert_eq!(key(&one).len(), 64);
    }

    #[test]
    fn a_recording_whose_request_does_not_hash_to_its_name_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("x.json");
        std::fs::write(&path, r#"{"request": {"a": 1}, "response": {}}"#).unwrap();
        assert!(recorded(&path, "x").unwrap_err().contains("does not hash"));
    }

    #[test]
    fn a_miss_under_replay_names_the_mode_that_records() {
        let directory = tempfile::tempdir().unwrap();
        let settings = Settings {
            recordings: PathBuf::from(".hql/runs"),
            ..Settings::default()
        };
        let model = Model {
            api: Api::Messages,
            endpoint: "http://127.0.0.1:9".to_owned(),
            model: "m".to_owned(),
            api_key_env: None,
            max_tokens: 1,
            temperature: None,
        };
        let failure =
            send(directory.path(), &settings, &model, &serde_json::json!({})).unwrap_err();
        assert!(failure.contains("run with --ai record"), "{failure}");
    }
}
