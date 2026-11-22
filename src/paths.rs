//! A path as a program reads it: text with `/` between its segments, on every
//! host. [`of`] is where a path of the host becomes that text, and the rest
//! is what `join`, `parent`, `name`, `stem` and `extension` compute, from the
//! text alone. See [HQL-0040](../doc/proposals/HQL-0040/index.hmd) and
//! [HQL-0063](../doc/proposals/HQL-0063/index.hmd).

use std::path::Path;

/// The text of a path the host gives, as a program reads it. On Windows the
/// host writes `\` between the segments and opens a canonical path with
/// `\\?\`; elsewhere its text is that text already.
pub(crate) fn of(path: &Path) -> String {
    let text = path.display().to_string();
    if cfg!(windows) { read(&text) } else { text }
}

/// A Windows path's text with `/` between its segments, and without the
/// `\\?\` of an extended-length drive path or share. Any other
/// extended-length path, a volume's among them, keeps its `\\?\`: without it
/// the text names another place.
fn read(text: &str) -> String {
    let plain = match text.strip_prefix(r"\\?\") {
        Some(rest) => match rest.strip_prefix(r"UNC\") {
            Some(share) => format!(r"\\{share}"),
            None if drive(rest.as_bytes(), b'\\') => rest.to_owned(),
            None => text.to_owned(),
        },
        None => text.to_owned(),
    };
    plain.replace('\\', "/")
}

/// Whether `text` opens with a drive: a letter and `:`, at its end or
/// followed by `separator`.
fn drive(text: &[u8], separator: u8) -> bool {
    matches!(text, [letter, b':', rest @ ..]
        if letter.is_ascii_alphabetic() && rest.first().is_none_or(|next| *next == separator))
}

/// How much of `text` is its root: `/`, a drive root `C:/`, or a share
/// `//server/share`; nothing for a relative path.
fn root(text: &str) -> usize {
    if let Some(rest) = text.strip_prefix("//") {
        let mut segments = rest.split('/');
        if let (Some(server), Some(share)) = (segments.next(), segments.next())
            && !server.is_empty()
            && !share.is_empty()
        {
            return 2 + server.len() + 1 + share.len();
        }
    }
    if text.starts_with('/') {
        1
    } else if text.len() >= 3 && drive(text.as_bytes(), b'/') {
        3
    } else {
        0
    }
}

/// The text without trailing separators, a root kept whole.
fn trimmed(text: &str) -> &str {
    let trimmed = text.trim_end_matches('/');
    let root = root(text);
    if trimmed.len() < root {
        &text[..root]
    } else {
        trimmed
    }
}

/// `text` with `segment` appended after a `/`; onto `.` or an empty
/// path, `segment` alone. A `segment` with a root, `/`, a drive root or a
/// share, is a path of its own and is `segment` itself (HQL-0062), as a
/// path given on the command line is joined to the working directory.
pub(crate) fn join(text: &str, segment: &str) -> String {
    // On Windows a segment from the command line is the host's text.
    let written = if cfg!(windows) {
        read(segment)
    } else {
        segment.to_owned()
    };
    if root(&written) > 0 {
        return written;
    }
    let base = trimmed(text);
    match base {
        "" | "." => segment.to_owned(),
        _ if base.ends_with('/') => format!("{base}{segment}"),
        _ => format!("{base}/{segment}"),
    }
}

/// `text` without its last segment, or `None` for one segment or a root.
pub(crate) fn parent(text: &str) -> Option<&str> {
    let base = trimmed(text);
    let root = root(base);
    if root > 0 && base.len() == root {
        return None;
    }
    // Below a share the rest opens with the `/` after it; below `/` and a
    // drive root the root holds that `/` itself.
    match base[root..].rfind('/') {
        Some(0) | None if root > 0 => Some(&base[..root]),
        Some(at) => Some(&base[..root + at]),
        None => None,
    }
}

/// The last segment of `text`; of a root, nothing.
pub(crate) fn name(text: &str) -> &str {
    let base = trimmed(text);
    let root = root(base);
    if root > 0 && base.len() == root {
        return "";
    }
    base.rsplit('/').next().unwrap_or(base)
}

/// Where the extension's `.` is in the last segment: after its first
/// character, so `.env` has none.
fn dot(name: &str) -> Option<usize> {
    name.rfind('.').filter(|at| *at > 0)
}

/// The last segment without its extension.
pub(crate) fn stem(text: &str) -> &str {
    let name = name(text);
    dot(name).map_or(name, |at| &name[..at])
}

/// The text after the last `.` of the last segment.
pub(crate) fn extension(text: &str) -> Option<&str> {
    let name = name(text);
    dot(name).map(|at| &name[at + 1..])
}

/// The order key of a path: its segments compared in turn, each by its
/// text. A separator below every character a segment holds makes the
/// text order of the key that order.
pub(crate) fn order_key(text: &str) -> String {
    text.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("\u{0}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_operation_reads_the_text_alone() {
        assert_eq!(join("data", "birds.csv"), "data/birds.csv");
        assert_eq!(join("data/", "birds.csv"), "data/birds.csv");
        assert_eq!(join(".", "a"), "a");
        assert_eq!(join("/", "a"), "/a");
        assert_eq!(join("/home/ada", "/tmp"), "/tmp");
        assert_eq!(join("data", "/"), "/");
        assert_eq!(join("data", "C:/Users"), "C:/Users");
        assert_eq!(join("data", "//server/share/a"), "//server/share/a");
        assert_eq!(join("data", "C:"), "data/C:");
        assert_eq!(parent("a/b"), Some("a"));
        assert_eq!(parent("a/b/"), Some("a"));
        assert_eq!(parent("a"), None);
        assert_eq!(parent("/a"), Some("/"));
        assert_eq!(parent("/a/b"), Some("/a"));
        assert_eq!(parent("/"), None);
        assert_eq!(name("data/birds.csv"), "birds.csv");
        assert_eq!(name("/"), "");
        assert_eq!(stem("data/birds.csv"), "birds");
        assert_eq!(stem("archive.tar.gz"), "archive.tar");
        assert_eq!(extension("a/b.csv"), Some("csv"));
        assert_eq!(extension("a/.env"), None);
        assert_eq!(extension("a/b"), None);
        assert_eq!(stem(".env"), ".env");
    }

    #[test]
    fn a_drive_root_and_a_share_are_roots() {
        assert_eq!(parent("C:/Users"), Some("C:/"));
        assert_eq!(parent("C:/Users/ada"), Some("C:/Users"));
        assert_eq!(parent("C:/"), None);
        assert_eq!(name("C:/"), "");
        assert_eq!(name("C:/Users"), "Users");
        assert_eq!(join("C:/", "Users"), "C:/Users");
        assert_eq!(parent("//server/share/notes"), Some("//server/share"));
        assert_eq!(
            parent("//server/share/notes/a.md"),
            Some("//server/share/notes")
        );
        assert_eq!(parent("//server/share"), None);
        assert_eq!(parent("//server/share/"), None);
        assert_eq!(name("//server/share"), "");
        assert_eq!(join("//server/share", "notes"), "//server/share/notes");
        // `C:` alone is the current folder of a drive, another place, and a
        // name that happens to hold a `:` is a segment like any other.
        assert_eq!(parent("C:"), None);
        assert_eq!(name("C:"), "C:");
        assert_eq!(parent("a:b/c"), Some("a:b"));
        // `\` is a character of a name: the operations do not ask the host.
        assert_eq!(parent(r"C:\data"), None);
    }

    #[test]
    fn a_windows_path_is_read_with_slashes_and_without_its_prefix() {
        assert_eq!(read(r"C:\Users\ada\notes"), "C:/Users/ada/notes");
        assert_eq!(read(r"\\?\C:\Users\ada\notes"), "C:/Users/ada/notes");
        assert_eq!(read(r"\\?\C:"), "C:");
        assert_eq!(read(r"\\server\share\notes"), "//server/share/notes");
        assert_eq!(read(r"\\?\UNC\server\share\notes"), "//server/share/notes");
        assert_eq!(read(r"books\owl.hmd"), "books/owl.hmd");
        // A volume is neither a drive nor a share, and keeps its prefix.
        assert_eq!(
            read(r"\\?\Volume{b75e2c83}\notes"),
            "//?/Volume{b75e2c83}/notes"
        );
    }

    #[test]
    fn paths_order_by_their_segments() {
        let mut texts = vec!["a-b", "a/b", "a", "./a/c"];
        texts.sort_by_key(|text| order_key(text));
        assert_eq!(texts, ["a", "a/b", "./a/c", "a-b"]);
    }
}
