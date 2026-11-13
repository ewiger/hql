// status: valid-now
// feature: two vault endpoints read side by side, each in name order
// implementation: implemented
// environment: knowledge-v1
// vault: endpoints/station
// expected-type: List<Seq<String>>
// expected-json: [["kite","owl","wren"],["finch","heron"]]

// `source("journal")` is the vault `[endpoint.journal]` configures: a
// `Vault`, whose `cards` are its cards in name order. The id is the only
// thing the program names; where the documents are is configuration.
journal = source("journal")
archive = source("archive")
[journal.cards | map(c => c.name), archive.cards | map(c => c.name)]
