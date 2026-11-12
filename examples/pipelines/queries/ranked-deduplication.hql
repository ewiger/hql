// status: valid-now
// feature: the replay deduplicated by `over`: the first row of each key, latest first, which is SQL's `row_number() over (...) = 1`
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// expected-type: List<Data>
// expected: [{amount: 120.50, id: 1}, {amount: 42.00, id: 2}, {amount: 42.00, id: 3}, {amount: 7.99, id: 4}, {amount: 120.50, id: 5}, {amount: 18.00, id: 6}]

// The kept rows are in input order: each at the position of its row.
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

rows : Seq<Event> = source("silver_replayed")
rows
| over(by = r => r.id, order = (a, b) => compare(b.at, a.at),
       into = (r, w) => {row: r, n: w | row_number})
| filter(x => x.n == 1)
| map(x => {id: x.row.id, amount: x.row.amount})
