// status: valid-now
// feature: Reading a metadata leaf as an Int, then summing it
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: Int
// expected: 1589

// A page count read out of a header is a `Data` leaf, and `+` is not defined
// on a tree. `somes` drops the cards with no `pages`; `of_type(Int)` keeps the
// leaves whose value is an integer and reads each as the `Int` it holds, the
// structural test a tree gets. After it the sum is ordinary arithmetic, and it
// matches total-pages.hql, which sums the same numbers written as literals.
cards
| map(c => c.metadata.pages)
| somes
| of_type(Int)
| sum
