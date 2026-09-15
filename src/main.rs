//! CLI argument parsing and I/O around the HQL library.

#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf, process::ExitCode};

/// HQL — Hyper Query Language (experimental).
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Type-check and evaluate one expression.
    Eval {
        /// An integer, Boolean, or integer addition expression.
        #[arg(allow_hyphen_values = true)]
        expression: String,
    },
    /// Parse and type-check a UTF-8 .hql file without evaluating it.
    Check {
        /// Path to a file containing one expression.
        file: PathBuf,
    },
}

fn run(cli: Cli) -> Result<String, String> {
    match cli.command {
        Command::Eval { expression } => hql::eval(&expression)
            .map(|value| value.to_string())
            .map_err(|error| format!("<expression>: {error}")),
        Command::Check { file } => {
            let source = fs::read_to_string(&file)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            hql::check(&source)
                .map(|ty| ty.to_string())
                .map_err(|error| format!("{}: {error}", file.display()))
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
