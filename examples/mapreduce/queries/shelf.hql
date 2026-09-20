// status: valid-now
// feature: Counting per genre by hand, before `group` exists
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: Data
// expected-json: {"books":6,"essays":2,"novels":3,"poetry":1}

// `size` is a reduction: it consumes its whole input and produces one value.
// Its combiner is known to the executor (a sum), so a backend may count each
// partition and add the counts. One filter per genre is the shape `group`
// replaces — see by-genre.hql for the same answer as one pipeline.
books = cards
| filter(c => c.metadata.category == "book")

{
    books: books | size,
    novels: books | filter(c => c.metadata.genre == "novel") | size,
    essays: books | filter(c => c.metadata.genre == "essay") | size,
    poetry: books | filter(c => c.metadata.genre == "poetry") | size,
}
