// status: invalid
// feature: A function the module does not declare is a name error at the qualified call
// implementation: implemented
// environment: pure
// modules: error-handling/lib
// phase: check
// kind: unknown-name
// severity: error

// `shapes.geometry` declares `area` and nothing named `volume`. The import
// resolves and the qualified call does not.
import shapes.geometry

geometry.volume(3)
