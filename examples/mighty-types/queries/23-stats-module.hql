// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// modules: mighty-types/lib
// feature: A statistics module written in HQL over math, imported by a program
// expected-type: List<Float>
// expected: [5.0, 4.0, 2.0, 4.5]

// `mighty.stats` imports `math` for itself; a program that imports
// `mighty.stats` sees `stats.mean` and not `math`, since a module's
// imports are its own.
import mighty.stats as stats

data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]

[stats.mean(data), stats.variance(data), stats.stddev(data), stats.median(data)] | somes
