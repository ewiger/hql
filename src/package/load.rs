//! Loading a package's library into the process.
//!
//! This is the one place the crate uses `unsafe`: opening a library runs its
//! initialisers, calling a symbol trusts its signature, and reading the
//! extension trusts that the library's copy of this crate is this build.
//! The [`Header`] is what makes the last of those a checked claim: it has a
//! layout of its own, and the extension behind it is read only once the
//! header states this build.

#![allow(unsafe_code)]

use crate::ext::{self, Header};
use crate::extensions::Extension;
use std::path::Path;

/// The signature of the symbol a package's library exports.
type Entry = unsafe extern "C" fn() -> Header;

/// The text a header points at.
///
/// # Safety
///
/// `start` and `length` are a header's, from a library that exports the
/// entry symbol: they describe a string constant of that library.
unsafe fn stated<'a>(start: *const u8, length: usize) -> &'a str {
    if start.is_null() {
        return "";
    }
    // SAFETY: the caller's contract: a string constant of the library.
    let bytes = unsafe { std::slice::from_raw_parts(start, length) };
    std::str::from_utf8(bytes).unwrap_or("")
}

/// Open a package's library and return the extension it carries. The
/// library stays loaded for the rest of the process: the extension and its
/// functions are held for as long.
///
/// # Errors
///
/// Returns why the library cannot be used: it does not open, it exports no
/// entry symbol, or it was built against another `hql`, with another
/// compiler, or with other layouts of the types the two exchange. A library
/// that is refused is closed again.
pub(super) fn open(package: &str, path: &Path) -> Result<&'static Extension, String> {
    // SAFETY: the library is a package's, built by `hql package` from a
    // crate the user chose to install; its initialisers are that crate's.
    let library = unsafe { libloading::Library::new(path) }
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let header = {
        // SAFETY: the symbol is the one `hql::extension!` writes, whose
        // signature is `Entry` in every version of ABI 1.
        let entry = unsafe { library.get::<Entry>(b"hql_extension_v1\0") }.map_err(|_| {
            format!(
                "{} exports no `{}`: the crate writes `hql::extension!` once",
                path.display(),
                ext::ENTRY
            )
        })?;
        // SAFETY: as above; the function takes nothing and returns a
        // `repr(C)` value.
        unsafe { entry() }
    };
    if header.abi != ext::ABI {
        return Err(format!(
            "package `{package}` states extension ABI {}; this hql loads ABI {}",
            header.abi,
            ext::ABI
        ));
    }
    // SAFETY: the header is the library's, read while the library is open.
    let (hql, rustc) = unsafe {
        (
            stated(header.hql, header.hql_len),
            stated(header.rustc, header.rustc_len),
        )
    };
    if hql != ext::HQL || rustc != ext::RUSTC {
        return Err(format!(
            "package `{package}` was built against hql {hql} with rustc {rustc}; this is hql \
             {} with rustc {}: run `hql package rebuild {package}`",
            ext::HQL,
            ext::RUSTC
        ));
    }
    if header.layout != ext::layout() {
        return Err(format!(
            "package `{package}` was built with other versions or features of the crates hql \
             depends on, so the values the two exchange differ in size: run `hql package \
             rebuild {package}`"
        ));
    }
    if header.extension.is_null() {
        return Err(format!("package `{package}` carries no extension"));
    }
    // SAFETY: the header states this build of this crate, so the extension
    // is a value of this crate's `Extension`; it is a static of the library,
    // which is never unloaded.
    let extension: &'static Extension = unsafe { &*header.extension };
    std::mem::forget(library);
    Ok(extension)
}
