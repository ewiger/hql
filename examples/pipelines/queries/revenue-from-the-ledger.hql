// status: valid-now
// feature: a sqlite table read as a declared record and totalled per customer
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// expected-type: Set<Data>
// expected: {{customer: "ada", total: 135.75}, {customer: "grace", total: 42.00}, {customer: "linus", total: 7.99}}

// A table ends, so `source` reads it whole as a `Seq<Event>`; a `numeric`
// amount keeps its digits as a `Decimal`.
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

orders : Seq<Event> = source("ledger")
orders
| filter(o => o.kind == "order_placed")
| group(by = o => o.customer)
| map(g => {customer: g.key, total: g.items | map(o => o.amount) | sum})
