// status: invalid
// feature: from stats import mean does not bind the name stats
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

from stats import mean

stats.median([1, 2, 3])
