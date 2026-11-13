// status: valid-now
// feature: Architecture decisions by date, projected into records
// implementation: implemented
// environment: knowledge-v1
// vault: todo/wiki
// expected-type: Seq<Data>
// expected-json: [{"date":"2026-10-10","status":"accepted","title":"Event execution model"},{"date":"2026-10-18","status":"accepted","title":"Reduce and merge"},{"date":"2026-10-19","status":"proposed","title":"Stream IO model"}]

// An ISO date in a header is a `String` leaf, and ISO dates sort as text in
// date order. The match reads it as that `String`; `Data` itself is not
// `Orderable`, so the leaf cannot be the sort key as it stands.
cards
| filter(c => c.metadata.type == "adr")
| sort(by = c => match c.metadata.date { some(String d) => d, _ => "" })
| map(c => {
    title: c.title,
    status: c.metadata.status,
    date: c.metadata.date
  })
