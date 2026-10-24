// status: valid-now
// feature: Table<R> is a table typed by its row: built from records of R, it is a Table<R>
// implementation: implemented
// environment: pure
// expected-type: Table<Book>
// expected-json: [{"title": "Dune", "pages": 412}]

type Book {
  title : String
  pages : Int
}

shelf : Table<Book> = [Book(title = "Dune", pages = 412)] | Table
shelf
