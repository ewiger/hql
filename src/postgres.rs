//! PostgreSQL at the edge: the columns and rows of one table, and writes to
//! it, for the configured endpoints of `kind = "postgres"`.
//!
//! Every value crosses the connection as text and is converted by the
//! column's type, so a `numeric` keeps its digits and a `jsonb` column reads
//! as the tree it holds. Nothing here is reached by checking a program: a
//! connection is made only when a source is read or a target written.

use crate::data::Data;
use crate::endpoints::WriteMode;
use postgres::{Client, GenericClient, NoTls};
use std::collections::BTreeMap;

/// A connection to the database a connection string names.
///
/// # Errors
/// Names what failed; the connection string is never repeated, since it may
/// hold a password.
fn connect(dsn: &str) -> Result<Client, String> {
    Client::connect(dsn, NoTls).map_err(|error| {
        format!(
            "no connection to the database: {}",
            crate::transaction::postgres_message(&error)
        )
    })
}

/// Run `f` on a client of the database `dsn` names: the one an open
/// transaction scope holds (HQL-0051), or one connected for this call
/// alone.
fn on_client<T>(dsn: &str, f: impl FnOnce(&mut Client) -> Result<T, String>) -> Result<T, String> {
    let mut f = Some(f);
    if let Some(result) =
        crate::transaction::with_postgres(dsn, |client| f.take().map(|f| f(client))).flatten()
    {
        return result;
    }
    let mut client = connect(dsn)?;
    f.take().map_or_else(
        || Err("a client was used twice".to_owned()),
        |f| f(&mut client),
    )
}

/// A name quoted as an SQL identifier, so it is read exactly as written.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A table name, schema-qualified where it holds a dot: `audit.events`.
fn table_name(table: &str) -> String {
    table.split('.').map(quoted).collect::<Vec<_>>().join(".")
}

fn located(table: &str) -> impl Fn(postgres::Error) -> String + '_ {
    move |error| {
        format!(
            "table `{table}`: {}",
            crate::transaction::postgres_message(&error)
        )
    }
}

/// The columns of one table, each its name and its type's name, `int8`, in
/// the table's order.
///
/// # Errors
/// Names the table and what failed: the connection, or the query.
pub(crate) fn columns(dsn: &str, table: &str) -> Result<Vec<(String, String)>, String> {
    on_client(dsn, |client| {
        described(
            client,
            &format!("SELECT * FROM {} LIMIT 0", table_name(table)),
            table,
        )
    })
}

fn described(
    client: &mut impl GenericClient,
    query: &str,
    table: &str,
) -> Result<Vec<(String, String)>, String> {
    let statement = client.prepare(query).map_err(located(table))?;
    Ok(statement
        .columns()
        .iter()
        .map(|column| (column.name().to_owned(), column.type_().name().to_owned()))
        .collect())
}

/// The rows of one table, each a `Data` map from column to value: every
/// column, or only `columns` where a boundary names the ones read, ordered
/// by `order` where it names columns and as the table is scanned otherwise.
///
/// # Errors
/// Names the table and what failed: the connection, the query, or a value
/// its column's type does not hold.
pub(crate) fn rows(
    dsn: &str,
    table: &str,
    order: &[String],
    columns: Option<&[String]>,
) -> Result<Vec<Data>, String> {
    on_client(dsn, |client| rows_on(client, table, order, columns))
}

fn rows_on(
    client: &mut Client,
    table: &str,
    order: &[String],
    columns: Option<&[String]>,
) -> Result<Vec<Data>, String> {
    let selected = columns.filter(|columns| !columns.is_empty()).map_or_else(
        || "*".to_owned(),
        |columns| {
            columns
                .iter()
                .map(|column| quoted(column))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let order = if order.is_empty() {
        String::new()
    } else {
        format!(
            " ORDER BY {}",
            order
                .iter()
                .map(|column| quoted(column))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let query = format!("SELECT {selected} FROM {}{order}", table_name(table));
    let types = described(client, &query, table)?;
    let messages = client.simple_query(&query).map_err(located(table))?;
    let mut read = Vec::new();
    for message in messages {
        let postgres::SimpleQueryMessage::Row(row) = message else {
            continue;
        };
        let mut fields = BTreeMap::new();
        for (index, (column, ty)) in types.iter().enumerate() {
            let value = match row.get(index) {
                None => Data::Empty,
                Some(text) => value_of(text, ty).ok_or_else(|| {
                    format!("table `{table}`, column `{column}`: {text:?} is not a {ty}")
                })?,
            };
            fields.insert(column.clone(), value);
        }
        read.push(Data::Map(fields));
    }
    Ok(read)
}

/// A column's text as the value its type holds: an integer, a float, a
/// `numeric` with its digits, a boolean, a JSON tree, or the text itself.
fn value_of(text: &str, ty: &str) -> Option<Data> {
    Some(match ty {
        "int2" | "int4" | "int8" | "oid" => Data::Int(text.parse().ok()?),
        "float4" | "float8" => Data::Float(text.parse().ok()?),
        "numeric" => Data::Decimal(crate::decimal::Decimal::parse(text)?),
        "bool" => match text {
            "t" => Data::Bool(true),
            "f" => Data::Bool(false),
            _ => return None,
        },
        "json" | "jsonb" => crate::io::data_of(serde_json::from_str(text).ok()?),
        _ => Data::Str(text.to_owned()),
    })
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

/// Run one statement that changes rows, `UPDATE` or `DELETE`, with its
/// parameters bound, and give the number of rows it changed (HQL-0051).
///
/// # Errors
/// Names what failed: the connection, or the statement as the server
/// refused it.
pub(crate) fn execute(
    dsn: &str,
    statement: &str,
    params: &[crate::mutation::Param],
) -> Result<u64, String> {
    use crate::mutation::Param;
    let bound: Vec<&(dyn postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|param| match param {
            Param::Int(n) => n as &(dyn postgres::types::ToSql + Sync),
            Param::Float(x) => x as &(dyn postgres::types::ToSql + Sync),
            Param::Bool(b) => b as &(dyn postgres::types::ToSql + Sync),
            Param::Text(text) | Param::Numeric(text) => {
                text as &(dyn postgres::types::ToSql + Sync)
            }
        })
        .collect();
    on_client(dsn, |client| {
        client.execute(statement, &bound).map_err(|error| {
            format!(
                "statement refused: {}",
                crate::transaction::postgres_message(&error)
            )
        })
    })
}

/// A table opened for writing, its `mode` applied: rows are inserted one at
/// a time, each committed as it is written.
pub(crate) struct Writer {
    /// The writer's own client, or none where an open transaction scope
    /// holds the database and each row joins its attempt (HQL-0051).
    client: Option<Client>,
    dsn: String,
    insert: postgres::Statement,
    table: String,
}

impl Writer {
    /// Open `table` for writing as `mode` says, its columns each a name and
    /// the type it is declared with.
    ///
    /// # Errors
    /// Names the table and what failed: the connection, a table `create`
    /// finds already there, or a statement.
    pub(crate) fn open(
        dsn: &str,
        table: &str,
        mode: WriteMode,
        columns: &[(String, String)],
    ) -> Result<Self, String> {
        if let Some(insert) =
            crate::transaction::with_postgres(dsn, |client| prepared(client, table, mode, columns))
        {
            return Ok(Self {
                client: None,
                dsn: dsn.to_owned(),
                insert: insert?,
                table: table.to_owned(),
            });
        }
        let mut client = connect(dsn)?;
        let insert = prepared(&mut client, table, mode, columns)?;
        Ok(Self {
            client: Some(client),
            dsn: dsn.to_owned(),
            insert,
            table: table.to_owned(),
        })
    }

    /// Insert one row, a value per column in the order the writer was
    /// opened with.
    ///
    /// # Errors
    /// Names the table and the statement's failure.
    pub(crate) fn insert(&mut self, row: &[Option<String>]) -> Result<(), String> {
        match &mut self.client {
            Some(client) => insert(client, &self.insert, row, &self.table),
            None => crate::transaction::with_postgres(&self.dsn, |client| {
                insert(client, &self.insert, row, &self.table)
            })
            .unwrap_or_else(|| Err("the transaction scope closed under a write".to_owned())),
        }
    }
}

/// Write `rows` to `table` as `mode` says, in one transaction: either every
/// row is written or none is.
///
/// # Errors
/// As [`Writer::open`], and a row the table refuses.
pub(crate) fn write(
    dsn: &str,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
    rows: &[Vec<Option<String>>],
) -> Result<(), String> {
    // Inside a transaction scope the rows join the attempt (HQL-0051);
    // outside one they are their own transaction.
    if crate::transaction::holds_postgres(dsn) {
        return on_client(dsn, |client| {
            let statement = prepared(client, table, mode, columns)?;
            for row in rows {
                insert(client, &statement, row, table)?;
            }
            Ok(())
        });
    }
    let mut client = connect(dsn)?;
    let mut transaction = client.transaction().map_err(located(table))?;
    let statement = prepared(&mut transaction, table, mode, columns)?;
    for row in rows {
        insert(&mut transaction, &statement, row, table)?;
    }
    transaction.commit().map_err(located(table))
}

/// Apply `mode` to the table and prepare the statement that inserts one row.
fn prepared(
    client: &mut impl GenericClient,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
) -> Result<postgres::Statement, String> {
    if columns.is_empty() {
        return Err(format!(
            "table `{table}`: no field of the record is written; a field with `write = false` \
             is not a column"
        ));
    }
    let name = table_name(table);
    let held = client
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&name])
        .map_err(located(table))?
        .get::<_, bool>(0);
    match mode {
        WriteMode::Create if held => {
            return Err(format!(
                "table `{table}`: it exists, and `mode = \"create\"` writes only a new one"
            ));
        }
        // The table keeps its definition, its indexes and its grants: only
        // its rows are replaced.
        WriteMode::Replace if held => {
            client
                .batch_execute(&format!("TRUNCATE {name}"))
                .map_err(located(table))?;
        }
        _ if !held => {
            let declared: Vec<String> = columns
                .iter()
                .map(|(column, ty)| format!("{} {ty}", quoted(column)))
                .collect();
            client
                .batch_execute(&format!("CREATE TABLE {name} ({})", declared.join(", ")))
                .map_err(located(table))?;
        }
        _ => {}
    }
    // Each value is sent as text and cast to the column's type by the
    // server, so a `numeric` keeps its digits.
    let names: Vec<String> = columns.iter().map(|(column, _)| quoted(column)).collect();
    let slots: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(index, (_, ty))| format!("${}::text::{ty}", index + 1))
        .collect();
    client
        .prepare(&format!(
            "INSERT INTO {name} ({}) VALUES ({})",
            names.join(", "),
            slots.join(", ")
        ))
        .map_err(located(table))
}

fn insert(
    client: &mut impl GenericClient,
    statement: &postgres::Statement,
    row: &[Option<String>],
    table: &str,
) -> Result<(), String> {
    let parameters: Vec<&(dyn postgres::types::ToSql + Sync)> = row
        .iter()
        .map(|value| value as &(dyn postgres::types::ToSql + Sync))
        .collect();
    client
        .execute(statement, &parameters)
        .map(drop)
        .map_err(located(table))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_reads_as_its_type_holds_it() {
        assert_eq!(value_of("42", "int8"), Some(Data::Int(42)));
        assert_eq!(value_of("t", "bool"), Some(Data::Bool(true)));
        assert_eq!(value_of("x", "bool"), None);
        let Some(Data::Decimal(digits)) = value_of("10.50", "numeric") else {
            panic!("numeric reads as a Decimal");
        };
        assert_eq!(digits.to_string(), "10.50");
        assert_eq!(
            value_of(r#"{"a": [1]}"#, "jsonb"),
            Some(Data::Map(BTreeMap::from([(
                "a".to_owned(),
                Data::List(vec![Data::Int(1)])
            )])))
        );
        assert_eq!(
            value_of("2026-11-12 10:00:00+00", "timestamptz"),
            Some(Data::Str("2026-11-12 10:00:00+00".to_owned()))
        );
    }

    #[test]
    fn a_value_is_written_as_text() {
        assert_eq!(text_of(&Data::Empty), None);
        assert_eq!(text_of(&Data::Bool(false)).as_deref(), Some("false"));
        assert_eq!(
            text_of(&Data::List(vec![Data::Int(1), Data::Str("a".to_owned())])).as_deref(),
            Some(r#"[1,"a"]"#)
        );
    }

    #[test]
    fn a_dotted_table_name_is_schema_qualified() {
        assert_eq!(table_name("audit.events"), "\"audit\".\"events\"");
        assert_eq!(table_name("odd\"name"), "\"odd\"\"name\"");
    }
}
