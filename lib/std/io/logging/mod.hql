// Provided by the `std` extension. See doc/proposals/HQL-0021/README.md.

/// Logging: structured events written to a destination, apart from the
/// result. A logger owns no writer: it names a producer and holds a
/// destination, as `sink` would write it (module `io`): `stderr` or a file
/// in a format, a network endpoint, or a `Channel<LogEvent>` that keeps the
/// events as values for the host. A program imports the module:
/// `import logging`.
module io.logging

import core
import io

/// How severe an event is. The five levels are ordered from least to most
/// severe: `Trace`, `Debug`, `Info`, `Warn`, `Error`. A level is tested
/// with `e.level | is(Error)`, and a threshold is passed as a type:
/// `level = Info`.
abstract type LogLevel

/// The least severe `LogLevel`, below `Debug`.
type Trace : LogLevel

/// The `LogLevel` above `Trace` and below `Info`.
type Debug : LogLevel

/// The `LogLevel` above `Debug` and below `Warn`; a logger's threshold
/// unless written.
type Info  : LogLevel

/// The `LogLevel` above `Info` and below `Error`.
type Warn  : LogLevel

/// The most severe `LogLevel`, above `Warn`.
type Error : LogLevel

/// One logged event: its `level`, the `logger` that produced it, its
/// `message` and its `data`. Written as `Text` it is one line, the level,
/// the logger and the message, then the data as an HQL literal unless it is
/// empty:
///
///     INFO birds loaded {count: 312}
///
/// Written as `Json` it is one document with these four keys.
type LogEvent {
    level   : LogLevel
    logger  : String   // the producer's name, dotted by convention: hql.query
    message : String
    data    : Data     // structured context, {:} when none is given
}

/// A producer of events: its `name` and the least severe `level` it
/// writes. Its destination is not a field: what a logger writes to is
/// visible only where it is constructed.
type Logger {
    name  : String
    level : LogLevel   // the least severe level it writes
}

/// A `Logger` named `name` that writes to `to` in `format` each event at
/// least as severe as `level`, `Info` unless written. The format is omitted
/// for a `Channel<LogEvent>`, which takes the events as values.
/// Construction is pure and writes nothing:
/// `Logger(stderr, Text, name = "birds")`.
fn Logger<F : Format, L : LogLevel>(to: Writable, format: Type<F>, *, name: String, level: Type<L> = Info) : Logger

/// Writes one `Trace` event with `message` and `data` to `log`'s
/// destination when `log`'s threshold is `Trace`, and gives `Unit`.
/// Effectful. Called directly or through a pipe: `trace(log, "step")` is
/// `log | trace("step")`.
effectful fn trace(log: Logger, message: String, data: Data = {:}) : Unit

/// Writes one `Debug` event with `message` and `data` to `log`'s
/// destination when `log`'s threshold is at most `Debug`, and gives `Unit`.
/// Effectful: `log | debug("parsed", {rows: 3})`.
effectful fn debug(log: Logger, message: String, data: Data = {:}) : Unit

/// Writes one `Info` event with `message` and `data` to `log`'s destination
/// when `log`'s threshold is at most `Info`, and gives `Unit`. Effectful:
/// with `log = Logger(stdout, Text, name = "birds")`,
/// `log | info("loaded", {count: 312})` writes
/// `INFO birds loaded {count: 312}`.
effectful fn info(log: Logger, message: String, data: Data = {:}) : Unit

/// Writes one `Warn` event with `message` and `data` to `log`'s destination
/// when `log`'s threshold is at most `Warn`, and gives `Unit`. Effectful:
/// `log | warn("no sightings")`.
effectful fn warn(log: Logger, message: String, data: Data = {:}) : Unit

/// Writes one `Error` event with `message` and `data` to `log`'s
/// destination, whatever `log`'s threshold, and gives `Unit`. Effectful:
/// `log | error("unreadable", {path: "birds.json"})`.
effectful fn error(log: Logger, message: String, data: Data = {:}) : Unit

/// The cell `hql render` is running, bound as `cell`. The events written
/// to its `logs` are shown beside the cell's result and never saved into
/// the document.
type Cell {
    logs : Channel<LogEvent>
}
