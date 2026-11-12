// status: valid-now
// feature: the two latest orders of each customer: `row_number` within each partition, then `filter`
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [2, 3, 5, 6]

type Order {
    id       : Int
    customer : String
    at       : Int
}

type Ranked {
    id : Int
    n  : Int
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
       into = (o, w) => Ranked(id = o.id, n = w | row_number))
| filter(x => x.n <= 2)
| map(x => x.id)
