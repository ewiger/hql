// Provided by the `std` extension.
//
// See doc/proposals/HQL-0047/index.hmd and doc/proposals/HQL-0048/index.hmd.

/// SQLite: one table of a database file, as a transport read by `source`
/// and `tap` and written by `target` like any other endpoint (module `io`).
/// The `sqlite` kind of a configured endpoint is this transport under an
/// id, with a schema boundary. A program imports the module:
/// `import io.db.sqlite`.
module io.db.sqlite

import core
import io

/// One table of an SQLite database file. It is on this machine and carries
/// rows of named columns, so it takes no format. A table ends and reads the
/// same rows each time in one evaluation, so it is `Replayable`: read, it
/// is a `Seq`, a row per element. The database file is one atomic scope
/// over every table it holds, and a table's rows are inserted, changed and
/// removed in place (HQL-0051).
type Sqlite : {Direct, RowTransport, Replayable, Writable, Transactional, Insertable, Updatable, Deletable}

/// The table `table` of the database file at `path`, as a `Sqlite`
/// transport; its parameters are the keys of the `sqlite` kind. A relative
/// path resolves as a file's does. Rows are read in the order of the
/// columns `order` names, and in `rowid` order without them. Constructing
/// it opens nothing.
///
/// A constructed table has no schema boundary: a row is read as `Data`, or
/// as the type an annotation names, and a declared record is written a
/// column per field, the table replaced. Its columns, `mode` and
/// `strictness` are configured on a `sqlite` endpoint.
fn sqlite(path: String, table: String, order: List<String> = []) : Sqlite
