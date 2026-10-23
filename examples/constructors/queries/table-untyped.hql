// status: valid-now
// feature: Table written without its row is Table<Data>
// implementation: implemented
// environment: pure
// expected-type: Table<Data>
// expected-json: [{"pages": 412, "title": "Dune"}]

rows : Table = [{title: "Dune", pages: 412}] | Table
rows
