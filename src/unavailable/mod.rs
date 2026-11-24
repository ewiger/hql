//! What a build without the `native` feature has in place of the modules
//! that reach a server (HQL-0061). Each stand-in has the functions of the
//! module it replaces, so the checker and the evaluator are the same code in
//! both builds: a program that names a server endpoint still checks, and
//! fails only where it would have connected.

pub(crate) mod kafka;
pub(crate) mod nats;
pub(crate) mod net;
pub(crate) mod postgres;
pub(crate) mod process;

/// The failure every stand-in gives: what was asked for, and that this build
/// cannot reach it.
pub(crate) fn reason(what: &str) -> String {
    format!(
        "{what} is unavailable in this build of hql: it was built without the `native` feature, \
         so it reaches no server"
    )
}
