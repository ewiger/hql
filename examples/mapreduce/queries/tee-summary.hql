// status: valid-now
// feature: tee keeps side results of a pipeline while the main line continues
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: List<Int>
// expected: [3, 6, 2]

// Both branches read the filtered books, evaluated once. `shelved` counts
// every book, `short` the books under 200 pages, and the main line goes on to
// count the novels. No branch changes what the main line receives.
novels = cards
| filter(c => c.metadata.category == "book")
| tee shelved = size
| tee short =
    filter(c => match c.metadata.pages { some(Int n) => n < 200, _ => false })
    | size
| filter(c => c.metadata.genre == "novel")
| size

[novels, shelved, short]
