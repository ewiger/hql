// status: proposed
// feature: The full package: filter, group, map and sort as one pipeline
// implementation: pending
// proposal: doc/proposals/HQL-0004/README.md
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: List<Data>
// expected-json: [{"books":1,"genre":"poetry"},{"books":2,"genre":"essay"},{"books":3,"genre":"novel"}]

// The answer shelf.hql computes with one filter per genre, as one pipeline
// over the metadata: filter (elementwise), group (partition by genre), map
// (elementwise over the groups), sort (barrier). The genre is a Data leaf and
// Data is not Orderable, so the groups are ordered by an explicit comparison
// of their sizes; the shelf is built so that no two genres tie.
cards
| filter(c => c.metadata.category == "book")
| group(by = c => c.metadata.genre)
| sort(by = (a, b) => compare(size(a.items), size(b.items)))
| map(g => { genre: g.key, books: size(g.items) })
