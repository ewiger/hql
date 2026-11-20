//! PostgreSQL, in a build that reaches no server.

use super::reason;
use crate::data::Data;
use crate::endpoints::WriteMode;

const WHAT: &str = "a `postgres` endpoint";

pub(crate) fn columns(_dsn: &str, _table: &str) -> Result<Vec<(String, String)>, String> {
    Err(reason(WHAT))
}

pub(crate) fn rows(
    _dsn: &str,
    _table: &str,
    _order: &[String],
    _columns: Option<&[String]>,
) -> Result<Vec<Data>, String> {
    Err(reason(WHAT))
}

/// A value as the text a column of its type is written with; `None` is SQL
/// `NULL`.
#[must_use]
pub(crate) fn text_of(value: &Data) -> Option<String> {
    Some(match value {
        Data::Empty => return None,
        Data::Bool(truth) => truth.to_string(),
        Data::Int(n) => n.to_string(),
        Data::Float(x) => x.to_string(),
        Data::Decimal(d) => d.to_string(),
        Data::Time(time) => time.to_string(),
        Data::Bytes(octets) => crate::data::base64_text(octets),
        Data::Str(text) => text.clone(),
        Data::List(_) | Data::Map(_) => value.to_json().to_string(),
    })
}

pub(crate) fn execute(
    _dsn: &str,
    _statement: &str,
    _params: &[crate::mutation::Param],
) -> Result<u64, String> {
    Err(reason(WHAT))
}

/// A table open for writing; never constructed in this build.
pub(crate) struct Writer {
    never: std::convert::Infallible,
}

impl Writer {
    pub(crate) fn open(
        _dsn: &str,
        _table: &str,
        _mode: WriteMode,
        _columns: &[(String, String)],
    ) -> Result<Self, String> {
        Err(reason(WHAT))
    }

    pub(crate) fn insert(&mut self, _row: &[Option<String>]) -> Result<(), String> {
        match self.never {}
    }
}

pub(crate) fn write(
    _dsn: &str,
    _table: &str,
    _mode: WriteMode,
    _columns: &[(String, String)],
    _rows: &[Vec<Option<String>>],
) -> Result<(), String> {
    Err(reason(WHAT))
}
