// status: valid-now
// feature: under `strictness = "open"` an external field no field reads is not read, so a closed record reads a wider source
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: Int
// expected: 4

// `Tally` is closed and declares no `Observer`. Read through `ledger`, whose
// strictness is the default `compatible`, the column would pass to the type
// and be refused as undeclared; `ledger_open` does not read it.
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("ledger_open")
tallies | map(t => t.count) | sum
