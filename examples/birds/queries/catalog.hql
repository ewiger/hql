// status: valid-now
// feature: Bird cards filtered and sorted by name
// implementation: implemented
// environment: knowledge-v1
// vault: birds/wiki
// expected-type: Seq<String>
// expected: [Barn owl, Common raven, European robin]

bird_cards = cards
| filter(c => c.metadata.category == "bird")

// Sorting supplies positions; the collection and result types are inferred.
bird_cards
| sort(by = c => c.name)
| map(c => c.title)
