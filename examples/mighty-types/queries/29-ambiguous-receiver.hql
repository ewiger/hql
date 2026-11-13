// status: invalid
// feature: Two imports declaring one receiver function are an ambiguity
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// phase: check
// kind: unknown-name
// severity: error

// `mighty.text` declares `is_blank(self)`, and so does the prelude's
// `string`. Neither is this unit's own, so the call is ambiguous and import
// order does not choose; `text.is_blank("   ")` qualifies it.
import mighty.text

"   ".is_blank()
