// status: valid-now
// feature: Open tasks by priority, the list a notebook cell writes
// implementation: implemented
// environment: knowledge-v1
// vault: todo/wiki
// expected-type: Seq<String>
// expected: ["Fix broken backlinks", "Write HQL documentation", "Publish HyperMarkdown 0.4.2 announcement"]

// A priority read from a header is an `Option<Data>`, and `Data` is not
// `Orderable`, so `sort(by = c => c.metadata.priority)` is refused. The match
// reads the leaf as the `Int` it holds, which is a sort key; the arm for a card
// with no priority, or a priority that is not an integer, puts it first.
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => match c.metadata.priority { some(Int p) => p, _ => 0 })
| map(c => c.title)
