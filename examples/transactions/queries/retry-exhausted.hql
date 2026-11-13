// status: valid-now
// feature: tx.retry rolls the attempt back and runs the block again, and with no attempt left the scope is err exhausted
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: String
// expected: exhausted after 3 attempts; balances [150, 40]

import io.db.sqlite

type Account { id : Int, owner : String, balance : Int }
type Transfer { id : Int, from_id : Int, to_id : Int, amount : Int }

accounts  = sqlite("ledger.db", table = "accounts", order = ["id"])
transfers = sqlite("ledger.db", table = "transfers", order = ["id"])

opened = [Account(id = 1, owner = "ada", balance = 150), Account(id = 2, owner = "grace", balance = 40)]
| target(accounts)
history = [Transfer(id = 1, from_id = 2, to_id = 1, amount = 10)] | target(transfers)

outcome = with transaction(attempts = 3) tx on accounts {
    update accounts | of_type(Account) | set(a => {balance: 0})
    tx.retry()
}

match outcome {
    ok(n) => "committed",
    err(f) => fmt("{} after {} attempts; balances {}", f.kind, f.attempts, source(accounts) | of_type(Account) | map(a => a.balance)),
}
