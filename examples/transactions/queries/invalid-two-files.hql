// status: invalid
// feature: the participants share one atomic scope: two SQLite files are two systems
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

import io.db.sqlite

type Account { id : Int, owner : String, balance : Int }

accounts  = sqlite("ledger.db", table = "accounts", order = ["id"])
transfers = sqlite("ledger.db", table = "transfers", order = ["id"])

archive = sqlite("archive.db", table = "accounts", order = ["id"])

with transaction tx on {accounts, archive} { 1 }
