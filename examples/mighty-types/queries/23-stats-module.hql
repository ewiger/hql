// status: valid-now
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// feature: A statistics module written in HQL over math, imported by a program
// expected-type: List<Float>
// expected: [5.0, 4.0, 2.0, 4.5]

// `mighty.stats` imports `math` for itself; a program that imports
// `mighty.stats` sees `stats.mean` and not `math`, since a module's
// imports are its own.
import mighty.stats as stats

data = [2.0f, 4.0f, 4.0f, 4.0f, 5.0f, 5.0f, 7.0f, 9.0f]

[stats.mean(data), stats.variance(data), stats.stddev(data), stats.median(data)] | somes
