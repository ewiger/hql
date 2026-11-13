//! Mutations (HQL-0051): `insert`, `update … set` and `delete`, each one
//! statement of the endpoint's system over the rows it holds.
//!
//! A clause's lambda is not evaluated row by row: the system runs it. A
//! sub-expression that reads no field of the row is evaluated in HQL and
//! sent as a bound parameter, so no value reaches the system as statement
//! text; one that does is translated, and the checker admitted only what
//! translates. The result is the number of rows the system reports changed.

use crate::ast::Verb;
use crate::diagnostics::Diagnostic;
use crate::endpoints::Kind as Source;
use crate::execution::{Expr, Field, Kind};
use crate::extensions::EvalCx;
use crate::types::{TypeRef, Value};
use std::ops::Range;

/// The one system a transaction's participants share (HQL-0051): every
/// participant a table of one SQLite file or of one PostgreSQL database.
pub(crate) fn shared_system(
    participants: &[std::sync::Arc<crate::io::Endpoint>],
    vault: &crate::vault::Vault,
) -> Result<crate::transaction::System, String> {
    use crate::transaction::System;
    let mut shared: Option<System> = None;
    for endpoint in participants {
        let configured = match endpoint.configured_id() {
            Some(id) => crate::extensions::io::configured_endpoint(id, vault)?.clone(),
            None => crate::extensions::io::constructed_table(endpoint).ok_or_else(|| {
                format!(
                    "{} takes part in no transaction: a participant is a table of a database",
                    endpoint.describe()
                )
            })?,
        };
        let system = match &configured.kind {
            Source::Sqlite { path, .. } => System::Sqlite {
                path: crate::io::database_path(path),
            },
            Source::Postgres { dsn_env, .. } => System::Postgres {
                dsn: crate::endpoints::secret(dsn_env, "dsn_env")
                    .map_err(|message| format!("endpoint `{}`: {message}", configured.id))?,
            },
            Source::Spark {
                server, token_env, ..
            } => System::Spark {
                url: crate::extensions::io::spark_server(
                    &configured,
                    server,
                    token_env.as_deref(),
                )?
                .url,
            },
            other => {
                return Err(format!(
                    "endpoint `{}` is a `{}` endpoint, which takes part in no transaction",
                    configured.id,
                    other.name()
                ));
            }
        };
        match &shared {
            None => shared = Some(system),
            Some(held) if *held == system => {}
            Some(held) => {
                let what = match held {
                    System::Sqlite { .. } => "database file",
                    System::Postgres { .. } => "database",
                    System::Spark { .. } => "table",
                };
                return Err(format!(
                    "a transaction's participants are in one {what}, and {} is in another",
                    endpoint.describe()
                ));
            }
        }
    }
    shared.ok_or_else(|| "a transaction has at least one participant".to_owned())
}

/// Which system's text a statement is written in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Sqlite,
    Postgres,
    /// Spark SQL binds no parameter: each value is written as a literal the
    /// endpoint reads, text escaped as the kind escapes it.
    Spark,
}

/// A value bound to a statement, by what the system is sent.
#[derive(Debug, Clone)]
pub(crate) enum Param {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    /// A decimal as its digits, cast by the system.
    Numeric(String),
}

impl Param {
    /// The cast a PostgreSQL placeholder is written with, so a parameter's
    /// type is never inferred.
    fn cast(&self) -> &'static str {
        match self {
            Self::Int(_) => "::int8",
            Self::Float(_) => "::float8",
            Self::Bool(_) => "::bool",
            Self::Text(_) => "::text",
            // The digits are sent as text, so the server types the
            // parameter as text and casts it, rather than typing it as
            // numeric and refusing the text.
            Self::Numeric(_) => "::text::numeric",
        }
    }
}

/// An identifier quoted as every statement writes one.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A table name, schema-qualified where it holds a dot.
fn table_name(table: &str, dialect: Dialect) -> String {
    match dialect {
        Dialect::Postgres => table.split('.').map(quoted).collect::<Vec<_>>().join("."),
        Dialect::Spark => crate::spark::table_name(table),
        Dialect::Sqlite => quoted(table),
    }
}

/// Run a mutation: resolve the endpoint to the table it names, translate
/// the clauses, and run the one statement.
#[expect(clippy::too_many_arguments, reason = "the lowered node's parts")]
pub(crate) fn run(
    cx: &mut dyn EvalCx,
    verb: Verb,
    target: &Expr,
    rows: Option<&Expr>,
    row: &TypeRef,
    filters: &[(String, Expr)],
    set: Option<&(String, Vec<(String, Expr)>)>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    if let crate::io::Policy::Document { .. } = crate::io::policy() {
        return Err(failed(format!(
            "a document changes no table: `{}` is refused while a document is rendered",
            verb.word()
        )));
    }
    let endpoint = match cx.evaluate(target)? {
        Value::Endpoint(endpoint) => endpoint,
        other => {
            return Err(Diagnostic::runtime(
                target.span.clone(),
                format!("`{}` changes an endpoint, not {}", verb.word(), other.type_of()),
            ));
        }
    };
    let configured = match endpoint.configured_id() {
        Some(id) => crate::extensions::io::configured_endpoint(id, cx.vault())
            .map_err(failed)?
            .clone(),
        None => crate::extensions::io::constructed_table(&endpoint).ok_or_else(|| {
            failed(format!(
                "`{}` changes a table of a database, and {} is not one",
                verb.word(),
                endpoint.describe()
            ))
        })?,
    };
    let (dialect, table) = match &configured.kind {
        Source::Sqlite { table, .. } => (Dialect::Sqlite, table.clone()),
        Source::Postgres { table, .. } => (Dialect::Postgres, table.clone()),
        Source::Spark { table, .. } => (Dialect::Spark, table.clone()),
        other => {
            return Err(failed(format!(
                "`{}` changes a table of a database, and endpoint `{}` is a `{}` endpoint",
                verb.word(),
                configured.id,
                other.name()
            )));
        }
    };
    if let Some(rows) = rows {
        let written = cx.evaluate(rows)?;
        return crate::extensions::io::insert_rows(cx, &configured, row, written, span);
    }
    // Each field of the row under the column the boundary holds it in.
    let boundary = crate::extensions::io::boundary_for(&configured, row, None).map_err(failed)?;
    let column = |field: &str| -> String {
        boundary
            .as_ref()
            .and_then(|boundary| {
                boundary
                    .fields
                    .iter()
                    .find(|held| held.field.name == field)
            })
            .map_or_else(
                || field.to_owned(),
                |held| crate::boundary::written(&held.external.value),
            )
    };
    let mut translator = Translator {
        cx,
        dialect,
        params: Vec::new(),
        verb,
    };
    let mut conditions = Vec::new();
    for (parameter, body) in filters {
        conditions.push(translator.expression(body, parameter, &column)?);
    }
    let selection = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    let table = table_name(&table, dialect);
    let statement = match (verb, set) {
        (Verb::Delete, _) => format!("DELETE FROM {table}{selection}"),
        (Verb::Update, Some((parameter, entries))) => {
            let mut assignments = Vec::with_capacity(entries.len());
            for (field, value) in entries {
                let value = translator.expression(value, parameter, &column)?;
                assignments.push(format!("{} = {value}", quoted(&column(field))));
            }
            format!("UPDATE {table} SET {}{selection}", assignments.join(", "))
        }
        (Verb::Update, None) | (Verb::Insert, _) => {
            return Err(failed(format!(
                "`{}` was lowered without its clauses",
                verb.word()
            )));
        }
    };
    let params = translator.params;
    let changed = match &configured.kind {
        Source::Sqlite { path, .. } => {
            let path = crate::io::database_path(path);
            let count = crate::endpoints::sqlite_execute(&path, &statement, &params)
                .map_err(|message| failed(format!("endpoint `{}`: {message}", configured.id)))?;
            u64::try_from(count).unwrap_or(u64::MAX)
        }
        Source::Postgres { dsn_env, .. } => {
            crate::io::policy_permits_network().map_err(failed)?;
            let dsn = crate::endpoints::secret(dsn_env, "dsn_env")
                .map_err(|message| failed(format!("endpoint `{}`: {message}", configured.id)))?;
            crate::postgres::execute(&dsn, &statement, &params)
                .map_err(|message| failed(format!("endpoint `{}`: {message}", configured.id)))?
        }
        Source::Spark {
            server, token_env, ..
        } => {
            crate::io::policy_permits_network().map_err(failed)?;
            let server =
                crate::extensions::io::spark_server(&configured, server, token_env.as_deref())
                    .map_err(failed)?;
            // The endpoint reports the rows a Delta statement changed;
            // another format reports none, and the count is unknown.
            crate::spark::execute(&server, &statement)
                .map_err(|message| failed(format!("endpoint `{}`: {message}", configured.id)))?
                .map_or(0, |n| u64::try_from(n).unwrap_or(0))
        }
        _ => unreachable!("refused above"),
    };
    Ok(Value::Int(i64::try_from(changed).unwrap_or(i64::MAX)))
}

/// Writes a clause as the system's text, binding what HQL computes.
struct Translator<'a> {
    cx: &'a mut dyn EvalCx,
    dialect: Dialect,
    params: Vec<Param>,
    verb: Verb,
}

impl Translator<'_> {
    /// Bind a value and write its placeholder; absence is `NULL`.
    fn bind(&mut self, value: Value, span: &Range<usize>) -> Result<String, Diagnostic> {
        let param = match value {
            Value::Int(n) => Param::Int(n),
            Value::Float(x) => Param::Float(x),
            Value::Bool(b) => Param::Bool(b),
            Value::Str(text) => Param::Text(text.to_string()),
            Value::Decimal(d) => Param::Numeric(d.to_string()),
            Value::Time(time) => Param::Text(time.to_string()),
            Value::Present(held) => return self.bind(held.as_ref().clone(), span),
            Value::Absent(_) => return Ok("NULL".to_owned()),
            Value::Data(tree) => match tree.as_ref() {
                crate::data::Data::Empty => return Ok("NULL".to_owned()),
                crate::data::Data::Int(n) => Param::Int(*n),
                crate::data::Data::Float(x) => Param::Float(*x),
                crate::data::Data::Bool(b) => Param::Bool(*b),
                crate::data::Data::Str(text) => Param::Text(text.clone()),
                crate::data::Data::Decimal(d) => Param::Numeric(d.to_string()),
                crate::data::Data::Time(time) => Param::Text(time.to_string()),
                crate::data::Data::List(_) | crate::data::Data::Map(_) => {
                    Param::Text(tree.to_json().to_string())
                }
            },
            other => {
                return Err(Diagnostic::runtime(
                    span.clone(),
                    format!(
                        "`{}` sends a scalar to the endpoint's system, not {}",
                        self.verb.word(),
                        other.type_of()
                    ),
                ));
            }
        };
        if self.dialect == Dialect::Spark {
            return Ok(match param {
                Param::Int(n) => n.to_string(),
                Param::Float(x) => x.to_string(),
                Param::Bool(b) => if b { "TRUE" } else { "FALSE" }.to_owned(),
                Param::Text(text) => crate::spark::literal(&text),
                Param::Numeric(digits) => digits,
            });
        }
        self.params.push(param);
        let n = self.params.len();
        Ok(match self.dialect {
            Dialect::Sqlite => format!("?{n}"),
            Dialect::Postgres => format!("${n}{}", self.params[n - 1].cast()),
            Dialect::Spark => unreachable!("written above"),
        })
    }

    /// The text of a clause over the row `parameter`.
    fn expression(
        &mut self,
        expression: &Expr,
        parameter: &str,
        column: &dyn Fn(&str) -> String,
    ) -> Result<String, Diagnostic> {
        if !mentions(expression, parameter) {
            let value = self.cx.evaluate(expression)?;
            return self.bind(value, &expression.span);
        }
        let not_translated = || {
            Diagnostic::runtime(
                expression.span.clone(),
                format!(
                    "`{}` runs on the endpoint's system, and this is not translated there",
                    self.verb.word()
                ),
            )
        };
        match &expression.kind {
            Kind::Field(inner, field)
                if matches!(&inner.kind, Kind::Ident(held) if held == parameter) =>
            {
                let name = match field {
                    Field::Record(name) | Field::Key(name) => name.clone(),
                    _ => return Err(not_translated()),
                };
                Ok(quoted(&column(&name)))
            }
            Kind::Arith { first, rest } => {
                let mut text = self.expression(first, parameter, column)?;
                for (operation, right, _) in rest {
                    let right = self.expression(right, parameter, column)?;
                    let symbol = match operation {
                        crate::ast::Arith::Add => "+",
                        crate::ast::Arith::Sub => "-",
                        crate::ast::Arith::Mul => "*",
                        crate::ast::Arith::Div => "/",
                        crate::ast::Arith::Rem => "%",
                    };
                    text = format!("({text} {symbol} {right})");
                }
                Ok(text)
            }
            Kind::Compare {
                left,
                right,
                negated,
            } => {
                let left = self.expression(left, parameter, column)?;
                let right = self.expression(right, parameter, column)?;
                Ok(format!(
                    "({left} {} {right})",
                    if *negated { "<>" } else { "=" }
                ))
            }
            Kind::Relate {
                left,
                right,
                relation,
            } => {
                let left = self.expression(left, parameter, column)?;
                let right = self.expression(right, parameter, column)?;
                Ok(format!("({left} {} {right})", relation.symbol()))
            }
            Kind::Logic { first, rest } => {
                let mut text = self.expression(first, parameter, column)?;
                for (connective, right) in rest {
                    let right = self.expression(right, parameter, column)?;
                    let word = match connective {
                        crate::ast::Logic::And => "AND",
                        crate::ast::Logic::Or => "OR",
                    };
                    text = format!("({text} {word} {right})");
                }
                Ok(text)
            }
            Kind::Not(inner) => Ok(format!(
                "(NOT {})",
                self.expression(inner, parameter, column)?
            )),
            // An operator resolved through a core trait: `==` through `Eq`,
            // arithmetic through `Add` and its kin.
            Kind::Method {
                trait_name,
                receiver,
                arguments,
                ..
            } if arguments.len() == 1 => {
                let symbol = match trait_name.as_str() {
                    "Eq" => "=",
                    "Add" => "+",
                    "Sub" => "-",
                    "Mul" => "*",
                    "Div" => "/",
                    "Rem" => "%",
                    _ => return Err(not_translated()),
                };
                let left = self.expression(receiver, parameter, column)?;
                let right = self.expression(&arguments[0], parameter, column)?;
                Ok(format!("({left} {symbol} {right})"))
            }
            Kind::Step {
                step,
                input,
                arguments,
                ..
            } => match step.name {
                "is_none" | "is_some" if arguments.is_empty() => {
                    let inner = self.expression(input, parameter, column)?;
                    Ok(format!(
                        "({inner} IS {})",
                        if step.name == "is_none" { "NULL" } else { "NOT NULL" }
                    ))
                }
                "contains" if arguments.len() == 1 && !mentions(input, parameter) => {
                    let collection = self.cx.evaluate(input)?;
                    let members = collection.elements().ok_or_else(not_translated)?;
                    let tested = self.expression(&arguments[0].value, parameter, column)?;
                    if members.is_empty() {
                        return Ok(match self.dialect {
                            Dialect::Sqlite => "0".to_owned(),
                            Dialect::Postgres | Dialect::Spark => "FALSE".to_owned(),
                        });
                    }
                    let mut bound = Vec::with_capacity(members.len());
                    for member in members {
                        bound.push(self.bind(member, &input.span)?);
                    }
                    Ok(format!("({tested} IN ({}))", bound.join(", ")))
                }
                _ => Err(not_translated()),
            },
            _ => Err(not_translated()),
        }
    }
}

/// Whether a lowered expression reads `name`.
fn mentions(expression: &Expr, name: &str) -> bool {
    match &expression.kind {
        Kind::Ident(held) => held == name,
        Kind::Lambda { .. } | Kind::Generate { .. } | Kind::Mutation { .. } => true,
        _ => expression
            .children()
            .into_iter()
            .any(|child| mentions(child, name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_name_is_quoted_and_schema_qualified_for_postgres() {
        assert_eq!(table_name("audit.events", Dialect::Postgres), "\"audit\".\"events\"");
        assert_eq!(table_name("audit.events", Dialect::Sqlite), "\"audit.events\"");
        assert_eq!(quoted("odd\"name"), "\"odd\"\"name\"");
    }

    #[test]
    fn a_postgres_placeholder_carries_its_cast() {
        assert_eq!(Param::Int(1).cast(), "::int8");
        assert_eq!(Param::Numeric("1.50".to_owned()).cast(), "::text::numeric");
    }
}
