// status: valid-now
// feature: two rows equal under the order of `max_by` are resolved by input position, the earlier kept
// implementation: implemented
// environment: pure
// expected-type: Set<Int>
// expected: {15}

// One order revised twice in one second: `at` alone does not decide
// between the two, so the first by input position is kept.
type Event {
    id     : Int
    at     : String
    seq    : Int
    amount : Int
}

rows = [
    Event(id = 6, at = "2026-11-02T11:50:00Z", seq = 1, amount = 15),
    Event(id = 6, at = "2026-11-02T11:50:00Z", seq = 2, amount = 18),
]
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| map(r => r.amount)
