// status: valid-now
// feature: A pipeline of stages, run one after another on localhost
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: List<String>
// expected: [Dune, Frankenstein, The Hobbit]

// Three stages. Each `|` is a boundary the executor sees: filter and map are
// elementwise and may be fused or split across partitions; sort is a barrier
// that needs the whole input before it can emit anything. Under localhost
// they simply run in order, each to completion.
cards
| filter(c => c.metadata.genre == "novel")
| sort(by = c => c.name)
| map(c => c.title)
