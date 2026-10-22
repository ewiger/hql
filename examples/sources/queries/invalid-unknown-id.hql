// status: invalid
// feature: an id no configuration declares is a name error
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// Without a vault no source is configured. Where `[sources.counts]` is
// declared, `count` is within edit distance 2 and the diagnostic suggests it.
source("count")
