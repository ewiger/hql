// status: valid-now
// feature: A block body of query steps over the vault, its last line the result
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: List<String>
// expected-json: ["Dune", "Frankenstein", "The Hobbit"]

// Each binding names one step of the query, and the last line is the value.
// The function is called like any other, here with the genre it filters by.
fn titles_of(genre: String) : Seq<String> {
    books = cards | filter(c => c.metadata.category == "book")
    picked = books | filter(c => c.metadata.genre == genre)
    picked | map(c => c.title) | sort
}

List(titles_of("novel"))
