// status: invalid
// feature: a scope inside a scope is refused: a nested scope is a savepoint, which HQL-0051 reserves
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

import io.db.sqlite

type Account { id : Int, owner : String, balance : Int }

accounts  = sqlite("ledger.db", table = "accounts", order = ["id"])
transfers = sqlite("ledger.db", table = "transfers", order = ["id"])

with transaction tx on accounts {
    with transaction inner on accounts { 1 }
}
