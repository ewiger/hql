// status: invalid
// feature: A table renderer writes only a Table
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `pipe` is the Markdown encoder for a `Table`, and a list of records is not
// one: build the table first, `xs | Table | markdown`.
[{title: "Dune"}] | format(Markdown, renderer = "pipe")
