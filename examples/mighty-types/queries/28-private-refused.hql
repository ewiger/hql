// status: invalid
// feature: A private receiver function joins no importer's receiver lookup
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// error: NameError
// stage: typecheck

// `words` is private to `mighty.url`, so `"a b".words()` reaches nothing
// here, and `url.words("a b")` is refused the same way.
import mighty.url

"a b".words()
