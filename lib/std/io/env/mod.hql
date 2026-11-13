// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/05-env.md.

/// What a program reads from the invocation that runs it: environment
/// variables and named arguments. The environment is read through a grant:
/// a program run alone is granted every variable; a workspace's `hql.toml`
/// narrows the grant to `env = ["NAME", "PREFIX_*"]`, a `*` matching any
/// suffix; `hql render` grants none. A variable that is unset, withheld by
/// the grant, or whose value is not UTF-8 reads as absent, and the three
/// are indistinguishable. Arguments are the `--arg name=value` pairs of the
/// invocation, always granted. Each operation reads a value fixed for the
/// whole evaluation. A program imports the module, `import io.env`, and
/// reaches it as `env`.
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
