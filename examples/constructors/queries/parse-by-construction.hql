// status: valid-now
// feature: Text is parsed by constructing Data in a format whose decoder reads Data
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"events": [{"a": 1}, {"a": 2}], "owl": [1, 2], "rows": [{"name": "owl", "wings": "2"}, {"name": "bat", "wings": "2"}]}

// `fn Data<F : Format>(text: String, as: Type<F>) : Data` in `std/core/mod.hql`.
// Json and Yaml give one document; JsonLines and Csv, framed formats, give
// the list of their records.
doc = Data("{\"owl\": [1, 2], \"name\": \"barn\"}", Json)
rows = "name,wings\nowl,2\nbat,2" | Data(Csv)
{ owl: doc.owl, rows: rows, events: "{\"a\":1}\n{\"a\":2}" | Data(JsonLines) }
