// status: valid-now
// feature: Bird cards as a set and a sorted list
// implementation: implemented
// environment: knowledge-v1
// expected-type: List<String>
// expected: [Barn owl, Common raven, European robin]

bird_cards : Set<Card> = cards
| filter(c => c.metadata.category == "bird")

// Materialization chooses positions; sorting makes their order explicit.
List(bird_cards)
| sort(by = c => c.name)
| map(c => c.title)
