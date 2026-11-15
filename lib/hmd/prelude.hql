// The `hmd` package's implicit import list (HQL-0059). Not a module: it has
// no name, cannot be imported, and applies only while `hmd` is active, that
// is, once a unit, `[modules] import`, `--import` or the host imports a
// module of `hmd`. `prelude = false` under `[modules]` withholds it, as it
// withholds `std`'s.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.hmd.
import hmd.vault
import hmd.search
import hmd.search.lexical
import hmd.search.semantic
