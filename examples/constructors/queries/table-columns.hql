// status: valid-now
// feature: Table(xs, columns = [...]) writes its columns in the order named
// implementation: implemented
// environment: pure
// expected-type: String
// expected-json: "| year | title      | pages |\n| ---- | ---------- | ----- |\n| 1965 | Dune       | 412   |\n| 1937 | The Hobbit | 310   |"

// `fn Table<T>(rows: Collection<T>, columns: List<String> = []) : Table<T>` in
// `lib/std/format/mod.hql`. One row per record; `columns` names the fields in the
// order every format writes them, here not the order `Book` declares.
type Book {
  title : String
  pages : Int
  year  : Int
}

[Book(title = "Dune", pages = 412, year = 1965), Book(title = "The Hobbit", pages = 310, year = 1937)]
| Table(columns = ["year", "title", "pages"])
| markdown
