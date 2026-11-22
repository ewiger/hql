// Provided by the `std` extension.
//
// See doc/proposals/HQL-0064/parts/02-child-processes.hmd.

/// Child processes: a `Command` names a program and its arguments, and
/// starts nothing until an operation of `io` uses it. `source` and `tap`
/// start it and read its standard output, `target` and `sink` start it and
/// write its standard input, and `run` starts it, waits for it and gives a
/// `Result`. No shell reads the command line: no word is split and no `*`
/// is expanded. `hql` starts each child in a process group of its own, and
/// a child that is terminated takes every process it started with it.
/// Which programs a run may start is the `commands` grant of a project's
/// `.hql/config.toml`; a document starts none. A program imports the
/// module, `from io.process import *`.
module io.process

import core
import io

/// A signal, by its POSIX name without the `SIG` prefix: `TERM`, `KILL`.
/// `number` is the number this platform gives it.
type Signal {
    name   : String
    number : Int
}

/// A program to run, with its arguments. Read, it is the child's standard
/// output; written, its standard input. Each read or write starts the
/// program once, as each read of an `Http` endpoint is one request.
type Command : {Replayable, Writable} {
    program : String
    args    : List<String>
}

/// What a child that exited with status `0` wrote.
type Output {
    stdout : String
    stderr : String
}

/// Why a command did not succeed. `reason` is `not-found`,
/// `not-permitted`, `exited`, `signalled`, `timed-out` or
/// `invalid-text`. `status` is set when the child exited, `signal` when a
/// signal ended it. `stdout` and `stderr` hold what the child wrote before
/// it ended.
type CommandFailure : DataContract {
    command : Command
    reason  : String
    status? : Int
    signal? : Signal
    stdout  : String
    stderr  : String
}

/// The command that runs `program` with `args`. A `program` without `/`
/// is looked up in `PATH`; a relative one with `/` resolves against the
/// directory of this source, as a file's path does. `dir` is the child's
/// working directory, this process's own unless given. `env` is added to
/// the environment the child inherits, the variables the `env` grant
/// gives this program. A child still running after `timeout_ms`
/// milliseconds is terminated. Constructing it starts nothing.
fn command(program: String, args: List<String> = [], *, dir: String = "",
           env: Data = {:}, timeout_ms: Int = 0) : Command

/// Starts the command, writes `input` to its standard input, waits for it
/// to end, and gives what it wrote: `ok(Output)` when it exits `0`, and
/// `err(CommandFailure)` with the reason otherwise.
effectful fn run(self: Command, input: String = "") : Result<Output, CommandFailure>
