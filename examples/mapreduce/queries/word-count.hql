// status: proposed
// feature: Word count, the MapReduce example, as group then map
// implementation: pending
// proposal: doc/proposals/HQL-0004/README.md
// environment: pure
// expected-type: List<Data>
// expected-json: [{"count":3,"word":"owl"},{"count":1,"word":"raven"},{"count":1,"word":"robin"}]

// `group` is the partition stage: every occurrence travels to the group of
// its key, and the groups come back as a Set because partitions finish in no
// promised order. Reducing each group is an ordinary `map` over the groups,
// which is where a backend may run the groups in parallel. Sorting by key is
// what makes the answer positional; String is Orderable, so the key selector
// is enough.
words = ["owl", "raven", "owl", "robin", "owl"]

words
| group(by = w => w)
| sort(by = g => g.key)
| map(g => { word: g.key, count: size(g.items) })
