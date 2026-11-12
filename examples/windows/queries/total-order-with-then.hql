// status: proposed
// feature: `then` composes two comparisons into a total order, so the change sequence decides a tie on `at`
// implementation: pending
// proposal: doc/proposals/HQL-0050/README.md
// environment: pure
// expected-type: Set<Int>
// expected: {18}

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
| map(g => g.items | max_by((a, b) => compare(a.at, b.at) | then(compare(a.seq, b.seq))))
| somes
| map(r => r.amount)
