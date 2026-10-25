// status: invalid
// feature: an id no configuration declares is a name error
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// Without a vault no source is configured. Where `[sources.counts]` is
// declared, `count` is within edit distance 2 and the diagnostic suggests it.
source("count")
