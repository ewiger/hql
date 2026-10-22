// status: valid-now
// implementation: implemented
// environment: pure
// feature: Sets, maps and the operations over them
// expected-type: List<String>
// expected-json: ["[1, 2, 3, 4, 5]","[3, 4]","[1, 2]","2","one,three,two","1,2,3","6","4","4","[3, 1, 2]","[[1, 2], [3, 4], [5, 6]]"]

// `union`, `intersection` and `difference` combine sets; a map is built
// from key and value sequences or with `to_map`, read with `get`, extended
// with `with`, and projected to `keys`, `values()` and `entries()`.
a = Set([1, 2, 3, 4])
b = Set([3, 4, 5])
m = Map(["one", "two", "three"], [1, 2, 3])

[
    union(a, b) | sort,
    str(union(a, b) | sort),
    str(intersection(a, b) | sort),
    str(difference(a, b) | sort),
    str(m.get("two").unwrap_or(0)),
    m.keys | sort | join(","),
    m.values() | sort | map(str) | join(","),
    str(m.entries().map(e => e.second).sum()),
    str(m.with("four", 4).keys | size),
    str(to_map(["apple", "pear", "fig"], w => w, w => w.length()).get("pear").unwrap_or(0)),
    str([3, 1, 3, 2, 1].distinct()),
    str([1, 2, 3, 4, 5, 6] | collect(2)),
]
