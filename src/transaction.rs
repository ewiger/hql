//! Transaction scopes (HQL-0051): the connection an attempt runs on, the
//! control call that ends it, and what the system said when it refused.
//!
//! A scope opens one connection to the one system its participants share
//! and begins a transaction on it. While the attempt runs, every read and
//! write of that system, by a configured id or a constructed transport,
//! runs on that connection, so a read sees the attempt's own writes and a
//! `sink` commits no row of its own. Evaluation is sequential on this
//! machine, so the connection lives in the thread that evaluates.

use crate::diagnostics::Diagnostic;
use crate::types::Value;
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// What a control call asks of the scope.
#[derive(Debug, Clone)]
pub(crate) enum Control {
    Commit(Value),
    Rollback(String),
    Retry,
}

/// Why a system refused an attempt, where the refusal is one the scope
/// acts on: a concurrent transaction, retried while attempts remain, or a
/// row the system's constraints refuse, never retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Conflict,
    Constraint,
}

/// The one system a scope's participants share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum System {
    Sqlite {
        path: PathBuf,
    },
    Postgres {
        dsn: String,
    },
    /// A Spark SQL endpoint: one table, whose one statement is its own
    /// commit, so the scope opens nothing and holds nothing.
    Spark {
        url: String,
    },
}

enum Active {
    /// A Spark scope: nothing held, the one statement its own commit.
    Spark,
    Sqlite {
        path: PathBuf,
        connection: rusqlite::Connection,
    },
    Postgres {
        dsn: String,
        client: postgres::Client,
    },
}

thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
    static CONTROL: RefCell<Option<Control>> = const { RefCell::new(None) };
    static FAILURE: Cell<Option<Class>> = const { Cell::new(None) };
}

/// The message of the failure a control call leaves its block by. The
/// scope recognizes it by the control it recorded, never by the text.
const LEAVING: &str = "a transaction control call outside its scope";

/// Whether an attempt is running on this thread.
pub(crate) fn in_scope() -> bool {
    ACTIVE.with(|active| active.borrow().is_some())
}

pub(crate) fn set_control(control: Control) {
    if let Control::Commit(value) = &control {
        COMMITTED.with(|held| *held.borrow_mut() = Some(value.clone()));
    }
    CONTROL.with(|held| *held.borrow_mut() = Some(control));
}

pub(crate) fn take_control() -> Option<Control> {
    CONTROL.with(|held| held.borrow_mut().take())
}

thread_local! {
    static COMMITTED: RefCell<Option<Value>> = const { RefCell::new(None) };
}

/// The value a `commit` call asked the scope to commit with, taken once:
/// the scope reads it after the control it belongs to.
pub(crate) fn take_control_value() -> Option<Value> {
    COMMITTED.with(|held| held.borrow_mut().take())
}

/// The failure a control call returns to leave its block.
pub(crate) fn leaving(span: Range<usize>) -> Diagnostic {
    Diagnostic::runtime(span, LEAVING)
}

fn note_failure(class: Class) {
    if in_scope() {
        FAILURE.with(|held| held.set(Some(class)));
    }
}

/// Note a concurrent writer's refusal by a system whose errors are text,
/// a Delta table's among them.
pub(crate) fn note_conflict() {
    note_failure(Class::Conflict);
}

/// The class of the last refusal a participant's system gave in this
/// attempt, taken once.
pub(crate) fn take_failure() -> Option<Class> {
    FAILURE.with(|held| held.take())
}

/// An SQLite failure as a message naming the database, its class noted
/// for the scope it happened in.
pub(crate) fn sqlite_message(path: &Path, error: &rusqlite::Error) -> String {
    use rusqlite::ffi::ErrorCode;
    match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => note_failure(Class::Conflict),
        Some(ErrorCode::ConstraintViolation) => note_failure(Class::Constraint),
        _ => {}
    }
    format!("{}: {error}", path.display())
}

/// A PostgreSQL failure as the server's own sentence, its class noted for
/// the scope it happened in. A connection string is never repeated.
pub(crate) fn postgres_message(error: &postgres::Error) -> String {
    use postgres::error::SqlState;
    match error.code() {
        Some(code)
            if *code == SqlState::T_R_SERIALIZATION_FAILURE
                || *code == SqlState::T_R_DEADLOCK_DETECTED =>
        {
            note_failure(Class::Conflict);
        }
        Some(code) if code.code().starts_with("23") => note_failure(Class::Constraint),
        _ => {}
    }
    match error.as_db_error() {
        Some(db) => db.message().to_owned(),
        None => error.to_string(),
    }
}

fn same_file(held: &Path, asked: &Path) -> bool {
    if held == asked {
        return true;
    }
    match (std::fs::canonicalize(held), std::fs::canonicalize(asked)) {
        (Ok(held), Ok(asked)) => held == asked,
        _ => false,
    }
}

/// Run `f` on the attempt's connection to the database file at `path`,
/// where one is open; `None` where no attempt holds that file.
pub(crate) fn with_sqlite<T>(path: &Path, f: impl FnOnce(&rusqlite::Connection) -> T) -> Option<T> {
    ACTIVE.with(|active| match active.borrow().as_ref() {
        Some(Active::Sqlite {
            path: held,
            connection,
        }) if same_file(held, path) => Some(f(connection)),
        _ => None,
    })
}

/// Run `f` on the attempt's client to the database `dsn` names, where one
/// is open; `None` where no attempt holds that database.
pub(crate) fn with_postgres<T>(dsn: &str, f: impl FnOnce(&mut postgres::Client) -> T) -> Option<T> {
    ACTIVE.with(|active| match active.borrow_mut().as_mut() {
        Some(Active::Postgres { dsn: held, client }) if held == dsn => Some(f(client)),
        _ => None,
    })
}

/// Whether an attempt holds the database `dsn` names, so a write runs on
/// its transaction rather than one of its own.
pub(crate) fn holds_postgres(dsn: &str) -> bool {
    with_postgres(dsn, |_| ()).is_some()
}

/// Begin an attempt: one connection to the system, a transaction begun on
/// it at `isolation` where the system takes one, and the attempt's state
/// cleared.
pub(crate) fn begin(system: &System, isolation: Option<&str>) -> Result<(), String> {
    if in_scope() {
        return Err("a transaction scope is open already".to_owned());
    }
    FAILURE.with(|held| held.set(None));
    CONTROL.with(|held| *held.borrow_mut() = None);
    COMMITTED.with(|held| *held.borrow_mut() = None);
    let active = match system {
        System::Spark { .. } => Active::Spark,
        System::Sqlite { path } => {
            let connection = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            )
            .map_err(|error| sqlite_message(path, &error))?;
            // A database another connection holds is waited for briefly,
            // then refused as a conflict, which the scope retries while
            // attempts remain.
            connection
                .busy_timeout(std::time::Duration::from_millis(100))
                .map_err(|error| sqlite_message(path, &error))?;
            connection
                .execute_batch("BEGIN")
                .map_err(|error| sqlite_message(path, &error))?;
            Active::Sqlite {
                path: path.clone(),
                connection,
            }
        }
        System::Postgres { dsn } => {
            let mut client = postgres::Client::connect(dsn, postgres::NoTls).map_err(|error| {
                format!(
                    "no connection to the database: {}",
                    postgres_message(&error)
                )
            })?;
            let level = match isolation {
                Some("ReadCommitted") => " ISOLATION LEVEL READ COMMITTED",
                Some("RepeatableRead") => " ISOLATION LEVEL REPEATABLE READ",
                Some("Serializable") => " ISOLATION LEVEL SERIALIZABLE",
                _ => "",
            };
            client
                .batch_execute(&format!("BEGIN{level}"))
                .map_err(|error| postgres_message(&error))?;
            Active::Postgres {
                dsn: dsn.clone(),
                client,
            }
        }
    };
    ACTIVE.with(|held| *held.borrow_mut() = Some(active));
    Ok(())
}

/// Commit the attempt and close its connection. A refusal names its class
/// where the scope acts on it.
pub(crate) fn commit() -> Result<(), (Option<Class>, String)> {
    let Some(active) = ACTIVE.with(|held| held.borrow_mut().take()) else {
        return Err((None, "no transaction scope is open".to_owned()));
    };
    FAILURE.with(|held| held.set(None));
    let result = match active {
        Active::Spark => Ok(()),
        Active::Sqlite { path, connection } => connection
            .execute_batch("COMMIT")
            .map_err(|error| sqlite_message_in(&path, &error)),
        Active::Postgres { mut client, .. } => client
            .batch_execute("COMMIT")
            .map_err(|error| postgres_message_in(&error)),
    };
    result.map_err(|message| (FAILURE.with(|held| held.take()), message))
}

/// Roll the attempt back and close its connection. A failure to roll back
/// is the connection's loss, which the close completes.
pub(crate) fn rollback() {
    let active = ACTIVE.with(|held| held.borrow_mut().take());
    FAILURE.with(|held| held.set(None));
    CONTROL.with(|held| *held.borrow_mut() = None);
    match active {
        Some(Active::Sqlite { connection, .. }) => {
            let _ = connection.execute_batch("ROLLBACK");
        }
        Some(Active::Postgres { mut client, .. }) => {
            let _ = client.batch_execute("ROLLBACK");
        }
        Some(Active::Spark) | None => {}
    }
}

/// As [`sqlite_message`], with the class noted whether or not the scope is
/// still registered: the commit has taken the connection out already.
fn sqlite_message_in(path: &Path, error: &rusqlite::Error) -> String {
    use rusqlite::ffi::ErrorCode;
    match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => {
            FAILURE.with(|held| held.set(Some(Class::Conflict)));
        }
        Some(ErrorCode::ConstraintViolation) => {
            FAILURE.with(|held| held.set(Some(Class::Constraint)));
        }
        _ => {}
    }
    format!("{}: {error}", path.display())
}

fn postgres_message_in(error: &postgres::Error) -> String {
    use postgres::error::SqlState;
    match error.code() {
        Some(code)
            if *code == SqlState::T_R_SERIALIZATION_FAILURE
                || *code == SqlState::T_R_DEADLOCK_DETECTED =>
        {
            FAILURE.with(|held| held.set(Some(Class::Conflict)));
        }
        Some(code) if code.code().starts_with("23") => {
            FAILURE.with(|held| held.set(Some(Class::Constraint)));
        }
        _ => {}
    }
    match error.as_db_error() {
        Some(db) => db.message().to_owned(),
        None => error.to_string(),
    }
}
