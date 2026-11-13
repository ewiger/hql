//! The sorted walk: a folder's entries sorted by name, depth first, each
//! folder followed at once by what it holds. The vault reads its documents
//! in this order and `io.fs` lists entries in it, from this one
//! implementation, so the two orders agree by construction. See
//! [HQL-0020](../doc/proposals/HQL-0020/index.hmd) and
//! [HQL-0040](../doc/proposals/HQL-0040/index.hmd).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a listed entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Folder,
    Link,
}

impl Kind {
    /// The word `io.fs` gives an entry of this kind.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Link => "link",
        }
    }
}

/// What a stat reads of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stat {
    pub kind: Kind,
    /// A file's length in bytes; `None` for a folder or a link.
    pub size: Option<u64>,
    /// The entry's own modification time, a link's and not its target's
    /// where links are reported; `None` where the platform gives none.
    pub modified: Option<SystemTime>,
}

/// One entry a walk reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Listed {
    /// Its path below the folder walked.
    pub relative: PathBuf,
    pub stat: Stat,
}

/// What a walk does at a symbolic link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Links {
    /// Report the link as a `Link` and never follow it, so a walk cannot
    /// cycle or leave the folder through one.
    Report,
    /// Read the link as what it points to: the vault's reading.
    Follow,
}

/// How a walk reads a tree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Walk {
    pub links: Links,
    /// Whether entries whose name opens with `.` are listed, and hidden
    /// folders entered.
    pub hidden: bool,
}

/// The entry at `path`, as `links` reads it; `Ok(None)` where nothing is
/// there.
///
/// # Errors
/// A metadata read that fails for a reason other than absence.
pub(crate) fn stat(path: &Path, links: Links) -> io::Result<Option<Stat>> {
    let metadata = match links {
        Links::Report => fs::symlink_metadata(path),
        Links::Follow => fs::metadata(path),
    };
    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let kind = if metadata.file_type().is_symlink() {
        Kind::Link
    } else if metadata.is_dir() {
        Kind::Folder
    } else {
        Kind::File
    };
    let size = (kind == Kind::File).then_some(metadata.len());
    Ok(Some(Stat {
        kind,
        size,
        modified: metadata.modified().ok(),
    }))
}

/// The entries directly in `folder`, sorted by name.
///
/// # Errors
/// The folder cannot be read, or an entry's metadata cannot.
pub(crate) fn entries(folder: &Path, walk: Walk) -> io::Result<Vec<Listed>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(folder)? {
        let name = entry?.file_name();
        if !walk.hidden && name.to_str().is_some_and(|name| name.starts_with('.')) {
            continue;
        }
        names.push(name);
    }
    names.sort();
    let mut listed = Vec::with_capacity(names.len());
    for name in names {
        // An entry removed between the listing and its stat is not there.
        if let Some(stat) = stat(&folder.join(&name), walk.links)? {
            listed.push(Listed {
                relative: PathBuf::from(name),
                stat,
            });
        }
    }
    Ok(listed)
}

/// Visit every entry below `root`, the root itself excluded, in the order
/// of the sorted walk. `visit` returns whether to go on; the walk returns
/// whether it reached its end.
///
/// # Errors
/// A folder or an entry that cannot be read, or what `visit` returns.
pub(crate) fn walk<E: From<io::Error>>(
    root: &Path,
    walk: Walk,
    visit: &mut dyn FnMut(Listed) -> Result<bool, E>,
) -> Result<bool, E> {
    below(root, Path::new(""), walk, visit)
}

fn below<E: From<io::Error>>(
    root: &Path,
    prefix: &Path,
    walk: Walk,
    visit: &mut dyn FnMut(Listed) -> Result<bool, E>,
) -> Result<bool, E> {
    for listed in entries(&root.join(prefix), walk)? {
        let relative = prefix.join(&listed.relative);
        let folder = listed.stat.kind == Kind::Folder;
        if !visit(Listed {
            relative: relative.clone(),
            ..listed
        })? {
            return Ok(false);
        }
        if folder && !below(root, &relative, walk, visit)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A `Path`'s text, read as segments separated by `/`: what `join`,
/// `parent`, `name`, `stem` and `extension` compute, from the text alone.
pub(crate) mod paths {
    /// The text without trailing separators, the root `/` kept.
    fn trimmed(text: &str) -> &str {
        let trimmed = text.trim_end_matches('/');
        if trimmed.is_empty() && text.starts_with('/') {
            "/"
        } else {
            trimmed
        }
    }

    /// `text` with `segment` appended after a `/`; onto `.` or an empty
    /// path, `segment` alone.
    pub(crate) fn join(text: &str, segment: &str) -> String {
        let base = trimmed(text);
        match base {
            "" | "." => segment.to_owned(),
            "/" => format!("/{segment}"),
            _ => format!("{base}/{segment}"),
        }
    }

    /// `text` without its last segment, or `None` for one segment.
    pub(crate) fn parent(text: &str) -> Option<&str> {
        let base = trimmed(text);
        match base.rfind('/') {
            _ if base == "/" => None,
            Some(0) => Some("/"),
            Some(at) => Some(&base[..at]),
            None => None,
        }
    }

    /// The last segment of `text`.
    pub(crate) fn name(text: &str) -> &str {
        let base = trimmed(text);
        if base == "/" {
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
            assert_eq!(parent("a/b"), Some("a"));
            assert_eq!(parent("a/b/"), Some("a"));
            assert_eq!(parent("a"), None);
            assert_eq!(parent("/a"), Some("/"));
            assert_eq!(parent("/"), None);
            assert_eq!(name("data/birds.csv"), "birds.csv");
            assert_eq!(stem("data/birds.csv"), "birds");
            assert_eq!(stem("archive.tar.gz"), "archive.tar");
            assert_eq!(extension("a/b.csv"), Some("csv"));
            assert_eq!(extension("a/.env"), None);
            assert_eq!(extension("a/b"), None);
            assert_eq!(stem(".env"), ".env");
        }

        #[test]
        fn paths_order_by_their_segments() {
            let mut texts = vec!["a-b", "a/b", "a", "./a/c"];
            texts.sort_by_key(|text| order_key(text));
            assert_eq!(texts, ["a", "a/b", "./a/c", "a-b"]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for folder in ["a", "a/b", "a-b", ".hidden"] {
            fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        for (file, text) in [
            ("a/b/c.md", "c"),
            ("a/z.md", "zz"),
            ("a-b/x.md", "xxx"),
            ("top.md", ""),
            (".hidden/h.md", "h"),
        ] {
            fs::write(dir.path().join(file), text).unwrap();
        }
        dir
    }

    fn walked(root: &Path, walk: Walk) -> Vec<(String, Kind, Option<u64>)> {
        let mut seen = Vec::new();
        super::walk::<io::Error>(root, walk, &mut |listed| {
            seen.push((
                listed.relative.display().to_string(),
                listed.stat.kind,
                listed.stat.size,
            ));
            Ok(true)
        })
        .unwrap();
        seen
    }

    #[test]
    fn a_folder_is_followed_at_once_by_what_it_holds_in_name_order() {
        let dir = tree();
        let all = Walk {
            links: Links::Report,
            hidden: true,
        };
        let seen = walked(dir.path(), all);
        let paths: Vec<_> = seen.iter().map(|(path, _, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                ".hidden",
                ".hidden/h.md",
                "a",
                "a/b",
                "a/b/c.md",
                "a/z.md",
                "a-b",
                "a-b/x.md",
                "top.md"
            ]
        );
        // The walk's order is the order of paths compared by segments.
        let mut sorted: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        sorted.sort();
        assert_eq!(sorted, paths.iter().map(PathBuf::from).collect::<Vec<_>>());
        assert_eq!(seen[2].1, Kind::Folder);
        assert_eq!(seen[2].2, None);
        assert_eq!(seen[5], ("a/z.md".to_owned(), Kind::File, Some(2)));
        let visible = walked(
            dir.path(),
            Walk {
                hidden: false,
                ..all
            },
        );
        assert!(visible.iter().all(|(path, _, _)| !path.contains("hidden")));
    }

    #[test]
    fn a_walk_stops_when_its_visitor_does() {
        let dir = tree();
        let mut count = 0;
        let ended = super::walk::<io::Error>(
            dir.path(),
            Walk {
                links: Links::Report,
                hidden: false,
            },
            &mut |_| {
                count += 1;
                Ok(count < 2)
            },
        )
        .unwrap();
        assert!(!ended);
        assert_eq!(count, 2);
    }

    #[cfg(unix)]
    #[test]
    fn a_reported_link_is_not_followed() {
        let dir = tree();
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("loop")).unwrap();
        let report = Walk {
            links: Links::Report,
            hidden: false,
        };
        let seen = walked(dir.path(), report);
        assert!(seen.contains(&("loop".to_owned(), Kind::Link, None)));
        assert!(!seen.iter().any(|(path, _, _)| path.starts_with("loop/")));
        let followed = walked(
            dir.path(),
            Walk {
                links: Links::Follow,
                ..report
            },
        );
        assert!(followed.iter().any(|(path, _, _)| path == "loop/z.md"));
    }
}
