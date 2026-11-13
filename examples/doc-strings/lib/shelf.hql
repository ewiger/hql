// Module `shelf`: a workspace module whose declarations carry doc strings,
// under the module root `examples/doc-strings/lib`. The sessions beside it
// print them with `hql help`; until doc strings are parsed, `///` is an
// ordinary comment and the module checks as it is.

/// Books and the pages they hold.
module shelf

/// A book: its title and its number of pages.
type Book { title : String, pages : Int }

/// The pages of every book in `books`, added up. An empty list has `0`.
fn pages(books: List<Book>) : Int = books | map(b => b.pages) | sum
