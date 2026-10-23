// status: invalid
// feature: a Table<Book> is not a Table<Film>
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

type Book {
  title : String
}

type Film {
  title : String
}

films : Table<Film> = [Book(title = "Dune")] | Table
films
