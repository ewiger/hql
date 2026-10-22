// status: invalid
// feature: A Table column over a declared record names one of its fields
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `Book` declares no `isbn`, so the column is refused before the program
// runs. Over `Data`, a missing field is an empty cell instead.
type Book {
  title : String
}

[Book(title = "Dune")] | Table(columns = ["title", "isbn"])
