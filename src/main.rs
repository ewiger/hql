//! The `hql` binary: a thin shell around the library's command-line host.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(hql::cli::main(hql::cli::parse()))
}
