// status: invalid
// feature: a transactional endpoint the block changes must be named in the set after on
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
    delete transfers
}
