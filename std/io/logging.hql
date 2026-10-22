// Logging: structured events written to a sink, apart from the result.
// Provided by the `logging` extension, which a program imports:
// `import logging`.
//
// See doc/proposals/HQL-0021/README.md.
//
// A logger owns no writer. It names a producer and holds a destination, as
// sink would write it (io/mod.hql): stderr or a file in a format, a network
// endpoint, or a Channel<LogEvent> that keeps the events as values for the
// host.

// How severe an event is. The five levels are ordered from least to most
// severe: Trace, Debug, Info, Warn, Error. A level is tested with
// `e.level | is(Error)`, and a threshold is passed as a type: `level = Info`.
abstract type LogLevel
type Trace : LogLevel
type Debug : LogLevel
type Info  : LogLevel
type Warn  : LogLevel
type Error : LogLevel

// One event. Formatted as Text it is one line, the level, the logger and the
// message, then the data as an Hql literal unless it is empty:
//     INFO birds loaded {count: 312}
// Formatted as Json it is one document with these four keys.
type LogEvent {
    level   : LogLevel
    logger  : String   // the producer's name, dotted by convention: hql.query
    message : String
    data    : Data     // structured context, {:} when none is given
}

// A producer of events. Its destination is not a field: what a logger writes
// to is visible only where it is constructed.
type Logger {
    name  : String
    level : LogLevel   // the least severe level it writes
}

// Construction, and the five operations, provided by the `logging` extension.
// Construction is pure; each operation is effectful, returns Unit, and writes
// one event when its level is at least the logger's threshold. The format
// is omitted for a Channel<LogEvent>, which takes the events as values, and
// the level is Info unless written. Called directly or through a pipe:
// info(log, "loaded") is log | info("loaded").
fn Logger<F : Format, L : LogLevel>(to: Writable, format: Type<F>, *, name: String, level: Type<L> = Info) : Logger
effectful fn trace(log: Logger, message: String, data: Data = {:}) : Unit
effectful fn debug(log: Logger, message: String, data: Data = {:}) : Unit
effectful fn info(log: Logger, message: String, data: Data = {:}) : Unit
effectful fn warn(log: Logger, message: String, data: Data = {:}) : Unit
effectful fn error(log: Logger, message: String, data: Data = {:}) : Unit

// The cell `hql render` is running, bound as `cell` as `vault` is. Its logs
// are shown beside the cell's result and never saved into the document.
type Cell {
    logs : Channel<LogEvent>
}
