// status: valid-now
// feature: `variance` folds a count, a mean and squared deviations, then projects
// implementation: implemented
// environment: pure
// expected-type: Option<Float>
// expected: 10738.805555555557

// The population variance: the mean squared distance from the mean. Its
// state is not a number of the input's type but a count, a running mean and
// a sum of squared deviations, which two partial states combine pairwise;
// the answer is that sum divided by the count, and it is absent over an
// empty input, as `average` is. The same numbers as total-pages.hql.
pages = [412, 280, 310, 172, 320, 95]

pages | variance
