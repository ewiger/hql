// status: valid-now
// feature: a default may be a list or a `Data` literal of literals
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [0, 2]

type Post : DataContract = {
    id: String
    tags: List<String> = []
}

xs : List<Data> = [{id: "p-1"}, {id: "p-2", tags: ["hql", "data"]}]
xs | Post | map(p => p.tags.size()) | List
