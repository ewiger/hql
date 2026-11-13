// status: invalid
// feature: A table renderer writes only a Table
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `pipe` is the Markdown encoder for a `Table`, and a list of records is not
// one: build the table first, `xs | Table | markdown`.
[{title: "Dune"}] | format(Markdown, renderer = "pipe")
