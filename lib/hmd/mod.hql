// See doc/proposals/HQL-0059/index.hmd.

/// HyperMarkdown in HQL: the builtin package `hmd`, beside `std`. Importing
/// `hmd`, with `import hmd`, `--import hmd` or `[modules] import`, makes it
/// active for the run: its prelude imports `hmd.vault` and `hmd.search`, the
/// host binds `vault`, `cards` and `graph`, and a document reference
/// `@[[..]]` resolves. Plain HQL has none of these.
module hmd

import core
