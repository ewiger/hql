// status: proposed
// feature: the two latest orders of each customer: `row_number` within each partition, then `filter`
// implementation: pending
// proposal: doc/proposals/HQL-0050/README.md
// environment: pure
// expected-type: List<Int>
// expected: [2, 3, 5, 6]

type Order {
    id       : Int
    customer : String
    at       : Int
}

orders = [
    Order(id = 1, customer = "ada", at = 1),
    Order(id = 2, customer = "ada", at = 3),
    Order(id = 3, customer = "bob", at = 2),
    Order(id = 4, customer = "ada", at = 2),
    Order(id = 5, customer = "bob", at = 1),
    Order(id = 6, customer = "ada", at = 4),
]
orders
| over(by = o => o.customer, order = (a, b) => compare(b.at, a.at),
       into = (o, w) => {id: o.id, n: w | row_number})
| filter(x => x.n <= 2)
| map(x => x.id)
