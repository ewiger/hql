// status: valid-now
// feature: Each format writes a Table with its own encoder, no renderer named
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"csv": "title,pages\r\nDune,412\r\nThe Hobbit,", "json": "[\n  {\n    \"title\": \"Dune\",\n    \"pages\": 412\n  },\n  {\n    \"title\": \"The Hobbit\",\n    \"pages\": null\n  }\n]", "text": "title       pages\n----------  -----\nDune        412\nThe Hobbit"}

// Every format writes the columns in the order `columns` names them. The
// second row holds no `pages`: over `Data`, a missing field is an empty
// cell, and `null` in Json.
t = Table([{title: "Dune", pages: 412}, {title: "The Hobbit"}], columns = ["title", "pages"])
{ csv: t | csv, json: t | json, text: t | text }
