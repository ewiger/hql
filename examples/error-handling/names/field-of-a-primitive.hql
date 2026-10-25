// status: invalid
// feature: A field read on an Int is a name error inside a lambda, with no element ever visited
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `n` is an `Int`, because `xs` is `List<Int>`, and an `Int` has no field.
// The checker rejects the lambda's body; `map` never runs it over an
// element.
xs : List<Int> = [1, 2]
xs | map(n => n.name)
