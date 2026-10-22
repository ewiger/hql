//! Endpoints and the host's I/O: where a program's values come from and go
//! to, what the host grants, and what one evaluation has already written.
//!
//! An endpoint is a value and performs nothing; `source`, `tap`, `target` and
//! `sink` in [`crate::extensions::io`] use one. The I/O itself lives here, at
//! the edge, so the evaluator stays a function of its program and its inputs.
//! See [HQL-0021](../../doc/proposals/HQL-0021/README.md).

use crate::data::Data;
use crate::types::{Formatted, TypeConstructor, TypeRef, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Where values come from or go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// The program's standard input.
    Stdin,
    /// The program's standard output.
    Stdout,
    /// The program's standard error.
    Stderr,
    /// A whole file, by the path the program wrote.
    File(PathBuf),
    /// The end of a file: where an append goes and a follower waits.
    FileEnd(PathBuf),
    /// A channel: values of one type, held in memory for one execution scope.
    Channel(String, TypeRef),
    /// An HTTP resource: read by a `GET` once per evaluation, written by one
    /// request per document.
    Http {
        /// The resource.
        url: String,
        /// The method a write sends.
        method: String,
        /// Header names and their values, sent with every request.
        headers: BTreeMap<String, String>,
    },
    /// A WebSocket connection: one document per message.
    WebSocket(String),
    /// A TCP connection, carrying documents as their format separates them.
    Socket(String, u16),
}

impl Endpoint {
    /// The endpoint's type.
    #[must_use]
    pub fn type_of(&self) -> TypeRef {
        match self {
            Self::Stdin => TypeRef::STDIN,
            Self::Stdout => TypeRef::STDOUT,
            Self::Stderr => TypeRef::STDERR,
            Self::File(_) => TypeRef::FILE,
            Self::FileEnd(_) => TypeRef::FILE_END,
            Self::Channel(_, element) => crate::types::builtin::CHANNEL.apply([element.clone()]),
            Self::Http { .. } => TypeRef::HTTP,
            Self::WebSocket(_) => TypeRef::WEB_SOCKET,
            Self::Socket(..) => TypeRef::SOCKET,
        }
    }

    /// The path a file endpoint names, as written.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) | Self::FileEnd(path) => Some(path),
            _ => None,
        }
    }

    /// How a diagnostic names the endpoint.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Stdin => "<stdin>".to_owned(),
            Self::Stdout => "<stdout>".to_owned(),
            Self::Stderr => "<stderr>".to_owned(),
            Self::File(path) | Self::FileEnd(path) => path.display().to_string(),
            Self::Channel(name, _) => format!("channel {name:?}"),
            Self::Http { url, .. } | Self::WebSocket(url) => url.clone(),
            Self::Socket(host, port) => format!("{host}:{port}"),
        }
    }
}

impl fmt::Display for Endpoint {
    /// The literal that writes the endpoint.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdin => f.write_str("stdin"),
            Self::Stdout => f.write_str("stdout"),
            Self::Stderr => f.write_str("stderr"),
            Self::File(path) => write!(f, "file({:?})", path.display().to_string()),
            Self::FileEnd(path) => write!(f, "file({:?}).end", path.display().to_string()),
            Self::Channel(name, element) => write!(f, "Channel<{element}>({name:?})"),
            Self::Http {
                url,
                method,
                headers,
            } => {
                write!(f, "http({url:?}")?;
                if method != "GET" {
                    write!(f, ", method = {method:?}")?;
                }
                if !headers.is_empty() {
                    let entries: Vec<String> = headers
                        .iter()
                        .map(|(name, value)| format!("{name:?}: {value:?}"))
                        .collect();
                    write!(f, ", headers = {{{}}}", entries.join(", "))?;
                }
                f.write_str(")")
            }
            Self::WebSocket(url) => write!(f, "websocket({url:?})"),
            Self::Socket(host, port) => write!(f, "socket({host:?}, {port})"),
        }
    }
}

/// What a host grants a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Policy {
    /// `hql eval`, `hql run`, `hql repl`: every endpoint.
    Open,
    /// `hql render`: files under the document's directory are read, and
    /// nothing is written.
    Document {
        /// The directory reads must stay under.
        root: PathBuf,
    },
}

/// How one evaluation has written a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Written {
    Replaced,
    Appended,
}

#[derive(Debug)]
struct Host {
    policy: Policy,
    base: PathBuf,
    written: HashMap<PathBuf, Written>,
    /// Each channel of the scope, by name: its element type and its values in
    /// the order they were written.
    channels: BTreeMap<String, (TypeRef, Vec<Value>)>,
    /// Every logger's threshold, when the host overrides it.
    log_level: Option<String>,
    /// Each HTTP body read this evaluation, by URL: a resource is fetched once.
    fetched: HashMap<String, Arc<Vec<u8>>>,
    /// Whether the program itself was read from standard input, which then
    /// holds no data.
    program_on_stdin: bool,
    /// The format `--from` names for standard input, read where a program
    /// names none.
    stdin_format: Option<TypeConstructor>,
}

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host {
        policy: Policy::Open,
        base: PathBuf::from("."),
        written: HashMap::new(),
        channels: BTreeMap::new(),
        log_level: None,
        fetched: HashMap::new(),
        program_on_stdin: false,
        stdin_format: None,
    });
}

/// Set what the host grants, and the directory relative paths resolve
/// against: the document, the program file, or the working directory.
pub fn set_host(policy: Policy, base: PathBuf) {
    HOST.with(|host| {
        let mut host = host.borrow_mut();
        host.policy = policy;
        host.base = base;
        host.program_on_stdin = false;
    });
}

/// Say that the program was read from standard input, so `source(stdin, …)`
/// in it is refused.
pub fn set_program_on_stdin() {
    HOST.with(|host| host.borrow_mut().program_on_stdin = true);
}

/// Whether the program was read from standard input.
#[must_use]
pub fn program_on_stdin() -> bool {
    HOST.with(|host| host.borrow().program_on_stdin)
}

/// Name the format standard input is read in where a program names none:
/// `--from json`.
pub fn set_stdin_format(format: Option<TypeConstructor>) {
    HOST.with(|host| host.borrow_mut().stdin_format = format);
}

/// The format `--from` named for standard input, if any.
#[must_use]
pub fn stdin_format() -> Option<TypeConstructor> {
    HOST.with(|host| host.borrow().stdin_format)
}

/// The most values one channel holds in a scope.
const CHANNEL_BOUND: usize = 1_000_000;

/// Override every logger's threshold: `trace`, `debug`, `info`, `warn` or
/// `error`.
pub fn set_log_level(level: Option<String>) {
    HOST.with(|host| host.borrow_mut().log_level = level);
}

/// The threshold the host imposes on every logger, if any.
#[must_use]
pub fn log_level() -> Option<String> {
    HOST.with(|host| host.borrow().log_level.clone())
}

/// What the host grants.
#[must_use]
pub fn policy() -> Policy {
    HOST.with(|host| host.borrow().policy.clone())
}

/// Start an evaluation: nothing has been written yet, and every channel is
/// empty.
pub(crate) fn begin() {
    HOST.with(|host| {
        let mut host = host.borrow_mut();
        host.written.clear();
        host.channels.clear();
        host.fetched.clear();
    });
}

/// Open a channel by name, refusing a name the scope holds at another type.
///
/// # Errors
/// Names both element types.
pub(crate) fn open_channel(name: &str, element: &TypeRef) -> Result<(), String> {
    HOST.with(|host| {
        let mut host = host.borrow_mut();
        match host.channels.get(name) {
            Some((held, _)) if held != element => Err(format!(
                "the channel {name:?} holds {held}, so it is not also a Channel<{element}>"
            )),
            Some(_) => Ok(()),
            None => {
                host.channels
                    .insert(name.to_owned(), (element.clone(), Vec::new()));
                Ok(())
            }
        }
    })
}

/// Append a value to a channel.
///
/// # Errors
/// Names the channel when it already holds as many values as a scope allows.
pub(crate) fn push(name: &str, value: Value) -> Result<(), String> {
    HOST.with(|host| {
        if let Some((_, values)) = host.borrow_mut().channels.get_mut(name) {
            if values.len() >= CHANNEL_BOUND {
                return Err(format!(
                    "the channel {name:?} holds {CHANNEL_BOUND} values, as many as one scope keeps"
                ));
            }
            values.push(value);
        }
        Ok(())
    })
}

/// What a channel holds now, in write order.
pub(crate) fn held(name: &str) -> Vec<Value> {
    HOST.with(|host| {
        host.borrow()
            .channels
            .get(name)
            .map(|(_, values)| values.clone())
            .unwrap_or_default()
    })
}

/// The value a channel holds at a position, if one has been written there.
fn held_at(name: &str, index: usize) -> Option<Value> {
    HOST.with(|host| {
        host.borrow()
            .channels
            .get(name)
            .and_then(|(_, values)| values.get(index).cloned())
    })
}

/// The channel a cell's logs are written to, opened for the scope.
#[must_use]
pub fn cell_logs() -> Endpoint {
    let _ = open_channel(CELL_LOGS, &TypeRef::LOG_EVENT);
    Endpoint::Channel(CELL_LOGS.to_owned(), TypeRef::LOG_EVENT)
}

/// The name of a cell's logs channel.
pub const CELL_LOGS: &str = "cell.logs";

/// Every channel the scope wrote, by name, for the host to show beside the
/// result.
#[must_use]
pub fn channels() -> Vec<(String, Vec<Value>)> {
    HOST.with(|host| {
        host.borrow()
            .channels
            .iter()
            .map(|(name, (_, values))| (name.clone(), values.clone()))
            .collect()
    })
}

/// A path as the host reads it: relative to the source unit.
fn resolved(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    HOST.with(|host| host.borrow().base.join(path))
}

/// A file a read may open, under the host's policy.
fn readable(path: &Path) -> Result<PathBuf, String> {
    let full = resolved(path);
    if let Policy::Document { root } = policy() {
        let inside = fs::canonicalize(&full)
            .ok()
            .zip(fs::canonicalize(&root).ok())
            .is_some_and(|(file, root)| file.starts_with(root));
        if !inside {
            return Err(format!(
                "{}: a document reads only files under its own directory",
                path.display()
            ));
        }
    }
    if HOST.with(|host| host.borrow().written.get(&full).copied()) == Some(Written::Replaced) {
        return Err(format!(
            "{}: this evaluation has replaced the file, so reading it again would see its own \
             output",
            path.display()
        ));
    }
    Ok(full)
}

/// Record a write, refusing a second replacement or a replacement mixed with
/// an append.
fn record(path: &Path, how: Written) -> Result<PathBuf, String> {
    let full = resolved(path);
    HOST.with(|host| {
        let mut host = host.borrow_mut();
        match (host.written.get(&full), how) {
            (None, _) | (Some(Written::Appended), Written::Appended) => {
                host.written.insert(full.clone(), how);
                Ok(full)
            }
            (Some(Written::Replaced), Written::Replaced) => Err(format!(
                "{}: this evaluation has already replaced the file; append to `file(p).end`, or \
                 write it once",
                path.display()
            )),
            (Some(_), _) => Err(format!(
                "{}: one evaluation either replaces a file or appends to its end, not both",
                path.display()
            )),
        }
    })
}

/// The format a path's extension implies.
#[must_use]
pub fn format_of_extension(path: &Path) -> Option<TypeConstructor> {
    let extension = path.extension()?.to_str()?;
    let format = match extension {
        "json" => "Json",
        "jsonl" | "ndjson" => "JsonLines",
        "yaml" | "yml" => "Yaml",
        "csv" => "Csv",
        // A plain text file is read as lines unless a format says otherwise.
        "txt" | "log" => "Lines",
        "md" => "Markdown",
        "hmd" => "Hmd",
        "hql" => "Hql",
        "d2" => "D2",
        "mmd" => "Mermaid",
        _ => return None,
    };
    Some(TypeConstructor(format))
}

/// The type one document of a format reads as, or why it is not read.
///
/// # Errors
/// Names a format that describes a whole picture and is written only.
pub fn document_type(format: TypeConstructor) -> Result<TypeRef, String> {
    match format.0 {
        "Json" | "JsonLines" | "Yaml" | "Csv" => Ok(TypeRef::DATA),
        // Text is a String; a document in a text format is its text.
        "Text" | "Lines" | "Markdown" | "Hmd" | "Hql" => Ok(TypeRef::STR),
        other => Err(format!("{other} is written, not read")),
    }
}

/// Whether a format's documents can be separated, so read one at a time.
/// `Text` is one whole document; `Lines` frames it one record per line.
fn separable(format: TypeConstructor) -> bool {
    matches!(format.0, "Json" | "JsonLines" | "Yaml" | "Csv" | "Lines")
}

/// A tree read from JSON or YAML.
fn data_of(json: serde_json::Value) -> Data {
    match json {
        serde_json::Value::Null => Data::Empty,
        serde_json::Value::Bool(value) => Data::Bool(value),
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(Data::Int)
            .or_else(|| number.as_f64().map(Data::Float))
            .unwrap_or(Data::Empty),
        serde_json::Value::String(text) => Data::Str(text),
        serde_json::Value::Array(items) => Data::List(items.into_iter().map(data_of).collect()),
        serde_json::Value::Object(entries) => Data::Map(
            entries
                .into_iter()
                .map(|(key, value)| (key, data_of(value)))
                .collect(),
        ),
    }
}

fn tree(json: serde_json::Value) -> Value {
    Value::Data(Arc::new(data_of(json)))
}

/// One CSV record: the fields of a line, with quoted fields that may hold
/// commas, doubled quotes and line breaks. Answers `None` at the end.
fn csv_record(
    lines: &mut dyn Iterator<Item = std::io::Result<String>>,
) -> Option<Result<Vec<String>, String>> {
    let mut line = match lines.next()? {
        Ok(line) => line,
        Err(error) => return Some(Err(error.to_string())),
    };
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    loop {
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, quoted) {
                ('"', true) if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                ('"', true) => quoted = false,
                ('"', false) if field.is_empty() => quoted = true,
                (',', false) => fields.push(std::mem::take(&mut field)),
                (c, _) => field.push(c),
            }
        }
        if !quoted {
            break;
        }
        // A quoted field runs on to the next line.
        field.push('\n');
        line = match lines.next() {
            Some(Ok(next)) => next,
            Some(Err(error)) => return Some(Err(error.to_string())),
            None => return Some(Err("a quoted field is not closed".to_owned())),
        };
    }
    fields.push(field);
    Some(Ok(fields))
}

fn row(header: &[String], fields: Vec<String>) -> Value {
    let map: BTreeMap<String, Data> = header
        .iter()
        .cloned()
        .zip(fields.into_iter().map(Data::Str))
        .collect();
    Value::Data(Arc::new(Data::Map(map)))
}

/// Every document of a configured file source, in file order. `spread`
/// reads a Json file as the elements of the one array it holds.
/// Configuration is the access boundary, so the document policy that
/// confines a document's own reads does not apply.
pub(crate) fn read_configured(
    path: &Path,
    format: TypeConstructor,
    spread: bool,
) -> Result<Vec<Value>, String> {
    let file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if spread {
        let json: serde_json::Value = serde_json::from_reader(BufReader::new(file))
            .map_err(|error| format!("{}: not JSON: {error}", path.display()))?;
        let serde_json::Value::Array(elements) = json else {
            return Err(format!(
                "{}: a `json` source holds one array, whose elements it reads",
                path.display()
            ));
        };
        return Ok(elements.into_iter().map(tree).collect());
    }
    pieces(Box::new(file), format, path.display().to_string()).collect()
}

/// Documents read one at a time from a reader.
type Pieces = Box<dyn Iterator<Item = Result<Value, String>> + Send>;

/// The documents of a reader, one at a time, as `format` separates them.
fn pieces(reader: Box<dyn Read + Send>, format: TypeConstructor, origin: String) -> Pieces {
    let located = move |error: String| format!("{origin}: {error}");
    match format.0 {
        "Json" => Box::new(
            serde_json::Deserializer::from_reader(reader)
                .into_iter::<serde_json::Value>()
                .map(move |document| {
                    document.map(tree).map_err(|error| {
                        located(format!(
                            "not JSON at line {}, column {}: {error}",
                            error.line(),
                            error.column()
                        ))
                    })
                }),
        ),
        // Line framing: the terminator belongs to the representation, so a
        // record is the line without it.
        "Lines" => Box::new(BufReader::new(reader).lines().map(move |line| {
            line.map(|line| Value::Str(line.into()))
                .map_err(|error| located(error.to_string()))
        })),
        // One JSON document per line: the line is the frame.
        "JsonLines" => Box::new(BufReader::new(reader).lines().enumerate().filter_map(
            move |(index, line)| {
                match line {
                    Err(error) => Some(Err(located(error.to_string()))),
                    Ok(line) if line.trim().is_empty() => None,
                    Ok(line) => Some(
                        serde_json::from_str::<serde_json::Value>(&line)
                            .map(tree)
                            .map_err(|error| {
                                located(format!("not JSON at line {}: {error}", index + 1))
                            }),
                    ),
                }
            },
        )),
        "Csv" => {
            let mut lines = BufReader::new(reader).lines();
            let header = csv_record(&mut lines);
            match header {
                None => Box::new(std::iter::empty()),
                Some(Err(error)) => Box::new(std::iter::once(Err(located(error)))),
                Some(Ok(header)) => Box::new(std::iter::from_fn(move || {
                    loop {
                        let record = csv_record(&mut lines)?;
                        return Some(match record {
                            // A blank line separates nothing.
                            Ok(fields) if fields.len() == 1 && fields[0].is_empty() => continue,
                            Ok(fields) => Ok(row(&header, fields)),
                            Err(error) => Err(located(error)),
                        });
                    }
                })),
            }
        }
        // YAML documents are split by `---`, which the parser finds only in
        // the whole text, and a whole-text format is one document.
        _ => {
            let mut text = String::new();
            let mut reader = reader;
            if let Err(error) = reader.read_to_string(&mut text) {
                return Box::new(std::iter::once(Err(located(error.to_string()))));
            }
            let documents: Vec<Result<Value, String>> = whole(&text, format)
                .unwrap_or_else(|error| vec![Err(error)])
                .into_iter()
                .map(|document| document.map_err(&located))
                .collect();
            Box::new(documents.into_iter())
        }
    }
}

/// Text parsed in a format whose decoder reads `Data`: the one document of a
/// whole-text format, Json or Yaml, or the list of the records of a framed
/// one, JsonLines or Csv.
///
/// # Errors
/// Text that does not parse in the format, or a whole-text format holding
/// other than one document.
pub(crate) fn parse_text(text: &str, format: TypeConstructor) -> Result<Data, String> {
    let reader = Box::new(std::io::Cursor::new(text.as_bytes().to_vec()));
    let documents = pieces(reader, format, format!("Data(…, {format})"))
        .map(|document| match document? {
            Value::Data(data) => Ok(data.as_ref().clone()),
            Value::Str(text) => Ok(Data::Str(text.to_string())),
            other => Err(format!("{format} decodes no Data here, but {other:?}")),
        })
        .collect::<Result<Vec<Data>, String>>()?;
    match format.0 {
        "JsonLines" | "Csv" => Ok(Data::List(documents)),
        _ => {
            let count = documents.len();
            let mut documents = documents.into_iter();
            match (documents.next(), count) {
                (Some(document), 1) => Ok(document),
                _ => Err(format!(
                    "the text holds {count} {format} documents, and `Data(s, {format})` parses one"
                )),
            }
        }
    }
}

/// The documents of a whole text in a format that is not read piece by piece.
fn whole(text: &str, format: TypeConstructor) -> Result<Vec<Result<Value, String>>, String> {
    use serde::Deserialize;
    match format.0 {
        "Yaml" => Ok(serde_yaml_ng::Deserializer::from_str(text)
            .map(|document| {
                serde_json::Value::deserialize(document)
                    .map(tree)
                    .map_err(|error| format!("not YAML: {error}"))
            })
            .collect()),
        _ => Ok(vec![Ok(Value::Formatted(Arc::new(Formatted {
            text: text.to_owned(),
            format,
        })))]),
    }
}

/// Every document a replayable endpoint holds, in order.
///
/// # Errors
/// Names the file, and the place in it, when it cannot be read or decoded.
pub(crate) fn read_all(endpoint: &Endpoint, format: TypeConstructor) -> Result<Vec<Value>, String> {
    match endpoint {
        Endpoint::File(path) => {
            let full = readable(path)?;
            let file =
                fs::File::open(&full).map_err(|error| format!("{}: {error}", path.display()))?;
            pieces(Box::new(file), format, path.display().to_string()).collect()
        }
        Endpoint::Http { url, headers, .. } => {
            let body = fetched(url, headers)?;
            pieces(
                Box::new(std::io::Cursor::new(body.as_ref().clone())),
                format,
                url.clone(),
            )
            .collect()
        }
        other => Err(format!("{} is not read as a whole", other.describe())),
    }
}

/// The body of an HTTP resource, fetched once per evaluation.
fn fetched(url: &str, headers: &BTreeMap<String, String>) -> Result<Arc<Vec<u8>>, String> {
    if let Policy::Document { .. } = policy() {
        return Err(format!("{url}: a document reads nothing from the network"));
    }
    if let Some(body) = HOST.with(|host| host.borrow().fetched.get(url).cloned()) {
        return Ok(body);
    }
    let body = Arc::new(crate::net::fetch(url, headers)?);
    HOST.with(|host| {
        host.borrow_mut()
            .fetched
            .insert(url.to_owned(), Arc::clone(&body));
    });
    Ok(body)
}

/// A connection not yet made.
type Connect = Box<dyn FnOnce() -> Result<Box<dyn Read + Send>, String> + Send>;

/// A reader that connects when it is first read, so a stream that is never
/// traversed opens nothing.
struct Lazy {
    connect: Option<Connect>,
    inner: Option<Box<dyn Read + Send>>,
}

impl Read for Lazy {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(connect) = self.connect.take() {
            self.inner = Some(connect().map_err(std::io::Error::other)?);
        }
        match self.inner.as_mut() {
            Some(inner) => inner.read(buffer),
            None => Ok(0),
        }
    }
}

/// The messages of a WebSocket, each decoded as the documents it holds.
fn messages(url: String, format: TypeConstructor) -> Pieces {
    let mut connection: Option<crate::net::Connection> = None;
    let mut ready: std::collections::VecDeque<Result<Value, String>> =
        std::collections::VecDeque::new();
    let mut ended = false;
    Box::new(std::iter::from_fn(move || {
        loop {
            if let Some(document) = ready.pop_front() {
                return Some(document);
            }
            if ended {
                return None;
            }
            if connection.is_none() {
                match crate::net::websocket(&url) {
                    Ok(opened) => connection = Some(opened),
                    Err(error) => {
                        ended = true;
                        return Some(Err(error));
                    }
                }
            }
            let open = connection.as_mut()?;
            match crate::net::message(open, &url) {
                None => ended = true,
                Some(Err(error)) => {
                    ended = true;
                    return Some(Err(error));
                }
                Some(Ok(text)) => ready.extend(pieces(
                    Box::new(std::io::Cursor::new(text.into_bytes())),
                    format,
                    url.clone(),
                )),
            }
        }
    }))
}

/// Where a stream of documents comes from.
pub(crate) enum Reading {
    /// Standard input, read once: what has been read is kept, so a second use
    /// of one binding sees the same documents.
    Shared(Mutex<Shared>),
    /// A file, opened afresh on each traversal and read one document at a
    /// time.
    File {
        path: PathBuf,
        format: TypeConstructor,
    },
    /// The end of a file: documents appended after the traversal began.
    Follow {
        path: PathBuf,
        format: TypeConstructor,
    },
    /// A channel's values written from a position on.
    Channel { name: String, from: usize },
    /// An HTTP resource's body, fetched once and read one document at a time.
    Http {
        url: String,
        headers: BTreeMap<String, String>,
        format: TypeConstructor,
    },
}

/// What standard input has given so far.
pub(crate) struct Shared {
    seen: Vec<Value>,
    rest: Option<Pieces>,
}

impl fmt::Debug for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shared(_) => f.write_str("Reading::Shared(<stdin>)"),
            Self::File { path, format } => {
                write!(f, "Reading::File({}, {})", path.display(), format.0)
            }
            Self::Follow { path, format } => {
                write!(f, "Reading::Follow({}, {})", path.display(), format.0)
            }
            Self::Channel { name, from } => write!(f, "Reading::Channel({name:?}, {from})"),
            Self::Http { url, format, .. } => write!(f, "Reading::Http({url}, {})", format.0),
        }
    }
}

impl Reading {
    /// Standard input in a format, read when a traversal first asks.
    pub(crate) fn stdin(format: TypeConstructor) -> Self {
        Self::shared(pieces(
            Box::new(std::io::stdin()),
            format,
            "<stdin>".to_owned(),
        ))
    }

    /// A live endpoint other than standard input, connected when a traversal
    /// first asks, and read once however often the stream is traversed.
    ///
    /// # Errors
    /// Names an endpoint that is not live.
    pub(crate) fn live(endpoint: &Endpoint, format: TypeConstructor) -> Result<Self, String> {
        if let Policy::Document { .. } = policy() {
            return Err(format!(
                "{}: a document reads nothing from the network",
                endpoint.describe()
            ));
        }
        match endpoint {
            Endpoint::Stdin => Ok(Self::stdin(format)),
            Endpoint::Socket(host, port) => {
                let (host, port) = (host.clone(), *port);
                let reader = Lazy {
                    connect: Some(Box::new(move || {
                        crate::net::socket(&host, port)
                            .map(|stream| Box::new(stream) as Box<dyn Read + Send>)
                    })),
                    inner: None,
                };
                Ok(Self::shared(pieces(
                    Box::new(reader),
                    format,
                    endpoint.describe(),
                )))
            }
            Endpoint::WebSocket(url) => Ok(Self::shared(messages(url.clone(), format))),
            other => Err(format!("{} is not read as it arrives", other.describe())),
        }
    }

    fn shared(rest: Pieces) -> Self {
        Self::Shared(Mutex::new(Shared {
            seen: Vec::new(),
            rest: Some(rest),
        }))
    }

    /// Drive the documents into `sink`, which answers whether it wants
    /// another. Answers whether the documents ran out with the sink still
    /// wanting more.
    ///
    /// # Errors
    /// The first document that cannot be read, or the sink's own failure.
    pub(crate) fn drive<E>(
        &self,
        sink: &mut dyn FnMut(Value) -> Result<bool, E>,
        failed: impl Fn(String) -> E,
    ) -> Result<bool, E> {
        match self {
            Self::Shared(shared) => {
                let mut index = 0;
                loop {
                    let next = {
                        let mut shared = shared
                            .lock()
                            .map_err(|_| failed("standard input was poisoned".to_owned()))?;
                        if let Some(value) = shared.seen.get(index) {
                            Some(value.clone())
                        } else {
                            match shared.rest.as_mut().and_then(Iterator::next) {
                                Some(Ok(value)) => {
                                    shared.seen.push(value.clone());
                                    Some(value)
                                }
                                Some(Err(error)) => return Err(failed(error)),
                                None => {
                                    shared.rest = None;
                                    None
                                }
                            }
                        }
                    };
                    let Some(value) = next else {
                        return Ok(true);
                    };
                    index += 1;
                    if !sink(value)? {
                        return Ok(false);
                    }
                }
            }
            Self::File { path, format } => {
                let full = readable(path).map_err(&failed)?;
                let file = fs::File::open(&full)
                    .map_err(|error| failed(format!("{}: {error}", path.display())))?;
                for document in pieces(Box::new(file), *format, path.display().to_string()) {
                    if !sink(document.map_err(&failed)?)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Follow { path, format } => follow(path, *format, sink, failed),
            Self::Http {
                url,
                headers,
                format,
            } => {
                let body = fetched(url, headers).map_err(&failed)?;
                let reader = std::io::Cursor::new(body.as_ref().clone());
                for document in pieces(Box::new(reader), *format, url.clone()) {
                    if !sink(document.map_err(&failed)?)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            // The scope runs one evaluation at a time, so what has not been
            // written when the reader catches up is not written while it waits.
            Self::Channel { name, from } => {
                let mut index = *from;
                while let Some(value) = held_at(name, index) {
                    index += 1;
                    if !sink(value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

/// Wait at the end of a file and give each document appended after the
/// follow began, as `tail -F` does. Never ends by itself.
fn follow<E>(
    path: &Path,
    format: TypeConstructor,
    sink: &mut dyn FnMut(Value) -> Result<bool, E>,
    failed: impl Fn(String) -> E,
) -> Result<bool, E> {
    let full = readable_or_missing(path).map_err(&failed)?;
    let pause = Duration::from_millis(50);
    // The header of a CSV file is its first line, wherever the follow starts.
    let mut header: Option<Vec<String>> = None;
    let mut offset = fs::metadata(&full).map(|meta| meta.len()).ok();
    let mut pending = String::new();
    loop {
        let Ok(mut file) = fs::File::open(&full) else {
            std::thread::sleep(pause);
            continue;
        };
        let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        let start = match offset {
            // A file that shrank was replaced: read it from the start.
            Some(at) if at <= length => at,
            _ => 0,
        };
        if format.0 == "Csv" && header.is_none() {
            let mut first = String::new();
            let _ =
                BufReader::new(fs::File::open(&full).map_err(|error| failed(error.to_string()))?)
                    .read_line(&mut first);
            if !first.is_empty() {
                let mut lines =
                    std::iter::once(Ok(first.trim_end_matches(['\r', '\n']).to_owned()));
                header = csv_record(&mut lines).and_then(Result::ok);
            }
        }
        if length > start {
            file.seek(SeekFrom::Start(start))
                .map_err(|error| failed(error.to_string()))?;
            let mut added = String::new();
            file.read_to_string(&mut added)
                .map_err(|error| failed(format!("{}: {error}", path.display())))?;
            offset = Some(length);
            pending.push_str(&added);
            for value in complete(&mut pending, format, header.as_deref(), start == 0)
                .map_err(|error| failed(format!("{}: {error}", path.display())))?
            {
                if !sink(value)? {
                    return Ok(false);
                }
            }
        } else {
            offset = Some(length);
        }
        std::thread::sleep(pause);
    }
}

/// A path that may not exist yet, under the host's policy.
fn readable_or_missing(path: &Path) -> Result<PathBuf, String> {
    let full = resolved(path);
    if full.exists() {
        readable(path)
    } else if let Policy::Document { .. } = policy() {
        Err(format!(
            "{}: a document reads only files under its own directory",
            path.display()
        ))
    } else {
        Ok(full)
    }
}

/// Take the complete documents off the front of `pending`, leaving a partial
/// one for the next read.
fn complete(
    pending: &mut String,
    format: TypeConstructor,
    header: Option<&[String]>,
    from_start: bool,
) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    match format.0 {
        "Json" | "JsonLines" => {
            let mut stream =
                serde_json::Deserializer::from_str(pending).into_iter::<serde_json::Value>();
            let mut consumed = 0;
            loop {
                match stream.next() {
                    Some(Ok(value)) => {
                        consumed = stream.byte_offset();
                        values.push(tree(value));
                    }
                    Some(Err(error)) if error.is_eof() => break,
                    Some(Err(error)) => return Err(format!("not JSON: {error}")),
                    None => break,
                }
            }
            pending.drain(..consumed);
        }
        _ => {
            let Some(end) = pending.rfind('\n') else {
                return Ok(values);
            };
            let ready: String = pending.drain(..=end).collect();
            let mut lines = ready.lines().map(|line| Ok(line.to_owned()));
            if format.0 == "Csv" {
                if from_start {
                    // The header is the first line, already read.
                    lines.next();
                }
                let header = header.unwrap_or_default();
                while let Some(record) = csv_record(&mut lines) {
                    values.push(row(header, record?));
                }
            } else {
                for line in lines {
                    values.push(Value::Str(
                        line.map_err(|error: std::io::Error| error.to_string())?
                            .into(),
                    ));
                }
            }
        }
    }
    Ok(values)
}

/// Where written text goes.
pub(crate) enum Destination {
    Stdout,
    Stderr,
    File(fs::File),
    /// A channel takes values, not text.
    Channel(String),
    /// One request per document.
    Http {
        url: String,
        method: String,
        headers: BTreeMap<String, String>,
    },
    /// One line per document, as the format separates them.
    Socket(std::net::TcpStream),
    /// One message per document.
    WebSocket(String, Box<crate::net::Connection>),
}

impl Destination {
    /// Open an endpoint to be written: a whole file is replaced, its end is
    /// appended to. Answers whether the destination already holds documents,
    /// so a header is not written twice.
    ///
    /// # Errors
    /// Names the endpoint when the host refuses it or it cannot be opened.
    pub(crate) fn open(endpoint: &Endpoint) -> Result<(Self, bool), String> {
        if let Endpoint::Channel(name, _) = endpoint {
            return Ok((Self::Channel(name.clone()), false));
        }
        if let Policy::Document { .. } = policy() {
            return Err(format!(
                "{}: a document writes nothing outside itself",
                endpoint.describe()
            ));
        }
        match endpoint {
            Endpoint::Stdout => Ok((Self::Stdout, false)),
            Endpoint::Stderr => Ok((Self::Stderr, false)),
            Endpoint::Stdin => Err("standard input is not written".to_owned()),
            Endpoint::Channel(name, _) => Ok((Self::Channel(name.clone()), false)),
            Endpoint::File(path) => {
                let full = record(path, Written::Replaced)?;
                let file = fs::File::create(&full)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                Ok((Self::File(file), false))
            }
            Endpoint::Http {
                url,
                method,
                headers,
            } => Ok((
                Self::Http {
                    url: url.clone(),
                    method: method.clone(),
                    headers: headers.clone(),
                },
                false,
            )),
            Endpoint::Socket(host, port) => {
                Ok((Self::Socket(crate::net::socket(host, *port)?), false))
            }
            Endpoint::WebSocket(url) => Ok((
                Self::WebSocket(url.clone(), Box::new(crate::net::websocket(url)?)),
                false,
            )),
            Endpoint::FileEnd(path) => {
                let full = record(path, Written::Appended)?;
                let file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&full)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                let held = file.metadata().map(|meta| meta.len() > 0).unwrap_or(false);
                Ok((Self::File(file), held))
            }
        }
    }

    /// Write one document and a line break, at once.
    ///
    /// # Errors
    /// The write's failure.
    pub(crate) fn line(&mut self, text: &str) -> Result<(), String> {
        let result = match self {
            Self::Stdout => {
                let mut out = std::io::stdout();
                writeln!(out, "{text}").and_then(|()| out.flush())
            }
            Self::Stderr => {
                let mut err = std::io::stderr();
                writeln!(err, "{text}").and_then(|()| err.flush())
            }
            Self::File(file) => writeln!(file, "{text}").and_then(|()| file.flush()),
            Self::Channel(name) => {
                return Err(format!("the channel {name:?} takes values, not text"));
            }
            Self::Http {
                url,
                method,
                headers,
            } => return crate::net::send(url, method, headers, text),
            Self::Socket(stream) => return crate::net::line(stream, text),
            Self::WebSocket(url, connection) => return crate::net::post(connection, url, text),
        };
        result.map_err(|error| error.to_string())
    }
}

/// Write one completed document to an endpoint. A whole file is replaced
/// atomically: written beside its destination and renamed over it.
///
/// # Errors
/// Names the endpoint when the host refuses it or the write fails.
pub(crate) fn write_whole(endpoint: &Endpoint, text: &str) -> Result<(), String> {
    let Endpoint::File(path) = endpoint else {
        let (mut destination, _) = Destination::open(endpoint)?;
        return destination.line(text);
    };
    if let Policy::Document { .. } = policy() {
        return Err(format!(
            "{}: a document writes nothing outside itself",
            endpoint.describe()
        ));
    }
    let full = record(path, Written::Replaced)?;
    let parent = full
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if !parent.is_dir() {
        return Err(format!(
            "{}: the directory {} does not exist",
            path.display(),
            parent.display()
        ));
    }
    let name = full
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let beside = parent.join(format!(".{name}.hql-{}", std::process::id()));
    let written = fs::File::create(&beside).and_then(|mut file| {
        writeln!(file, "{text}")?;
        file.sync_all()
    });
    if let Err(error) = written.and_then(|()| fs::rename(&beside, &full)) {
        let _ = fs::remove_file(&beside);
        return Err(format!("{}: {error}", path.display()));
    }
    Ok(())
}

/// Whether reading a format piece by piece is possible.
#[must_use]
pub fn streams(format: TypeConstructor) -> bool {
    separable(format)
}
