// status: invalid
// feature: A module lists no two traits that require a function of the same name
// implementation: implemented
// environment: pure
// modules: traits/lib
// phase: check
// kind: type-mismatch
// severity: error

// `shapes.label` lists `Titled` and `Headed`, which both require `title`.
// Its one `title` cannot be two implementations; such traits are
// implemented with `impl`.
import shapes.label

Label(text = "x").title()
