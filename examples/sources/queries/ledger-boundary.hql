// status: valid-now
// feature: a CSV source read through its boundary: each field of `Tally` reads the column its configuration names
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: List<String>
// expected: ["wren 2", "owl 1", "kite 1"]

// `ledger.csv` heads its columns `Species Name` and `Total`; hql.toml maps
// `species` and `count` to them. `Tally` is an open record, so the
// `Observer` column it does not declare is kept and not read.
type Tally : Data = {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("ledger")
tallies | map(t => fmt("{} {}", t.species, t.count)) | List
