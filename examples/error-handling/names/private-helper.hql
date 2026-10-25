// status: invalid
// feature: A private declaration is reached by no importer
// implementation: implemented
// environment: pure
// modules: error-handling/lib
// phase: check
// kind: unknown-name
// severity: error

// `shapes.geometry` declares `square` private. The module's own `area` calls
// it, and this program does not reach it, qualified or not. Run with
// `--modules examples/error-handling/lib`; the module-contract session pins
// what is printed.
import shapes.geometry

geometry.square(3)
