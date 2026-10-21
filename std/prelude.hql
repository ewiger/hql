// The standard library's implicit import list. Not a module: it has no name,
// cannot be imported, and is the one file at a module root with a role. Every
// module named here is visible in every source unit as if the unit had
// imported it; `prelude = false` in hql.toml withholds the list, and then
// every module is imported where it is used.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.
import core
import string
import seq
import math
import option
import graph
import format
import io
