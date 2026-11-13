// status: valid-now
// feature: `count_by` counts the occurrences of each key as one reduction
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: Map<Option<Data>, Int>
// expected: Map {"essay": 2, "novel": 3, "poetry": 1}

// The answer by-genre.hql computes with `group` and a `size` per group, as
// one step. `count_by(by)` carries a count per key and combines two partial
// counts by `+` per key, so it is a reduction a backend may split, where
// `group` is a partition followed by a map. Its longhand is a fold that
// raises one key's count per element:
// reduce(initial = Map<K, Int>([], []),
//        by = (m, x) => m | with(by(x), (m | get(by(x)) | unwrap_or(0)) + 1)).
// A genre read from metadata is an optional tree, and the map is keyed by it.
cards
| filter(c => c.metadata.category == "book")
| count_by(c => c.metadata.genre)
