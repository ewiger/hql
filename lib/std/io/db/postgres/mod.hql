// Provided by the `std` extension.
//
// See doc/proposals/HQL-0047/index.hmd, doc/proposals/HQL-0048/index.hmd and
// doc/proposals/HQL-0051/index.hmd.

/// PostgreSQL: one table of a database, as a transport read by `source` and
/// `tap` and written by `target` like any other endpoint (module `io`). The
/// `postgres` kind of a configured endpoint is this transport under an id,
/// with a schema boundary. A program imports the module:
/// `import io.db.postgres`.
module io.db.postgres

import core
import io

/// One table of a PostgreSQL database. It is on the network and carries
/// rows of named columns, so it takes no format. A table ends and reads the
/// same rows each time in one evaluation, so it is `Replayable`: read, it
/// is a `Seq`, a row per element. The database is one atomic scope over
/// every table it holds, and a table's rows are inserted, changed and
/// removed in place (HQL-0051).
type Postgres : {Network, RowTransport, Replayable, Writable, Transactional, Insertable, Updatable, Deletable}

/// The table `table` of the database whose connection string the
/// environment variable `dsn_env` holds, as a `Postgres` transport; its
/// parameters are the keys of the `postgres` kind, each written by name. A
/// connection string may hold a password, so a program names the variable
/// and never the string.
/// `table` is schema-qualified where it holds a dot, `audit.events`. Rows
/// are read in the order of the columns `order` names, and as the table is
/// scanned without them. Constructing it opens no connection.
///
/// A constructed table has no schema boundary: a row is read as `Data`, or
/// as the type an annotation names, and a declared record is written a
/// column per field, the table's rows replaced. Its columns, `mode` and
/// `strictness` are configured on a `postgres` endpoint.
fn postgres(*, dsn_env: String, table: String, order: List<String> = []) : Postgres
