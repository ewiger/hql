// status: valid-now
// feature: a configured CSV source decoded as a declared record
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: Int
// expected: 4

// A `csv` source is a `Seq<Data>`; read where a `Seq<Tally>` is expected,
// each row is decoded as a `Tally`, its cells parsed as the fields declare.
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("counts")
tallies | map(t => t.count) | sum
