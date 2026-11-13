// status: valid-now
// feature: a replayed table deduplicated by key, the latest copy of each kept by `max_by`
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// expected-type: List<Data>
// expected: [{amount: 120.50, id: 1}, {amount: 42.00, id: 2}, {amount: 42.00, id: 3}, {amount: 7.99, id: 4}, {amount: 120.50, id: 5}, {amount: 18.00, id: 6}]

// Nine delivered rows, six events: two delivered twice, one corrected.
// `group` is one group per key, `max_by` the latest copy in each, which is
// SQL's `row_number() over (partition by id order by at desc) = 1`.
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| map(r => {id: r.id, amount: r.amount})
