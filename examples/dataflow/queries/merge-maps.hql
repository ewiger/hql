// status: valid-now
// feature: `merge` of two maps keeps its meaning: a later key wins, `reduce({:}, merge)` folds with it, and an annotated reference selects the map form
// implementation: implemented
// environment: pure
// expected-type: List<Data>
// expected: [{a: 3, b: 2}, {a: 2, b: 3}, {a: 1, b: 2}]

maps : List<Data> = [{a: 1}, {b: 2}, {a: 3}]
combine : (Data, Data) -> Data = merge
[maps | reduce({:}, merge), {a: 1} | merge({a: 2, b: 3}), combine({a: 1}, {b: 2})]
