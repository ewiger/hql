// status: valid-now
// feature: an update matching no row counts 0, and the transfer that reads the count rolls its debit back with it
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Data
// expected: {balances: [150, 40], outcome: "rolled_back: no such account", recorded: 1}

import io.db.sqlite

type Account { id : Int, owner : String, balance : Int }
type Transfer { id : Int, from_id : Int, to_id : Int, amount : Int }

accounts  = sqlite("ledger.db", table = "accounts", order = ["id"])
transfers = sqlite("ledger.db", table = "transfers", order = ["id"])

opened = [Account(id = 1, owner = "ada", balance = 150), Account(id = 2, owner = "grace", balance = 40)]
| target(accounts)
history = [Transfer(id = 1, from_id = 2, to_id = 1, amount = 10)] | target(transfers)

outcome = with transaction(attempts = 3) tx on {accounts, transfers} {
    sender = source(accounts) | of_type(Account) | filter(a => a.id == 1) | one
    tx.rollback("insufficient balance") given sender.balance < 100
    debited = update accounts | of_type(Account) | filter(a => a.id == 1) | set(a => {balance: a.balance - 100})
    credited = update accounts | of_type(Account) | filter(a => a.id == 9) | set(a => {balance: a.balance + 100})
    tx.rollback("no such account") given debited != 1 or credited != 1
    insert transfers Transfer(id = 2, from_id = 1, to_id = 2, amount = 100)
}

{
    outcome: match outcome {
        ok(n) => fmt("committed: {} transfer recorded", n),
        err(f) => f.kind + ": " + f.reason,
    },
    balances: source(accounts) | of_type(Account) | map(a => a.balance),
    recorded: source(transfers) | size,
}
