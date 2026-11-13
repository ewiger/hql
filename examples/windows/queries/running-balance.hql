// status: valid-now
// feature: a running balance per account: the `running` frame summed, one value per ledger entry
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [100, 50, 70, 70, 80]

type Posting {
    account : String
    seq     : Int
    amount  : Int
}

ledger = [
    Posting(account = "a", seq = 1, amount = 100),
    Posting(account = "b", seq = 1, amount = 50),
    Posting(account = "a", seq = 2, amount = -30),
    Posting(account = "b", seq = 2, amount = 20),
    Posting(account = "a", seq = 3, amount = 10),
]
ledger
| over(by = t => t.account, order = t => t.seq,
       into = (t, w) => w | running | map(x => x.amount) | sum)
