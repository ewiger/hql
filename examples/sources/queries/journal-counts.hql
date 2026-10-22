// status: valid-now
// feature: a vault source joined to a CSV source by a key on each side
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: Seq<Data>
// expected-json: [{"count":"1","title":"Kite overhead"},{"count":"1","title":"Owl at dusk"},{"count":"2","title":"Wren at dawn"}]

// The binary join of HQL-0013: each journal card meets the count rows whose
// `species` equals its header's, and `into` builds one result per pair, in
// the order of the cards, then of the rows.
journal = source("journal")
journal.cards
| join(source("counts"),
       left  = c => c.header.species,
       right = r => r.species,
       into  = (c, r) => {title: c.title, count: r.count})
