// status: valid-now
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// feature: A module's contract is its public declarations
// expected-type: List<String>
// expected-json: ["hello-big-world","-"]

// `mighty.url` exports `slug` and `separator`; its private helper `words` is
// used by `slug` inside the module and reached by no importer.
import mighty.url

["Hello  Big World".slug(), url.separator]
