// Provided by the `std` extension.
//
// See doc/proposals/HQL-0040/index.hmd.

/// The file system as data: folders, globs over paths, and the entries a
/// folder holds, with their kinds, sizes and modification times.
/// Constructing a `Folder` or a `Glob` opens, stats and resolves nothing;
/// the listing operations read the file system, each once, where it is
/// written, and give a `Seq<Entry>` in the order of a sorted walk, never
/// the file system's. A listing is granted exactly where reading a file at
/// the same path is. A program imports the module: `import io.fs`, and
/// reaches it as `fs`.
module io.fs

import core
import io

/// A folder of the file system at `path`: an endpoint that holds files and
/// folders, as a database holds tables. It is not a `Seq<File>` itself;
/// `entries`, `walk` and `glob` list what it holds.
type Folder : Catalog<File> {
    path : Path
}

/// A pattern over paths, matched against a path relative to the folder it
/// is applied to: `*` matches within one segment, `**` any number of whole
/// segments, `?` one character other than `/`, and `[abc]` one character of
/// the set, `[a-z]` one of the range.
type Glob {
    pattern : String
}

/// One thing a folder holds: its `path`, the listed folder's path joined
/// with the entry's path below it, so `file(e.path)` reads it; its `name`,
/// the last segment; its `kind`, `"file"`, `"folder"` or `"link"`; its
/// `size`, a file's length in bytes, absent for a folder or a link; and its
/// `modified`, the time the entry was last modified, a link's own and not
/// its target's, absent where the platform does not record one. A link is
/// reported and not followed.
type Entry {
    path      : Path
    name      : String
    kind      : String
    size?     : Int
    modified? : Instant
}

/// The `Folder` at `path`, a `String` or a `Path`. Constructing it opens
/// nothing; a relative path resolves as a file's does.
fn folder(path: String) : Folder
fn folder(path: Path) : Folder

/// The `Glob` of `pattern`. A pattern that does not parse, an unclosed `[`
/// or a `**` that is not a whole segment, is an evaluation error naming it.
fn Glob(pattern: String) : Glob

/// Whether `path`, read as relative to the folder the glob is applied to,
/// matches `self`. It reads the text and never the file system.
fn matches(self: Glob, path: Path) : Bool

/// The entries directly in `self`, sorted by name, an entry whose name
/// opens with `.` included. A folder that does not exist, or is not a
/// folder, is an evaluation error naming its path.
effectful fn entries(self: Folder) : Seq<Entry>

/// Every entry below `self`, the folder itself excluded: depth first, each
/// folder's entries sorted by name, and each folder followed at once by
/// what it holds. This is the sorted walk a vault reads its cards in; the
/// vault skips the entries whose name opens with `.`, and `walk` lists them.
effectful fn walk(self: Folder) : Seq<Entry>

/// The entries of `walk(self)` whose path below `self` matches `pattern`,
/// in walk order.
effectful fn glob(self: Folder, pattern: Glob) : Seq<Entry>

/// Whether something is at `self`: a file, a folder or a link. A missing
/// path is `false`, not an error.
effectful fn exists(self: Path) : Bool

/// The `Entry` at `self`, `some(Entry)`, or `none` where nothing is there.
effectful fn entry(self: Path) : Option<Entry>
