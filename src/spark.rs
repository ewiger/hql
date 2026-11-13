//! Spark SQL at the edge: the columns and rows of one table, and writes to
//! it, for the configured endpoints of `kind = "spark"`.
//!
//! The endpoint is a Databricks SQL warehouse or cluster, or a Spark Thrift
//! Server: each answers HiveServer2's Thrift protocol over HTTP, which is
//! what this module speaks, one request per HTTP call. A statement is SQL
//! text, with no bound parameters, so every value is written as a literal
//! the server casts to the column's type. Nothing here is reached by
//! checking a program: a session is opened only when a table is read or
//! written.

use crate::data::Data;
use crate::endpoints::WriteMode;
use std::collections::BTreeMap;
use std::io::Read;
use std::time::Duration;

/// How long a connection may take before it is a failure.
const DEADLINE: Duration = Duration::from_secs(10);

/// How many rows one fetch asks for.
const FETCH: i64 = 10_000;

/// How many rows one `INSERT` statement holds. Each statement is one commit
/// to the table.
const BATCH: usize = 500;

/// The protocol version asked for, `HIVE_CLI_SERVICE_PROTOCOL_V8`: from
/// version 6 a result is held column by column.
const PROTOCOL: i32 = 7;

/// Where an endpoint is and what it is asked with.
#[derive(Debug, Clone)]
pub(crate) struct Server {
    /// The HTTP address requests are posted to.
    pub(crate) url: String,
    /// The access token, where the endpoint asks for one.
    pub(crate) token: Option<String>,
}

/// The Thrift binary protocol: the values a reply holds, and the bytes of a
/// request.
mod thrift {
    pub(super) const BOOL: u8 = 2;
    pub(super) const I16: u8 = 6;
    pub(super) const I32: u8 = 8;
    pub(super) const I64: u8 = 10;
    pub(super) const STRING: u8 = 11;
    pub(super) const STRUCT: u8 = 12;
    const CALL: i32 = 1;
    const EXCEPTION: i32 = 3;
    const VERSION: i32 = 0x8001_0000_u32.cast_signed();

    /// A value read from a reply.
    #[derive(Debug, Clone, PartialEq)]
    pub(super) enum Value {
        Bool(bool),
        Int(i64),
        Double(f64),
        Bytes(Vec<u8>),
        Struct(Vec<(i16, Value)>),
        List(Vec<Value>),
        Map(Vec<(Value, Value)>),
    }

    impl Value {
        /// The field of a struct with this id.
        pub(super) fn field(&self, id: i16) -> Option<&Value> {
            match self {
                Self::Struct(fields) => fields
                    .iter()
                    .find(|(held, _)| *held == id)
                    .map(|(_, value)| value),
                _ => None,
            }
        }

        pub(super) fn int(&self) -> Option<i64> {
            match self {
                Self::Int(n) => Some(*n),
                _ => None,
            }
        }

        pub(super) fn bytes(&self) -> Option<&[u8]> {
            match self {
                Self::Bytes(bytes) => Some(bytes),
                _ => None,
            }
        }

        pub(super) fn text(&self) -> Option<String> {
            self.bytes()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        }

        pub(super) fn list(&self) -> &[Value] {
            match self {
                Self::List(values) => values,
                _ => &[],
            }
        }
    }

    /// The bytes of a call: a message naming the method, holding one struct
    /// whose first field is the request.
    pub(super) struct Call(Vec<u8>);

    impl Call {
        pub(super) fn new(method: &str) -> Self {
            let mut call = Self(Vec::new());
            call.0.extend((VERSION | CALL).to_be_bytes());
            call.string(method.as_bytes());
            call.0.extend(0_i32.to_be_bytes());
            call.begin(1);
            call
        }

        fn header(&mut self, ty: u8, id: i16) {
            self.0.push(ty);
            self.0.extend(id.to_be_bytes());
        }

        fn string(&mut self, bytes: &[u8]) {
            let length = i32::try_from(bytes.len()).unwrap_or(i32::MAX);
            self.0.extend(length.to_be_bytes());
            self.0.extend(bytes);
        }

        /// Open a struct held in field `id`; `end` closes it.
        pub(super) fn begin(&mut self, id: i16) {
            self.header(STRUCT, id);
        }

        pub(super) fn end(&mut self) {
            self.0.push(0);
        }

        pub(super) fn bool(&mut self, id: i16, value: bool) {
            self.header(BOOL, id);
            self.0.push(u8::from(value));
        }

        pub(super) fn i16(&mut self, id: i16, value: i16) {
            self.header(I16, id);
            self.0.extend(value.to_be_bytes());
        }

        pub(super) fn i32(&mut self, id: i16, value: i32) {
            self.header(I32, id);
            self.0.extend(value.to_be_bytes());
        }

        pub(super) fn i64(&mut self, id: i16, value: i64) {
            self.header(I64, id);
            self.0.extend(value.to_be_bytes());
        }

        pub(super) fn bytes(&mut self, id: i16, value: &[u8]) {
            self.header(STRING, id);
            self.string(value);
        }

        /// The call's bytes, the request and the arguments closed.
        pub(super) fn finish(mut self) -> Vec<u8> {
            self.end();
            self.end();
            self.0
        }
    }

    struct Reader<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl Reader<'_> {
        fn take(&mut self, count: usize) -> Result<&[u8], String> {
            let end = self
                .at
                .checked_add(count)
                .filter(|end| *end <= self.bytes.len())
                .ok_or("the reply ends before its value does")?;
            let taken = &self.bytes[self.at..end];
            self.at = end;
            Ok(taken)
        }

        fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
            let mut held = [0; N];
            held.copy_from_slice(self.take(N)?);
            Ok(held)
        }

        fn byte(&mut self) -> Result<u8, String> {
            Ok(self.array::<1>()?[0])
        }

        fn i32(&mut self) -> Result<i32, String> {
            Ok(i32::from_be_bytes(self.array()?))
        }

        fn count(&mut self) -> Result<usize, String> {
            usize::try_from(self.i32()?).map_err(|_| "the reply holds a negative size".to_owned())
        }

        fn value(&mut self, ty: u8, depth: usize) -> Result<Value, String> {
            if depth > 64 {
                return Err("the reply nests deeper than a result does".to_owned());
            }
            Ok(match ty {
                BOOL => Value::Bool(self.byte()? != 0),
                3 => Value::Int(i64::from(self.byte()?.cast_signed())),
                4 => Value::Double(f64::from_be_bytes(self.array()?)),
                I16 => Value::Int(i64::from(i16::from_be_bytes(self.array()?))),
                I32 => Value::Int(i64::from(self.i32()?)),
                I64 => Value::Int(i64::from_be_bytes(self.array()?)),
                STRING => {
                    let length = self.count()?;
                    Value::Bytes(self.take(length)?.to_vec())
                }
                STRUCT => {
                    let mut fields = Vec::new();
                    loop {
                        let ty = self.byte()?;
                        if ty == 0 {
                            break;
                        }
                        let id = i16::from_be_bytes(self.array()?);
                        fields.push((id, self.value(ty, depth + 1)?));
                    }
                    Value::Struct(fields)
                }
                13 => {
                    let (key, value) = (self.byte()?, self.byte()?);
                    let count = self.count()?;
                    let mut entries = Vec::new();
                    for _ in 0..count {
                        entries.push((self.value(key, depth + 1)?, self.value(value, depth + 1)?));
                    }
                    Value::Map(entries)
                }
                14 | 15 => {
                    let element = self.byte()?;
                    let count = self.count()?;
                    let mut values = Vec::new();
                    for _ in 0..count {
                        values.push(self.value(element, depth + 1)?);
                    }
                    Value::List(values)
                }
                other => {
                    return Err(format!(
                        "the reply holds a value of the unknown type {other}"
                    ));
                }
            })
        }
    }

    /// The result a reply holds: the struct in field 0 of its one argument.
    ///
    /// # Errors
    /// Says what is malformed, or repeats the exception the server answered
    /// with.
    pub(super) fn reply(bytes: &[u8]) -> Result<Value, String> {
        let mut reader = Reader { bytes, at: 0 };
        let version = reader.i32()?;
        if version & VERSION != VERSION {
            return Err("the endpoint did not answer in the Thrift binary protocol".to_owned());
        }
        let name = reader.count()?;
        reader.take(name)?;
        reader.i32()?;
        let body = reader.value(STRUCT, 0)?;
        if version & 0xff == EXCEPTION {
            return Err(body
                .field(1)
                .and_then(Value::text)
                .unwrap_or_else(|| "the endpoint refused the call".to_owned()));
        }
        body.field(0)
            .cloned()
            .ok_or_else(|| "the reply holds no result".to_owned())
    }
}

use thrift::{Call, Value};

/// A handle's two identifiers, as the server gave them.
#[derive(Debug, Clone)]
struct Handle {
    guid: Vec<u8>,
    secret: Vec<u8>,
}

impl Handle {
    /// Read from a `THandleIdentifier`.
    fn read(identifier: Option<&Value>) -> Option<Self> {
        let identifier = identifier?;
        Some(Self {
            guid: identifier.field(1)?.bytes()?.to_vec(),
            secret: identifier.field(2)?.bytes()?.to_vec(),
        })
    }

    /// Written as a `THandleIdentifier` in field 1 of the struct open.
    fn write(&self, call: &mut Call) {
        call.begin(1);
        call.bytes(1, &self.guid);
        call.bytes(2, &self.secret);
        call.end();
    }
}

/// A statement the server is running or has run.
struct Operation {
    handle: Handle,
    kind: i32,
    has_result: bool,
}

/// One cell of a result, as its column holds it.
#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Bool(bool),
    Int(i64),
    Double(f64),
    Text(String),
}

/// What a query answers with: its columns, each a name and a type, and its
/// rows.
type Answer = (Vec<(String, String)>, Vec<Vec<Option<Cell>>>);

/// A session with an endpoint: statements run in it one at a time.
struct Session {
    agent: ureq::Agent,
    url: String,
    authorization: String,
    handle: Handle,
}

impl Session {
    /// # Errors
    /// Says that the endpoint did not open a session, and why; the token is
    /// never repeated.
    fn open(server: &Server) -> Result<Self, String> {
        let agent = ureq::AgentBuilder::new().timeout_connect(DEADLINE).build();
        // A warehouse reads a token as the password of the user `token`; a
        // Thrift Server without authentication reads any user, and asks
        // that it has a password.
        let authorization = format!(
            "Basic {}",
            base64(
                match &server.token {
                    Some(token) => format!("token:{token}"),
                    None => "hql:hql".to_owned(),
                }
                .as_bytes()
            )
        );
        let mut session = Self {
            agent,
            url: server.url.clone(),
            authorization,
            handle: Handle {
                guid: Vec::new(),
                secret: Vec::new(),
            },
        };
        let mut call = Call::new("OpenSession");
        call.i32(1, PROTOCOL);
        let opened = session.call(call)?;
        session.handle = Handle::read(opened.field(3).and_then(|handle| handle.field(1)))
            .ok_or("the endpoint opened no session")?;
        Ok(session)
    }

    /// Post one call and read its result, the status checked.
    fn call(&self, call: Call) -> Result<Value, String> {
        let response = self
            .agent
            .post(&self.url)
            .set("Content-Type", "application/x-thrift")
            .set("Authorization", &self.authorization)
            .send_bytes(&call.finish())
            .map_err(|error| match error {
                ureq::Error::Status(status, _) => {
                    format!("the Spark SQL endpoint answered HTTP {status}")
                }
                ureq::Error::Transport(transport) => {
                    // The address is not repeated: one read from the
                    // environment may hold credentials.
                    format!("no Spark SQL endpoint answered: {}", transport.kind())
                }
            })?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("the Spark SQL endpoint's reply was cut short: {error}"))?;
        let result = thrift::reply(&bytes)?;
        match result.field(1) {
            Some(status) if !matches!(status.field(1).and_then(Value::int), Some(0 | 1)) => {
                Err(first_line(
                    &status
                        .field(5)
                        .and_then(Value::text)
                        .unwrap_or_else(|| "the endpoint refused the call".to_owned()),
                ))
            }
            _ => Ok(result),
        }
    }

    fn with_session(&self, method: &str) -> Call {
        let mut call = Call::new(method);
        call.begin(1);
        self.handle.write(&mut call);
        call.end();
        call
    }

    fn with_operation(method: &str, operation: &Operation) -> Call {
        let mut call = Call::new(method);
        call.begin(1);
        operation.handle.write(&mut call);
        call.i32(2, operation.kind);
        call.bool(3, operation.has_result);
        call.end();
        call
    }

    /// Run a statement to its end.
    fn execute(&self, statement: &str) -> Result<Operation, String> {
        let mut call = self.with_session("ExecuteStatement");
        call.bytes(2, statement.as_bytes());
        // Run without holding the HTTP request open: the state is polled.
        call.bool(4, true);
        let started = self.call(call)?;
        let handle = started
            .field(2)
            .ok_or("the endpoint started no statement")?;
        let operation = Operation {
            handle: Handle::read(handle.field(1)).ok_or("the endpoint started no statement")?,
            kind: handle
                .field(2)
                .and_then(Value::int)
                .and_then(|kind| i32::try_from(kind).ok())
                .unwrap_or(0),
            has_result: matches!(handle.field(3), Some(Value::Bool(true))),
        };
        let mut wait = Duration::from_millis(20);
        loop {
            let status = self.call(Self::with_operation("GetOperationStatus", &operation))?;
            match status.field(2).and_then(Value::int) {
                // Finished.
                Some(2) => return Ok(operation),
                // Initialized, running or pending.
                Some(0 | 1 | 7) => {
                    std::thread::sleep(wait);
                    wait = (wait * 2).min(Duration::from_millis(500));
                }
                _ => {
                    let message = status
                        .field(5)
                        .and_then(Value::text)
                        .unwrap_or_else(|| "the statement did not finish".to_owned());
                    self.close(&operation);
                    return Err(first_line(&message));
                }
            }
        }
    }

    fn close(&self, operation: &Operation) {
        // A statement left open ends with its session.
        let _ = self.call(Self::with_operation("CloseOperation", operation));
    }

    /// Run a statement that gives no rows.
    fn run(&self, statement: &str) -> Result<(), String> {
        let operation = self.execute(statement)?;
        self.close(&operation);
        Ok(())
    }

    /// Run a query: its columns, each a name and a type, and its rows.
    fn query(&self, statement: &str) -> Result<Answer, String> {
        let operation = self.execute(statement)?;
        let result = self.result(&operation);
        self.close(&operation);
        result
    }

    fn result(&self, operation: &Operation) -> Result<Answer, String> {
        let metadata = self.call(Self::with_operation("GetResultSetMetadata", operation))?;
        let columns: Vec<(String, String)> = metadata
            .field(2)
            .and_then(|schema| schema.field(1))
            .map(Value::list)
            .unwrap_or_default()
            .iter()
            .map(|column| {
                (
                    column.field(1).and_then(Value::text).unwrap_or_default(),
                    type_name(column.field(2)),
                )
            })
            .collect();
        let mut rows = Vec::new();
        loop {
            let mut call = Self::with_operation("FetchResults", operation);
            // The next rows, in order.
            call.i32(2, 0);
            call.i64(3, FETCH);
            call.i16(4, 0);
            let fetched = self.call(call)?;
            let set = fetched.field(3).ok_or("the endpoint fetched no rows")?;
            if !set.field(2).map(Value::list).unwrap_or_default().is_empty() {
                return Err(
                    "the endpoint answered row by row, as a protocol before version 6 does"
                        .to_owned(),
                );
            }
            let held: Vec<Vec<Option<Cell>>> = set
                .field(3)
                .map(Value::list)
                .unwrap_or_default()
                .iter()
                .map(cells)
                .collect();
            let count = held.first().map_or(0, Vec::len);
            if count == 0 {
                return Ok((columns, rows));
            }
            for index in 0..count {
                rows.push(
                    held.iter()
                        .map(|column| column.get(index).cloned().flatten())
                        .collect(),
                );
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.handle.guid.is_empty() {
            let _ = self.call(self.with_session("CloseSession"));
        }
    }
}

/// The cells of one `TColumn`: the one field it holds is its values and the
/// bits that mark which of them are null.
fn cells(column: &Value) -> Vec<Option<Cell>> {
    let Value::Struct(fields) = column else {
        return Vec::new();
    };
    let Some((_, held)) = fields.first() else {
        return Vec::new();
    };
    let nulls = held.field(2).and_then(Value::bytes).unwrap_or_default();
    held.field(1)
        .map(Value::list)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let null = nulls
                .get(index / 8)
                .is_some_and(|byte| byte & (1 << (index % 8)) != 0);
            if null {
                return None;
            }
            Some(match value {
                Value::Bool(truth) => Cell::Bool(*truth),
                Value::Int(n) => Cell::Int(*n),
                Value::Double(x) => Cell::Double(*x),
                other => Cell::Text(other.text().unwrap_or_default()),
            })
        })
        .collect()
}

/// A column's type by name, `BIGINT` or `DECIMAL(12,2)`, from its
/// `TTypeDesc`.
fn type_name(described: Option<&Value>) -> String {
    let primitive = described
        .and_then(|described| described.field(1))
        .map(Value::list)
        .unwrap_or_default()
        .first()
        .and_then(|entry| entry.field(1));
    let id = primitive
        .and_then(|primitive| primitive.field(1))
        .and_then(Value::int);
    let name = match id {
        Some(0) => "BOOLEAN",
        Some(1) => "TINYINT",
        Some(2) => "SMALLINT",
        Some(3) => "INT",
        Some(4) => "BIGINT",
        Some(5) => "FLOAT",
        Some(6) => "DOUBLE",
        Some(8) => "TIMESTAMP",
        Some(9) => "BINARY",
        Some(10) => "ARRAY",
        Some(11) => "MAP",
        Some(12) => "STRUCT",
        Some(15) => "DECIMAL",
        Some(17) => "DATE",
        _ => "STRING",
    };
    let qualifier = |wanted: &str| -> Option<i64> {
        let Value::Map(entries) = primitive?.field(2)?.field(1)? else {
            return None;
        };
        entries
            .iter()
            .find(|(key, _)| key.bytes() == Some(wanted.as_bytes()))
            .and_then(|(_, value)| value.field(1))
            .and_then(Value::int)
    };
    match (name, qualifier("precision"), qualifier("scale")) {
        ("DECIMAL", Some(precision), Some(scale)) => format!("DECIMAL({precision},{scale})"),
        _ => name.to_owned(),
    }
}

/// A cell as the value its column's type holds: a `DECIMAL` keeps its
/// digits, and every type without a value of its own reads as its text.
fn value_of(cell: Cell, ty: &str) -> Option<Data> {
    Some(match cell {
        Cell::Bool(truth) => Data::Bool(truth),
        Cell::Int(n) => Data::Int(n),
        Cell::Double(x) => Data::Float(x),
        Cell::Text(text) if ty.starts_with("DECIMAL") => {
            Data::Decimal(crate::decimal::Decimal::parse(&text)?)
        }
        Cell::Text(text) => Data::Str(text),
    })
}

/// What a server's message says failed: its first line, without the names
/// of the exceptions that carried it, and without the plan and the stack
/// trace that follow.
fn first_line(message: &str) -> String {
    let line = message.lines().next().unwrap_or(message);
    line.rsplit_once("Exception: ")
        .map_or(line, |(_, said)| said)
        .trim()
        .to_owned()
}

fn base64(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let group = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let index = [
            group[0] >> 2,
            (group[0] & 0x03) << 4 | group[1] >> 4,
            (group[1] & 0x0f) << 2 | group[2] >> 6,
            group[2] & 0x3f,
        ];
        for (place, digit) in index.iter().enumerate() {
            if place <= chunk.len() {
                text.push(char::from(DIGITS[usize::from(*digit)]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// A name quoted as an SQL identifier, so it is read exactly as written.
pub(crate) fn quoted(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// A table name, qualified where it holds dots: `main.sales.orders`.
pub(crate) fn table_name(table: &str) -> String {
    table.split('.').map(quoted).collect::<Vec<_>>().join(".")
}

/// Text as an SQL string literal. Text of letters, digits and plain
/// punctuation is written between quotes; any other text is written as the
/// hexadecimal digits of its bytes, so no character of a value is ever read
/// as SQL, whatever the session's rules for escapes are.
pub(crate) fn literal(text: &str) -> String {
    let plain = text.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, ' ' | '-' | '_' | '.' | ':' | ',' | '+' | '/' | '@')
    });
    if plain {
        return format!("'{text}'");
    }
    let digits: String = text.bytes().map(|byte| format!("{byte:02X}")).collect();
    format!("decode(unhex('{digits}'), 'UTF-8')")
}

/// A value as the SQL that writes it to a column of type `ty`; `None` is
/// `NULL`.
fn cast(value: Option<&str>, ty: &str) -> String {
    match value {
        None => "NULL".to_owned(),
        Some(text) => format!("CAST({} AS {ty})", literal(text)),
    }
}

fn located(table: &str) -> impl Fn(String) -> String + '_ {
    move |message| format!("table `{table}`: {message}")
}

/// Run one statement that changes rows, `UPDATE` or `DELETE`, and give the
/// number of rows it changed where the endpoint reports one, as a Delta
/// table does in `num_affected_rows` (HQL-0051). A concurrent writer's
/// refusal is noted as a conflict for the transaction scope it happened in.
///
/// # Errors
/// Names what failed: the session, or the statement as the endpoint
/// refused it.
pub(crate) fn execute(server: &Server, statement: &str) -> Result<Option<i64>, String> {
    let session = Session::open(server)?;
    let operation = session.execute(statement).inspect_err(|message| {
        if message.contains("Concurrent")
            || message.contains("ProtocolChanged")
            || message.contains("MetadataChanged")
        {
            crate::transaction::note_conflict();
        }
    })?;
    let count = if operation.has_result {
        session.result(&operation).ok().and_then(|(types, rows)| {
            let (_, ty) = types.first()?;
            match value_of(rows.first()?.first()?.clone()?, ty)? {
                Data::Int(n) => Some(n),
                _ => None,
            }
        })
    } else {
        None
    };
    session.close(&operation);
    Ok(count)
}

/// The columns of one table, each its name and its type's name, `BIGINT`,
/// in the table's order.
///
/// # Errors
/// Names the table and what failed: the session, or the query.
pub(crate) fn columns(server: &Server, table: &str) -> Result<Vec<(String, String)>, String> {
    let session = Session::open(server)?;
    session
        .query(&format!("SELECT * FROM {} LIMIT 0", table_name(table)))
        .map(|(columns, _)| columns)
        .map_err(located(table))
}

/// The rows of one table, each a `Data` map from column to value: every
/// column, or only `columns` where a boundary names the ones read, ordered
/// by `order` where it names columns and as the table is scanned otherwise.
///
/// # Errors
/// Names the table and what failed: the session, the query, or a value its
/// column's type does not hold.
pub(crate) fn rows(
    server: &Server,
    table: &str,
    order: &[String],
    columns: Option<&[String]>,
) -> Result<Vec<Data>, String> {
    let session = Session::open(server)?;
    let list = |names: &[String]| {
        names
            .iter()
            .map(|name| quoted(name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let selected = columns
        .filter(|columns| !columns.is_empty())
        .map_or_else(|| "*".to_owned(), list);
    let order = if order.is_empty() {
        String::new()
    } else {
        format!(" ORDER BY {}", list(order))
    };
    let (types, held) = session
        .query(&format!(
            "SELECT {selected} FROM {}{order}",
            table_name(table)
        ))
        .map_err(located(table))?;
    held.into_iter()
        .map(|row| {
            let mut fields = BTreeMap::new();
            for ((column, ty), cell) in types.iter().zip(row) {
                let value = match cell {
                    None => Data::Empty,
                    Some(cell) => value_of(cell.clone(), ty).ok_or_else(|| {
                        format!("table `{table}`, column `{column}`: {cell:?} is not a {ty}")
                    })?,
                };
                fields.insert(column.clone(), value);
            }
            Ok(Data::Map(fields))
        })
        .collect()
}

/// A value as the text a column is written with; `None` is SQL `NULL`.
#[must_use]
pub(crate) fn text_of(value: &Data) -> Option<String> {
    Some(match value {
        Data::Empty => return None,
        Data::Bool(truth) => truth.to_string(),
        Data::Int(n) => n.to_string(),
        Data::Float(x) => x.to_string(),
        Data::Decimal(d) => d.to_string(),
        Data::Time(time) => time.to_string(),
        Data::Str(text) => text.clone(),
        Data::List(_) | Data::Map(_) => value.to_json().to_string(),
    })
}

/// Apply `mode` to the table: created where it is not held, and refused by
/// `create` where it is. Whether `replace` is still to empty it is the
/// caller's to do.
fn prepare(
    session: &Session,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
) -> Result<(), String> {
    if columns.is_empty() {
        return Err(format!(
            "table `{table}`: no field of the record is written; a field with `write = false` \
             is not a column"
        ));
    }
    let declared: Vec<String> = columns
        .iter()
        .map(|(column, ty)| format!("{} {ty}", quoted(column)))
        .collect();
    let exists = match mode {
        WriteMode::Create => "",
        WriteMode::Append | WriteMode::Replace => "IF NOT EXISTS ",
    };
    // The table's format is the endpoint's default: Delta in a lakehouse.
    session
        .run(&format!(
            "CREATE TABLE {exists}{} ({})",
            table_name(table),
            declared.join(", ")
        ))
        .map_err(|message| {
            if mode == WriteMode::Create && message.to_ascii_lowercase().contains("already exists")
            {
                format!("table `{table}`: it exists, and `mode = \"create\"` writes only a new one")
            } else {
                located(table)(message)
            }
        })
}

/// The statement that empties a table and keeps its definition. A Delta
/// table takes no `TRUNCATE`; it deletes every row in one commit.
fn emptied(table: &str) -> String {
    format!("DELETE FROM {}", table_name(table))
}

/// The statement that writes `rows`, each a value per column: after the
/// rows held, or in their place.
fn insert(
    table: &str,
    columns: &[(String, String)],
    rows: &[Vec<Option<String>>],
    overwrite: bool,
) -> String {
    let names: Vec<String> = columns.iter().map(|(column, _)| quoted(column)).collect();
    let values: Vec<String> = rows
        .iter()
        .map(|row| {
            let cells: Vec<String> = row
                .iter()
                .zip(columns)
                .map(|(value, (_, ty))| cast(value.as_deref(), ty))
                .collect();
            format!("({})", cells.join(", "))
        })
        .collect();
    format!(
        "INSERT {} {} ({}) VALUES {}",
        if overwrite { "OVERWRITE" } else { "INTO" },
        table_name(table),
        names.join(", "),
        values.join(", ")
    )
}

/// A table opened for writing, its `mode` applied: rows are written in
/// statements of [`BATCH`], each one commit.
pub(crate) struct Writer {
    session: Session,
    table: String,
    columns: Vec<(String, String)>,
    pending: Vec<Vec<Option<String>>>,
}

impl Writer {
    /// Open `table` for writing as `mode` says, its columns each a name and
    /// the type it is declared with.
    ///
    /// # Errors
    /// Names the table and what failed: the session, a table `create`
    /// finds already there, or a statement.
    pub(crate) fn open(
        server: &Server,
        table: &str,
        mode: WriteMode,
        columns: &[(String, String)],
    ) -> Result<Self, String> {
        let session = Session::open(server)?;
        prepare(&session, table, mode, columns)?;
        if mode == WriteMode::Replace {
            session.run(&emptied(table)).map_err(located(table))?;
        }
        Ok(Self {
            session,
            table: table.to_owned(),
            columns: columns.to_vec(),
            pending: Vec::new(),
        })
    }

    /// Queue one row, a value per column in the order the writer was opened
    /// with, writing the queue once it is a batch.
    ///
    /// # Errors
    /// Names the table and the statement's failure.
    pub(crate) fn insert(&mut self, row: Vec<Option<String>>) -> Result<(), String> {
        self.pending.push(row);
        if self.pending.len() >= BATCH {
            self.flush()?;
        }
        Ok(())
    }

    /// Write every queued row.
    ///
    /// # Errors
    /// Names the table and the statement's failure.
    pub(crate) fn flush(&mut self) -> Result<(), String> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let rows = std::mem::take(&mut self.pending);
        self.session
            .run(&insert(&self.table, &self.columns, &rows, false))
            .map_err(located(&self.table))
    }
}

/// Write `rows` to `table` as `mode` says, in statements of [`BATCH`] rows.
/// A statement is one commit: up to a batch, either every row is written or
/// none is, and `replace` puts the rows in place of the ones held in that
/// same commit.
///
/// # Errors
/// As [`Writer::open`], and a row the table refuses.
pub(crate) fn write(
    server: &Server,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
    rows: &[Vec<Option<String>>],
) -> Result<(), String> {
    let session = Session::open(server)?;
    prepare(&session, table, mode, columns)?;
    if rows.is_empty() {
        if mode == WriteMode::Replace {
            session.run(&emptied(table)).map_err(located(table))?;
        }
        return Ok(());
    }
    for (index, batch) in rows.chunks(BATCH).enumerate() {
        let overwrite = mode == WriteMode::Replace && index == 0;
        session
            .run(&insert(table, columns, batch, overwrite))
            .map_err(located(table))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reply to `method` whose result is the fields `write` adds.
    fn replied(write: impl FnOnce(&mut Call)) -> Vec<u8> {
        // A call's layout is a reply's, but for the message type and the id
        // of the field that holds the result.
        let mut call = Call::new("M");
        write(&mut call);
        let mut bytes = call.finish();
        bytes[3] = 2;
        let field = 4 + 4 + 1 + 4 + 1;
        bytes[field..field + 2].copy_from_slice(&0_i16.to_be_bytes());
        bytes
    }

    #[test]
    fn a_reply_is_read_back_as_the_values_written() {
        let bytes = replied(|call| {
            call.begin(1);
            call.i32(1, 0);
            call.end();
            call.bytes(2, b"guid");
            call.i64(3, -7);
            call.bool(4, true);
            call.i16(5, 9);
        });
        let result = thrift::reply(&bytes).unwrap();
        assert_eq!(result.field(1).unwrap().field(1), Some(&Value::Int(0)));
        assert_eq!(result.field(2).unwrap().bytes(), Some(&b"guid"[..]));
        assert_eq!(result.field(3), Some(&Value::Int(-7)));
        assert_eq!(result.field(4), Some(&Value::Bool(true)));
        assert_eq!(result.field(5), Some(&Value::Int(9)));
    }

    #[test]
    fn a_reply_cut_short_or_in_another_protocol_is_refused() {
        let bytes = replied(|call| call.bytes(2, b"guid"));
        assert!(thrift::reply(&bytes[..bytes.len() - 4]).is_err());
        assert!(thrift::reply(b"<html>no</html>").is_err());
        assert!(thrift::reply(&[]).is_err());
    }

    #[test]
    fn a_column_reads_its_values_and_its_nulls() {
        // A `TColumn` whose one field holds three texts, the second null.
        let column = Value::Struct(vec![(
            7,
            Value::Struct(vec![
                (
                    1,
                    Value::List(vec![
                        Value::Bytes(b"a".to_vec()),
                        Value::Bytes(Vec::new()),
                        Value::Bytes(b"c".to_vec()),
                    ]),
                ),
                (2, Value::Bytes(vec![0b0000_0010])),
            ]),
        )]);
        assert_eq!(
            cells(&column),
            vec![
                Some(Cell::Text("a".to_owned())),
                None,
                Some(Cell::Text("c".to_owned()))
            ]
        );
    }

    #[test]
    fn a_cell_reads_as_its_type_holds_it() {
        assert_eq!(value_of(Cell::Int(42), "BIGINT"), Some(Data::Int(42)));
        assert_eq!(
            value_of(Cell::Bool(true), "BOOLEAN"),
            Some(Data::Bool(true))
        );
        let Some(Data::Decimal(digits)) = value_of(Cell::Text("10.50".to_owned()), "DECIMAL(12,2)")
        else {
            panic!("a DECIMAL reads as a Decimal");
        };
        assert_eq!(digits.to_string(), "10.50");
        assert_eq!(value_of(Cell::Text("x".to_owned()), "DECIMAL(12,2)"), None);
        assert_eq!(
            value_of(Cell::Text("2026-11-12 10:00:00".to_owned()), "TIMESTAMP"),
            Some(Data::Str("2026-11-12 10:00:00".to_owned()))
        );
    }

    #[test]
    fn no_character_of_a_value_is_read_as_sql() {
        assert_eq!(literal("order_placed"), "'order_placed'");
        assert_eq!(literal("2026-11-01T09:15:00Z"), "'2026-11-01T09:15:00Z'");
        assert_eq!(literal("it's"), "decode(unhex('69742773'), 'UTF-8')");
        assert_eq!(literal("a\\"), "decode(unhex('615C'), 'UTF-8')");
        assert_eq!(literal("é"), "decode(unhex('C3A9'), 'UTF-8')");
        assert_eq!(cast(None, "STRING"), "NULL");
        assert_eq!(
            cast(Some("120.50"), "DECIMAL(12,2)"),
            "CAST('120.50' AS DECIMAL(12,2))"
        );
    }

    #[test]
    fn rows_are_written_in_one_statement_after_or_in_place_of_the_rows_held() {
        let columns = [
            ("id".to_owned(), "BIGINT".to_owned()),
            ("note".to_owned(), "STRING".to_owned()),
        ];
        let rows = [
            vec![Some("1".to_owned()), None],
            vec![Some("2".to_owned()), Some("ok".to_owned())],
        ];
        assert_eq!(
            insert("main.sales.orders", &columns, &rows, false),
            "INSERT INTO `main`.`sales`.`orders` (`id`, `note`) VALUES \
             (CAST('1' AS BIGINT), NULL), (CAST('2' AS BIGINT), CAST('ok' AS STRING))"
        );
        assert!(insert("t", &columns, &rows, true).starts_with("INSERT OVERWRITE `t` ("));
        assert_eq!(table_name("odd`name"), "`odd``name`");
    }

    #[test]
    fn a_failure_says_what_failed_without_the_exceptions_that_carried_it() {
        assert_eq!(
            first_line(
                "org.apache.hive.service.cli.HiveSQLException: Error running query: \
                 [TABLE_OR_VIEW_NOT_FOUND] org.apache.spark.sql.AnalysisException: \
                 [TABLE_OR_VIEW_NOT_FOUND] The table or view `default`.`notes` cannot be \
                 found.\n\tat org.apache.spark"
            ),
            "[TABLE_OR_VIEW_NOT_FOUND] The table or view `default`.`notes` cannot be found."
        );
        assert_eq!(first_line("no such warehouse"), "no such warehouse");
    }

    #[test]
    fn a_token_is_sent_as_basic_credentials() {
        assert_eq!(base64(b"hql:hql"), "aHFsOmhxbA==");
        assert_eq!(base64(b"token:abc"), "dG9rZW46YWJj");
        assert_eq!(base64(b"ab"), "YWI=");
    }
}
