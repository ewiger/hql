// status: invalid
// feature: A private receiver function joins no importer's receiver lookup
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// phase: check
// kind: unknown-name
// severity: error

// `words` is private to `mighty.url`, so `url.words("a b")` reaches nothing
// here. `"a b".words()` reaches the prelude's own `string.words`, never the
// private one.
import mighty.url

url.words("a b")
