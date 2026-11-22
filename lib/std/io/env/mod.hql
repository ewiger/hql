// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/05-env.hmd.

/// What a program reads from the invocation that runs it: environment
/// variables, named arguments and positional arguments. The environment is
/// read through a grant: a program run alone is granted every variable; a
/// project's `.hql/config.toml` narrows the grant to
/// `env = ["NAME", "PREFIX_*"]`, a `*` matching any suffix; `hql render`
/// grants none. A variable that is unset, withheld by the grant, or whose
/// value is not UTF-8 reads as absent, and the three are indistinguishable.
/// Named arguments are the `--arg name=value` pairs of the invocation, and
/// positional arguments are the words after the program,
/// `hql report.hql 2026 q1`; both are always granted. Each operation reads
/// a value fixed for the whole evaluation. A program imports the module,
/// `import io.env`, and reaches it as `env`, `env.var(…)`, or binds its
/// functions, `from io.env import *`.
module io.env

import core
import io

/// The value of the environment variable `name`, or `none` where it is
/// unset, withheld by the grant, or not UTF-8:
/// `env.var("API_TOKEN") | unwrap_or("")`.
effectful fn var(name: String) : Option<String>

/// Every granted environment variable that is set and UTF-8, by name.
effectful fn vars() : Map<String, String>

/// The value of the invocation's `--arg name=value`, or `none` where no
/// `--arg` names it. An argument is text; a program converts it, as
/// `env.arg("limit") | and_then(s => s.to_int())`.
effectful fn arg(name: String) : Option<String>

/// Every `--arg name=value` of the invocation, by name.
effectful fn args() : Map<String, String>

/// The invocation's positional arguments, the words after the program
/// file or the inline program, in order: `./report.hql 2026 q1` gives
/// `["2026", "q1"]`. The program's own path is not among them; it is
/// `fs.source_dir()` and the file's name. The running process is
/// `io.process` (HQL-0064).
effectful fn argv() : Seq<String>
