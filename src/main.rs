//! The `hql` binary: a thin shell around the library's command-line host.

#![forbid(unsafe_code)]

use std::process::ExitCode;

/// How much stack the host runs on. A deeply recursive program is a deeply
/// recursive evaluation, and the main thread's default would end one a few
/// hundred calls deep.
const STACK: usize = 256 * 1024 * 1024;

fn main() -> ExitCode {
    let host = std::thread::Builder::new()
        .name("hql".to_owned())
        .stack_size(STACK)
        .spawn(|| hql::cli::main(hql::cli::parse()));
    match host.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => ExitCode::from(code),
        // The host panicked, or could not be started at all: a defect, not a
        // program failure, reported as one.
        Ok(Err(_)) | Err(_) => ExitCode::from(101),
    }
}
