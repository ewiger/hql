//! Records what this build of `hql` was compiled with.
//!
//! An installed package's library holds Rust types of this crate, which are
//! the same types only under the same compiler (HQL-0057). The compiler's
//! version and the target are therefore part of what a build is, and are read
//! back as `hql::ext::RUSTC` and `hql::ext::TARGET`.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTC");
    let compiler = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let version = Command::new(compiler)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| {
            text.trim()
                .strip_prefix("rustc ")
                .unwrap_or(text.trim())
                .to_owned()
        })
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=HQL_RUSTC={version}");
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=HQL_TARGET={target}");
}
