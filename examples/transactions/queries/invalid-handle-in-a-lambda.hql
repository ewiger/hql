// status: invalid
// feature: a control call is a statement of the block, never inside a lambda
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
    [1, 2] | map(x => tx.rollback("no"))
}
