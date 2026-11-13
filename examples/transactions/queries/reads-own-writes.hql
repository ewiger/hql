// status: valid-now
// feature: a read of a participant inside the block sees the attempt's own update, and the scope commits with the block's last value
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Result<Seq<Int>, TransactionFailure>
// expected: ok([150, 45])

import io.db.sqlite

type Account { id : Int, owner : String, balance : Int }
type Transfer { id : Int, from_id : Int, to_id : Int, amount : Int }

accounts  = sqlite("ledger.db", table = "accounts", order = ["id"])
transfers = sqlite("ledger.db", table = "transfers", order = ["id"])

opened = [Account(id = 1, owner = "ada", balance = 150), Account(id = 2, owner = "grace", balance = 40)]
| target(accounts)
history = [Transfer(id = 1, from_id = 2, to_id = 1, amount = 10)] | target(transfers)

with transaction tx on accounts {
    update accounts | of_type(Account) | filter(a => a.id == 2) | set(a => {balance: a.balance + 5})
    source(accounts) | of_type(Account) | map(a => a.balance)
}
