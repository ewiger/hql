// status: valid-now
// implementation: implemented
// environment: pure
// feature: a Data tree read as a declared record by its shape
// expected-type: List<String>
// expected: ["Dune, 412 pages, sf; Emma, 474 pages, no genre; not a book", "2 books, 886 pages"]

// A tree is a `Book` when it has every required field at its type; an
// optional field without an entry is absent, and an entry the record does
// not name is ignored. `match`, `is` and `of_type` read a tree this way, and
// the value they bind is a `Book`.
type Book {
    title  : String
    pages  : Int
    genre? : String
}

shelf : List<Data> = [
    {title: "Dune", pages: 412, genre: "sf"},
    {title: "Emma", pages: 474, year: 1815},
    {title: "Untitled draft"},
]

fn describe(d: Data) : String = match d {
    Book b => fmt("{}, {} pages, {}", b.title, b.pages, b.genre.unwrap_or("no genre")),
    _      => "not a book",
}

books = shelf | of_type(Book)
[shelf | map(describe) | join("; "), fmt("{} books, {} pages", books | count, books | map(b => b.pages) | sum)]
