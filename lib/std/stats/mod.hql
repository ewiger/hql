// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/02-stats.md.

/// Descriptive statistics over a collection of numbers: the mean, the
/// sample and population variance and standard deviation, the median,
/// quantiles, the mode, covariance and correlation. A statistic of `Int` or
/// `Float` values is a `Float`; a statistic of `Decimal` values is a
/// `Decimal`, exact except where a quotient does not terminate, which is
/// rounded half to even to 100 significant digits as `Decimal` `/` is. An
/// `Int` statistic is computed exactly and rounded to a `Float` once. A
/// statistic that is undefined over its input, the mean of nothing or the
/// sample variance of one value, is `none`. Not in the prelude: a program
/// writes `import stats`. The prelude's `variance` is the population
/// variance of `Int` and `Float` values, so a program importing `stats`
/// writes `stats.variance` for the sample variance of those.
module stats

import core

// Reductions: each folds a partial state, which partitions of an input
// combine pairwise, so an executor may split the fold.

/// The arithmetic mean, the sum divided by the count, or `none` over an
/// empty collection: `[1, 2, 3, 4] | mean` is `2.5`.
reduction fn mean(self: Collection<Int>) : Option<Float>
reduction fn mean(self: Collection<Float>) : Option<Float>
reduction fn mean(self: Collection<Decimal>) : Option<Decimal>

/// The sample variance, the sum of squared deviations from the mean divided
/// by `n - 1`, or `none` over fewer than two values:
/// `[1, 2, 3, 4] | stats.variance` is `1.6666666666666667`.
reduction fn variance(self: Collection<Int>) : Option<Float>
reduction fn variance(self: Collection<Float>) : Option<Float>
reduction fn variance(self: Collection<Decimal>) : Option<Decimal>

/// The sample standard deviation, the square root of `variance`, or `none`
/// over fewer than two values. A `Decimal` result is the square root
/// rounded half to even to 100 significant digits.
reduction fn stdev(self: Collection<Int>) : Option<Float>
reduction fn stdev(self: Collection<Float>) : Option<Float>
reduction fn stdev(self: Collection<Decimal>) : Option<Decimal>

/// The population variance, the sum of squared deviations from the mean
/// divided by `n`, or `none` over an empty collection:
/// `[1, 2, 3, 4] | pvariance` is `1.25`.
reduction fn pvariance(self: Collection<Int>) : Option<Float>
reduction fn pvariance(self: Collection<Float>) : Option<Float>
reduction fn pvariance(self: Collection<Decimal>) : Option<Decimal>

/// The population standard deviation, the square root of `pvariance`, or
/// `none` over an empty collection. A `Decimal` result is the square root
/// rounded half to even to 100 significant digits.
reduction fn pstdev(self: Collection<Int>) : Option<Float>
reduction fn pstdev(self: Collection<Float>) : Option<Float>
reduction fn pstdev(self: Collection<Decimal>) : Option<Decimal>

/// The sample covariance of the pairs' `first` and `second` values, the sum
/// of the products of their deviations from their means divided by
/// `n - 1`, or `none` over fewer than two pairs.
reduction fn covariance(self: Collection<Pair<Int, Int>>) : Option<Float>
reduction fn covariance(self: Collection<Pair<Float, Float>>) : Option<Float>
reduction fn covariance(self: Collection<Pair<Decimal, Decimal>>) : Option<Decimal>

/// Pearson's correlation coefficient of the pairs' `first` and `second`
/// values, the covariance divided by the product of the two standard
/// deviations, or `none` over fewer than two pairs or where either side has
/// zero variance.
reduction fn correlation(self: Collection<Pair<Int, Int>>) : Option<Float>
reduction fn correlation(self: Collection<Pair<Float, Float>>) : Option<Float>
reduction fn correlation(self: Collection<Pair<Decimal, Decimal>>) : Option<Float>

// Order statistics: each needs the whole collection, so each is a barrier.

/// The middle value of the collection sorted ascending, the mean of the two
/// middle values where the count is even; `quantile(0.5)`. `none` over an
/// empty collection: `[3, 1, 4, 2] | median` is `2.5`.
fn median(self: Collection<Int>) : Option<Float>
fn median(self: Collection<Float>) : Option<Float>
fn median(self: Collection<Decimal>) : Option<Decimal>

/// The `q` quantile, by linear interpolation between the sorted values
/// (Hyndman and Fan's type 7, the default of R and NumPy): with the values
/// sorted as `s_0` to `s_(n-1)` and `h = (n - 1) * q`, it is
/// `s_floor(h) + (h - floor(h)) * (s_(floor(h) + 1) - s_floor(h))`. `none`
/// over an empty collection; a `q` outside `0.0` to `1.0` is an evaluation
/// error. `q` is a `Float` or a `Decimal`, so a literal is written as is:
/// `prices | quantile(0.95)`. Over `Int` and `Decimal` values the quantile
/// is computed exactly, a `Float` `q` read by its shortest decimal digits.
fn quantile(self: Collection<Int>, q: Float) : Option<Float>
fn quantile(self: Collection<Float>, q: Float) : Option<Float>
fn quantile(self: Collection<Decimal>, q: Float) : Option<Decimal>
fn quantile(self: Collection<Int>, q: Decimal) : Option<Float>
fn quantile(self: Collection<Float>, q: Decimal) : Option<Float>
fn quantile(self: Collection<Decimal>, q: Decimal) : Option<Decimal>

/// The `n - 1` cut points that divide the sorted values into `n` groups of
/// equal probability, the quantiles at `1/n` to `(n-1)/n` by the same
/// interpolation as `quantile`, each computed as
/// `(s_j * (n - d) + s_(j+1) * d) / n` with one division. Empty over an
/// empty collection; an `n` below `2` is an evaluation error:
/// `[1, 2, 3, 4, 5] | quantiles(4)` is `[2.0, 3.0, 4.0]`.
fn quantiles(self: Collection<Int>, n: Int) : List<Float>
fn quantiles(self: Collection<Float>, n: Int) : List<Float>
fn quantiles(self: Collection<Decimal>, n: Int) : List<Decimal>

/// The most frequent value, and among equally frequent values the least by
/// the type's order, or `none` over an empty collection:
/// `[3, 1, 3, 1, 2] | mode` is `1`.
fn mode<T : Orderable>(self: Collection<T>) : Option<T>
