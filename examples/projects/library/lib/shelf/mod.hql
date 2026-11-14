/// The titles on a shelf, read from a file kept beside this module.
module shelf

import io
import io.fs

/// The name of the directory holding this module.
fn home() : String = fs.source_dir().name()

/// Each line of `titles.txt`, the file beside this module.
fn titles() : List<String> = source(file("titles.txt"), Lines) | List
