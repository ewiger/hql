// Provided by the `std` extension.
//
// See doc/proposals/HQL-0047/README.md, doc/proposals/HQL-0048/README.md,
// doc/proposals/HQL-0049/README.md and doc/proposals/HQL-0051/README.md.

/// Spark SQL: one table behind a Spark SQL endpoint, a Databricks SQL
/// warehouse or a Spark Thrift Server, as a transport read by `source` and
/// `tap` and written by `target` like any other endpoint (module `io`). The
/// `spark` kind of a configured endpoint is this transport under an id,
/// with a schema boundary. A program imports the module:
/// `import io.db.spark`.
module io.db.spark

import core
import io

/// One table behind a Spark SQL endpoint. It is on the network and carries
/// rows of named columns, so it takes no format. A table ends and reads
/// the same rows each time in one evaluation, so it is `Replayable`: read,
/// it is a `Seq`, a row per element. A Delta table commits one statement
/// atomically, so a transaction scope over it holds this one table and at
/// most one statement; its rows are inserted, changed and removed in place
/// where the table's format allows it (HQL-0051).
type Spark : {Network, RowTransport, Replayable, Writable, Transactional, Insertable, Updatable, Deletable}

/// The table `table`, qualified by catalog and schema where it holds dots,
/// behind the Spark SQL endpoint at `url` or at the address the
/// environment variable `url_env` holds, one of the two, as a `Spark`
/// transport; its parameters are the keys of the `spark` kind, each
/// written by name. `token_env` names the variable holding an access
/// token, where the endpoint asks for one. Rows are read in the order of
/// the columns `order` names, and as the table is scanned without them.
/// Constructing it opens no session.
///
/// A constructed table has no schema boundary: a row is read as `Data`, or
/// as the type an annotation names, and a declared record is written a
/// column per field, the table's rows replaced. Its columns, `mode` and
/// `strictness` are configured on a `spark` endpoint.
fn spark(*, table: String, url: String = "", url_env: String = "", token_env: String = "", order: List<String> = []) : Spark
